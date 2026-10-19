//! Which served repository the words of a request are about — the read behind
//! the dispatch panel's repository chip, `read_request_repository`.
//!
//! **Deterministic and cheap, so it can run on a pause in typing**: no model, a
//! stat per candidate path, and one open of each repository's git config. The
//! signals are tiers, strongest first, and **the first tier that names anyone
//! decides**: one name is the answer, two names are no answer. Falling through
//! from a tier that named two to a weaker one would pick between repositories
//! the strongest evidence could not.

use std::path::{Component, Path, PathBuf};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::ForgeAddress;
use ipc::{ManifestId, RequestRepository};

use crate::daemon::Fleet;

/// A name shorter than this is not read as a word: it would match inside
/// ordinary sentences.
const SHORTEST_NAME: usize = 3;

/// The most path-looking tokens stat-ed per repository, so a pasted log costs
/// the same as a sentence.
const MOST_PATHS: usize = 64;

/// What the read knows about one served repository.
pub(crate) struct Candidate {
    pub manifest: String,
    pub root: PathBuf,
    /// The Manifest id and the root directory's name, the words that mean it.
    pub names: Vec<String>,
    pub remote: Option<ForgeAddress>,
    /// How many Sessions the repository has had, which breaks a tie.
    pub sessions: usize,
}

impl Candidate {
    pub(crate) fn of(manifest: &str, root: &str, sessions: usize) -> Candidate {
        let root = PathBuf::from(root);
        let mut names = vec![manifest.to_string()];
        if let Some(directory) = root.file_name().and_then(|name| name.to_str()) {
            if !directory.eq_ignore_ascii_case(manifest) {
                names.push(directory.to_string());
            }
        }
        Candidate {
            manifest: manifest.to_string(),
            root,
            names,
            remote: None,
            sessions,
        }
    }
}

/// The repositories `words` point at under each signal, by position.
struct Hits {
    linked: Vec<bool>,
    named: Vec<bool>,
    pathed: Vec<bool>,
}

pub(crate) fn read(words: &str, candidates: &[Candidate]) -> RequestRepository {
    let hits = Hits {
        linked: linked(words, candidates),
        named: named(words, candidates),
        pathed: pathed(words, candidates),
    };
    let decided = [&hits.linked, &hits.named, &hits.pathed]
        .into_iter()
        .find_map(|tier| {
            let mut named = tier
                .iter()
                .enumerate()
                .filter(|(_, hit)| **hit)
                .map(|(at, _)| at);
            match (named.next(), named.next()) {
                (None, _) => None,
                (Some(only), None) => Some(Some(only)),
                (Some(_), Some(_)) => Some(None),
            }
        })
        .unwrap_or(match candidates.len() {
            1 => Some(0),
            _ => None,
        });
    // Stable, so repositories tied on everything stay in the order they were added.
    let mut ranked: Vec<usize> = (0..candidates.len()).collect();
    ranked.sort_by_key(|&at| {
        (
            std::cmp::Reverse(hits.linked[at]),
            std::cmp::Reverse(hits.named[at]),
            std::cmp::Reverse(hits.pathed[at]),
            std::cmp::Reverse(candidates[at].sessions),
        )
    });
    RequestRepository {
        manifest_id: decided.map(|at| ManifestId::carried(candidates[at].manifest.clone())),
        likely: ranked
            .into_iter()
            .map(|at| ManifestId::carried(candidates[at].manifest.clone()))
            .collect(),
    }
}

/// A ticket or pull request link into the repository the candidate pushes to.
fn linked(words: &str, candidates: &[Candidate]) -> Vec<bool> {
    let links = adapters::linked_in(words);
    candidates
        .iter()
        .map(|one| one.remote.as_ref().is_some_and(|remote| links.contains(remote)))
        .collect()
}

/// A name as a whole word, case-insensitively. **A longer name wins**: with
/// `armada` and `armada-mods` both served, `armada-mods` in the words is the
/// second's, though the first is also there as a word.
fn named(words: &str, candidates: &[Candidate]) -> Vec<bool> {
    // ASCII-lowered, so a byte offset in this is one in `words`.
    let words = words.to_ascii_lowercase();
    let mut spans: Vec<(usize, usize, usize)> = Vec::new();
    for (at, one) in candidates.iter().enumerate() {
        for name in one.names.iter().filter(|name| name.chars().count() >= SHORTEST_NAME) {
            let name = name.to_ascii_lowercase();
            for (start, _) in words.match_indices(&name) {
                let end = start + name.len();
                let before = words[..start].chars().next_back();
                let after = words[end..].chars().next();
                if !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric) {
                    spans.push((at, start, end));
                }
            }
        }
    }
    let mut hit = vec![false; candidates.len()];
    for &(at, start, end) in &spans {
        let swallowed = spans.iter().any(|&(other, from, to)| {
            other != at && from <= start && end <= to && to - from > end - start
        });
        if !swallowed {
            hit[at] = true;
        }
    }
    hit
}

/// An `@path`, or a token with a `/`, that exists in the candidate's tree or
/// lies under its root.
fn pathed(words: &str, candidates: &[Candidate]) -> Vec<bool> {
    let tokens: Vec<&str> = words
        .split_whitespace()
        .filter_map(path_in)
        .take(MOST_PATHS)
        .collect();
    let mut hit = vec![false; candidates.len()];
    for token in tokens {
        let path = Path::new(token);
        if path.is_absolute() {
            // Under the deepest root holding it, so a repository nested in
            // another belongs to the inner one.
            let owner = candidates
                .iter()
                .enumerate()
                .filter(|(_, one)| path.starts_with(&one.root))
                .max_by_key(|(_, one)| one.root.components().count());
            if let Some((at, _)) = owner {
                hit[at] = true;
            }
        } else if path.components().all(|part| matches!(part, Component::Normal(_))) {
            for (at, one) in candidates.iter().enumerate() {
                hit[at] = hit[at] || one.root.join(path).exists();
            }
        }
    }
    hit
}

/// The path a token looks like, trimmed of the punctuation a sentence puts
/// round one: a `:12:3` or `#L12` after it, a leading `./`, a mention's `@`.
fn path_in(token: &str) -> Option<&str> {
    let token = token.trim_matches(|c| "()[]{}<>\"'`,;".contains(c));
    let (mention, token) = match token.strip_prefix('@') {
        Some(rest) => (true, rest),
        None => (false, token),
    };
    let token = token.trim_end_matches(['.', ':', '?', '!']);
    let token = token.split('#').next().unwrap_or_default();
    let token = match token.split_once(':') {
        Some((head, line)) if line.chars().all(|c| c.is_ascii_digit() || c == ':') => head,
        _ => token,
    };
    let token = token.strip_prefix("./").unwrap_or(token);
    if token.is_empty() || token.contains("://") || !(mention || token.contains('/')) {
        return None;
    }
    Some(token)
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `read_request_repository`: every served repository is a candidate, a
    /// Manifest or none, named by the id a Job there carries.
    pub(crate) async fn request_repository(&self, words: String) -> RequestRepository {
        let mut candidates: Vec<Candidate> = {
            let store = self.store().lock().await;
            self.repositories()
                .served()
                .iter()
                .map(|one| {
                    let id = one.manifest().id().as_str();
                    let sessions = store
                        .find_sessions(&store::SessionSearch {
                            manifest_id: Some(id),
                            ..Default::default()
                        })
                        .map_or(0, |found| found.len());
                    Candidate::of(id, one.root(), sessions)
                })
                .collect()
        };
        tokio::task::spawn_blocking(move || {
            for one in &mut candidates {
                one.remote = adapters::remote_of(&one.root);
            }
            read(&words, &candidates)
        })
        .await
        .expect("reading a request's repository panicked")
    }
}
