//! `github-checks`: a commit's check runs and commit statuses whose names start with a prefix,
//! each a part. Read with the token `gh` holds, as every other call to GitHub here is
//! (`crate::github_api`), in the repository the checkout's `origin` names.

use std::path::Path;

use core_model::{Reading, ReadingState};
use serde::Deserialize;

use super::{folded, Found};
use crate::github_api::{GitHubApi, GitHubRepository, NotGitHub};

pub(super) const READER: &str = "github-checks";
pub(super) const TAKES: &[&str] = &["sha", "context"];

/// The most pages of check runs read: 3,000, past what any commit carries.
const MOST_PAGES: usize = 30;

#[derive(Deserialize)]
struct CheckRuns {
    #[serde(default)]
    check_runs: Vec<CheckRun>,
}

#[derive(Deserialize)]
pub(crate) struct CheckRun {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) conclusion: Option<String>,
    #[serde(default)]
    pub(crate) html_url: Option<String>,
    #[serde(default)]
    pub(crate) details_url: Option<String>,
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
    Ok(reading(runs, combined.statuses, sha, context))
}

/// What a commit's runs and statuses come to, those starting with `context` only.
pub(crate) fn reading(runs: Vec<CheckRun>, statuses: Vec<Status>, sha: &str, context: &str) -> Reading {
    let mut found: Vec<Found> = runs
        .into_iter()
        .filter(|run| run.name.starts_with(context))
        .map(|run| Found {
            state: run_state(run.status.as_deref(), run.conclusion.as_deref()),
            name: run.name,
            link: run.html_url.or(run.details_url),
        })
        .collect();
    found.extend(statuses.into_iter().filter(|status| status.context.starts_with(context)).map(|status| Found {
        state: match status.state.as_str() {
            "success" => ReadingState::Passed,
            "failure" | "error" => ReadingState::Failed,
            _ => ReadingState::Running,
        },
        name: status.context,
        link: status.target_url,
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
