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

    /// `owner/name`, lowercased: the same on every machine whichever way it cloned.
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }

    /// The forge's host, lowercased, without a user or a port.
    pub fn host(&self) -> &str {
        &self.host
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
    for (address, _, _) in tickets_in(words) {
        if !found.contains(&address) {
            found.push(address);
        }
    }
    found
}

/// The issues a link in `words` names in `repository`, by number, ascending and each once. A link
/// into any other repository names none of its issues here, and a bare `#123` is not a link.
pub fn issues_linked_in(words: &str, repository: &ForgeAddress) -> Vec<u64> {
    let mut numbers: Vec<u64> = tickets_in(words)
        .filter(|(address, marker, _)| address == repository && *marker == "issues")
        .filter_map(|(_, _, number)| number.parse().ok())
        .collect();
    numbers.sort_unstable();
    numbers.dedup();
    numbers
}

/// The number of the pull request that `link`, an address and nothing else, names in `repository`.
/// A link into another repository, or to an issue, names none.
pub fn pull_linked_in(link: &str, repository: &ForgeAddress) -> Option<u64> {
    let link = link.trim();
    let (address, marker, number) = tickets_in(link).next()?;
    (address == *repository && marker == "pull" && !link.contains(char::is_whitespace))
        .then(|| number.parse().ok())?
}

/// The web links in `words`, each once and in the order written: `http` and `https` only, without
/// the punctuation that ended a sentence. A link to this machine, or to one of `repository`'s own
/// issues or pull requests (the Job already holds those), is not kept; a checkout with no forge
/// has no such tickets to drop.
///
/// Splits on `://` as `tickets_in` does, so `[text](url)` yields the url.
pub fn links_in(words: &str, repository: Option<&ForgeAddress>) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for (at, _) in words.match_indices("://") {
        let scheme = words[..at].rsplit(|c: char| !c.is_ascii_alphabetic()).next().unwrap_or_default();
        if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
            continue;
        }
        let rest = words[at + 3..]
            .split(|c: char| c.is_whitespace() || "()[]<>\"'`".contains(c))
            .next()
            .unwrap_or_default()
            .trim_end_matches(['.', ',', ';', ':', '!', '?']);
        let link = format!("{}://{rest}", scheme.to_ascii_lowercase());
        if rest.is_empty() || is_this_machine(rest) || tickets_in(&link).any(|(address, _, _)| repository == Some(&address)) {
            continue;
        }
        if !found.contains(&link) {
            found.push(link);
        }
    }
    found
}

/// Whether the address after `://` is `localhost`, a loopback or the unspecified address.
fn is_this_machine(rest: &str) -> bool {
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit('@').next().unwrap_or(authority).to_ascii_lowercase();
    let host = match host.strip_prefix('[') {
        Some(inside) => inside.split(']').next().unwrap_or_default().to_string(),
        None => host.split(':').next().unwrap_or_default().to_string(),
    };
    host == "localhost" || host.ends_with(".localhost") || matches!(host.as_str(), "127.0.0.1" | "::1" | "0.0.0.0")
}

/// Each issue or pull request link in `words`: the repository, `issues` or `pull`, and the number
/// as written, in the order they appear.
fn tickets_in(words: &str) -> impl Iterator<Item = (ForgeAddress, &str, &str)> {
    words.match_indices("://").filter_map(|(at, _)| {
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
            return None;
        };
        let host = host.rsplit('@').next().unwrap_or(host);
        let host = host.split(':').next().unwrap_or(host);
        if !matches!(marker, "issues" | "pull")
            || number.is_empty()
            || !number.chars().all(|c| c.is_ascii_digit())
        {
            return None;
        }
        ForgeAddress::of(host, owner, repo).map(|address| (address, marker, number))
    })
}
