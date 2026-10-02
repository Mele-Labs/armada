//! The command-line git a slot is driven with, for the reason
//! `crate::reclaim`'s `porcelain` gives: a guard takes the answer a person
//! would be shown.

use std::path::Path;
use std::process::{Command, Output};

fn run(at: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(at)
        .args(args)
        // Nobody is at a terminal to answer a credential prompt.
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
}

/// What git printed, trimmed, or what it said when it refused.
pub(super) fn git(at: &Path, args: &[&str]) -> Result<String, String> {
    let ran = run(at, args).map_err(|why| format!("git would not run: {why}"))?;
    if !ran.status.success() {
        return Err(format!(
            "`git {}` in {}: {}",
            args.join(" "),
            at.display(),
            String::from_utf8_lossy(&ran.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).trim().to_string())
}

/// Whether git answered at all.
pub(super) fn git_ok(at: &Path, args: &[&str]) -> bool {
    run(at, args).is_ok_and(|ran| ran.status.success())
}

/// Whether `path` is the top of a checkout of its own, rather than a directory
/// inside the repository's main one.
pub(super) fn is_checkout(path: &Path) -> bool {
    let Ok(top) = git(path, &["rev-parse", "--show-toplevel"]) else {
        return false;
    };
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    Path::new(&top).canonicalize().is_ok_and(|top| top == path)
}

/// How many commits `rev-list` names for `revs`. A count git cannot give reads
/// as one: unknown is not landed.
pub(super) fn count(at: &Path, revs: &[&str]) -> usize {
    let mut args = vec!["rev-list", "--count"];
    args.extend_from_slice(revs);
    git(at, &args)
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(1)
}

/// Every path `git status` reports, untracked included and ignored not.
pub(super) fn dirty(at: &Path) -> Result<Vec<String>, String> {
    let said =
        run(at, &["status", "--porcelain"]).map_err(|why| format!("git would not run: {why}"))?;
    if !said.status.success() {
        return Err(String::from_utf8_lossy(&said.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&said.stdout)
        .lines()
        .filter_map(|line| line.get(3..))
        .map(str::to_string)
        .collect())
}
