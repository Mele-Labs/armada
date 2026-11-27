//! The merge queue's unfinished runs, a commit's failed jobs with why, and stopping a run, over
//! GitHub's API. The same asks `crate::queue_runs` makes of `gh`, classified by the same functions.

use adapter_traits::{InfraFailure, QueueRuns};
use serde::Deserialize;

use super::writes::Nothing;
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::main_ci::is_a_commit;
use crate::queue_runs::{failure_of, run_of_row, MOST_READ};

#[derive(Deserialize)]
struct Runs {
    workflow_runs: Option<Vec<Option<Run>>>,
}

#[derive(Deserialize)]
struct Run {
    id: Option<u64>,
    head_branch: Option<String>,
    head_sha: Option<String>,
    status: Option<String>,
}

pub(super) fn queue_runs(api: &GitHubApi, repository: &GitHubRepository, base: &str) -> Result<Option<QueueRuns>, ApiFailure> {
    let answer = api.get(&format!("{}/actions/runs?event=merge_group&per_page=100", repository.path()))?;
    let found: Runs = answer.decode("the merge queue's runs")?;
    Ok(Some(
        found
            .workflow_runs
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .filter_map(|run| {
                let id = run.id?.to_string();
                run_of_row(base, &id, &run.head_branch.unwrap_or_default(), &run.head_sha.unwrap_or_default(), &run.status.unwrap_or_default())
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct Checks {
    check_runs: Option<Vec<Option<Check>>>,
}

#[derive(Deserialize)]
struct Check {
    id: Option<u64>,
    name: Option<String>,
    conclusion: Option<String>,
    html_url: Option<String>,
    details_url: Option<String>,
}

#[derive(Deserialize)]
struct Note {
    message: Option<String>,
    title: Option<String>,
}

/// The failed jobs of `commit`, each with why. **One whose annotations will not read makes the
/// whole answer silence**, as the `gh` side does.
pub(super) fn infra_failures(
    api: &GitHubApi,
    repository: &GitHubRepository,
    commit: &str,
) -> Result<Option<Vec<InfraFailure>>, ApiFailure> {
    if !is_a_commit(commit) {
        return Ok(None);
    }
    let answer = api.get(&format!("{}/commits/{commit}/check-runs?per_page=100", repository.path()))?;
    let found: Checks = answer.decode("a commit's jobs")?;
    let failed = found
        .check_runs
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .filter(|check| matches!(check.conclusion.as_deref(), Some("failure" | "timed_out" | "startup_failure")))
        .take(MOST_READ);
    let mut out = Vec::new();
    for check in failed {
        let Some(id) = check.id else { continue };
        let notes = api.get(&format!("{}/check-runs/{id}/annotations?per_page=100", repository.path()))?;
        let notes: Vec<Option<Note>> = notes.decode("a job's annotations")?;
        let text = notes
            .into_iter()
            .flatten()
            .flat_map(|note| [note.message, note.title])
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        let url = check.html_url.or(check.details_url).unwrap_or_default();
        out.push(failure_of(check.name.as_deref().unwrap_or_default(), &url, &text));
    }
    Ok(Some(out))
}

/// Stop run `id`.
pub(super) fn cancel(api: &GitHubApi, repository: &GitHubRepository, id: u64) -> Result<Result<(), String>, ApiFailure> {
    api.post(&format!("{}/actions/runs/{id}/cancel", repository.path()), &Nothing {})?;
    Ok(Ok(()))
}
