//! The features a Job's commits name, read off their `Feature: <slug>` trailer lines. `docs/concepts/feature-catalog.md`,
//! *How a Job's work is filed*.
//!
//! **Git is asked, never the Job.** A commit names a feature by a trailer in its last paragraph, one or more lines,
//! each line one slug or several separated by commas. The files a commit changed are filed under the features it names.

use std::path::Path;

use crate::clash::NotChecked;
use crate::delivery::{read_in, said};

/// One commit that names features, with the files it changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamingCommit {
    pub sha: String,
    /// The slugs its trailers name, once each, in the order written.
    pub features: Vec<String>,
    /// Repository-relative.
    pub files: Vec<String>,
}

/// The slugs named by `Feature:` lines in the last paragraph of `message`. A line elsewhere in the body is prose.
pub fn trailer_slugs(message: &str) -> Vec<String> {
    let trimmed = message.trim_end();
    let last = trimmed.rsplit("\n\n").next().unwrap_or(trimmed);
    let mut slugs: Vec<String> = Vec::new();
    for line in last.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if !key.trim().eq_ignore_ascii_case("feature") {
            continue;
        }
        for slug in value.split(',').map(str::trim) {
            let clean = !slug.is_empty()
                && slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '/' | '.'));
            if clean && !slugs.iter().any(|held| held == slug) {
                slugs.push(slug.to_string());
            }
        }
    }
    slugs
}

/// The commit `worktree` stands at, whole.
pub fn head(worktree: &Path) -> Option<String> {
    git(worktree, &["rev-parse", "--verify", "--quiet", "HEAD"]).ok().map(|out| out.trim().to_string()).filter(|sha| !sha.is_empty())
}

/// The commits from `base` to `HEAD` of `worktree` that name a feature, oldest first, each with the files it changed.
pub fn commits_naming(worktree: &Path, base: &str) -> Result<Vec<NamingCommit>, NotChecked> {
    let range = format!("{base}..HEAD");
    let log = git(worktree, &["log", "--reverse", "--no-color", "--format=%H%x1f%B%x1e", &range])?;
    let mut found = Vec::new();
    for entry in log.split('\u{1e}') {
        let Some((sha, message)) = entry.trim_start().split_once('\u{1f}') else {
            continue;
        };
        let features = trailer_slugs(message);
        if features.is_empty() {
            continue;
        }
        let listed = git(worktree, &["diff-tree", "--no-commit-id", "--name-only", "-r", "--no-renames", "-z", sha])?;
        let files = listed.split('\0').filter(|name| !name.is_empty()).map(str::to_string).collect();
        found.push(NamingCommit { sha: sha.to_string(), features, files });
    }
    Ok(found)
}

fn git(worktree: &Path, args: &[&str]) -> Result<String, NotChecked> {
    let command = args.iter().find(|arg| !arg.starts_with('-')).unwrap_or(&"");
    let ran = read_in(&worktree.to_string_lossy(), "git", args).map_err(|why| NotChecked(format!("git {command} could not run: {why}")))?;
    if !ran.status.success() {
        return Err(NotChecked(format!("git {command}: {}", said(&ran))));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).into_owned())
}
