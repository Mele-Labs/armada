//! Where a repository lives on its forge, and which forge repositories a
//! request's links name — so Fleet can say which served repository a request
//! is about without a model.
//!
//! **Host-agnostic on purpose.** Both sides are reduced to `host/owner/repo`
//! and compared, so no forge is named here and a self-hosted one matches as
//! well as any other. The path markers a ticket or pull request link carries
//! are the only forge-shaped fact this reads.

use std::path::Path;

use git2::Repository;

/// A repository on a forge, lowercased: forge names are not case-sensitive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForgeAddress {
    host: String,
    owner: String,
    repo: String,
}

impl ForgeAddress {
    fn of(host: &str, owner: &str, repo: &str) -> Option<ForgeAddress> {
        let repo = repo.strip_suffix(".git").unwrap_or(repo);
        if host.is_empty() || owner.is_empty() || repo.is_empty() {
            return None;
        }
        Some(ForgeAddress {
            host: host.to_ascii_lowercase(),
            owner: owner.to_ascii_lowercase(),
            repo: repo.to_ascii_lowercase(),
        })
    }

    /// A remote's URL in any of its three spellings: `https://host/o/r.git`,
    /// `ssh://git@host:22/o/r.git` and `git@host:o/r.git`. A path on this
    /// machine is no forge and reads as `None`.
    pub fn of_remote(url: &str) -> Option<ForgeAddress> {
        let url = url.trim();
        let (host, path) = match url.split_once("://") {
            Some((_, rest)) => rest.split_once('/')?,
            None => url.split_once(':')?,
        };
        let host = host.rsplit('@').next()?;
        let host = host.split(':').next()?;
        let mut segments = path.split('/').filter(|one| !one.is_empty());
        ForgeAddress::of(host, segments.next()?, segments.next()?)
    }
}

/// The repository a checkout pushes to: `origin`, or the first remote it has.
pub fn remote_of(root: &Path) -> Option<ForgeAddress> {
    let repository = Repository::open(root).ok()?;
    let remote = repository.find_remote("origin").ok().or_else(|| {
        let names = repository.remotes().ok()?;
        repository.find_remote(names.iter().flatten().next()?).ok()
    })?;
    ForgeAddress::of_remote(remote.url()?)
}

/// Every repository an issue or pull request link in `words` points into.
///
/// Scans for `://` and reads `host/owner/repo/(issues|pull)/<number>` after
/// it, rather than parsing URLs, for `issue_lookup`'s reason.
pub fn linked_in(words: &str) -> Vec<ForgeAddress> {
    let mut found: Vec<ForgeAddress> = Vec::new();
    for (at, _) in words.match_indices("://") {
        let rest = &words[at + 3..];
        let link = rest
            .split(|c: char| c.is_whitespace() || "()[]<>\"'`".contains(c))
            .next()
            .unwrap_or_default();
        let link = link.split(['?', '#']).next().unwrap_or_default();
        let mut segments = link.split('/');
        let (Some(host), Some(owner), Some(repo), Some(marker), Some(number)) = (
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
        ) else {
            continue;
        };
        let host = host.rsplit('@').next().unwrap_or(host);
        let host = host.split(':').next().unwrap_or(host);
        if !matches!(marker, "issues" | "pull")
            || number.is_empty()
            || !number.chars().all(|c| c.is_ascii_digit())
        {
            continue;
        }
        if let Some(address) = ForgeAddress::of(host, owner, repo) {
            if !found.contains(&address) {
                found.push(address);
            }
        }
    }
    found
}
