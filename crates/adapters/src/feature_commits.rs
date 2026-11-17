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

/// Whether `slug` is what a `Feature:` line can carry: lowercase letters, digits, dashes, underscores, dots and slashes.
fn clean(slug: &str) -> bool {
    !slug.is_empty() && slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '/' | '.'))
}

/// The slugs of a comma separated `value`, each once, added to `slugs`.
fn push_slugs(value: &str, slugs: &mut Vec<String>) {
    for slug in value.split(',').map(str::trim) {
        if clean(slug) && !slugs.iter().any(|held| held == slug) {
            slugs.push(slug.to_string());
        }
    }
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
        push_slugs(value, &mut slugs);
    }
    slugs
}

/// The slugs named by `Feature:` anywhere in `text`, where it starts a line or follows a quote. **Read from the words of
/// a shell command or a brief, not from a commit**: `git commit -m "Feature: a, b"` and a heredoc's own line both name
/// a feature, and a `Feature:` in the middle of a sentence is prose.
pub fn feature_lines_in(text: &str) -> Vec<String> {
    let mut slugs: Vec<String> = Vec::new();
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        let mut from = 0;
        while let Some(found) = lower[from..].find("feature:") {
            let at = from + found;
            let before = line[..at].chars().next_back();
            let starts = line[..at].trim().is_empty() || matches!(before, Some('"' | '\''));
            let value = &line[at + "feature:".len()..];
            let value = value.split(['"', '\'']).next().unwrap_or(value);
            if starts {
                push_slugs(value, &mut slugs);
            }
            from = at + "feature:".len();
        }
    }
    slugs
}

/// The slugs between backticks in `text`, each once. Only a word that is a feature's slug in the project counts, so a
/// backticked command or path names nothing.
pub fn backticked_slugs_in(text: &str) -> Vec<String> {
    let mut slugs: Vec<String> = Vec::new();
    for (at, part) in text.split('`').enumerate() {
        let word = part.trim();
        if at % 2 == 1 && clean(word) && !slugs.iter().any(|held| held == word) {
            slugs.push(word.to_string());
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
