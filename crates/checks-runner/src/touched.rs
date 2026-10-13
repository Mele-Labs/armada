//! The test modules a change touched, read off the changed files themselves:
//! what an agent's `armada check test --changed` runs, where CI and the merge
//! line run every member the change reaches.
//!
//! **Three shapes count.** A file that is itself a test module
//! (`src/tests/x.rs`, declared `#[cfg(test)] mod tests;` above it) runs as its
//! module path, `tests::x`. A source file with an inline `#[cfg(test)] mod
//! tests { … }` runs that module. An integration test (`tests/x.rs`) runs the
//! `#[test]` functions it defines. A file that only *declares* `mod tests;`, as
//! `lib.rs` does, touches none: counting it would run a crate's whole suite
//! for adding a `mod` line. How a path becomes nextest's name for a test is
//! the third Rust fact here, with [`reach`](crate::reach) and `holding`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use core_model::Narrowing;

use crate::named::member_of;

/// What a change touched.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Touched {
    /// The members holding a touched test, sorted.
    pub members: Vec<String>,
    /// What to run in them: module paths from a crate's root, and the names of
    /// integration test functions. Sorted, without duplicates.
    pub names: Vec<String>,
}

impl Touched {
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// The tests `changed` touches, in the members `narrowing` derives from its
/// `under`, less those it excepts. `root` is where `changed`'s paths start.
pub fn touched(root: &Path, narrowing: &Narrowing, changed: &[String]) -> Touched {
    let mut members = BTreeSet::new();
    let mut names = BTreeSet::new();
    for path in changed {
        let Some((member, rest)) = member_of(narrowing, path) else {
            continue;
        };
        if narrowing.except().iter().any(|dropped| dropped == member) {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(path)) else {
            // Deleted: nothing of it is left to run.
            continue;
        };
        let found = match rest.split_once('/') {
            Some(("src", inside)) => in_source(&src_of(root, narrowing, member), inside, &text),
            Some(("tests", _)) if rest.ends_with(".rs") => integration(&text),
            _ => continue,
        };
        if !found.is_empty() {
            members.insert(member.to_string());
            names.extend(found);
        }
    }
    Touched {
        members: members.into_iter().collect(),
        names: names.into_iter().collect(),
    }
}

fn src_of(root: &Path, narrowing: &Narrowing, member: &str) -> PathBuf {
    root.join(narrowing.under().unwrap_or_default())
        .join(member)
        .join("src")
}

/// The names a file under a member's `src` touches. `inside` is its path below
/// `src`, `src` is that directory.
fn in_source(src: &Path, inside: &str, text: &str) -> Vec<String> {
    let Some(module) = module_path(inside) else {
        return Vec::new();
    };
    if text.lines().any(|line| line.trim() == "#![cfg(test)]") || declared_for_test(src, &module) {
        return vec![module.join("::")];
    }
    modules_declared(text)
        .into_iter()
        .filter(|(_, inline)| *inline)
        .map(|(name, _)| {
            let mut path = module.clone();
            path.push(name);
            path.join("::")
        })
        .collect()
}

/// The module a file under `src` is, from the crate root: `tests/x.rs` and
/// `tests/x/mod.rs` are `["tests", "x"]`. `None` for a path that is no `.rs`.
///
/// A binary's own root (`bin/x.rs`, `bin/x/main.rs`, `main.rs`) is the crate
/// root of its own tests, so it is empty like `lib.rs`.
fn module_path(inside: &str) -> Option<Vec<String>> {
    let mut parts: Vec<&str> = inside.strip_suffix(".rs")?.split('/').collect();
    if parts.first() == Some(&"bin") && parts.len() >= 2 {
        parts.drain(..2);
        if parts.is_empty() {
            parts.push("main");
        }
    }
    if parts == ["lib"] || parts == ["main"] {
        return Some(Vec::new());
    }
    if parts.last() == Some(&"mod") {
        parts.pop();
    }
    Some(parts.into_iter().map(str::to_string).collect())
}

/// Whether some module on the way down to `module` is declared `#[cfg(test)]`
/// by the file above it. `lib.rs` or `main.rs` declares the first; each later
/// one is declared in the file of the module before it.
fn declared_for_test(src: &Path, module: &[String]) -> bool {
    (0..module.len()).any(|at| {
        let parent: Vec<&str> = module[..at].iter().map(String::as_str).collect();
        let files = match parent.as_slice() {
            [] => vec![src.join("lib.rs"), src.join("main.rs")],
            above => vec![
                src.join(format!("{}.rs", above.join("/"))),
                src.join(above.join("/")).join("mod.rs"),
            ],
        };
        files
            .iter()
            .filter_map(|file| fs::read_to_string(file).ok())
            .any(|text| {
                modules_declared(&text)
                    .iter()
                    .any(|(name, _)| *name == module[at])
            })
    })
}

/// Every `#[cfg(test)] mod name;` or `#[cfg(test)] mod name {` in `text`, with
/// whether it is inline. Read line by line, so a `mod` split across lines or
/// declared by a macro is missed — which under-counts, and says so by running
/// nothing.
fn modules_declared(text: &str) -> Vec<(String, bool)> {
    let mut found = Vec::new();
    let mut after_cfg_test = false;
    for line in text.lines() {
        let mut line = line.trim_start();
        if let Some(rest) = line.strip_prefix("#[cfg(test)]") {
            after_cfg_test = true;
            line = rest.trim_start();
        }
        if line.is_empty() || line.starts_with("#[") || line.starts_with("//") {
            continue;
        }
        if after_cfg_test {
            found.extend(module_declaration(line));
        }
        after_cfg_test = false;
    }
    found
}

/// `mod name;` or `mod name {`, with any `pub` and its restriction before it.
fn module_declaration(line: &str) -> Option<(String, bool)> {
    let mut line = line;
    if let Some(rest) = line.strip_prefix("pub") {
        let rest = rest.trim_start();
        line = match rest.strip_prefix('(') {
            Some(restricted) => restricted.split_once(')')?.1.trim_start(),
            None => rest,
        };
    }
    let rest = line.strip_prefix("mod ")?.trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    match rest[name.len()..].trim_start().chars().next()? {
        _ if name.is_empty() => None,
        ';' => Some((name, false)),
        '{' => Some((name, true)),
        _ => None,
    }
}

/// The functions an integration test file marks `#[test]` or `#[….test]`.
fn integration(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut marked = false;
    for line in text.lines() {
        let line = line.trim_start();
        if let Some(attribute) = line.strip_prefix("#[") {
            let path = attribute.split(['(', ']']).next().unwrap_or_default();
            marked |= path == "test" || path.ends_with("::test");
            continue;
        }
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if marked {
            let name = line
                .split_once("fn ")
                .map(|(_, rest)| rest.trim_start())
                .map(|rest| rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_'))
                .map(Iterator::collect::<String>);
            found.extend(name.filter(|name| !name.is_empty()));
        }
        marked = false;
    }
    found
}
