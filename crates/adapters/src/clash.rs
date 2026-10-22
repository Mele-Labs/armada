//! Whether two worktrees' changes, or one worktree's and a base's, would conflict if merged.
//! `docs/concepts/fleet.md`, *Write clashes*.
//!
//! **Whole trees are compared**, each worktree as it stands (`snapshot::tree_as_it_stands`: the
//! real index is never written), against the tree of their merge base. A write no hook saw is in
//! the tree all the same.

use std::fmt;
use std::path::Path;

use crate::delivery::{read_in, said};

/// What merging the two sides would do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Clean,
    /// Repository-relative, sorted.
    Conflict(Vec<String>),
}

/// Git would not answer, so nothing is known either way. **Never a clash.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotChecked(pub String);

impl fmt::Display for NotChecked {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.0)
    }
}

/// Whether the changes in `one` conflict with the changes in `other`, both standing in worktrees of
/// one repository.
pub fn between(one: &Path, other: &Path) -> Result<Verdict, NotChecked> {
    let theirs = head_of(other)?;
    let (ours_tree, theirs_tree) = (tree_of(one)?, tree_of(other)?);
    merged(one, &head_of(one)?, &theirs, &ours_tree, &theirs_tree)
}

/// Whether the changes in `worktree` conflict with `tip`, a commit of the base.
pub fn against(worktree: &Path, tip: &str) -> Result<Verdict, NotChecked> {
    let tip_tree = rev_parse(worktree, &format!("{tip}^{{tree}}"))?;
    merged(worktree, &head_of(worktree)?, tip, &tree_of(worktree)?, &tip_tree)
}

/// The paths `tip` changed since it and the worktree's `HEAD` last met.
pub fn changed_on(worktree: &Path, tip: &str) -> Result<Vec<String>, NotChecked> {
    let base = merge_base(worktree, &head_of(worktree)?, tip)?;
    let range = format!("{base}..{tip}");
    let listed = git(worktree, &["diff", "--name-only", "-z", &range])?;
    Ok(listed
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect())
}

/// The paths that differ between the commits `from` and `to`, as a repository at `repo` holds them.
/// Deleted paths and both sides of a rename are named: a test that reached a file is stale whether
/// the file changed or went.
pub fn changed_between(repo: &Path, from: &str, to: &str) -> Result<Vec<String>, NotChecked> {
    let listed = git(repo, &["diff", "--name-only", "--no-renames", "-z", from, to])?;
    Ok(listed
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect())
}

/// The paths the worktree as it stands changed since it and `base` last met: its commits and what
/// it has not committed yet.
pub fn changed_here(worktree: &Path, base: &str) -> Result<Vec<String>, NotChecked> {
    let met = merge_base(worktree, &head_of(worktree)?, base)?;
    let listed = git(worktree, &["diff", "--name-only", "-z", &met, &tree_of(worktree)?])?;
    Ok(listed
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect())
}

/// The commit on `tip`'s first-parent line that brought `path` in since the worktree's `HEAD` and
/// `tip` last met, as `abc1234 subject`. A merge's subject names its pull request.
pub fn landed_in(worktree: &Path, tip: &str, path: &str) -> Option<String> {
    let base = merge_base(worktree, &head_of(worktree).ok()?, tip).ok()?;
    let range = format!("{base}..{tip}");
    let args = ["log", "--first-parent", "-1", "--format=%h %s", &range, "--", path];
    let said = git(worktree, &args).ok()?;
    let line = said.trim();
    (!line.is_empty()).then(|| line.to_string())
}

/// One file as each of two sides changed it, from where they split.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDiffs {
    /// The first worktree's change, as `git diff` prints it. Empty where it left the file alone.
    pub ours: String,
    /// The other side's.
    pub theirs: String,
}

/// `path` as the worktree `one` and the worktree `other` each changed it since their `HEAD`s last
/// met, both as they stand. Read through scratch indexes, so neither worktree is written.
pub fn file_between(one: &Path, other: &Path, path: &str) -> Result<FileDiffs, NotChecked> {
    let base = merge_base(one, &head_of(one)?, &head_of(other)?)?;
    Ok(FileDiffs {
        ours: file_diff(one, &base, &tree_of(one)?, path)?,
        theirs: file_diff(one, &base, &tree_of(other)?, path)?,
    })
}

/// `path` as the worktree changed it, and as `tip`, a commit of the base, changed it since it and the
/// worktree's `HEAD` last met.
pub fn file_against(worktree: &Path, tip: &str, path: &str) -> Result<FileDiffs, NotChecked> {
    let base = merge_base(worktree, &head_of(worktree)?, tip)?;
    Ok(FileDiffs {
        ours: file_diff(worktree, &base, &tree_of(worktree)?, path)?,
        theirs: file_diff(worktree, &base, tip, path)?,
    })
}

/// Whether the repository `worktree` is in holds this commit.
pub fn holds(worktree: &Path, commit: &str) -> bool {
    rev_parse(worktree, &format!("{commit}^{{commit}}")).is_ok()
}

/// Whether `branch` exists in the repository at `repo`. Git that could not run
/// says nothing about it, so that reads as yes.
pub fn branch_exists(repo: &Path, branch: &str) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(true)
}

fn tree_of(worktree: &Path) -> Result<String, NotChecked> {
    crate::snapshot::tree_as_it_stands(worktree)
        .map_err(|why| NotChecked(format!("{} could not be read as a tree: {why:?}", worktree.display())))
}

fn head_of(worktree: &Path) -> Result<String, NotChecked> {
    rev_parse(worktree, "HEAD")
}

fn rev_parse(worktree: &Path, name: &str) -> Result<String, NotChecked> {
    Ok(git(worktree, &["rev-parse", "--verify", "--quiet", name])?.trim().to_string())
}

fn merge_base(worktree: &Path, one: &str, other: &str) -> Result<String, NotChecked> {
    Ok(git(worktree, &["merge-base", one, other])?.trim().to_string())
}

/// What `git diff` prints for the one file between two trees or commits. **The path is literal**: a
/// name with `[` or `*` in it is that file, and no pattern.
fn file_diff(worktree: &Path, from: &str, to: &str, path: &str) -> Result<String, NotChecked> {
    let literal = format!(":(literal){path}");
    let args = [
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
        "--no-renames",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        from,
        to,
        "--",
        &literal,
    ];
    git(worktree, &args)
}

/// `git merge-tree` over two trees, on the tree of their commits' merge base. Exit 0 is clean and 1
/// is a conflict, with the conflicted paths after the tree id; anything else is not a verdict.
fn merged(
    worktree: &Path,
    one_commit: &str,
    other_commit: &str,
    one: &str,
    other: &str,
) -> Result<Verdict, NotChecked> {
    let base = merge_base(worktree, one_commit, other_commit)?;
    let base = format!("--merge-base={base}^{{tree}}");
    let args = ["merge-tree", "--write-tree", "--name-only", "-z", &base, one, other];
    let ran = read_in(&worktree.to_string_lossy(), "git", &args)
        .map_err(|why| NotChecked(format!("git merge-tree could not run: {why}")))?;
    match ran.status.code() {
        Some(0) => Ok(Verdict::Clean),
        Some(1) => {
            let out = String::from_utf8_lossy(&ran.stdout);
            // The tree id, then each conflicted path, then an empty entry before git's messages.
            let mut paths: Vec<String> = out
                .split('\0')
                .skip(1)
                .take_while(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect();
            paths.sort();
            paths.dedup();
            Ok(Verdict::Conflict(paths))
        }
        _ => Err(NotChecked(format!("git merge-tree: {}", said(&ran)))),
    }
}

fn git(worktree: &Path, args: &[&str]) -> Result<String, NotChecked> {
    let ran = read_in(&worktree.to_string_lossy(), "git", args)
        .map_err(|why| NotChecked(format!("git {} could not run: {why}", args[0])))?;
    if !ran.status.success() {
        return Err(NotChecked(format!("git {}: {}", args[0], said(&ran))));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).into_owned())
}
