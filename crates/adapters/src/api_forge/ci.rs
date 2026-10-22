//! What the forge says about a base branch over GitHub's API: where it stands, what CI ran on a
//! commit, a failed job's log, and the pull request that merged a commit. The same asks
//! `crate::main_ci` makes of `gh`, folded by the same functions.
//!
//! **A log is two requests**: GitHub answers the job's log with a redirect to a short-lived
//! address on its storage. The first carries the token; the second carries none, and is read as
//! it arrives, so only the last [`LOG_TAIL`] bytes are ever held.

use adapter_traits::{CiRun, CiRuns, FromOutside, MergedPull};
use serde::Deserialize;

use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};
use crate::main_ci::{
    folded_rows, is_a_commit, named_in, queue_branch_prefix, tail, without_timestamps, RunRow, LOG_TAIL,
};

/// The most pages of a commit's jobs read: 3,000 jobs, past what any workflow runs.
const MOST_RUN_PAGES: usize = 30;

#[derive(Deserialize)]
struct Ref {
    object: Option<Object>,
}

#[derive(Deserialize)]
struct Object {
    sha: Option<String>,
}

/// The commit `base` is at: the ref and nothing dearer.
pub(super) fn head(api: &GitHubApi, repository: &GitHubRepository, base: &str) -> Result<Option<String>, ApiFailure> {
    let answer = api.get(&format!("{}/git/ref/heads/{base}", repository.path()))?;
    let found: Ref = answer.decode("a branch's ref")?;
    Ok(found.object.and_then(|it| it.sha).filter(|sha| is_a_commit(sha.trim())))
}

#[derive(Deserialize)]
struct CheckRuns {
    check_runs: Option<Vec<Option<CheckRun>>>,
}

#[derive(Deserialize)]
pub(crate) struct CheckRun {
    id: Option<u64>,
    name: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
    html_url: Option<String>,
    details_url: Option<String>,
}

/// Every job on `commit`, every page of them.
pub(super) fn runs(api: &GitHubApi, repository: &GitHubRepository, commit: &str) -> Result<Option<CiRuns>, ApiFailure> {
    if !is_a_commit(commit) {
        return Ok(None);
    }
    let paged = api.get_paged(&format!("{}/commits/{commit}/check-runs?per_page=100", repository.path()), MOST_RUN_PAGES)?;
    let mut rows = Vec::new();
    for page in &paged.pages {
        let found: CheckRuns = page.decode("a commit's jobs")?;
        rows.extend(rows_of(found.check_runs.unwrap_or_default().into_iter().flatten()));
    }
    Ok(Some(folded_rows(rows)))
}

/// The rows `gh`'s reduction prints: the job's page, else the one it names for its details.
pub(crate) fn rows_of(runs: impl IntoIterator<Item = CheckRun>) -> Vec<RunRow> {
    runs.into_iter()
        .map(|run| RunRow {
            id: run.id.map(|it| it.to_string()).unwrap_or_default(),
            name: run.name.unwrap_or_default(),
            status: run.status.unwrap_or_default(),
            conclusion: run.conclusion.unwrap_or_default(),
            url: run.html_url.or(run.details_url).unwrap_or_default(),
        })
        .collect()
}

#[derive(Deserialize)]
struct WorkflowRuns {
    workflow_runs: Option<Vec<Option<WorkflowRun>>>,
}

#[derive(Deserialize)]
struct WorkflowRun {
    head_branch: Option<String>,
    head_sha: Option<String>,
}

/// The commit the merge queue last tested pull request `number` on: the newest run's, as `gh`
/// reads the newest hundred.
pub(super) fn queue_commit_of(
    api: &GitHubApi,
    repository: &GitHubRepository,
    base: &str,
    number: u64,
) -> Result<Option<String>, ApiFailure> {
    let answer = api.get(&format!("{}/actions/runs?event=merge_group&per_page=100", repository.path()))?;
    let found: WorkflowRuns = answer.decode("the merge queue's runs")?;
    let prefix = queue_branch_prefix(base, number);
    Ok(found
        .workflow_runs
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .filter(|run| run.head_branch.as_deref().is_some_and(|branch| branch.starts_with(&prefix)))
        .filter_map(|run| run.head_sha)
        .find(|sha| is_a_commit(sha.trim()))
        .map(|sha| sha.trim().to_string()))
}

/// The end of a job's log, its timestamps taken off. `Ok(None)` for a handle that is no job id.
pub(super) fn log(api: &GitHubApi, repository: &GitHubRepository, run: &CiRun) -> Result<Option<FromOutside>, ApiFailure> {
    let id = run.handle.as_written();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Ok(None);
    }
    let stored = api.get_redirect_target(&format!("{}/actions/jobs/{id}/logs", repository.path()))?;
    let kept = tail(api.download(&stored)?, LOG_TAIL);
    Ok(Some(FromOutside::verbatim(without_timestamps(&String::from_utf8_lossy(&kept)))))
}

#[derive(Deserialize)]
struct PullOf {
    number: Option<u64>,
    html_url: Option<String>,
    merge_commit_sha: Option<String>,
    head: Option<Head>,
}

#[derive(Deserialize)]
struct Head {
    #[serde(rename = "ref")]
    branch: Option<String>,
}

#[derive(Deserialize)]
struct CommitOf {
    commit: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    message: Option<String>,
}

/// The pull request that put `commit` on the base: the forge's own mapping where it names one
/// whose merge commit this is, otherwise what a merge commit's subject says. A mapping the forge
/// will not answer falls to the subject, as `gh`'s does.
pub(super) fn merged_by(
    api: &GitHubApi,
    repository: &GitHubRepository,
    commit: &str,
) -> Result<Option<MergedPull>, ApiFailure> {
    if !is_a_commit(commit) {
        return Ok(None);
    }
    let mapped = api
        .get(&format!("{}/commits/{commit}/pulls", repository.path()))
        .and_then(|answer| answer.decode::<Vec<Option<PullOf>>>("the pull requests of a commit"));
    if let Ok(pulls) = mapped {
        if let Some(pull) = pulls.into_iter().flatten().find_map(|pull| mapped_pull(pull, commit)) {
            return Ok(Some(pull));
        }
    }
    let answer = api.get(&format!("{}/commits/{commit}", repository.path()))?;
    let found: CommitOf = answer.decode("a commit")?;
    let message = found.commit.and_then(|it| it.message).unwrap_or_default();
    Ok(named_in(message.lines().next().unwrap_or_default().trim()))
}

fn mapped_pull(pull: PullOf, commit: &str) -> Option<MergedPull> {
    (pull.merge_commit_sha.as_deref() == Some(commit)).then_some(())?;
    let non_empty = |text: Option<String>| text.filter(|it| !it.is_empty()).map(FromOutside::verbatim);
    Some(MergedPull {
        number: pull.number?,
        url: non_empty(pull.html_url),
        branch: non_empty(pull.head.and_then(|it| it.branch)),
    })
}
