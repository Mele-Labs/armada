//! Which live Jobs a call is shown for an item: Fleet finds them, the call only decides between
//! them. A Job shares something with the item when, in the same repository, a path of one is a
//! path of the other or under it, they name the same issue, or their words share two uncommon
//! ones. **A Job sharing nothing is not shown**, so the call cannot name it, and an item no Job
//! shares anything with is a new Job without a call.

use std::collections::BTreeSet;

use super::arriving::{Arrival, LiveJob};
use super::sources::Filed;
use super::thening::paths_in;

/// Words this short, or this common in a request, say nothing about what the work is.
const SHORTEST_WORD: usize = 5;
const COMMON: &[&str] = &[
    "about", "after", "again", "agent", "before", "being", "build", "change", "could", "every", "fixed", "first", "issue", "should",
    "their", "there", "these", "thing", "think", "those", "where", "which", "while", "would", "write", "armada",
];
/// Shared words that make two Jobs near, where nothing else does.
const WORDS_NEAR: usize = 2;
/// The most shared words a call is shown for one pair.
const WORDS_SHOWN: usize = 5;
/// The most open issues one item is offered as ones it may be.
const ISSUES_SHOWN: usize = 8;
/// The most open issues one merged pull request is asked about.
const CLOSES_SHOWN: usize = 3;

/// One live Job, and what it shares with one item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Near {
    pub(super) job: LiveJob,
    pub(super) paths: Vec<String>,
    pub(super) issues: Vec<String>,
    pub(super) words: Vec<String>,
}

impl Near {
    /// What the two share, as the call reads it.
    pub(super) fn said(&self) -> String {
        let mut parts = Vec::new();
        if !self.paths.is_empty() {
            parts.push(format!("paths {}", self.paths.join(", ")));
        }
        if !self.issues.is_empty() {
            parts.push(format!("issues {}", self.issues.join(", ")));
        }
        if !self.words.is_empty() {
            parts.push(format!("words {}", self.words.join(", ")));
        }
        parts.join("; ")
    }
}

/// The live Jobs other than the item's own that share something with it, in `live`'s order.
pub(super) fn near(item: &Arrival, live: &[LiveJob]) -> Vec<Near> {
    let words = uncommon(&format!("{} {}", item.subject, item.words));
    live.iter()
        .filter(|job| job.session_id != item.session_id && job.manifest_id == item.manifest_id)
        .filter_map(|job| {
            let paths: Vec<String> = shared_paths(&item.paths, &job.paths);
            let issues: Vec<String> = item.issues.iter().filter(|one| job.issues.contains(one)).cloned().collect();
            let theirs = uncommon(&format!("{} {}", job.title, job.words));
            let mut words: Vec<String> = words.intersection(&theirs).cloned().collect();
            if words.len() < WORDS_NEAR {
                words.clear();
            }
            words.truncate(WORDS_SHOWN);
            (!paths.is_empty() || !issues.is_empty() || !words.is_empty()).then(|| Near { job: job.clone(), paths, issues, words })
        })
        .collect()
}

/// The open issues whose titles share two uncommon words with `text`, those sharing most first.
/// **An issue sharing fewer is not offered**, so a link can name only one Fleet found near.
pub(super) fn open_near(text: &str, open: &[Filed]) -> Vec<Filed> {
    let words = uncommon(text);
    let mut near: Vec<(usize, &Filed)> = open
        .iter()
        .map(|one| (uncommon(&one.title).intersection(&words).count(), one))
        .filter(|(shared, _)| *shared >= WORDS_NEAR)
        .collect();
    near.sort_by(|one, other| other.0.cmp(&one.0));
    near.into_iter().take(ISSUES_SHOWN).map(|(_, one)| one.clone()).collect()
}

/// What a merged pull request did, as far as Fleet holds it.
pub(super) struct Finished<'a> {
    pub(super) title: &'a str,
    /// The words of the Job that held it: its plan, or its first message.
    pub(super) words: &'a str,
    /// The paths its plan names and it wrote.
    pub(super) paths: &'a [String],
    /// The issues it or its Job named, which are never asked about.
    pub(super) named: &'a [String],
}

/// The open issues a merged pull request may have finished without naming them, from `open` as each
/// issue and the start of its body. One counts when its words name a path the work wrote, or share two
/// uncommon words with the pull request's title and its Job's words. **An issue the work named is never
/// offered**, and at most [`CLOSES_SHOWN`] are, those sharing most first. None found means no call.
pub(super) fn done_near(done: &Finished<'_>, open: &[(Filed, String)]) -> Vec<Filed> {
    let words = uncommon(&format!("{} {}", done.title, done.words));
    let mut near: Vec<(usize, &Filed)> = open
        .iter()
        .filter(|(one, _)| !done.named.contains(&one.reference))
        .filter_map(|(one, excerpt)| {
            let said = format!("{} {excerpt}", one.title);
            let paths = shared_paths(done.paths, &paths_in(&said)).len();
            let shared = uncommon(&said).intersection(&words).count();
            let words = if shared >= WORDS_NEAR { shared } else { 0 };
            (paths + words > 0).then_some((paths * WORDS_NEAR + words, one))
        })
        .collect();
    near.sort_by(|one, other| other.0.cmp(&one.0));
    near.into_iter().take(CLOSES_SHOWN).map(|(_, one)| one.clone()).collect()
}

/// Each of `ours` that meets one of `theirs`, the more exact of the two named.
pub(super) fn shared_paths(ours: &[String], theirs: &[String]) -> Vec<String> {
    let mut shared = BTreeSet::new();
    for one in ours {
        for other in theirs {
            if let Some(met) = met(one, other) {
                shared.insert(met.to_string());
            }
        }
    }
    shared.into_iter().collect()
}

/// Where two paths meet: the same file, a file under a folder the other names, or under a pattern's
/// fixed start. The longer of the two, which names the place more exactly.
fn met<'a>(one: &'a str, other: &'a str) -> Option<&'a str> {
    let (a, b) = (fixed(one), fixed(other));
    if a.0.is_empty() || b.0.is_empty() {
        return None;
    }
    let under = |path: (&str, bool), place: (&str, bool)| {
        path.0 == place.0
            || (place.1 && path.0.starts_with(place.0))
            || (path.0.starts_with(place.0) && path.0[place.0.len()..].starts_with('/'))
    };
    if under(a, b) {
        Some(one)
    } else if under(b, a) {
        Some(other)
    } else {
        None
    }
}

/// A path up to its first pattern character, without a leading `./` or a trailing `/`, and
/// whether it was a pattern.
fn fixed(path: &str) -> (&str, bool) {
    let path = path.trim().trim_start_matches("./");
    match path.find(['*', '?', '[', '{']) {
        Some(at) => (&path[..at], true),
        None => (path.trim_end_matches('/'), false),
    }
}

/// The uncommon words of `text`, lowercased, with a plural's `s` dropped.
fn uncommon(text: &str) -> BTreeSet<String> {
    text.split(|one: char| !one.is_alphanumeric())
        .filter(|word| word.chars().count() >= SHORTEST_WORD)
        .map(|word| {
            let word = word.to_lowercase();
            match word.strip_suffix('s') {
                Some(single) if single.chars().count() >= SHORTEST_WORD && !single.ends_with('s') => single.to_string(),
                _ => word,
            }
        })
        .filter(|word| !COMMON.contains(&word.as_str()) && !word.chars().all(|one| one.is_ascii_digit()))
        .collect()
}
