//! A Job's tree at its derived path. Fleet leases pool slots; only test fixtures cut one.
//!
//! **Every refusal happens before anything is written.** v1's bug was that the
//! branch collision was discovered by `git worktree add` failing halfway, which
//! reported git's own sentence — the operation, not the reason — and left the
//! caller to guess. So the order below is the specification.
//!
//! 1. Open the repository.
//! 2. Refuse if the branch is already there. **Refused, never reused**: adding
//!    a worktree onto an existing branch checks it out, and two Jobs then
//!    commit to one line of history.
//! 3. Clear a stale registration for *this Job's name*, or refuse if the
//!    worktree it names is still live.
//! 4. Refuse if the path holds something git did not put there.
//! 5. Create `.armada/worktrees`, the branch, and the worktree.
//!
//! Step three is the one a person will actually hit, and
//! [`clear_a_stale_registration`] holds why and how narrowly it is done.
//!
//! **What is not here: no removal, no sweep, no retention.** A worktree
//! survives every terminal state because nothing Fleet hands a Drone can delete
//! one; removal is a person's act, at `armada clean`, through
//! [`reclaim`](crate::reclaim()). **Nor the base checkout** — shared, no
//! branch, and the opposite view of an existing directory: `crate::basing`.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use adapter_traits::{
    BaseCheckout, BaseSpec, CommitTime, Committed, SlotKept, SlotLeased, SlotParkRefused,
    SlotParked, SlotPool, SlotReading, SlotStanding, Vcs, Worktree, WorktreeSpec,
};
use git2::{BranchType, ErrorCode, Repository, WorktreeAddOptions};

use crate::error::{CommitWorkError, CreateWorktreeError};
use crate::gh_forge::GhForge;
use crate::leasing::LeaseRefused;

/// Version control against a real repository on this machine.
///
/// Holds nothing but the forge it asks. Every operation is scoped by the [`WorktreeSpec`] it is
/// handed, which is what names the repository — so one of these can serve every
/// Job on the machine and cannot be pointed at the wrong repository by having
/// been constructed against one earlier.
///
/// **`F` is the forge**, [`GhForge`] unless [`with_forge`](GitVcs::with_forge) names another.
#[derive(Debug, Default, Clone, Copy)]
pub struct GitVcs<F = GhForge> {
    pub(crate) forge: F,
}

/// What [`Vcs::changes`] counts. **Process-wide, not per value**: `GitVcs`
/// holds nothing, so every copy of it is the same adapter.
pub(crate) static CHANGES: AtomicU64 = AtomicU64::new(0);

/// `done`, after counting it as a change.
fn noted<T>(done: T) -> T {
    CHANGES.fetch_add(1, Ordering::Relaxed);
    // What this adapter did is not waited for from the file system: every
    // kept reading is read again.
    crate::leasing::acted();
    done
}

impl GitVcs {
    pub fn new() -> GitVcs {
        GitVcs { forge: GhForge }
    }
}

impl<F> GitVcs<F> {
    /// The same version control, asking `forge` where [`new`](GitVcs::new) asks `gh`.
    pub fn with_forge(forge: F) -> GitVcs<F> {
        GitVcs { forge }
    }
}

impl<F> Vcs for GitVcs<F> {
    type Error = CreateWorktreeError;
    type CommitError = CommitWorkError;

    fn create_worktree(&self, spec: &WorktreeSpec) -> Result<Worktree, Self::Error> {
        let repo = Repository::open(spec.repo_root()).map_err(|cause| {
            CreateWorktreeError::RepoUnreadable {
                repo: spec.repo_root().to_string(),
                cause,
            }
        })?;

        let branch_name = spec.branch();
        refuse_an_existing_branch(&repo, spec, &branch_name)?;
        clear_a_stale_registration(&repo, spec)?;

        let path = spec.worktree_path();
        refuse_an_occupied_path(Path::new(&path))?;

        let parent = spec.worktree_parent();
        fs::create_dir_all(&parent).map_err(|cause| CreateWorktreeError::ParentNotCreated {
            path: parent.clone(),
            cause,
        })?;

        add(&repo, spec, &branch_name, &path)
    }

    fn base_commit(
        &self,
        repo_root: &str,
        declared: Option<&str>,
    ) -> Result<Option<String>, Self::Error> {
        crate::basing::base_commit(repo_root, declared)
    }

    fn branches(
        &self,
        repo_root: &str,
        declared: Option<&str>,
    ) -> Result<Vec<adapter_traits::BranchListed>, Self::Error> {
        crate::basing::branches(repo_root, declared)
    }

    fn create_branch(
        &self,
        repo_root: &str,
        name: &str,
        start_point: &str,
    ) -> Result<(), Self::Error> {
        crate::basing::create_branch(repo_root, name, start_point)
    }

    fn base_checkout(&self, spec: &BaseSpec) -> Result<BaseCheckout, Self::Error> {
        crate::basing::base_checkout(spec)
    }

    fn drop_base_checkout(&self, spec: &BaseSpec) -> Result<(), Self::Error> {
        crate::basing::drop_base_checkout(spec)
    }

    fn commit_all(
        &self,
        worktree: &Worktree,
        message: &str,
        at: CommitTime,
    ) -> Result<Committed, Self::CommitError> {
        noted(crate::commit::commit_all(worktree, message, at))
    }

    fn commit_paths(
        &self,
        worktree: &Worktree,
        paths: &[&str],
        message: &str,
        at: CommitTime,
    ) -> Result<Committed, Self::CommitError> {
        noted(crate::commit::commit_paths(worktree, paths, message, at))
    }

    fn clone_repository(
        &self,
        url: &str,
        destination: &str,
        within: std::time::Duration,
    ) -> Result<(), adapter_traits::NotCloned> {
        crate::cloning::clone_repository(url, destination, within)
    }

    fn lease_slot(
        &self,
        pool: &SlotPool,
        spec: &WorktreeSpec,
        job_id: &str,
    ) -> Result<SlotLeased, Self::Error> {
        noted(crate::leasing::hosted::lease(pool, spec, job_id)).map_err(|refused| {
            CreateWorktreeError::SlotNotLeased {
                repo: pool.repo_root().to_string(),
                why: match refused {
                    LeaseRefused::Vcs(why) => why,
                    // A Job's lease cuts a new branch; these are the other
                    // lease's, said the same way.
                    other => other.said(),
                },
            }
        })
    }

    fn lease_existing_slot(
        &self,
        pool: &SlotPool,
        branch: &str,
        job_id: &str,
    ) -> Result<SlotLeased, Self::Error> {
        noted(crate::leasing::hosted::lease_existing(pool, branch, job_id)).map_err(|refused| {
            CreateWorktreeError::SlotNotLeased {
                repo: pool.repo_root().to_string(),
                why: refused.said(),
            }
        })
    }

    fn park_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        job_id: &str,
    ) -> Result<SlotParked, SlotParkRefused> {
        noted(crate::leasing::hosted::park(pool, slot, job_id))
    }

    fn park_hosted_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        session_id: &str,
    ) -> Result<SlotParked, SlotParkRefused> {
        noted(crate::leasing::hosted::park_hosted(pool, slot, session_id))
    }

    fn park_ended_session_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        session_id: &str,
    ) -> Result<SlotParked, SlotParkRefused> {
        noted(crate::leasing::hosted::park_ended_session(pool, slot, session_id))
    }

    fn release_session_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        holder: &str,
    ) -> Result<SlotParked, SlotParkRefused> {
        noted(crate::leasing::hosted::release_session(pool, slot, holder))
    }

    fn hand_session_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        from: &str,
        to_session: &str,
    ) -> Result<(), SlotParkRefused> {
        noted(crate::leasing::hosted::hand(pool, slot, from, to_session))
    }

    fn lease_review_bay(
        &self,
        pool: &SlotPool,
        pull_request: u64,
        holder: &str,
    ) -> Result<adapter_traits::ReviewBay, String> {
        noted(crate::leasing::hosted::lease_review(pool, pull_request, holder))
    }

    fn release_review_bay(&self, pool: &SlotPool, slot: u32, holder: &str) -> Result<(), String> {
        noted(crate::leasing::hosted::release_review(pool, slot, holder))
    }

    fn paths_touched_by(&self, pool: &SlotPool, authors: &[String], commits: usize) -> Vec<String> {
        crate::history::paths_touched_by(pool.repo_root(), authors, commits)
    }

    fn slot_standing(&self, pool: &SlotPool, slot: u32, job_id: &str) -> SlotStanding {
        crate::leasing::hosted::standing(pool, slot, job_id)
    }

    fn slot_after_merge(&self, pool: &SlotPool, slot: u32, merged: &str) -> adapter_traits::SlotAfterMerge {
        crate::leasing::hosted::after_merge(pool, slot, merged)
    }

    fn release_slot(&self, pool: &SlotPool, slot: u32, job_id: &str) -> Result<(), SlotKept> {
        noted(crate::leasing::hosted::release(pool, slot, job_id))
    }

    fn delete_repair_branch(
        &self,
        spec: &WorktreeSpec,
    ) -> Result<bool, adapter_traits::BranchKept> {
        noted(crate::reclaim::delete_repair_branch(spec)).map_err(adapter_traits::BranchKept)
    }

    fn delete_merged_branch(
        &self,
        pool: &SlotPool,
        branch: &str,
    ) -> Result<bool, adapter_traits::BranchKept> {
        // A merge lands on the remote first, and the remote-tracking base here is only as new as
        // the last fetch. A failed fetch leaves the check to what is here.
        let _ = crate::build_standing::fetch_main(pool.repo_root(), std::time::Duration::from_secs(20));
        noted(crate::reclaim::delete_merged_branch(pool.repo_root(), branch))
            .map_err(adapter_traits::BranchKept)
    }

    fn branch_is_merged(&self, pool: &SlotPool, branch: &str) -> bool {
        let _ = crate::build_standing::fetch_main(pool.repo_root(), std::time::Duration::from_secs(20));
        crate::reclaim::branch_is_merged(pool.repo_root(), branch)
    }

    fn slot_pool(&self, pool: &SlotPool) -> Vec<SlotReading> {
        crate::leasing::hosted::readings(pool)
    }

    fn slot_pool_kept(&self, pool: &SlotPool, full: bool) -> Vec<SlotReading> {
        crate::leasing::hosted::readings_kept(pool, full)
    }

    fn change_slot_pool(
        &self,
        pool: &SlotPool,
        change: adapter_traits::SlotChange,
    ) -> Result<u32, adapter_traits::SlotRefused> {
        noted(crate::leasing::hosted::change(pool, change))
    }

    fn session_work(
        &self,
        pool: &SlotPool,
        slot: u32,
    ) -> Result<adapter_traits::StrandedWork, adapter_traits::RescueRefused> {
        crate::leasing::hosted::session_work(pool, slot)
    }

    fn stranded_work(
        &self,
        pool: &SlotPool,
        slot: u32,
    ) -> Result<adapter_traits::StrandedWork, adapter_traits::RescueRefused> {
        crate::leasing::hosted::stranded_work(pool, slot)
    }

    fn stranded_diff(
        &self,
        pool: &SlotPool,
        slot: u32,
    ) -> Result<String, adapter_traits::RescueRefused> {
        crate::leasing::hosted::stranded_diff(pool, slot)
    }

    fn rescue_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        rescue: adapter_traits::SlotRescue,
    ) -> Result<adapter_traits::SlotRescued, adapter_traits::RescueRefused> {
        noted(crate::leasing::hosted::rescue(pool, slot, rescue))
    }

    fn changes(&self) -> u64 {
        CHANGES.load(Ordering::Relaxed)
    }
}

/// **The pre-flight v1 learned to write.**
///
/// Probing costs one ref lookup and turns git's *"could not create the
/// worktree"* into a sentence naming the branch and both ways out.
fn refuse_an_existing_branch(
    repo: &Repository,
    spec: &WorktreeSpec,
    branch: &str,
) -> Result<(), CreateWorktreeError> {
    match repo.find_branch(branch, BranchType::Local) {
        Ok(_) => Err(CreateWorktreeError::BranchExists {
            repo: spec.repo_root().to_string(),
            branch: branch.to_string(),
        }),
        // A branch that is not found is the ordinary case, and it is the only
        // error treated as "no". Anything else is the repository failing to
        // answer, which is not a licence to carry on and create one.
        Err(cause) if cause.code() == ErrorCode::NotFound => Ok(()),
        Err(cause) => Err(CreateWorktreeError::RepoUnreadable {
            repo: spec.repo_root().to_string(),
            cause,
        }),
    }
}

/// Clear the administrative record left by a worktree whose directory is gone.
///
/// Prunable, in libgit2's default sense, means exactly: not locked, not still
/// checked out, and no longer valid — the directory it points at is not there.
/// A live worktree is therefore never pruned; it is refused.
///
/// **The record outlives the directory.** git files one under
/// `.git/worktrees/<name>`, M1 has no cleanup on purpose, and `rm -rf` on a
/// checkout leaves the record behind because nothing about a directory tells
/// git it is gone. The next worktree at that path is then refused with *"is a
/// missing but already registered working tree"*.
///
/// The pruning is narrow in two ways that matter. **Only this Job's name** —
/// walking every registration and pruning what looks dead is a sweeper, and a
/// sweeper is the thing M1 must not have. **Never the working tree** —
/// [`git2::WorktreePruneOptions`] defaults leave the `working_tree` flag off,
/// and the defaults are passed as `None` rather than built, so there is no
/// field in this file that could be flipped to make a prune delete somebody's
/// files.
fn clear_a_stale_registration(
    repo: &Repository,
    spec: &WorktreeSpec,
) -> Result<(), CreateWorktreeError> {
    let name = spec.registration_name();
    let Ok(registered) = repo.find_worktree(name) else {
        return Ok(());
    };
    let registered_at = registered.path().display().to_string();

    // `None` is the whole safety argument: the defaults prune the record and
    // leave any working tree alone, and there is no options value here that
    // could say otherwise.
    match registered.is_prunable(None) {
        Ok(true) => registered.prune(None).map_err(|cause| {
            CreateWorktreeError::StaleRegistrationNotCleared {
                job_id: spec.job_id().to_string(),
                registered_at,
                cause,
            }
        }),
        Ok(false) => Err(CreateWorktreeError::WorktreeAlreadyLive {
            job_id: spec.job_id().to_string(),
            registered_at,
        }),
        Err(cause) => Err(CreateWorktreeError::StaleRegistrationNotCleared {
            job_id: spec.job_id().to_string(),
            registered_at,
            cause,
        }),
    }
}

/// Refuse a path holding something git has no record of.
///
/// An empty directory is fine — git will check out into it, and a directory
/// left by a partly-finished create is the ordinary shape of that.
fn refuse_an_occupied_path(path: &Path) -> Result<(), CreateWorktreeError> {
    let Ok(entries) = fs::read_dir(path) else {
        return Ok(());
    };
    let count = entries.count();
    if count == 0 {
        return Ok(());
    }
    Err(CreateWorktreeError::PathOccupied {
        path: path.display().to_string(),
        entries: count,
    })
}

/// Create the branch, then the worktree checked out on it.
///
/// The two are separate calls because libgit2 takes a *reference* to check out
/// rather than a branch name to create, which is why a half-finished create is
/// representable at all. It is not rolled back: see
/// [`CreateWorktreeError::WorktreeNotCreated`].
fn add(
    repo: &Repository,
    spec: &WorktreeSpec,
    branch_name: &str,
    path: &str,
) -> Result<Worktree, CreateWorktreeError> {
    let head = match repo.head() {
        Ok(head) => head,
        Err(cause) if cause.code() == ErrorCode::UnbornBranch => {
            return Err(CreateWorktreeError::NoCommitToBranchFrom {
                repo: spec.repo_root().to_string(),
            })
        }
        Err(cause) => {
            return Err(CreateWorktreeError::RepoUnreadable {
                repo: spec.repo_root().to_string(),
                cause,
            })
        }
    };
    let from = head
        .peel_to_commit()
        .map_err(|cause| CreateWorktreeError::RepoUnreadable {
            repo: spec.repo_root().to_string(),
            cause,
        })?;

    // `force` is false. The branch was already probed above, and passing true
    // here would silently reopen the case that probe exists to refuse.
    let branch = repo.branch(branch_name, &from, false).map_err(|cause| {
        CreateWorktreeError::WorktreeNotCreated {
            path: path.to_string(),
            branch: branch_name.to_string(),
            cause,
        }
    })?;
    let reference = branch.into_reference();

    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&reference));

    repo.worktree(spec.registration_name(), Path::new(path), Some(&options))
        .map_err(|cause| CreateWorktreeError::WorktreeNotCreated {
            path: path.to_string(),
            branch: branch_name.to_string(),
            cause,
        })?;

    Ok(Worktree::at(path, branch_name))
}
