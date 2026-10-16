//! Committing uncommitted files to the branch they are on, so a checkout can be
//! freed or removed without losing them. A park does it for a pool slot; a
//! Clear does it for a Job's own worktree, which `git worktree remove` would
//! otherwise empty. Nothing is pushed, and none is refused for where HEAD
//! points: a checkout on no branch or on the base gets a branch of its own.

use std::path::Path;

use git2::{Repository, WorktreeLockStatus};

use adapter_traits::WorktreeSpec;

use super::git::{count, dirty, git, git_ok, is_checkout, read};
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

/// What a Clear kept of a Job's own worktree: where, and what was committed.
#[derive(Debug)]
pub struct SavedWorktree {
    /// The branch the files went to: the one the worktree was on, or a new one
    /// where it was on none or on the base.
    pub branch: String,
    pub committed: Committed,
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

/// Whether `name` is the branch work is merged into, which a WIP commit must
/// not land on: what the remote's head names, else `main` or `master`.
fn is_a_base(at: &Path, name: &str) -> bool {
    matches!(name, "main" | "master")
        || read(at, &["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"])
            .is_ok_and(|head| head == format!("origin/{name}"))
}

/// Commit what is uncommitted in the Job's worktree, before the worktree is
/// removed. It goes to the branch the worktree is on. Where that is no branch
/// or the base, a new `kept/job-<id>` branch is cut at `HEAD` for it, and for
/// any commit no branch or remote holds. `None` where the worktree is absent,
/// not a checkout of its own, clean or locked: a lock is a person saying not
/// yet, and the reclaim reports it without a commit having been made first.
pub fn save_worktree(spec: &WorktreeSpec, job: &str) -> Result<Option<SavedWorktree>, String> {
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
    let on = git(at, &["branch", "--show-current"])?;
    let detached = on.is_empty();
    let files = dirty(at)?;
    // Commits a detach would leave on no branch: only a detached HEAD has any.
    let stranded = detached && count(at, &["HEAD", "--not", "--branches", "--remotes"]) > 0;
    if files.is_empty() && !stranded {
        return Ok(None);
    }
    let branch = if detached || is_a_base(at, &on) {
        cut_kept_branch(at, &format!("job-{job}"))?
    } else {
        on
    };
    let message =
        format!("WIP: uncommitted files of job {job}, saved before its worktree was removed");
    Ok(commit_all(at, &message)?.map(|committed| SavedWorktree { branch, committed }))
}
