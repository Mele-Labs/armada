//! The guide catalogue's half of the gate: every guide's piece is drawn.
//!
//! Guide 8 explained a bar on the run tree, and the Overview reframe of 29
//! September 2026 took the tree off every screen. The guide stayed in the
//! catalogue, openable from Guides, with nothing on a Job to open it — and every
//! other gate passed, because none of them reads a screen against a guide.
//!
//! **No TS parser, and the gate keeps no dependencies** — so this reads the one
//! shape a guide file has: `export const GUIDE_X: Guide = {` and a quoted
//! `piece:`. A guide is drawn when some source outside the guides directory
//! names its const, which is how a `GuideMark guide={GUIDE_X}` reaches it.

use std::fs;
use std::path::Path;

use crate::{files_with_ext, Report};

const GUIDES: &str = "packages/components/src/guides";

/// Where a screen could draw a mark.
const SEARCHED: &[&str] = &["packages", "apps"];

/// The pieces reached from the catalogue and nowhere else — decided by the
/// owner, 26 September 2026, in `guide.ts`. **Do not grow this list to make a
/// failure pass**: a guide with no mark is the thing the rule is there for.
const CATALOGUE_ONLY: &[&str] = &[
    "job.what",
    "job.before",
    "plan.what",
    "run.drone",
    "workflow.what",
];

/// Every guide's piece is drawn on some screen, or is one of the five the
/// catalogue alone reaches.
///
/// A mention in a story or a test does not count: a story mounts a component
/// on its own, and a person on a Job never meets it.
pub fn every_guides_piece_is_drawn(root: &Path) -> Report {
    let mut report = Report::new("every guide's piece is drawn on a screen");

    let dir = root.join(GUIDES);
    if !dir.is_dir() {
        report.fail(format!("{GUIDES} — the guide catalogue"));
        return report;
    }

    let mut sources = Vec::new();
    for top in SEARCHED {
        for path in files_with_ext(root, &root.join(top), &["ts", "tsx"]) {
            if counts_as_a_screen(&path) {
                if let Ok(text) = fs::read_to_string(root.join(&path)) {
                    sources.push(text);
                }
            }
        }
    }

    for path in files_with_ext(root, &dir, &["ts"]) {
        let Some(name) = path.rsplit('/').next() else {
            continue;
        };
        if !is_a_guide_file(name) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&path)) else {
            continue;
        };
        let (Some(constant), Some(piece)) = (exported_const(&text), piece_of(&text)) else {
            report.fail(format!(
                "{path} — no `export const GUIDE_…: Guide` with a quoted `piece`. \
                 The rule reads that shape and nothing else."
            ));
            continue;
        };
        if CATALOGUE_ONLY.contains(&piece.as_str()) {
            continue;
        }
        if !sources.iter().any(|s| names(s, &constant)) {
            report.fail(format!(
                "{path} — piece `{piece}` is drawn nowhere: no screen mentions \
                 `{constant}`. Draw a `GuideMark` for it, or retire the guide and add \
                 its number to `RETIRED_GUIDE_NUMBERS`."
            ));
        }
    }
    report
}

/// `NNN-slug.ts`, which is what a guide file is called.
fn is_a_guide_file(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".ts") else {
        return false;
    };
    let digits = stem.chars().take_while(|c| c.is_ascii_digit()).count();
    digits == 3 && stem[digits..].starts_with('-')
}

/// Not a story, not a test, and not the guides directory itself, whose index
/// names every guide by construction.
fn counts_as_a_screen(path: &str) -> bool {
    !(path.starts_with(&format!("{GUIDES}/"))
        || path.ends_with(".stories.tsx")
        || path.ends_with(".stories.ts")
        || path.ends_with(".test.ts")
        || path.ends_with(".test.tsx"))
}

fn exported_const(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.strip_prefix("export const ")?;
        let (name, _) = rest.split_once(':')?;
        Some(name.trim().to_string())
    })
}

fn piece_of(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("piece:")?;
        let (_, rest) = rest.split_once('"')?;
        let (inner, _) = rest.split_once('"')?;
        Some(inner.to_string())
    })
}

/// Whether `text` holds `name` as a whole identifier, so `GUIDE_PLAN` is not
/// found inside `GUIDE_PLAN_ASKS`.
fn names(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(at, _)| {
        let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let before = text[..at].chars().next_back().is_some_and(word);
        let after = text[at + name.len()..].chars().next().is_some_and(word);
        !before && !after
    })
}

#[cfg(test)]
mod tests;
