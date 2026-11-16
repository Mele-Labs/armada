//! The files changed between two commits, over GitHub's API: `compare/{base}...{head}`, the read
//! `gh api` makes for `crate::pull_request_patch::read_compared`. And the files one commit changed,
//! `commits/{sha}`, which lists them under the same `files`.

use adapter_traits::{ComparedPatches, PatchedFile};
use serde::Deserialize;

use super::patches::{listed, Listed};
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};

/// GitHub lists at most 300 files a compare; three pages of 100 hold them all.
const MOST_PAGES: usize = 3;
/// GitHub lists at most 3,000 files a commit, 30 pages of 100.
const MOST_COMMIT_PAGES: usize = 30;

#[derive(Deserialize)]
pub(crate) struct Compared {
    #[serde(default)]
    files: Vec<Listed>,
}

pub(super) fn read(api: &GitHubApi, repository: &GitHubRepository, base: &str, head: &str) -> Result<Option<ComparedPatches>, ApiFailure> {
    let (Some(base), Some(head)) = (commit_named(base), commit_named(head)) else { return Ok(None) };
    let path = format!("{}/compare/{base}...{head}?per_page=100", repository.path());
    let paged = match api.get_paged(&path, MOST_PAGES) {
        Ok(paged) => paged,
        // GitHub says 404 for a commit it does not have, which for a head it just gave is the base.
        Err(ApiFailure::Refused { status: 404, .. }) => return Ok(Some(ComparedPatches::BaseGone)),
        Err(failure) => return Err(failure),
    };
    let mut files = Vec::new();
    for page in &paged.pages {
        let compared: Compared = page.decode("a compare of two commits")?;
        files.extend(listed(compared.files));
    }
    Ok(Some(ComparedPatches::Patches(files)))
}

/// One commit's files against its first parent. `Ok(None)` for a name that is not a commit, and
/// for one the forge does not have, as `gh`'s silence.
pub(super) fn commit(api: &GitHubApi, repository: &GitHubRepository, sha: &str) -> Result<Option<Vec<PatchedFile>>, ApiFailure> {
    let Some(sha) = commit_named(sha) else { return Ok(None) };
    let path = format!("{}/commits/{sha}?per_page=100", repository.path());
    let paged = match api.get_paged(&path, MOST_COMMIT_PAGES) {
        Ok(paged) => paged,
        Err(ApiFailure::Refused { status: 404 | 422, .. }) => return Ok(None),
        Err(failure) => return Err(failure),
    };
    let mut files = Vec::new();
    for page in &paged.pages {
        let one: Compared = page.decode("a commit's files")?;
        files.extend(listed(one.files));
    }
    Ok(Some(files))
}

/// A commit goes into the path, so it is hex or nothing.
pub(crate) fn commit_named(sha: &str) -> Option<&str> {
    let sha = sha.trim();
    (!sha.is_empty() && sha.len() <= 64 && sha.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(sha)
}
