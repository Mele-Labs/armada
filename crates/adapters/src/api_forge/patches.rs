//! A pull request's patches over GitHub's API: `pulls/N/files`, the read `gh api` makes for
//! `crate::pull_request_patch`.

use adapter_traits::PatchedFile;
use serde::Deserialize;

use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};

/// GitHub lists at most 3,000 files a pull request, 30 pages of 100, so no list is cut short
/// of what the forge would give `gh`.
const MOST_PAGES: usize = 30;

/// One file; `patch` is absent where the forge will not show one (binary, or too large).
#[derive(Deserialize)]
pub(crate) struct Listed {
    filename: Option<String>,
    patch: Option<String>,
}

pub(super) fn read(api: &GitHubApi, repository: &GitHubRepository, number: u64) -> Result<Vec<PatchedFile>, ApiFailure> {
    let path = format!("{}/pulls/{number}/files?per_page=100", repository.path());
    let paged = api.get_paged(&path, MOST_PAGES)?;
    let mut files = Vec::new();
    for page in &paged.pages {
        files.extend(listed(page.decode("a pull request's files")?));
    }
    Ok(files)
}

/// The files in the forge's order, as `crate::pull_request_patch::listed` makes them.
pub(crate) fn listed(files: Vec<Listed>) -> Vec<PatchedFile> {
    files
        .into_iter()
        .map(|file| PatchedFile { path: file.filename.unwrap_or_default(), patch: file.patch })
        .collect()
}
