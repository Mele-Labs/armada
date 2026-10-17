//! Committing uncommitted files to the branch they are on, so a checkout can be
//! freed without losing them. A park does it for a pool slot. Nothing is
//! pushed, and none is refused for where HEAD points: a checkout on no branch
//! or on the base gets a branch of its own.

use std::path::Path;

use super::git::{dirty, git, git_ok};
use super::Committed;

/// Commit every uncommitted file in `at`, untracked in and ignored out. `None`
/// where there is nothing to commit.
///
/// `--no-verify`: the work is being kept, not offered, and a hook that refused
/// it would leave it in a checkout the person wants freed.
pub(super) fn commit_all(at: &Path, message: &str) -> Result<Option<Committed>, String> {
    let files = dirty(at)?;
    if files.is_empty() {
        return Ok(None);
    }
    git(at, &["add", "--all"])?;
    git(at, &["commit", "--quiet", "--no-verify", "-m", message])?;
    let commit = git(at, &["rev-parse", "HEAD"])?;
    Ok(Some(Committed { commit, files }))
}

/// Cut a branch at `at`'s `HEAD`, named `kept/<stem>` with a number after it
/// where that name is taken, and switch to it. The files in the checkout come
/// with the switch, so a commit made there holds them. The name of the branch.
pub(super) fn cut_kept_branch(at: &Path, stem: &str) -> Result<String, String> {
    let mut name = format!("kept/{stem}");
    let mut n = 1;
    while git_ok(at, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{name}")]) {
        n += 1;
        name = format!("kept/{stem}-{n}");
    }
    git(at, &["switch", "--quiet", "--create", &name])?;
    Ok(name)
}
