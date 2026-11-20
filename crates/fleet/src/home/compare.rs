//! What of a piece is compared between two Macs, and how one is shown beside the other.
//!
//! **One canonical form per kind**, so a row that reads the same reads it on every Mac and
//! after every crossing. Line endings and trailing blank lines never count. A memory note's own
//! stamps, the session it was written in and when, are the harness's and not the note.

use std::collections::BTreeMap;

use ipc::{HomeKind, ModKind};

use super::pieces::Piece;
use crate::mods::{manifest, MANIFEST};

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

/// A piece as the compare sheet shows it: its one file, or each file under its path.
pub(crate) fn shown(piece: &Piece) -> String {
    match piece.files.len() {
        1 => piece.files.values().next().cloned().unwrap_or_default(),
        _ => piece.files.iter().map(|(path, text)| format!("── {path} ──\n{text}")).collect::<Vec<_>>().join("\n"),
    }
}

/// The kind a mod's `mod.toml` names, where it names one this build knows.
pub(crate) fn mod_kind_of(piece: &Piece) -> Option<ModKind> {
    piece.files.get(&format!("{}/{MANIFEST}", piece.name)).and_then(|text| manifest::read(text, &piece.name).0.kind)
}
