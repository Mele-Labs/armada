//! Where a repository's files are, and which of them an agent has changed. `docs/concepts/fleet.md`,
//! *The territory*.
//!
//! **Git answers, and only from objects.** The directories are the base head's tree; a slot's changes
//! are its tree as it stands (`snapshot::tree_as_it_stands`, through a scratch index, so the real
//! one is never written) against the tree of its merge base. No directory is walked.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::delivery::{read_in, said};

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
    /// The first of those files, as git lists them, named relative to the directory: `src/lib.rs`.
    pub names: Vec<String>,
}

/// The commit `base` names in the repository at `root`.
pub fn head_of(root: &Path, base: &str) -> Result<String, NotRead> {
    let named = format!("{base}^{{commit}}");
    Ok(git(root, &["rev-parse", "--verify", "--quiet", &named])?.trim().to_string())
}

/// Every directory one or two names deep in the tree of `head`, with the files under it, by path.
/// Each names its first `most_names` files; the count stays exact. One listing, so naming costs no
/// second read of git.
pub fn directories_at(root: &Path, head: &str, most_names: usize) -> Result<Vec<Directory>, NotRead> {
    let listed = git(root, &["ls-tree", "-r", "--name-only", "-z", head])?;
    let mut found: BTreeMap<String, Directory> = BTreeMap::new();
    for file in listed.split('\0').filter(|name| !name.is_empty()) {
        let names: Vec<&str> = file.split('/').collect();
        // The last name is the file, so a file at the top is in no directory.
        let above = names.len() - 1;
        for depth in 1..=above.min(2) {
            let path = names[..depth].join("/");
            let directory = found.entry(path.clone()).or_insert_with(|| Directory { path, files: 0, names: Vec::new() });
            directory.files += 1;
            if directory.names.len() < most_names {
                directory.names.push(names[depth..].join("/"));
            }
        }
    }
    Ok(found.into_values().collect())
}

/// Every file in the tree of `head`, repository-relative and in git's order.
pub fn files_at(root: &Path, head: &str) -> Result<Vec<String>, NotRead> {
    let listed = git(root, &["ls-tree", "-r", "--name-only", "-z", head])?;
    Ok(listed.split('\0').filter(|name| !name.is_empty()).map(str::to_string).collect())
}

/// A file as the tree of `head` holds it: its text up to `most` bytes, and whether there was more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    pub text: String,
    pub cut: bool,
}

/// The text of `path` in the tree of `head`, read from git's objects and so whatever the checkout
/// holds. **`None` where the file is not text**: a NUL byte in what was read, or bytes that are not
/// UTF-8 once cut back to a character boundary.
pub fn text_at(root: &Path, head: &str, path: &str, most: usize) -> Result<Option<Text>, NotRead> {
    let named = format!("{head}:{path}");
    let ran = read_in(&root.to_string_lossy(), "git", &["cat-file", "blob", &named])
        .map_err(|why| NotRead(format!("git cat-file could not run: {why}")))?;
    if !ran.status.success() {
        return Err(NotRead(format!("git cat-file: {}", said(&ran))));
    }
    let bytes = ran.stdout;
    let cut = bytes.len() > most;
    let mut end = bytes.len().min(most);
    // A cut inside a multi-byte character is walked back to its start, so a text cut short is
    // still text; at most three bytes.
    while cut && end > 0 && end > most.saturating_sub(4) && std::str::from_utf8(&bytes[..end]).is_err() {
        end -= 1;
    }
    if bytes[..end].contains(&0) {
        return Ok(None);
    }
    Ok(std::str::from_utf8(&bytes[..end]).ok().map(|text| Text { text: text.to_string(), cut }))
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
    let ran = read_in(&dir.to_string_lossy(), "git", args)
        .map_err(|why| NotRead(format!("git {} could not run: {why}", args[0])))?;
    if !ran.status.success() {
        return Err(NotRead(format!("git {}: {}", args[0], said(&ran))));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).into_owned())
}
