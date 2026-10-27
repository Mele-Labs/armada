//! `github-checks`: a commit's check runs and commit statuses whose names start with a prefix,
//! each a part. Read with the token `gh` holds, as every other call to GitHub here is
//! (`crate::github_api`), in the repository the checkout's `origin` names.
//!
//! **A failed run says what failed**: its annotations, read for each failed run, else its own
//! output's title and summary, and the end of its output's text. A failed status gives its
//! description.

use std::collections::BTreeMap;
use std::path::Path;

use core_model::{Reading, ReadingState};
use serde::Deserialize;

use super::{evidence, folded, Found};
use crate::github_api::{GitHubApi, GitHubRepository, NotGitHub};

pub(super) const READER: &str = "github-checks";
pub(super) const TAKES: &[&str] = &["sha", "context"];

/// The most pages of check runs read: 3,000, past what any commit carries.
const MOST_PAGES: usize = 30;
/// The most failed runs whose annotations are read, so a commit with hundreds failing stays one reading.
const MOST_ANNOTATED: usize = 10;

#[derive(Deserialize)]
struct CheckRuns {
    #[serde(default)]
    check_runs: Vec<CheckRun>,
}

#[derive(Deserialize)]
pub(crate) struct CheckRun {
    #[serde(default)]
    pub(crate) id: Option<u64>,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) conclusion: Option<String>,
    #[serde(default)]
    pub(crate) html_url: Option<String>,
    #[serde(default)]
    pub(crate) details_url: Option<String>,
    #[serde(default)]
    pub(crate) output: Option<RunOutput>,
}

/// What a run wrote of itself.
#[derive(Default, Deserialize)]
pub(crate) struct RunOutput {
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) summary: Option<String>,
    #[serde(default)]
    pub(crate) text: Option<String>,
}

/// One line a run marked in a file.
#[derive(Deserialize)]
pub(crate) struct Annotation {
    #[serde(default)]
    pub(crate) path: Option<String>,
    #[serde(default)]
    pub(crate) start_line: Option<u64>,
    #[serde(default)]
    pub(crate) annotation_level: Option<String>,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) message: String,
}

#[derive(Deserialize)]
struct Combined {
    #[serde(default)]
    statuses: Vec<Status>,
}

#[derive(Deserialize)]
pub(crate) struct Status {
    pub(crate) context: String,
    pub(crate) state: String,
    #[serde(default)]
    pub(crate) target_url: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
}

pub(super) fn read(sha: &str, context: &str, cwd: &Path) -> Result<Reading, String> {
    let repository = GitHubRepository::of_checkout(cwd).map_err(|why| match why {
        NotGitHub::Elsewhere(address) => format!("{} is not on GitHub.", address.slug()),
        NotGitHub::NoRemote => format!("{} has no origin to read a repository from.", cwd.display()),
    })?;
    let api = GitHubApi::new();
    let said = |why: crate::github_api::ApiFailure| why.to_string();
    let paged = api
        .get_paged(&format!("{}/commits/{sha}/check-runs?per_page=100", repository.path()), MOST_PAGES)
        .map_err(said)?;
    let mut runs = Vec::new();
    for page in &paged.pages {
        let found: CheckRuns = page.decode("a commit's check runs").map_err(said)?;
        runs.extend(found.check_runs);
    }
    let combined: Combined = api
        .get(&format!("{}/commits/{sha}/status?per_page=100", repository.path()))
        .map_err(said)?
        .decode("a commit's statuses")
        .map_err(said)?;
    // A run whose annotations will not read still gives its output, so a refusal here is no broken reading.
    let mut annotations = BTreeMap::new();
    for id in runs
        .iter()
        .filter(|run| run.name.starts_with(context) && run_state(run.status.as_deref(), run.conclusion.as_deref()) == ReadingState::Failed)
        .filter_map(|run| run.id)
        .take(MOST_ANNOTATED)
    {
        let read = api
            .get(&format!("{}/check-runs/{id}/annotations?per_page=100", repository.path()))
            .and_then(|page| page.decode::<Vec<Annotation>>("a check run's annotations"));
        if let Ok(read) = read {
            annotations.insert(id, read);
        }
    }
    Ok(reading(runs, combined.statuses, sha, context, &annotations))
}

/// The exact lines `run` failed on: its failure annotations, else every annotation, else its
/// output's title and summary.
fn error_of(run: &CheckRun, annotations: &[Annotation]) -> Option<String> {
    let failures: Vec<&Annotation> = annotations.iter().filter(|one| one.annotation_level.as_deref() == Some("failure")).collect();
    let marked: Vec<&Annotation> = if failures.is_empty() { annotations.iter().collect() } else { failures };
    if !marked.is_empty() {
        let lines: Vec<String> = marked
            .iter()
            .take(evidence::ERROR_LINES)
            .map(|one| {
                let place = match (&one.path, one.start_line) {
                    (Some(path), Some(line)) => format!("{path}:{line}: "),
                    (Some(path), None) => format!("{path}: "),
                    _ => String::new(),
                };
                let said = match &one.title {
                    Some(title) if !title.trim().is_empty() && !one.message.contains(title.as_str()) => format!("{title}: {}", one.message),
                    _ => one.message.clone(),
                };
                format!("{place}{}", said.trim())
            })
            .collect();
        return evidence::head_of(&lines.join("\n"));
    }
    let output = run.output.as_ref()?;
    let said = [output.title.as_deref(), output.summary.as_deref()].into_iter().flatten().collect::<Vec<_>>().join("\n");
    evidence::head_of(&said)
}

/// What a commit's runs and statuses come to, those starting with `context` only, each failed run
/// with what `annotations` holds for it by id.
pub(crate) fn reading(runs: Vec<CheckRun>, statuses: Vec<Status>, sha: &str, context: &str, annotations: &BTreeMap<u64, Vec<Annotation>>) -> Reading {
    let mut found: Vec<Found> = runs
        .into_iter()
        .filter(|run| run.name.starts_with(context))
        .map(|run| {
            let state = run_state(run.status.as_deref(), run.conclusion.as_deref());
            let failed = state == ReadingState::Failed;
            let marked = run.id.and_then(|id| annotations.get(&id)).map(Vec::as_slice).unwrap_or_default();
            let error = failed.then(|| error_of(&run, marked)).flatten();
            let tail = failed.then(|| run.output.as_ref().and_then(|output| output.text.as_deref()).and_then(evidence::tail_of)).flatten();
            Found { state, name: run.name, link: run.html_url.or(run.details_url), error, tail }
        })
        .collect();
    found.extend(statuses.into_iter().filter(|status| status.context.starts_with(context)).map(|status| {
        let state = match status.state.as_str() {
            "success" => ReadingState::Passed,
            "failure" | "error" => ReadingState::Failed,
            _ => ReadingState::Running,
        };
        let error = status.description.filter(|_| state == ReadingState::Failed).and_then(|said| evidence::head_of(&said));
        Found { state, name: status.context, link: status.target_url, error, tail: None }
    }));
    let short: String = sha.chars().take(7).collect();
    folded(found, &format!("{context} on {short}"), true)
}

fn run_state(status: Option<&str>, conclusion: Option<&str>) -> ReadingState {
    match (status, conclusion) {
        (Some("completed"), Some("success" | "neutral")) => ReadingState::Passed,
        // A skipped run is failed, as `docs/concepts/watch.md` asks of every script: a skipped
        // deploy leaves the old code running.
        (Some("completed"), _) => ReadingState::Failed,
        (Some("in_progress"), _) => ReadingState::Running,
        _ => ReadingState::Waiting,
    }
}
