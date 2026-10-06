//! Committing uncommitted files to the branch they are on, so a checkout can be
//! freed or removed without losing them. A park does it for a pool slot; a
//! Clear does it for a Job's own worktree, which `git worktree remove` would
//! otherwise empty. Nothing is pushed.

use std::path::Path;

use git2::{Repository, WorktreeLockStatus};

use adapter_traits::WorktreeSpec;

use super::git::{dirty, git, is_checkout};
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

/// Why a Job's own worktree was not saved. Nothing was committed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotSaved {
    /// Detached, so there is no branch to commit to.
    OnNoBranch,
    /// On a branch other than the Job's.
    OnAnotherBranch { expected: String, on: String },
    /// git refused, and this is what it said.
    Vcs(String),
}

impl NotSaved {
    pub fn said(&self) -> String {
        match self {
            NotSaved::OnNoBranch => String::from(
                "the worktree is on a detached HEAD, so there is no branch to commit the \
                 uncommitted files to",
            ),
            NotSaved::OnAnotherBranch { expected, on } => format!(
                "the worktree is on branch {on}, not {expected}, so the uncommitted files were \
                 not committed"
            ),
            NotSaved::Vcs(why) => why.clone(),
        }
    }
}

/// Commit what is uncommitted in the Job's worktree to its branch, before the
/// worktree is removed. `None` where the worktree is absent, not a checkout of its own, clean or
/// locked:
/// a lock is a person saying not yet, and the reclaim reports it without a
/// commit having been made first.
pub fn save_worktree(spec: &WorktreeSpec, job: &str) -> Result<Option<Committed>, NotSaved> {
    let path = spec.worktree_path();
    let at = Path::new(&path);
    // A plain directory inside the repository answers git with the
    // repository's own checkout, and committing there would take whatever the
    // base checkout holds.
    if !at.exists() || !is_checkout(at) {
        return Ok(None);
    }
    if let Ok(repo) = Repository::open(spec.repo_root()) {
        let locked = repo
            .find_worktree(spec.registration_name())
            .and_then(|found| found.is_locked())
            .is_ok_and(|status| matches!(status, WorktreeLockStatus::Locked(_)));
        if locked {
            return Ok(None);
        }
    }
    let files = dirty(at).map_err(NotSaved::Vcs)?;
    if files.is_empty() {
        return Ok(None);
    }
    let on = git(at, &["branch", "--show-current"]).map_err(NotSaved::Vcs)?;
    if on.is_empty() {
        return Err(NotSaved::OnNoBranch);
    }
    let expected = spec.branch();
    if on != expected {
        return Err(NotSaved::OnAnotherBranch { expected, on });
    }
    let message =
        format!("WIP: uncommitted files of job {job}, saved before its worktree was removed");
    commit_all(at, &message).map_err(NotSaved::Vcs)
}
