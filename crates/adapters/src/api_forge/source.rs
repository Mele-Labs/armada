//! A repository's files over GitHub's API, for a kit source: the tree of its default branch, and
//! one file raw. The reads `gh api` makes in `crate::kit_pieces::ForgeReader`.
//!
//! **Not a checkout's repository**: a source is any repository a person names, so there is no
//! `origin` to route by, and the host is `github.com`.

use serde::Deserialize;

use crate::github_api::{ApiFailure, GitHubApi};

#[derive(Deserialize)]
struct Tree {
    tree: Option<Vec<Option<Entry>>>,
}

#[derive(Deserialize)]
struct Entry {
    #[serde(rename = "type")]
    kind: Option<String>,
    path: Option<String>,
}

/// Every file in the default branch, by path: the blobs of its tree, as `gh`'s filter keeps them.
pub(crate) fn paths(api: &GitHubApi, repository: &str) -> Result<Vec<String>, ApiFailure> {
    let tree: Tree = api.get(&format!("/repos/{}/git/trees/HEAD?recursive=1", escaped(repository)))?.decode("a repository's files")?;
    Ok(tree
        .tree
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .filter(|it| it.kind.as_deref() == Some("blob"))
        .filter_map(|it| it.path)
        .collect())
}

/// One file's bytes, as the forge stores them.
pub(crate) fn file(api: &GitHubApi, repository: &str, path: &str) -> Result<Vec<u8>, ApiFailure> {
    let at = format!("/repos/{}/contents/{}", escaped(repository), escaped(path));
    Ok(api.get_as(&at, "application/vnd.github.raw")?.body)
}

/// A path as one URL path, where `?` and `#` are not the start of a query or a fragment.
fn escaped(path: &str) -> String {
    path.replace('%', "%25").replace('?', "%3F").replace('#', "%23")
}
