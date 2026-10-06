//! Rule: every directory under `packages/surfaces/` is a whole surface package.
//!
//! The layer rule governs what a surface imports; this one governs what it
//! carries. A surface with no `vitest.config.ts` is tested by nothing, and one
//! with no `armada.yml` is gated by nothing, so a change to it would pass
//! every Check while its own tests never ran.
//!
//! Each surface needs a `package.json`, a `vitest.config.ts` and an
//! `armada.yml` that declares at least one Check and a non-empty `depends_on`.
//! Vacuously green while `packages/surfaces/` has nothing in it.
//!
//! **No YAML parser, and the gate keeps no dependencies.** The two keys sit at
//! the left margin in every manifest here, so a line reading is enough.

use std::fs;
use std::path::Path;

use crate::Report;

#[cfg(test)]
mod tests;

const SURFACES_DIR: &str = "packages/surfaces";
const FILES: &[&str] = &["package.json", "vitest.config.ts", "armada.yml"];

pub fn every_surface_is_a_whole_package(root: &Path) -> Report {
    let mut report = Report::new("every surface carries its package, its tests and its Checks");

    let Ok(entries) = fs::read_dir(root.join(SURFACES_DIR)) else {
        return report;
    };
    let mut dirs: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.') && name != "node_modules")
        .collect();
    dirs.sort();

    for name in dirs {
        let dir = format!("{SURFACES_DIR}/{name}");
        for file in FILES {
            if !root.join(&dir).join(file).is_file() {
                report.fail(format!("missing: {dir}/{file}"));
            }
        }
        if let Ok(text) = fs::read_to_string(root.join(&dir).join("armada.yml")) {
            check_manifest(&dir, &text, &mut report);
        }
    }
    report
}

/// The manifest declares a Check and a `depends_on`.
fn check_manifest(dir: &str, text: &str, report: &mut Report) {
    if !declares_a_check(text) {
        report.fail(format!(
            "missing: a Check under `checks:` in {dir}/armada.yml"
        ));
    }
    if !declares_depends_on(text) {
        report.fail(format!("missing: a `depends_on` in {dir}/armada.yml"));
    }
}

/// Lines that are not blank and not a comment, with their text.
fn code_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .collect()
}

/// `checks:` at the left margin, then an indented key beneath it.
fn declares_a_check(text: &str) -> bool {
    let lines = code_lines(text);
    let Some(at) = lines.iter().position(|l| l.trim_end() == "checks:") else {
        return false;
    };
    lines
        .get(at + 1)
        .is_some_and(|next| next.starts_with(' ') && next.trim_end().ends_with(':'))
}

/// `depends_on:` at the left margin, holding a flow list with an entry or a
/// block list with an item.
fn declares_depends_on(text: &str) -> bool {
    let lines = code_lines(text);
    let Some(at) = lines.iter().position(|l| l.starts_with("depends_on:")) else {
        return false;
    };
    let inline = lines[at]["depends_on:".len()..].trim();
    if !inline.is_empty() {
        return inline.starts_with('[')
            && inline.trim_matches(|c| c == '[' || c == ']').trim() != "";
    }
    lines
        .get(at + 1)
        .is_some_and(|next| next.trim_start().starts_with("- "))
}
