//! Where a repository's files are, and which of them an agent has changed. `docs/concepts/fleet.md`,
//! *The territory*.
//!
//! **Git answers, and only from objects.** The directories are the base head's tree; a slot's changes
//! are its tree as it stands (`snapshot::tree_as_it_stands`, through a scratch index, so the real
//! one is never written) against the tree of its merge base. No directory is walked.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::delivery::{run_in, said};

/// Git would not answer, so nothing is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotRead(pub String);

impl fmt::Display for NotRead {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.0)
    }
}

/// A directory of the tree and the files at or under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Directory {
    /// Repository-relative, one or two names deep: `crates` or `crates/fleet`.
    pub path: String,
    pub files: u32,
}

/// The commit `base` names in the repository at `root`.
pub fn head_of(root: &Path, base: &str) -> Result<String, NotRead> {
    let named = format!("{base}^{{commit}}");
    Ok(git(root, &["rev-parse", "--verify", "--quiet", &named])?.trim().to_string())
}

/// Every directory one or two names deep in the tree of `head`, with the files under it, by path.
pub fn directories_at(root: &Path, head: &str) -> Result<Vec<Directory>, NotRead> {
    let listed = git(root, &["ls-tree", "-r", "--name-only", "-z", head])?;
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for file in listed.split('\0').filter(|name| !name.is_empty()) {
        let names: Vec<&str> = file.split('/').collect();
        // The last name is the file, so a file at the top is in no directory.
        let above = names.len() - 1;
        for depth in 1..=above.min(2) {
            *counts.entry(names[..depth].join("/")).or_default() += 1;
        }
    }
    Ok(counts.into_iter().map(|(path, files)| Directory { path, files }).collect())
}

/// The paths a worktree has changed since it met `base`: committed on its branch, edited, deleted or
/// not yet tracked. Repository-relative, sorted.
pub fn changed_since(worktree: &Path, base: &str) -> Result<Vec<String>, NotRead> {
    let met = git(worktree, &["merge-base", "HEAD", base])?;
    let standing = crate::snapshot::tree_as_it_stands(worktree)
        .map_err(|why| NotRead(format!("{} could not be read as a tree: {why:?}", worktree.display())))?;
    let listed = git(worktree, &["diff-tree", "-r", "--name-only", "-z", met.trim(), &standing])?;
    let mut paths: Vec<String> =
        listed.split('\0').filter(|name| !name.is_empty()).map(str::to_string).collect();
    paths.sort();
    Ok(paths)
}

fn git(dir: &Path, args: &[&str]) -> Result<String, NotRead> {
    let ran = run_in(&dir.to_string_lossy(), "git", args)
        .map_err(|why| NotRead(format!("git {} could not run: {why}", args[0])))?;
    if !ran.status.success() {
        return Err(NotRead(format!("git {}: {}", args[0], said(&ran))));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).into_owned())
}
