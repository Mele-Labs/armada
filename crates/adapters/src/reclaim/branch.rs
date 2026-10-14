//! A person deleting one Job's branch after its checkout is gone.
//!
//! **The branch half of [`reclaim`](super::reclaim) with
//! [`UnmergedWork::Delete`]**, the path `armada clean --force` takes, and no
//! second deletion. What it adds is the guard: the checkout must be gone and
//! the branch must still stand at the tip the person was shown.

use adapter_traits::WorktreeSpec;
use git2::{BranchType, Repository};

use super::UnmergedWork;
use super::{
    delete_the_branch, on_disk, remove_the_worktree, BranchGone, RepoUnreadable, WorktreeGone,
};

/// Why a delete was not attempted. Nothing was touched on any of these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BranchRefused {
    /// The checkout is still there. Reclaiming it is the act that comes first.
    CheckoutOnDisk {
        path: String,
    },
    Absent {
        branch: String,
    },
    /// **The branch moved since the person looked**, so what they agreed to
    /// lose is not what is there.
    TipMoved {
        branch: String,
        asked: String,
        found: String,
    },
    Unreadable(RepoUnreadable),
}

/// Delete the branch one Job derived, only if it stands at `tip` and its
/// checkout is gone. `Deleted` carries the tip, for [`BranchGone`]'s reason.
pub fn delete_branch(spec: &WorktreeSpec, tip: &str) -> Result<BranchGone, BranchRefused> {
    let repo = Repository::open(spec.repo_root()).map_err(|cause| {
        BranchRefused::Unreadable(RepoUnreadable {
            repo: spec.repo_root().to_string(),
            why: cause.message().to_string(),
        })
    })?;
    if on_disk(spec) {
        return Err(BranchRefused::CheckoutOnDisk {
            path: spec.worktree_path(),
        });
    }
    let branch = spec.branch();
    let found = the_tip(&repo, &branch).ok_or_else(|| BranchRefused::Absent {
        branch: branch.clone(),
    })?;
    if found != tip {
        return Err(BranchRefused::TipMoved {
            branch,
            asked: tip.to_string(),
            found,
        });
    }
    // With nothing on disk this only clears a stale record, which would
    // otherwise keep the branch checked out in git's eyes.
    let worktree = remove_the_worktree(&repo, spec);
    Ok(delete_the_branch(
        &repo,
        &branch,
        None,
        &worktree,
        UnmergedWork::Delete,
    ))
}

fn the_tip(repo: &Repository, branch: &str) -> Option<String> {
    let found = repo.find_branch(branch, BranchType::Local).ok()?;
    Some(
        found
            .get()
            .target()
            .map(|oid| oid.to_string())
            .unwrap_or_else(|| String::from("an unresolved ref")),
    )
}

/// Delete the branch a repair Drone wrote on, whatever it holds: its fix is on
/// the Job's branch, or it failed and nobody will take it. **Fleet's act, and
/// only ever on a branch derived from a repair's own spec.** `false` is a branch
/// that was gone already; a slot with it checked out refuses and it stays.
pub fn delete_repair_branch(spec: &WorktreeSpec) -> Result<bool, String> {
    let repo = Repository::open(spec.repo_root()).map_err(|cause| cause.message().to_string())?;
    let held_by = WorktreeGone::Pooled {
        path: spec.worktree_path(),
    };
    match delete_the_branch(&repo, &spec.branch(), None, &held_by, UnmergedWork::Delete) {
        BranchGone::Deleted { .. } => Ok(true),
        BranchGone::Absent { .. } => Ok(false),
        BranchGone::NotDeleted { why, .. } | BranchGone::KeptUnanswered { why, .. } => Err(why),
        BranchGone::Kept { branch, .. } => Err(format!("{branch} was kept")),
    }
}

/// Whether `branch` is a local branch, not the base, whose every commit the base already holds.
/// **The base is read as the local branch or the remote's**, since a merge lands on the remote
/// first and the local branch can lag it.
pub fn branch_is_merged(repo_root: &str, branch: &str) -> bool {
    let Ok(repo) = Repository::open(repo_root) else {
        return false;
    };
    let Ok(found) = repo.find_branch(branch, BranchType::Local) else {
        return false;
    };
    let Some(tip) = found.get().target() else {
        return false;
    };
    let bases = crate::base::candidates(&repo, None);
    !bases.iter().any(|name| name == branch) && base_holds(&repo, &bases, tip)
}

fn base_holds(repo: &Repository, bases: &[String], tip: git2::Oid) -> bool {
    bases.iter().any(|name| {
        [name.clone(), format!("origin/{name}")].iter().any(|each| {
            let kind = if each.starts_with("origin/") { BranchType::Remote } else { BranchType::Local };
            repo.find_branch(each, kind)
                .ok()
                .and_then(|base| base.get().target())
                .is_some_and(|base| base == tip || repo.graph_descendant_of(base, tip).unwrap_or(false))
        })
    })
}

/// Delete a branch a session worked on, only when the base already holds every commit on it.
/// **The base is read as the local branch or the remote's**, since a merge lands on the remote
/// first and the local branch can lag it. `false` is a branch gone already; one with commits
/// the base lacks, or still checked out in a slot, is refused and stays.
pub fn delete_merged_branch(repo_root: &str, branch: &str) -> Result<bool, String> {
    let repo = Repository::open(repo_root).map_err(|cause| cause.message().to_string())?;
    let mut found = match repo.find_branch(branch, BranchType::Local) {
        Ok(found) => found,
        Err(cause) if cause.code() == git2::ErrorCode::NotFound => return Ok(false),
        Err(cause) => return Err(cause.message().to_string()),
    };
    let Some(tip) = found.get().target() else {
        return Err(format!("{branch} points at no commit"));
    };
    let bases = crate::base::candidates(&repo, None);
    if bases.iter().any(|name| name == branch) {
        return Err(format!("{branch} is the base"));
    }
    let held = base_holds(&repo, &bases, tip);
    if !held {
        return Err(format!("{branch} has commits the base does not"));
    }
    found.delete().map_err(|cause| cause.message().to_string())?;
    Ok(true)
}
