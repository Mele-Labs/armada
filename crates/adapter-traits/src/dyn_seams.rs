//! Fleet holds its Vcs, Delivery and WorkProduct as trait objects. `Vcs` and `WorkProduct` name
//! associated error types, which a trait object has to fix, and the real adapters and the fakes
//! use different ones. These two mirror the traits with the error boxed, and are implemented for
//! everything that implements the originals, so `Arc::new(adapter)` coerces to either.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use crate::*;

pub type BoxedError = Box<dyn core::error::Error + Send + Sync>;

/// [`Vcs`] and [`Delivery`] in one object, errors boxed.
pub trait DynVcs: Delivery + Send + Sync {
    /// The adapter itself, for a test that holds a fake and reaches its own controls.
    fn as_any(&self) -> &dyn core::any::Any;
    fn create_worktree(&self, spec: &WorktreeSpec) -> Result<Worktree, BoxedError>;
    fn base_commit(&self, repo_root: &str, declared: Option<&str>) -> Result<Option<alloc::string::String>, BoxedError>;
    fn branches(&self, repo_root: &str, declared: Option<&str>) -> Result<alloc::vec::Vec<BranchListed>, BoxedError>;
    fn create_branch(&self, repo_root: &str, name: &str, start_point: &str) -> Result<(), BoxedError>;
    fn base_checkout(&self, spec: &BaseSpec) -> Result<BaseCheckout, BoxedError>;
    fn drop_base_checkout(&self, spec: &BaseSpec) -> Result<(), BoxedError>;
    fn commit_all(&self, worktree: &Worktree, message: &str, at: CommitTime) -> Result<Committed, BoxedError>;
    fn commit_paths(&self, worktree: &Worktree, paths: &[&str], message: &str, at: CommitTime) -> Result<Committed, BoxedError>;
    fn clone_repository(&self, url: &str, destination: &str, within: core::time::Duration) -> Result<(), NotCloned>;
    fn lease_slot(&self, pool: &SlotPool, spec: &WorktreeSpec, job_id: &str) -> Result<SlotLeased, BoxedError>;
    fn lease_existing_slot(&self, pool: &SlotPool, branch: &str, job_id: &str) -> Result<SlotLeased, BoxedError>;
    fn park_slot(&self, pool: &SlotPool, slot: u32, job_id: &str) -> Result<SlotParked, SlotParkRefused>;
    fn park_hosted_slot(&self, pool: &SlotPool, slot: u32, session_id: &str) -> Result<SlotParked, SlotParkRefused>;
    fn park_ended_session_slot(&self, pool: &SlotPool, slot: u32, session_id: &str) -> Result<SlotParked, SlotParkRefused>;
    fn release_session_slot(&self, pool: &SlotPool, slot: u32, holder: &str) -> Result<SlotParked, SlotParkRefused>;
    fn hand_session_slot(&self, pool: &SlotPool, slot: u32, from: &str, to_session: &str) -> Result<(), SlotParkRefused>;
    fn slot_standing(&self, pool: &SlotPool, slot: u32, job_id: &str) -> SlotStanding;
    fn make_slot(&self, pool: &SlotPool, slot: u32) -> Result<(), alloc::string::String>;
    fn slot_dirty(&self, pool: &SlotPool, slot: u32) -> Result<bool, alloc::string::String>;
    fn slot_after_merge(&self, pool: &SlotPool, slot: u32, merged: &str) -> SlotAfterMerge;
    fn release_slot(&self, pool: &SlotPool, slot: u32, job_id: &str) -> Result<(), SlotKept>;
    fn lease_review_bay(&self, pool: &SlotPool, pull_request: u64, holder: &str) -> Result<ReviewBay, alloc::string::String>;
    fn release_review_bay(&self, pool: &SlotPool, slot: u32, holder: &str) -> Result<(), alloc::string::String>;
    fn paths_touched_by(&self, pool: &SlotPool, authors: &[alloc::string::String], commits: usize) -> alloc::vec::Vec<alloc::string::String>;
    fn delete_repair_branch(&self, spec: &WorktreeSpec) -> Result<bool, BranchKept>;
    fn delete_merged_branch(&self, pool: &SlotPool, branch: &str) -> Result<bool, BranchKept>;
    fn branch_is_merged(&self, pool: &SlotPool, branch: &str) -> bool;
    fn slot_pool(&self, pool: &SlotPool) -> alloc::vec::Vec<SlotReading>;
    fn slot_pool_kept(&self, pool: &SlotPool, full: bool) -> alloc::vec::Vec<SlotReading>;
    fn change_slot_pool(&self, pool: &SlotPool, change: SlotChange) -> Result<u32, SlotRefused>;
    fn session_work(&self, pool: &SlotPool, slot: u32) -> Result<StrandedWork, RescueRefused>;
    fn stranded_work(&self, pool: &SlotPool, slot: u32) -> Result<StrandedWork, RescueRefused>;
    fn stranded_diff(&self, pool: &SlotPool, slot: u32) -> Result<alloc::string::String, RescueRefused>;
    fn rescue_slot(&self, pool: &SlotPool, slot: u32, rescue: SlotRescue) -> Result<SlotRescued, RescueRefused>;
    fn changes(&self) -> u64;
}

impl<T> DynVcs for T
where
    T: Vcs + Delivery + Send + Sync + 'static,
    T::Error: core::error::Error + Send + Sync + 'static,
    T::CommitError: core::error::Error + Send + Sync + 'static,
{
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    fn create_worktree(&self, spec: &WorktreeSpec) -> Result<Worktree, BoxedError> {
        Vcs::create_worktree(self, spec).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn base_commit(&self, repo_root: &str, declared: Option<&str>) -> Result<Option<alloc::string::String>, BoxedError> {
        Vcs::base_commit(self, repo_root, declared).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn branches(&self, repo_root: &str, declared: Option<&str>) -> Result<alloc::vec::Vec<BranchListed>, BoxedError> {
        Vcs::branches(self, repo_root, declared).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn create_branch(&self, repo_root: &str, name: &str, start_point: &str) -> Result<(), BoxedError> {
        Vcs::create_branch(self, repo_root, name, start_point).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn base_checkout(&self, spec: &BaseSpec) -> Result<BaseCheckout, BoxedError> {
        Vcs::base_checkout(self, spec).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn drop_base_checkout(&self, spec: &BaseSpec) -> Result<(), BoxedError> {
        Vcs::drop_base_checkout(self, spec).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn commit_all(&self, worktree: &Worktree, message: &str, at: CommitTime) -> Result<Committed, BoxedError> {
        Vcs::commit_all(self, worktree, message, at).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn commit_paths(&self, worktree: &Worktree, paths: &[&str], message: &str, at: CommitTime) -> Result<Committed, BoxedError> {
        Vcs::commit_paths(self, worktree, paths, message, at).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn clone_repository(&self, url: &str, destination: &str, within: core::time::Duration) -> Result<(), NotCloned> {
        Vcs::clone_repository(self, url, destination, within)
    }
    fn lease_slot(&self, pool: &SlotPool, spec: &WorktreeSpec, job_id: &str) -> Result<SlotLeased, BoxedError> {
        Vcs::lease_slot(self, pool, spec, job_id).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn lease_existing_slot(&self, pool: &SlotPool, branch: &str, job_id: &str) -> Result<SlotLeased, BoxedError> {
        Vcs::lease_existing_slot(self, pool, branch, job_id).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn park_slot(&self, pool: &SlotPool, slot: u32, job_id: &str) -> Result<SlotParked, SlotParkRefused> {
        Vcs::park_slot(self, pool, slot, job_id)
    }
    fn park_hosted_slot(&self, pool: &SlotPool, slot: u32, session_id: &str) -> Result<SlotParked, SlotParkRefused> {
        Vcs::park_hosted_slot(self, pool, slot, session_id)
    }
    fn park_ended_session_slot(&self, pool: &SlotPool, slot: u32, session_id: &str) -> Result<SlotParked, SlotParkRefused> {
        Vcs::park_ended_session_slot(self, pool, slot, session_id)
    }
    fn release_session_slot(&self, pool: &SlotPool, slot: u32, holder: &str) -> Result<SlotParked, SlotParkRefused> {
        Vcs::release_session_slot(self, pool, slot, holder)
    }
    fn hand_session_slot(&self, pool: &SlotPool, slot: u32, from: &str, to_session: &str) -> Result<(), SlotParkRefused> {
        Vcs::hand_session_slot(self, pool, slot, from, to_session)
    }
    fn slot_standing(&self, pool: &SlotPool, slot: u32, job_id: &str) -> SlotStanding {
        Vcs::slot_standing(self, pool, slot, job_id)
    }
    fn make_slot(&self, pool: &SlotPool, slot: u32) -> Result<(), alloc::string::String> {
        Vcs::make_slot(self, pool, slot)
    }
    fn slot_dirty(&self, pool: &SlotPool, slot: u32) -> Result<bool, alloc::string::String> {
        Vcs::slot_dirty(self, pool, slot)
    }
    fn slot_after_merge(&self, pool: &SlotPool, slot: u32, merged: &str) -> SlotAfterMerge {
        Vcs::slot_after_merge(self, pool, slot, merged)
    }
    fn release_slot(&self, pool: &SlotPool, slot: u32, job_id: &str) -> Result<(), SlotKept> {
        Vcs::release_slot(self, pool, slot, job_id)
    }
    fn lease_review_bay(&self, pool: &SlotPool, pull_request: u64, holder: &str) -> Result<ReviewBay, alloc::string::String> {
        Vcs::lease_review_bay(self, pool, pull_request, holder)
    }
    fn release_review_bay(&self, pool: &SlotPool, slot: u32, holder: &str) -> Result<(), alloc::string::String> {
        Vcs::release_review_bay(self, pool, slot, holder)
    }
    fn paths_touched_by(&self, pool: &SlotPool, authors: &[alloc::string::String], commits: usize) -> alloc::vec::Vec<alloc::string::String> {
        Vcs::paths_touched_by(self, pool, authors, commits)
    }
    fn delete_repair_branch(&self, spec: &WorktreeSpec) -> Result<bool, BranchKept> {
        Vcs::delete_repair_branch(self, spec)
    }
    fn delete_merged_branch(&self, pool: &SlotPool, branch: &str) -> Result<bool, BranchKept> {
        Vcs::delete_merged_branch(self, pool, branch)
    }
    fn branch_is_merged(&self, pool: &SlotPool, branch: &str) -> bool {
        Vcs::branch_is_merged(self, pool, branch)
    }
    fn slot_pool(&self, pool: &SlotPool) -> alloc::vec::Vec<SlotReading> {
        Vcs::slot_pool(self, pool)
    }
    fn slot_pool_kept(&self, pool: &SlotPool, full: bool) -> alloc::vec::Vec<SlotReading> {
        Vcs::slot_pool_kept(self, pool, full)
    }
    fn change_slot_pool(&self, pool: &SlotPool, change: SlotChange) -> Result<u32, SlotRefused> {
        Vcs::change_slot_pool(self, pool, change)
    }
    fn session_work(&self, pool: &SlotPool, slot: u32) -> Result<StrandedWork, RescueRefused> {
        Vcs::session_work(self, pool, slot)
    }
    fn stranded_work(&self, pool: &SlotPool, slot: u32) -> Result<StrandedWork, RescueRefused> {
        Vcs::stranded_work(self, pool, slot)
    }
    fn stranded_diff(&self, pool: &SlotPool, slot: u32) -> Result<alloc::string::String, RescueRefused> {
        Vcs::stranded_diff(self, pool, slot)
    }
    fn rescue_slot(&self, pool: &SlotPool, slot: u32, rescue: SlotRescue) -> Result<SlotRescued, RescueRefused> {
        Vcs::rescue_slot(self, pool, slot, rescue)
    }
    fn changes(&self) -> u64 {
        Vcs::changes(self)
    }
}

/// [`WorkProduct`] as an object, errors boxed.
pub trait DynWork: Send + Sync {
    fn as_any(&self) -> &dyn core::any::Any;
    fn changed_files(&self, worktree: &Worktree) -> Result<Changed, BoxedError>;
    fn counted_files(&self, worktree: &Worktree) -> Result<Counted, BoxedError>;
    fn footprint(&self, worktree: &Worktree) -> Result<Footprint, BoxedError>;
    fn patch(&self, worktree: &Worktree) -> Result<Patch, BoxedError>;
    fn branch_work(&self, repository: &str, base: &str, branch: &str) -> Result<BranchWork, BoxedError>;
    fn commit_work(&self, repository: &str, sha: &str) -> Result<BranchWork, BoxedError>;
    fn unmerged_paths(&self, worktree: &Worktree) -> Result<Vec<String>, BoxedError>;
    fn measured(&self, worktree: &Worktree) -> Measured;
}

impl<T> DynWork for T
where
    T: WorkProduct + Send + Sync + 'static,
    T::Error: core::error::Error + Send + Sync + 'static,
{
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    fn changed_files(&self, worktree: &Worktree) -> Result<Changed, BoxedError> {
        WorkProduct::changed_files(self, worktree).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn counted_files(&self, worktree: &Worktree) -> Result<Counted, BoxedError> {
        WorkProduct::counted_files(self, worktree).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn footprint(&self, worktree: &Worktree) -> Result<Footprint, BoxedError> {
        WorkProduct::footprint(self, worktree).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn patch(&self, worktree: &Worktree) -> Result<Patch, BoxedError> {
        WorkProduct::patch(self, worktree).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn branch_work(&self, repository: &str, base: &str, branch: &str) -> Result<BranchWork, BoxedError> {
        WorkProduct::branch_work(self, repository, base, branch).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn commit_work(&self, repository: &str, sha: &str) -> Result<BranchWork, BoxedError> {
        WorkProduct::commit_work(self, repository, sha).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn unmerged_paths(&self, worktree: &Worktree) -> Result<Vec<String>, BoxedError> {
        WorkProduct::unmerged_paths(self, worktree).map_err(|e| BoxedError::from(alloc::boxed::Box::new(e)))
    }
    fn measured(&self, worktree: &Worktree) -> Measured {
        WorkProduct::measured(self, worktree)
    }
}
