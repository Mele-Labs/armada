//! A branch that takes a protocol minor it never declared a need for is refused.
//! `#1059`: on 6 Oct 2026 a branch bumped the protocol from 23.33 to 23.35
//! without declaring, took the number another branch held, and that branch
//! renumbered. The owner's answer: the checks refuse it.
//!
//! **One function, called by Fleet's merge act**, so a session and a Fleet Job
//! are held to the same rule. [`undeclared`] reads the
//! two ends of the branch from git, and says what to run where a watched path
//! changed and the paths the branch has declared a need on do not include it.
//! **Where those paths come from is the caller's**: Fleet asks its ledger.
//!
//! **Which paths, and what counts as a change, is [`WATCHED`] and nothing
//! else.** Another repository names its own by editing that list; there is no
//! file for it yet, since `.gitattributes` and `armada.yml` declare nothing of
//! the kind and this repository is the only one that needs it.
//!
//! **A migration is not watched.** Migrations are named, and two branches that
//! each append one both apply (`docs/practices/store-migrations.md`), so there
//! is no number to take. **A minor changed is the two `minor` values
//! differing**, unless `major` differs too: a major move is hand-made, outside
//! needs, and resets the minor on purpose.

use std::path::Path;
use std::process::Command;

use crate::needs::clean_path;

/// What counts as a change on a watched path.
#[derive(Clone, Copy, Debug)]
pub enum Change {
    /// `minor` differs, `major` does not.
    MinorChanged,
}

/// A path whose change needs a declared need, and the words to declare it in.
#[derive(Clone, Copy, Debug)]
pub struct Watched {
    pub path: &'static str,
    pub what: &'static str,
    pub change: Change,
}

/// This repository's watched paths: the only place they are written down.
pub const WATCHED: &[Watched] = &[Watched {
    path: "protocol-version.toml",
    what: "a minor",
    change: Change::MinorChanged,
}];

/// `Ok(None)` where the branch is in order, `Ok(Some(answer))` where it took a
/// number it never declared, `Err` where git could not be read.
///
/// `declared` is every path the branch has a standing need on.
///
/// `base` is whatever names the base (`origin/main`, a commit); the branch is
/// compared from where it left it, so a base that moved on is not the branch's
/// change.
pub fn undeclared(
    repo: &Path,
    base: &str,
    branch: &str,
    declared: &[String],
) -> Result<Option<String>, String> {
    let fork = git(repo, &["merge-base", base, branch])?.trim().to_string();
    let mut missing = Vec::new();
    for watched in WATCHED {
        let has = declared
            .iter()
            .any(|path| clean_path(path) == clean_path(watched.path));
        if !has && changed(repo, &fork, branch, watched)? {
            missing.push(watched);
        }
    }
    if missing.is_empty() {
        return Ok(None);
    }
    let run: Vec<String> = missing
        .iter()
        .map(|watched| format!("armada need {} \"{}\"", watched.path, watched.what))
        .collect();
    let paths: Vec<String> = WATCHED
        .iter()
        .map(|watched| format!("{} ({})", watched.path, watched.what))
        .collect();
    Ok(Some(format!(
        "{branch} changes {} with no need declared for it. Run {}, and use the number \
         it gives you (a number taken without one collides with whoever holds it). \
         A need is required on {}. The branch keeps its place: once declared, \
         land it again.",
        missing
            .iter()
            .map(|watched| watched.path)
            .collect::<Vec<_>>()
            .join(" and "),
        run.join(" and "),
        paths.join(" and "),
    )))
}

fn changed(repo: &Path, fork: &str, branch: &str, watched: &Watched) -> Result<bool, String> {
    let before = show(repo, fork, watched.path)?;
    let after = show(repo, branch, watched.path)?;
    if before == after {
        return Ok(false);
    }
    Ok(match watched.change {
        Change::MinorChanged => {
            number(&before, "major") == number(&after, "major")
                && number(&before, "minor") != number(&after, "minor")
        }
    })
}

/// The integer a `key = 12` line gives, where the file has one.
fn number(source: &str, key: &str) -> Option<u64> {
    source.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        if name.trim() != key {
            return None;
        }
        value.split('#').next()?.trim().parse().ok()
    })
}

/// `path` at `rev`, empty where it did not exist there.
fn show(repo: &Path, rev: &str, path: &str) -> Result<String, String> {
    let spec = format!("{rev}:{path}");
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["show", &spec])
        .output()
        .map_err(|why| format!("git: {why}"))?;
    Ok(if out.status.success() {
        String::from_utf8_lossy(&out.stdout).into_owned()
    } else {
        String::new()
    })
}

fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|why| format!("git: {why}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}
