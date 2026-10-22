//! A pull request's diff over GitHub's API: `pulls/N` asked for as a diff, the read `gh pr diff`
//! makes. GitHub refuses a diff past its limits (300 files, 20,000 lines) with a 406, which is
//! the forge's silence, as it is for `gh`.

use adapter_traits::PullRequestDiff;

use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};

const DIFF: &str = "application/vnd.github.v3.diff";

pub(super) fn read(
    api: &GitHubApi,
    repository: &GitHubRepository,
    number: u64,
) -> Result<Option<PullRequestDiff>, ApiFailure> {
    let answer = api.get_as(&format!("{}/pulls/{number}", repository.path()), DIFF)?;
    let patch = String::from_utf8_lossy(&answer.body).into_owned();
    Ok(Some(PullRequestDiff { files: files_of(&patch), patch }))
}

/// The paths a diff names, as `gh pr diff --name-only` lists them: the text after the last
/// ` b/` on each `diff --git` line.
pub(crate) fn files_of(patch: &str) -> Vec<String> {
    patch
        .lines()
        .filter(|line| line.starts_with("diff --git"))
        .filter_map(|line| line.rsplit_once(" b/").map(|(_, path)| path.trim().to_string()))
        .filter(|path| !path.is_empty())
        .collect()
}
