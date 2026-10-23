//! A kind's rows: each piece on this machine against the home's, by its name, as the same,
//! differing, only here or only there.
//!
//! **One canonical form per kind**, so a row that reads the same reads it on every machine and
//! after every sync. Line endings and trailing blank lines never count. A memory note's own
//! stamps, the session it was written in and when, are the harness's and not the note.

use std::collections::BTreeMap;

use ipc::{HomeKind, HomeRow, HomeRowState};

use super::pieces::Piece;

/// The front-matter keys a harness rewrites on its own and a person never wrote.
const STAMPS: &[&str] = &["originSessionId:", "modified:"];

fn settled(text: &str) -> String {
    let mut text = text.replace("\r\n", "\n").trim_end().to_string();
    text.push('\n');
    text
}

/// A memory note without the stamps in its front matter.
fn unstamped(text: &str) -> String {
    let Some(rest) = text.strip_prefix("---\n") else { return text.to_string() };
    let Some((block, body)) = rest.split_once("\n---") else { return text.to_string() };
    let kept: Vec<&str> = block.lines().filter(|line| !STAMPS.iter().any(|stamp| line.trim_start().starts_with(stamp))).collect();
    format!("---\n{}\n---{body}", kept.join("\n"))
}

/// What is compared of a piece. **Empty for a kind that is only there or not**: an allowlist
/// line is its command, and a plugin is its name.
pub(crate) fn canonical(kind: HomeKind, piece: &Piece) -> BTreeMap<String, String> {
    match kind {
        HomeKind::Allowlist | HomeKind::Plugins => BTreeMap::new(),
        HomeKind::Memory => piece.files.iter().map(|(path, text)| (path.clone(), unstamped(&settled(text)))).collect(),
        _ => piece.files.iter().map(|(path, text)| (path.clone(), settled(text))).collect(),
    }
}

pub(crate) fn same(kind: HomeKind, here: &Piece, home: &Piece) -> bool {
    canonical(kind, here) == canonical(kind, home)
}

/// A piece as the compare sheet shows it: its one file, or each file under its path.
fn shown(piece: &Piece) -> String {
    match piece.files.len() {
        1 => piece.files.values().next().cloned().unwrap_or_default(),
        _ => piece.files.iter().map(|(path, text)| format!("── {path} ──\n{text}")).collect::<Vec<_>>().join("\n"),
    }
}

type Key = (Option<String>, String);

fn keyed(pieces: &[Piece]) -> BTreeMap<Key, &Piece> {
    pieces.iter().map(|piece| ((piece.repository.clone(), piece.name.clone()), piece)).collect()
}

/// Every row of `kind`, by repository and then name.
pub(crate) fn rows(kind: HomeKind, here: &[Piece], home: &[Piece]) -> Vec<HomeRow> {
    let (here, home) = (keyed(here), keyed(home));
    let mut keys: Vec<&Key> = here.keys().chain(home.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .map(|key| {
            let (mine, theirs) = (here.get(key).copied(), home.get(key).copied());
            let state = match (mine, theirs) {
                (Some(mine), Some(theirs)) if same(kind, mine, theirs) => HomeRowState::Same,
                (Some(_), Some(_)) => HomeRowState::Differs,
                (Some(_), None) => HomeRowState::Here,
                _ => HomeRowState::Home,
            };
            let either = mine.or(theirs).expect("a key is on one side at least");
            let differs = state == HomeRowState::Differs;
            HomeRow {
                kind,
                name: key.1.clone(),
                state,
                says: mine.and_then(|one| one.says.clone()).or_else(|| theirs.and_then(|one| one.says.clone())),
                source: either.source.clone(),
                repository: key.0.clone(),
                key_here: (kind == HomeKind::McpServers).then_some(mine.is_some()),
                here: mine.filter(|_| differs).map(shown),
                there: theirs.filter(|_| differs).map(shown),
            }
        })
        .collect()
}
