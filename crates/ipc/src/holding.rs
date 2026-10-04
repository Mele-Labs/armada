//! What Fleet is holding disk for, and the test each one did not pass. The
//! wire form of `crates/fleet/src/holding.rs`, which is the definition.
//!
//! **Nothing here says how large a worktree is.** Bytes are not the decision —
//! which commits go, whether anything else has them, and which uncommitted
//! files exist nowhere but that checkout is — and a size beside those would be
//! the figure read first and meaning least.
//!
//! **A piloted worktree is not among `worktrees`**, so there is no variant
//! below to render by mistake. `#367`, and Fleet drops it through
//! `Holding::offerable` rather than trusting a client with a flag. Its pool
//! slot is still in `slots`, as held by its Job: a held slot offers no reclaim,
//! only `change_slot_pool`'s close, which leaves its holder be.

use serde::{Deserialize, Serialize};

use crate::enums::JobStatus;
use crate::ids::{Instant, JobId, ManifestId};

/// Every worktree Fleet is holding disk for, ordered by Job id.
///
/// **Complete, including the ones the sweep will take on its own.** A list
/// filtered to the held ones would be a list the sweep is about to change, and
/// a person who came looking for a worktree that is not on it would have no way
/// to tell "already gone" from "Fleet has it and will not say".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreesHeld {
    pub worktrees: Vec<WorktreeHeld>,
    /// Every slot of each served repository's pool, as `armada worktree
    /// --status` reads it. Since 23.16.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slots: Vec<WorktreeSlot>,
}

/// One slot of a repository's worktree pool.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeSlot {
    /// The Manifest whose repository the pool belongs to.
    pub manifest_id: ManifestId,
    pub slot: u32,
    pub path: String,
    pub held: SlotHolding,
    /// The branch `behind` is counted against.
    pub base: String,
    /// The branch it is on. Absent for a free slot, which sits detached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// When its holder took it, where that was recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<Instant>,
    /// Every `setup.seed.paths` entry is a directory in the slot.
    pub warm: bool,
    /// Commits on the base its checkout does not have. Absent where git could
    /// not count them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behind: Option<u32>,
    /// A person closed it: no lease takes it until it is reopened, and a
    /// holder keeps it until its lease ends. Since 23.17.
    #[serde(default)]
    pub closed: bool,
    /// What a stranded slot holds, which a Scrap would lose. Present only where
    /// `held` is `stranded`. Since 23.19.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stranded: Option<SlotStranded>,
    /// What a rescue Scout read of a stranded slot, while it reads and after.
    /// Since 23.19.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rescue: Option<SlotFinding>,
}

/// The work a stranded slot holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotStranded {
    /// Every path `git status` reports, untracked included.
    pub uncommitted: Vec<String>,
    /// Commits the base does not have, newest first.
    pub commits: Vec<SlotCommit>,
    /// Of those, how many are on neither the remote nor the base.
    pub unpushed: u32,
}

/// One commit on a stranded slot's branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotCommit {
    pub sha: String,
    pub subject: String,
}

/// Where a rescue Scout is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotFindingState {
    Reading,
    Answered,
    Stopped,
    Failed,
}

/// What a rescue Scout read of a stranded slot: the Finding, kept against the
/// slot. Since 23.19.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotFinding {
    pub state: SlotFindingState,
    /// The commit the slot was at when the Scout read it.
    pub commit: String,
    /// Whether uncommitted changes were on top of it.
    pub uncommitted: bool,
    /// Characters of the change dropped before the Scout was handed it. `0`
    /// where it got all of it.
    #[serde(default)]
    pub cut: u64,
    /// Every file it read, in the order first read.
    pub read: Vec<String>,
    /// Every search it ran.
    pub searched: Vec<String>,
    /// What it said last. Absent while it has said nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Why it failed, where it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// `rescue_slot`'s body: one act on a stranded slot. Since 23.19.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RescueSlot {
    pub act: RescueAct,
    pub slot: u32,
}

/// What a person does with a stranded slot. `start` and `stop` are the rescue
/// Scout; the other two are Fleet's, on the Finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RescueAct {
    /// Start a Scout reading the slot.
    Start,
    /// Stop it.
    Stop,
    /// Discard the work and free the slot.
    Scrap,
    /// Commit the uncommitted work, push the branch and free the slot.
    Stash,
}

/// What `rescue_slot` did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotRescued {
    pub manifest_id: ManifestId,
    pub slot: u32,
    /// The branch the slot was on, for a Scrap or a Stash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// A Scrap kept the branch, because it holds commits nothing else has.
    #[serde(default)]
    pub branch_kept: bool,
    /// The commit a Stash made of the uncommitted work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub committed: Option<String>,
}

/// `change_slot_pool`'s body: one change to a repository's pool, on this
/// machine only. Since 23.17.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSlotPool {
    pub act: SlotAct,
    /// The slot acted on. Absent for `add`, which picks its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
}

/// What a person does to the pool from Cleanup's bay grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotAct {
    /// One more slot, not made until a lease makes it.
    Add,
    /// Refused unless the slot is free or not made.
    Remove,
    Close,
    Open,
}

/// The slot `change_slot_pool` changed: the new one, for `add`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotPoolChanged {
    pub manifest_id: ManifestId,
    pub slot: u32,
}

/// Who holds a slot, or why nothing can. Tagged on `state`; widening it is a
/// major bump, for `HeldReason`'s reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SlotHolding {
    /// Never made. The next lease makes it.
    Unmade,
    /// A directory that is not a checkout. Nothing leases it until a person
    /// removes it.
    NotACheckout,
    /// A take or a release is under way.
    Busy,
    Free,
    /// One of Fleet's Jobs. The title is absent where the store no longer
    /// has the Job.
    Job {
        job_id: JobId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        job_title: Option<String>,
    },
    /// A process outside Fleet, as `ps` names it: `zsh (pid 4120)`.
    Session {
        holder: String,
    },
    /// Its holder is gone and it still holds work.
    Stranded {
        why: String,
    },
}

/// One Job's worktree, and every test it failed.
///
/// The Job is named as a person reads it — the title and where it got to —
/// because the id alone is a ULID and the decision is about work somebody
/// remembers doing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeHeld {
    pub job_id: JobId,
    pub job_title: String,
    pub status: JobStatus,
    /// When Armada last moved anything on this Job.
    ///
    /// **Not when the files in the checkout were last written**, and nothing
    /// on this seam can be: the dirty reading is `git status --porcelain`,
    /// which answers names and not times. It is a floor — a checkout whose Job
    /// stopped four days ago has been sitting at least that long — and it is
    /// carried because *twenty minutes* and *four days* are different decisions
    /// on the one reason where reclaiming destroys something.
    pub last_moved_at: Instant,
    /// The checkout on disk. **What a person goes and looks at**, and the one
    /// value here that is worth copying.
    pub path: String,
    /// Whether the checkout is still there. False exactly when
    /// `adapters::standing` reads it absent. Since 13.32.
    pub on_disk: bool,
    /// The branch the Job derived. Named even where it is already gone: it is
    /// what the commits are recoverable from.
    pub branch: String,
    /// **Empty is the whole of the safety claim.** Fleet's own sweep takes
    /// exactly the empty ones, so a row with nothing here is a row nobody has
    /// to decide about.
    pub held: Vec<HeldReason>,
}

impl WorktreeHeld {
    /// Whether Fleet will give this one back without being asked.
    pub fn provably_safe(&self) -> bool {
        self.held.is_empty()
    }
}

/// One test a worktree did not pass.
///
/// **Tagged on `why`, and each arm carries what its own decision needs.** The
/// count of commits without the branch they are on, or a claim of uncommitted
/// work without the filenames, is a row that asks somebody to guess.
///
/// **Not a `wire_enum`, because there is no registry for it.** `enums`'s rule
/// is a `core-model` key spelled through `as_wire`, and `reclaimed` already
/// argued the exception for this kind of value: the set is a reading of git and
/// of Fleet's scheduler, not a state the Job machine can be in.
///
/// **Widening it is a major bump.** Bridge branches on `why` to pick which
/// facts to show, which is `docs/practices/protocol.md`'s row for a variant the
/// other side matches on — `FleetCapacity.held_by`'s caveat does not reach it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum HeldReason {
    /// The Job is still moving, so it may still need its worktree. **Nothing
    /// may reclaim this one** — `reclaim_worktree` refuses a status that is not
    /// terminal, and a surface that offered it would be offering a 409.
    NotTerminal { status: JobStatus },
    /// The branch holds commits the base cannot reach.
    ///
    /// **Reclaiming does not destroy them.** There is no force on this seam, so
    /// the checkout goes and the branch stays exactly where it is — which is
    /// what makes this the reason a person can act on most freely, and why the
    /// tip travels: it is what the work is reachable from afterwards.
    Unmerged {
        base: String,
        commits: u32,
        tip: String,
    },
    /// Nothing could say what the branch would be merged into, so nothing can
    /// say whether it holds a copy of anything. Kept, for the same reason an
    /// unmerged branch is kept: the cost of guessing wrong is a lost commit.
    BaseUnanswered { detail: String },
    /// Files written and committed nowhere.
    ///
    /// **The one reason where reclaiming destroys something.** Uncommitted work
    /// leaves a branch level with its base, so every merged-ness reading says
    /// the checkout is disposable — and no branch carries these, so removing
    /// the directory is the end of them. Named file by file for that reason.
    Uncommitted { files: Vec<String> },
    /// Somebody locked the checkout, which is a person saying not yet. The
    /// reclaim leaves a locked worktree alone and says so.
    Locked { reason: String },
    /// A Job that depends on this one has not finished, so it may still need
    /// what this one wrote.
    DependedOn { by: Vec<JobId> },
    /// git would not say what is in the checkout. **Unanswered and clean must
    /// never read alike**, because only one of them can be taken back.
    Unreadable { detail: String },
}
