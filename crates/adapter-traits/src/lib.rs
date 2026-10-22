//! The seams: the traits an adapter implements, and nothing that implements
//! them.
//!
//! One trait per boundary, and `Secret<T>`. Version control splits into three
//! of them — [`Vcs`] leases a Job its worktree, [`WorkProduct`] reads one
//! at the gate, [`Delivery`] publishes what a finished one holds — because each
//! is held by a different caller and none may reach another's methods. The implementations live
//! in `adapters`, which is the only crate permitted to know whose API it is
//! talking to — a vendor's name outside `adapters` is the boundary having
//! leaked, and it leaks in comments first.
//!
//! # What may not enter this crate
//!
//! No runtime, no I/O, no vendor, checked by `cargo tree`. A trait that needs
//! `tokio` to *declare* has already chosen the implementation's runtime for it.
//!
//! # Why the traits are here at all
//!
//! The acceptance test drives a Job with every adapter faked — no process
//! spawned, no repository touched, no network opened. That is only cheap
//! because the seam is a trait: `fleet` must never reach for the real CLI
//! directly, anywhere.

//! # `no_std`, except under test
//!
//! Conditional for the one reason `core-model`'s is: the unit test harness
//! needs `std` to link. Every shipped build of this crate is `no_std` and
//! depends on nothing, which is what `cargo tree -p adapter-traits` shows.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod basing;
mod bucket;
mod checkout_send;
mod ci;
mod cloning;
mod commit;
mod delivery;
mod event;
mod harness;
mod judge;
mod landable;
mod link_lookup;
mod main_ci;
mod manifest_edit;
mod pull_request;
mod pull_request_detail;
mod pull_review;
mod pull_watch;
mod request;
mod secret;
mod setup;
mod sign_in;
mod slots;
mod store;
mod test_map;
mod text_model;
mod tracker;
mod under_review;
mod work_product;
mod worktree;

pub use basing::{BaseCheckout, BaseSpec, BaseSpecRefused, BranchListed};
pub use bucket::{Bucket, BucketError};
pub use ci::{
    CiCommand, CiConfiguration, CiNotFollowed, CiReading, FileEntry, FileRead, RepositoryFiles,
};
pub use checkout_send::{CheckoutSent, CheckoutToSend};
pub use cloning::NotCloned;
pub use commit::{CommitTime, Committed};
pub use delivery::{
    how_the_base_was_found, Base, BaseFetched, BaseMergedIn, BaseOnTheRemote, BranchMerged, BroughtUpToDate,
    CheckoutPulled, Delivery, FiledIssue, HeldBack, KeptCurrent, Landing, Mergeable, Merged, NotDelivered, NotFiled,
    NotMerged, NotRerun, Opened, Pushed, PushedOntoBase, Rendering, RepositoryStanding, Rerun,
    Review, Standing, WhatBecameOfIt,
};
pub use test_map::{CollectorSource, Lap, LapCost, LapGrain, NotRecorded, TestCollector, TestsToRecord};
pub use text_model::{NotEmbedded, TextModel, TextReading};
pub use event::{CallDetail, DroneEvent, FileAct, Reach, Speaker, TaskStatus, ToolUse};
pub use harness::{
    DroneHandle, DroneSpawnConfig, Effort, Environment, Launch, McpConfig,
    Model, Prompt, SpawnConfigRefused,
};
pub use judge::{Ask, CallProgress, Heard, JudgeCall, ModelClient, Reading};
pub use landable::{Landable, UncheckedHead};
pub use link_lookup::{IssueAddress, LinkLookup, LookupCall};
pub use main_ci::{
    CiRun, CiRuns, CiState, MergeQueue, MergedPull, OpenPull, OpenPulls, PullFacts,
    PullMergeable, PullReview, QueueEntry, QueueState, RecentlyMerged, RecentlyMergedPulls,
};
pub use manifest_edit::{ManifestEditSent, ManifestEditToSend, ManifestOnTheBase, PullRequestFor};
pub use pull_request::{PullRequestFacts, PullRequestStanding};
pub use pull_request_detail::{
    CheckRow, CheckStanding, CommitLine, FileChange, FilePatch, Mergeability, Omitted, PageFile,
    PatchedFile, PullRequestPageFacts, PullRequestWrite, ReviewSaid, ReviewStanding, Reviewer,
    ReviewerKind, Said,
};
pub use pull_review::{
    DiffSide, ListedChecks, ListedPull, ReviewComment, ReviewList, ReviewListAsk, ReviewThreadFacts,
    ThreadSaid,
};
pub use pull_watch::{PullQueue, WatchedCheck, WatchedPull, WatchedPulls};
pub use request::{
    Incoming, Outgoing, RequestFailure, RequestSecrets, RequestSender, SecretError, SecretVariable,
};
pub use secret::Secret;
pub use sign_in::{DeviceCode, Polled, SignIn, SignInError, SignedInAs};
pub use store::{Claimed, Holder, HolderKind, OwnerKey, RedKey, RepositoryKey, RunRow, Store, StoreError};
pub use tracker::{
    IssueComment, LinkedPullRequest, LinkedPullRequestState, NotTracked, OpenIssue, ReadIssue, Tracker,
    TrackerChoice, Trackers,
};
pub use setup::{
    HarnessSetup, Inventory, ItemAsked, ItemKind, ItemRead, ItemWhose, KindRead, SetupFiles, SetupItem,
    SetupKind, StyleApplied, StyleFacts, StyleFrom, Unreadable, WhatWasRead,
};
pub use slots::{
    BranchKept, CommitHome, RescueRefused, ReviewBay, SlotAfterMerge, SlotChange, SlotCommit,
    SlotHeld, SlotKept, SlotLeased, SlotParkRefused, SlotParked, SlotPool, SlotReading,
    SlotRefused, SlotRescue, SlotRescued, SlotStanding, StrandedWork, WorkShown,
};
pub use under_review::{
    FailureWeight, FromOutside, InlineContext, PullRequestDiff, Remark, ReviewVerdict, ReviewedBy,
    UnderReview, WhatPeopleSaid, WhatTheForgeRan,
};
pub use work_product::{
    BranchWork, Change, Changed, ChangedFile, Counted, CountedFile, Footprint, LineCount, Measured,
    Patch, WorkProduct,
};
pub use worktree::{derived, slot_path, Worktree, WorktreeSpec, WorktreeSpecRefused, SLOT_ROOT};

/// The agent harness: what a Drone is started as, and what its output means.
///
/// One headless agent CLI is the only implementation, and the trait exists so
/// it is not the only *possible* one — and, more immediately, so `testkit` can
/// drive a whole Job from a scripted stream. Which CLI is a question only
/// `adapters` is allowed to answer.
///
/// # Neither method starts a process
///
/// [`render`](AgentHarness::render) produces a [`Launch`]; Fleet starts it,
/// detached, through the one type in the workspace that can start anything. A
/// harness that spawned could spawn attached, and every confinement property
/// would then only be checkable by spawning the thing being confined. As it
/// stands they are assertions on a value.
///
/// **A second implementation is not cheap, and the trait is shaped to say so.**
/// The argument list *is* the permission model: what a Drone may run, what is
/// refused however broadly the allowlist grants, whether ambient servers are
/// excluded. A second harness re-expresses every one of those in its own
/// vocabulary, and the failure mode when it does not is the worst one in the
/// system — a missing capability does not fail, it waits. So
/// [`DroneSpawnConfig`] has no escape-hatch constructor: a second
/// implementation does not compile until it has answered every capability
/// question the first one answers.
pub trait AgentHarness {
    /// Errors this harness can raise. Named by the implementation so a caller
    /// matching on them is matching on something real.
    type Error;

    /// Read one line of the Drone's output.
    ///
    /// **Total, and never empty.** There is no `Result`: a line that does not
    /// decode comes back as [`DroneEvent::Unreadable`], because a decoder that
    /// can return an error is a decoder whose caller can drop the line. A line
    /// this vocabulary has no name for comes back as
    /// [`DroneEvent::Unrecognised`] for the same reason.
    ///
    /// **One line can be more than one event.** A turn carrying two tool calls
    /// is two, and answering with only the first would drop work the Drone did.
    fn read(&self, line: &str) -> alloc::vec::Vec<DroneEvent>;

    /// The file an edit call named, where `event` is one: a call this harness
    /// knows changes a file by naming it. `None` for every other event.
    ///
    /// **A shell call is never one, and cannot be.** A shell line may write a
    /// file or only read one, and nothing in the stream says which, so a
    /// `sed -i` or a formatter's write goes unseen here — spike 022's answer
    /// 10, and the plan says so to the person. Which tool names edit is the
    /// harness's vocabulary, so the question is asked of it rather than
    /// answered from a list of names anywhere else.
    fn edited(&self, event: &DroneEvent) -> Option<alloc::string::String>;

    /// A file read or a command run, where `event` is a call that did one. `None` for every other
    /// event, an edit call included. **Which tools these are is the harness's vocabulary**, asked of
    /// it for the reason `edited` is.
    fn used(&self, event: &DroneEvent) -> Option<ToolUse>;

    /// How the edit call `edited` names changed its file: written whole, or in part. **An edit unless
    /// the harness says otherwise**, since most of what changes a file changes part of it.
    fn edit_act(&self, _event: &DroneEvent) -> FileAct {
        FileAct::Edit
    }
}

/// Version control, as Fleet uses it.
///
/// # It has no push method, and that is not the only thing missing
///
/// **No push.** Push, pull request and merge are [`Delivery`]'s, held by Fleet
/// with the operator's credentials. A capability absent from the type cannot be
/// reached by a Drone that reasons its way around a denial. What a Drone itself
/// is handed is a narrower type again, which is neither trait — and it carries
/// no `git` at all, which is why the commit below is here.
///
/// **No removal of a Job's worktree, and no way to ask for one.** Removal is
/// driven by Job retention, never by a process ending — so the caller that
/// would decide is the retention pass. Rather than shipping a method nothing
/// may call, the method does not exist: a worktree survives every terminal
/// state because nothing in the workspace can delete one, not because everybody
/// remembered not to. A person removes them through `armada clean`, which
/// reaches `adapters::reclaim` and not this trait.
/// [`drop_base_checkout`](Vcs::drop_base_checkout) is not an exception: a base
/// checkout holds no work, and making it again is the whole of undoing the
/// delete.
///
/// **No "does it already exist" query.** Creation answers that itself, by
/// refusing; a separate probe would be a check-then-act that two Jobs can
/// interleave.
pub trait Vcs {
    /// Errors creating a worktree can raise. Named by the implementation, so a
    /// caller matching a branch collision against a disk failure is matching on
    /// something real rather than on a rendered sentence.
    type Error;

    /// Errors committing can raise, and **a second type rather than more
    /// variants of the first.** Creation fails at approval with no Drone and no
    /// work; a commit fails after a Job's Checks have passed and its work is on
    /// disk. Neither list is a list of things to do about the other.
    type CommitError;

    /// Create a Job's worktree on its own new branch, at the path its handle
    /// derives.
    ///
    /// **Fleet no longer calls this**: a Job leases a pool slot through
    /// [`lease_slot`](Vcs::lease_slot). It stays for the tests that cut a
    /// Job's tree as a fixture. A failure is the caller's to match on rather
    /// than something to log.
    ///
    /// The branch is derived from the Job id and is **new**: an existing branch
    /// of that name is refused, never checked out. v1 discovered why on a real
    /// machine — a worktree added onto an existing branch interleaves two Jobs'
    /// commits, and git's own refusal names the operation rather than the
    /// reason.
    fn create_worktree(&self, spec: &WorktreeSpec) -> Result<Worktree, Self::Error>;

    /// What commit this repository's base branch is at, right now.
    ///
    /// **A reading and not a checkout**, and it is separate from
    /// [`base_checkout`](Vcs::base_checkout) because the answer is what derives
    /// the [`BaseSpec`] — a caller cannot ask for the checkout without first
    /// being told which one it wants. That ordering is what makes a base branch
    /// that has moved reach a different directory rather than an out-of-date
    /// one; `basing` carries the argument.
    ///
    /// `declared` is `base:` from the Manifest, and the resolution is the
    /// implementation's — the same one [`Delivery`] uses, so a rebase and a
    /// photograph cannot disagree about which branch the base is. `None` back
    /// is a repository that names no base and has no `main` or `master` to
    /// infer one from: there is nothing to be *before*, which is a fact to
    /// report and not a failure.
    fn base_commit(
        &self,
        repo_root: &str,
        declared: Option<&str>,
    ) -> Result<Option<alloc::string::String>, Self::Error>;

    /// The repository's local branches, the base first and the rest by name.
    /// #1605, spike 022 slice 4.
    ///
    /// `declared` is `base:` from the Manifest, resolved as
    /// [`base_commit`](Vcs::base_commit) resolves it, so the branch marked
    /// base is the one a worktree is cut from when nobody says otherwise.
    fn branches(
        &self,
        repo_root: &str,
        declared: Option<&str>,
    ) -> Result<alloc::vec::Vec<BranchListed>, Self::Error>;

    /// Make the local branch `name` at the commit the local branch
    /// `start_point` is at, for a Job approved to start from a branch that
    /// does not exist yet. **Refused where `name` is already a branch**, so
    /// nothing a person made is moved.
    fn create_branch(
        &self,
        repo_root: &str,
        name: &str,
        start_point: &str,
    ) -> Result<(), Self::Error>;

    /// Check the repository out at one commit, detached, for every Job on that
    /// commit to share.
    ///
    /// **Idempotent, and that is the whole concurrency argument.** A checkout
    /// already at the spec's path and registered is answered as it stands
    /// rather than refused, so a second caller finds the first one's work
    /// instead of colliding with it. That is the opposite of
    /// [`create_worktree`](Vcs::create_worktree), which refuses an existing
    /// branch — a Job's worktree is a line of history two Jobs must never share
    /// and a base checkout is a photograph every Job may.
    ///
    /// **Detached, so no branch is derived and none is left behind.** Nothing
    /// commits here.
    ///
    /// **A [`BaseCheckout`] and never a [`Worktree`]**, so nothing that takes a
    /// Job's worktree can be handed this one by accident: `commit_all` and
    /// every `Delivery` method would each do something wrong with it, and a
    /// type they cannot accept is a stronger statement than a comment saying
    /// not to.
    fn base_checkout(&self, spec: &BaseSpec) -> Result<BaseCheckout, Self::Error>;

    /// Remove a base checkout and its administrative record.
    ///
    /// **The one removal on this trait, and it is here because nothing but
    /// Armada ever wrote in the directory.** The paragraph above about there
    /// being no removal is about a *Job's* worktree, which holds work a person
    /// may want; a base checkout holds a commit that is still in the
    /// repository and files a package manager wrote. Deleting one loses
    /// nothing that is not reproducible by making it again.
    ///
    /// A path that is not there is not an error — the reclaim that calls this
    /// walks a directory and races nothing but itself.
    fn drop_base_checkout(&self, spec: &BaseSpec) -> Result<(), Self::Error>;

    /// Commit everything in a Job's worktree onto the branch checked out in it.
    ///
    /// Called when a Job's **last** step advances, by Fleet, which is this
    /// trait's only holder. It is not on [`WorkProduct`]: the gate holds that
    /// one, and a gate that could commit could satisfy `diff_nonempty` on the
    /// Drone's behalf.
    ///
    /// Everything git can see is taken, untracked files included and ignored
    /// files excluded — the same set [`WorkProduct::changed_files`] counts, so
    /// what made `diff_nonempty` pass is what lands.
    fn commit_all(
        &self,
        worktree: &Worktree,
        message: &str,
        at: CommitTime,
    ) -> Result<Committed, Self::CommitError>;

    /// Commit only these paths, as the working directory holds them, on top of
    /// the branch tip in a Job's worktree.
    ///
    /// **Everything else is left exactly as it was** — staged, modified or
    /// untracked — and is not in the commit, which is the whole difference
    /// from [`commit_all`](Vcs::commit_all). Paths are repository-relative, and
    /// one the working directory no longer holds is committed as a deletion.
    /// Each path's index entry is brought to what was committed, so `git
    /// status` does not show it afterwards as a staged revert.
    ///
    /// [`Committed::NothingToCommit`] where the paths already match the tip.
    fn commit_paths(
        &self,
        worktree: &Worktree,
        paths: &[&str],
        message: &str,
        at: CommitTime,
    ) -> Result<Committed, Self::CommitError>;

    /// Clone `url` into `destination`, which git creates, and stop it if it
    /// is still running after `within`.
    ///
    /// **Whatever credentials git on this machine already has**, and nothing
    /// else: no prompt, because Fleet has no terminal to ask on. A destination
    /// that exists and is not empty is git's to refuse.
    fn clone_repository(
        &self,
        url: &str,
        destination: &str,
        within: core::time::Duration,
    ) -> Result<(), NotCloned>;

    /// Lease a slot from the repository's pool for one Job, held by
    /// `job_id`, with `spec`'s branch cut fresh from the base.
    ///
    /// **A Job that already holds a slot is handed that one, untouched**, so a
    /// dispatch that died between the lease and the record finds its own slot
    /// rather than resetting it or taking a second. [`SlotLeased::Full`] is
    /// every slot held and none by this Job; it waits, and is not an error.
    fn lease_slot(
        &self,
        pool: &SlotPool,
        spec: &WorktreeSpec,
        job_id: &str,
    ) -> Result<SlotLeased, Self::Error>;

    /// Lease a slot onto `branch`, which exists, at its tip, held by `job_id`.
    /// [`SlotLeased::Full`] is every slot held; it waits, and is not an error.
    /// **Not necessarily the slot the holder parked from.**
    fn lease_existing_slot(
        &self,
        pool: &SlotPool,
        branch: &str,
        job_id: &str,
    ) -> Result<SlotLeased, Self::Error>;

    /// Commit everything in the holder's slot to its own branch and give the
    /// slot back, pushing nothing: the work stays on the branch. Refused, the
    /// holder keeps the slot.
    fn park_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        job_id: &str,
    ) -> Result<SlotParked, SlotParkRefused>;

    /// [`park_slot`](Vcs::park_slot) for a slot a session Fleet hosts leased,
    /// which the pool holds under the session's id: the WIP commit names a
    /// session, and not a Job of that id.
    ///
    /// **A session rewrites its own branch inside its slot**, so this commits
    /// to the branch the checkout is on, not the one the lease names. Where
    /// it is on none or on the base, a `kept/` branch is cut for what there is
    /// to keep. Whatever still runs with its
    /// working directory in the slot is ended first, and nothing outside it.
    fn park_hosted_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        session_id: &str,
    ) -> Result<SlotParked, SlotParkRefused>;

    /// Commit everything in a slot a terminal session stood in to its branch
    /// and give it back, **only once the process that leased it has ended**:
    /// a holder still running is [`SlotParkRefused::HeldByAnother`], so a
    /// caller waiting for the end asks again. The WIP commit names the session.
    /// As [`park_hosted_slot`](Vcs::park_hosted_slot), it follows the branch the
    /// checkout is on and ends what still runs in the slot.
    fn park_ended_session_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        session_id: &str,
    ) -> Result<SlotParked, SlotParkRefused>;

    /// Commit everything in a slot held by an agent session to its branch and
    /// give the slot back, as [`park_slot`](Vcs::park_slot) does for a Job.
    /// **`holder` is who the person was shown**, as the pool names it
    /// (`nvim (pid 44698)`): a slot re-leased since is refused, not taken.
    /// The session's process is not touched.
    fn release_session_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        holder: &str,
    ) -> Result<SlotParked, SlotParkRefused>;

    /// Write a slot an agent session holds as `to_session`'s, as it stands: nothing is
    /// committed or released. **`from` is who holds it now, as the pool names it** (a
    /// process's `claude (pid 44698)`, a hosted session's `job <id>`); a slot re-leased
    /// since is refused. The new lease is held as a Job of `to_session`'s id.
    fn hand_session_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        from: &str,
        to_session: &str,
    ) -> Result<(), SlotParkRefused> {
        let _ = (pool, slot, from, to_session);
        Err(SlotParkRefused::NotLeased)
    }

    /// Where the slot a Job's record names stands now.
    fn slot_standing(&self, pool: &SlotPool, slot: u32, job_id: &str) -> SlotStanding;

    /// What slot `slot` holds now that the pull request from `merged`, the branch it was leased
    /// on, has merged. **Read only**: Fleet decides from it whether to give the slot back, and
    /// never gives back one that answers anything but [`SlotAfterMerge::Idle`]. An adapter that
    /// cannot read the slot answers `Moved`, which keeps it.
    fn slot_after_merge(&self, pool: &SlotPool, slot: u32, merged: &str) -> SlotAfterMerge {
        let _ = (pool, slot, merged);
        SlotAfterMerge::Moved
    }

    /// Give back the slot this Job holds, by the pool's rules: refused while
    /// the tree holds anything uncommitted, or commits its branch, the remote
    /// and the base all lack. Refused, the slot stays held and says why.
    fn release_slot(&self, pool: &SlotPool, slot: u32, job_id: &str) -> Result<(), SlotKept>;

    /// Lease a slot for `holder` to read pull request `pull_request` in: its head fetched from the
    /// forge and checked out there, on a branch of the pool's own that only this lease uses.
    /// **For a read and nothing else**: no work is expected in the slot, and none is kept when it
    /// is given back. [`ReviewBay::Full`] is every slot held; it waits. `Err` is the sentence for
    /// a head the forge would not give. A build with no slots to lease says so.
    fn lease_review_bay(
        &self,
        pool: &SlotPool,
        pull_request: u64,
        holder: &str,
    ) -> Result<ReviewBay, alloc::string::String> {
        let _ = (pool, pull_request, holder);
        Err(alloc::string::String::from("this build has no slot to read a pull request in"))
    }

    /// Give back a slot [`lease_review_bay`](Vcs::lease_review_bay) leased, **whatever a read left
    /// in it**, and delete the branch the lease cut. Refused only where the slot is no longer
    /// `holder`'s, or git would not move it.
    fn release_review_bay(&self, pool: &SlotPool, slot: u32, holder: &str) -> Result<(), alloc::string::String> {
        let _ = (pool, slot, holder);
        Ok(())
    }

    /// The paths the person's commits changed on the repository's default branch, newest first and
    /// each once: the last `commits` commits `authors` wrote, by name or by email. **The
    /// repository's own configured identity is added to `authors` here**, so a person whose commits
    /// carry a name the forge does not know is still found. History is read and nothing is
    /// written; a repository that will not answer has none.
    fn paths_touched_by(
        &self,
        pool: &SlotPool,
        authors: &[alloc::string::String],
        commits: usize,
    ) -> alloc::vec::Vec<alloc::string::String>;

    /// Delete the local branch an act cut for itself, once its work is
    /// committed or the act ended failed. **Handed the spec the branch was cut
    /// from and never a name**, so it can delete one act's branch and no other;
    /// a branch already gone is not an error. Refused while a slot still has it
    /// checked out, and the branch stays. `true` where it was deleted now and
    /// `false` where it was gone already.
    fn delete_repair_branch(&self, spec: &WorktreeSpec) -> Result<bool, BranchKept>;

    /// Delete a local branch a session worked on, once the base holds every
    /// commit on it. **Fleet's act at a session's `finish_up`, and nothing
    /// else's**: a branch with commits the base lacks, or one a slot still has
    /// checked out, is refused and stays. `true` where it was deleted now and
    /// `false` where it was gone already.
    fn delete_merged_branch(&self, pool: &SlotPool, branch: &str) -> Result<bool, BranchKept>;

    /// Whether the base already holds every commit on a local branch. **What lets a session
    /// delete its own branch by hand**: the read half of [`Vcs::delete_merged_branch`], which
    /// deletes nothing.
    fn branch_is_merged(&self, pool: &SlotPool, branch: &str) -> bool;

    /// Every slot in the pool, as `armada worktree --status` reads it.
    fn slot_pool(&self, pool: &SlotPool) -> alloc::vec::Vec<SlotReading>;

    /// [`slot_pool`](Vcs::slot_pool) for a reader that keeps what it reads:
    /// a slot nothing has been seen to touch since it was read comes from
    /// memory. `full` reads every slot again, which is how a reader makes
    /// sure a touch it was never told of is found by its own deadline.
    ///
    /// An adapter that sees no touches reads every slot, as `slot_pool` does.
    fn slot_pool_kept(&self, pool: &SlotPool, full: bool) -> alloc::vec::Vec<SlotReading> {
        let _ = full;
        self.slot_pool(pool)
    }

    /// Add, remove, close or reopen a slot on this machine, answering the
    /// slot it changed. The CLI's lease honours it as Fleet's does.
    fn change_slot_pool(&self, pool: &SlotPool, change: SlotChange) -> Result<u32, SlotRefused>;

    /// What a slot held by an agent session holds uncommitted, which a release
    /// commits. Read only: nothing here rescues it. Refused for any slot a
    /// session does not hold.
    fn session_work(&self, pool: &SlotPool, slot: u32) -> Result<StrandedWork, RescueRefused> {
        let _ = (pool, slot);
        Err(RescueRefused::NotStranded(alloc::string::String::from(
            "not held by a job",
        )))
    }

    /// What a stranded slot holds, refused for any slot that is not stranded.
    fn stranded_work(&self, pool: &SlotPool, slot: u32) -> Result<StrandedWork, RescueRefused>;

    /// The stranded slot's change against where its branch left the base,
    /// uncommitted changes to tracked files included. What a rescue Scout is
    /// handed, since it cannot run git.
    fn stranded_diff(
        &self,
        pool: &SlotPool,
        slot: u32,
    ) -> Result<alloc::string::String, RescueRefused>;

    /// Scrap or stash a stranded slot's work, and free the slot. Only a person
    /// asks for either.
    fn rescue_slot(
        &self,
        pool: &SlotPool,
        slot: u32,
        rescue: SlotRescue,
    ) -> Result<SlotRescued, RescueRefused>;

    /// How many times this adapter has changed a slot, a checkout or a branch
    /// the pool holds, since the process began. Only ever larger.
    ///
    /// **A tick for a reader that keeps a copy of what the pool shows**, so it
    /// can notice the pool moved without having been told by whichever caller
    /// moved it. An adapter that does not count answers 0 for ever, and that
    /// reader then falls back to its timer.
    fn changes(&self) -> u64 {
        0
    }
}

/// Credential access, brokered. A Drone never holds a secret directly, and what
/// comes back is a [`Secret`], which cannot be printed.
pub trait Secrets {
    type Error;

    fn resolve(&self, key: &str) -> Result<Secret<alloc::string::String>, Self::Error>;
}

/// One thing that can be up or down: version control, the container runtime,
/// the agent CLI, the credential store, the database, the host's own resources.
///
/// Which product each of those is, is `adapters`' business and not this crate's
/// — naming one here is how the seam stops being a seam.
///
/// It is a trait so `testkit` can fake an unhealthy container runtime. Without
/// it, every health failure path is untestable — which is the whole reason
/// Doctor's grid is worth having.
pub trait HealthProbe {
    /// The one shape every probe returns, so no crate reaches across a boundary
    /// to read another's status.
    fn probe(&self) -> HealthReport;
}

// The payload types below are named by the traits and defined where the domain
// lives. Declared here as opaque re-exports once `core-model` carries them.
pub use placeholders::*;

#[doc(hidden)]
mod placeholders {
    //! **Not the design.** These names are what the traits above take and
    //! return, and they belong in `core-model` beside the rest of the domain.
    //! They are not written yet because M1 step 1 owns the Job record and its
    //! neighbours, and fixing their shape here would settle that by accident.
    //!
    //! They exist as uninhabited types so the traits compile and the seam is
    //! readable. Replace them, do not grow them.
    //!
    //! **`WorktreeSpec` and `WorktreePath` were two of them and are gone.** M1
    //! step 7 replaced them with real types in [`worktree`](super::worktree),
    //! not in `core-model` — the domain crate takes no dependency on a
    //! filesystem layout, and this crate must derive the path without one.
    //!
    //! **`DroneSpawnConfig` and `DroneHandle` are gone too**, replaced by M1
    //! step 8 in [`harness`](super::harness), for the same reason and in the
    //! same place: what a Drone is confined to is derived from a worktree and a
    //! toolbelt, and neither is a domain fact.
    //!
    //! **`ModelRequest` and `ModelResponse` are gone**, replaced by
    //! [`judge`](super::judge)'s `Ask` and `JudgeCall`: what a one-shot call
    //! carries is a rendering question, not a domain one.

    macro_rules! not_yet {
        ($($name:ident),* $(,)?) => {$(
            #[derive(Debug)]
            pub enum $name {}
        )*};
    }

    not_yet!(HealthReport);
}
