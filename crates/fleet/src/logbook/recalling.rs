//! The two ways the Logbook is read mid-task: the entries a subagent is handed for the paths its
//! prompt names, and a search of the whole of it. `docs/concepts/logbook.md`, *Recall*.
//!
//! **Plain code.** A path glob is matched here, and a search is words in text, with no model.

use store::LogSection;

use super::serving::{served, Logged};

/// What a subagent is handed, in tokens, header included.
pub(crate) const SUBAGENT_TOKENS: usize = 800;

/// The most entries a search answers.
pub(crate) const MOST_RECALLED: usize = 20;

/// The paths a prompt names, repository-relative: each word with a `/` in it, cut of the
/// punctuation round it, of a `:line` after it, and of the repository's root or a slot's folder in
/// front. A link is no path.
pub(crate) fn named_paths(prompt: &str, root: &str) -> Vec<String> {
    let root = root.trim_end_matches('/');
    let mut found: Vec<String> = Vec::new();
    for word in prompt.split(|c: char| c.is_whitespace() || matches!(c, '`' | '"' | '\'' | '(' | ')' | '[' | ']' | '<' | '>' | ',')) {
        let word = word.trim_end_matches(['.', ':', ';']);
        let word = match word.rsplit_once(':') {
            Some((head, line)) if !line.is_empty() && line.bytes().all(|b| b.is_ascii_digit()) => head,
            _ => word,
        };
        if !word.contains('/') || word.contains("://") {
            continue;
        }
        let word = match word.split_once("/.armada/slots/slot-") {
            Some((_, rest)) => rest.split_once('/').map_or("", |(_, inside)| inside),
            None => word.strip_prefix(root).filter(|_| !root.is_empty()).unwrap_or(word),
        };
        let word = word.trim_start_matches("./").trim_start_matches('/');
        if !word.is_empty() && !found.iter().any(|one| one == word) {
            found.push(word.to_string());
        }
    }
    found
}

/// Whether `glob` matches `path`: `*` stands for any run of a name, `?` for one character of it,
/// `**` for any run of folders, and a folder named matches the globs of what is under it.
pub(crate) fn matches(glob: &str, path: &str) -> bool {
    let glob = glob.trim_start_matches("./").trim_start_matches('/');
    wild(glob.as_bytes(), path.as_bytes()) || wild(glob.as_bytes(), format!("{}/", path.trim_end_matches('/')).as_bytes())
}

fn wild(glob: &[u8], text: &[u8]) -> bool {
    match glob {
        [] => text.is_empty(),
        [b'*', b'*', b'/', rest @ ..] => {
            (0..=text.len()).any(|at| (at == 0 || text[at - 1] == b'/') && wild(rest, &text[at..]))
        }
        [b'*', b'*', rest @ ..] => (0..=text.len()).any(|at| wild(rest, &text[at..])),
        [b'*', rest @ ..] => {
            let name = text.iter().position(|c| *c == b'/').unwrap_or(text.len());
            (0..=name).any(|at| wild(rest, &text[at..]))
        }
        [b'?', rest @ ..] => text.first().is_some_and(|c| *c != b'/') && wild(rest, &text[1..]),
        [c, rest @ ..] => text.first() == Some(c) && wild(rest, &text[1..]),
    }
}

/// What a subagent is handed: standing entries only, the habits, then the repository's entries
/// whose paths match one `named`, or, where it names none, the repository's best-ranked, cut to
/// `budget` tokens in the order the start note serves.
pub(crate) fn for_subagent(all: Vec<Logged>, named: &[String], budget: usize) -> Vec<Logged> {
    let chosen = all
        .into_iter()
        .filter(|one| match one.entry.section {
            LogSection::Habits => true,
            LogSection::Repo => named.is_empty() || one.entry.paths.iter().any(|glob| named.iter().any(|path| matches(glob, path))),
            LogSection::Machine => false,
        })
        .collect();
    served(chosen, budget)
}

/// How many of the query's words an entry's text holds, in either case.
fn held(query: &[String], one: &Logged) -> usize {
    let text = one.entry.text.to_lowercase();
    query.iter().filter(|word| text.contains(word.as_str())).count()
}

/// The entries whose text holds any word of `query`, those holding more first, then those more
/// Sessions bore out, then the most recently confirmed, at most [`MOST_RECALLED`]. Faded and
/// candidate entries are in it. `section` narrows it.
pub(crate) fn searched(all: Vec<Logged>, query: &str, section: Option<LogSection>) -> Vec<Logged> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut hits: Vec<(usize, Logged)> = all
        .into_iter()
        .filter(|one| section.is_none_or(|section| one.entry.section == section))
        .map(|one| (held(&words, &one), one))
        .filter(|(count, _)| *count > 0)
        .collect();
    hits.sort_by_key(|(count, one)| std::cmp::Reverse((*count, one.borne_out(), one.entry.last_confirmed)));
    hits.into_iter().map(|(_, one)| one).take(MOST_RECALLED).collect()
}
