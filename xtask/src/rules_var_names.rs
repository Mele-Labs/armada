//! Every `var(--…)` names a custom property something declares.
//!
//! The browser drops a declaration whose `var()` resolves to nothing. On 29
//! September 2026 `var(--space-5)` and `var(--tracking-label)` did that on one
//! card with every gate green (#1668). **The check is the name, not its shape**:
//! `--text-label` used as a `font-size` is a real token, and a colour, and
//! catching it needs lengths told from colours, which this rule leaves out.
//!
//! **Declared means anywhere on the surface**, because a custom property
//! inherits down the DOM: `DroneBrief` reads what the `Chapter` around it sets.
//! A fallback does not excuse a name nothing sets — it is always the fallback.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::rules_stylesheets::strip_comments;
use crate::{files_with_ext, Report};

/// Where the shared names are declared.
const TOKENS: &str = "packages/tokens/src";

/// The trees a renderer ships from.
const TREES: [&str; 2] = ["packages", "apps"];

/// One `var()` read: the line it is on, the name it reads, whether the name is
/// only a stem a template string finishes (`var(--status-${hue})`), and
/// whether a fallback follows it.
#[derive(Debug, PartialEq)]
struct Read {
    line: usize,
    name: String,
    stem: bool,
    fallback: bool,
}

/// Reads every stylesheet under `packages/` and every `.tsx` under `packages/`
/// and `apps/`, against what the tokens, the stylesheets, and the style objects
/// and `setProperty` calls in any `.ts`/`.tsx` declare.
pub fn every_var_names_a_declared_property(root: &Path) -> Report {
    let mut report = Report::new("every var() names a custom property something declares");
    if !root.join(TOKENS).is_dir() {
        report.fail(format!(
            "{TOKENS} — the token files every var() resolves against"
        ));
        return report;
    }

    let mut declared = BTreeSet::new();
    let mut readers = Vec::new();
    for tree in TREES {
        for path in files_with_ext(root, &root.join(tree), &["css", "tsx", "ts"]) {
            let Ok(text) = fs::read_to_string(root.join(&path)) else {
                continue;
            };
            let code = strip_comments(&text);
            let css = path.ends_with(".css");
            declared.extend(declarations(&code, css));
            if path.ends_with(".tsx") || (css && tree == "packages") {
                readers.push((path, code));
            }
        }
    }

    for (path, code) in &readers {
        for read in reads(code, path.ends_with(".tsx")) {
            if resolves(&read, &declared) {
                continue;
            }
            let written = if read.stem {
                format!("var({}${{…}})", read.name)
            } else {
                format!("var({})", read.name)
            };
            let so = if read.fallback {
                "only its fallback is ever read"
            } else {
                "the browser drops the declaration it sits in"
            };
            report.fail(format!(
                "{path}:{} — `{written}` names a custom property nothing declares, so \
                 {so}. Not in {TOKENS}/, and no stylesheet or component sets it.{}",
                read.line,
                neighbours(&read.name, &declared)
            ));
        }
    }
    report
}

fn resolves(read: &Read, declared: &BTreeSet<String>) -> bool {
    if read.stem {
        // `var(--${name})` builds the whole name at runtime: no stem to hold.
        read.name == "--" || declared.iter().any(|d| d.starts_with(&read.name))
    } else {
        declared.contains(&read.name)
    }
}

/// The declared names sharing everything up to the last `-`, so `--space-5`
/// is told the ladder rather than told to go and find it.
fn neighbours(name: &str, declared: &BTreeSet<String>) -> String {
    let Some(at) = name.trim_end_matches('-').rfind('-').filter(|at| *at > 2) else {
        return String::new();
    };
    let family = &name[..=at];
    let near: Vec<&str> = declared
        .iter()
        .filter(|d| d.starts_with(family))
        .map(String::as_str)
        .take(16)
        .collect();
    if near.is_empty() {
        return String::new();
    }
    format!(" Declared under `{family}`: {}", near.join(", "))
}

fn ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// The `--name` starting at `at`, provided `--` opens a name rather than
/// sitting inside one: `.armada-row--active:hover` is a BEM modifier, not a
/// declaration of `--active`.
fn name_at(code: &str, at: usize) -> Option<String> {
    if !code[at..].starts_with("--") || code[..at].chars().next_back().is_some_and(ident) {
        return None;
    }
    Some(code[at..].chars().take_while(|c| ident(*c)).collect())
}

/// What one file declares. In a stylesheet, a name followed by `:`. Anywhere
/// else, a name that is the whole of a string literal — a style object's key,
/// `setProperty`'s argument — which `"var(--x)"` is not.
fn declarations(code: &str, css: bool) -> Vec<String> {
    let mut found = Vec::new();
    for (at, _) in code.match_indices("--") {
        let Some(name) = name_at(code, at) else {
            continue;
        };
        if name.len() <= 2 {
            continue;
        }
        let after = &code[at + name.len()..];
        let declares = if css {
            after.trim_start().starts_with(':')
        } else {
            let open = code[..at].chars().next_back();
            matches!(open, Some('"' | '\'' | '`')) && after.starts_with(open.unwrap_or('"'))
        };
        if declares {
            found.push(name);
        }
    }
    found
}

/// Every `var(--…)` in one file, with its line. A name a template literal
/// finishes is a stem, and only a `.tsx` writes one.
fn reads(code: &str, tsx: bool) -> Vec<Read> {
    let mut found = Vec::new();
    for (at, _) in code.match_indices("var(") {
        let start = at + 4;
        let rest = &code[start..];
        let start = start + rest.len() - rest.trim_start().len();
        let Some(name) = name_at(code, start) else {
            continue;
        };
        let after = &code[start + name.len()..];
        let stem = tsx && after.starts_with("${");
        let fallback = after.trim_start().starts_with(',');
        if name == "--" && !stem {
            continue;
        }
        // A `.tsx` comment can be a `//` line, which `strip_comments` leaves.
        let opened = code[..at].rfind('\n').map_or(0, |n| n + 1);
        if tsx && code[opened..at].trim_start().starts_with("//") {
            continue;
        }
        let line = code[..at].matches('\n').count() + 1;
        found.push(Read {
            line,
            name,
            stem,
            fallback,
        });
    }
    found
}

#[cfg(test)]
mod tests;
