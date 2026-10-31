//! The slots of a repository's worktree pool, who holds each, and what a person
//! can do with one that is stranded.
//!
//! The wire form of `crates/fleet/src/leasing.rs` and `rescuing.rs`.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};

/// Every slot of each served repository's pool, as `armada worktree --status`
/// reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreesHeld {
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
    /// Commits on its checkout the base does not have. Absent where git could
    /// not count them. Since 30 Oct 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ahead: Option<u32>,
    /// A person closed it: no lease takes it until it is reopened, and a
    /// holder keeps it until its lease ends. Since 23.17.
    #[serde(default)]
    pub closed: bool,
    /// What a slot holds that a Scrap would lose. Present where `held` is
    /// `stranded`, and where an agent session holds it and its checkout has
    /// uncommitted files, as `uncommitted` alone: what a `release` commits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stranded: Option<SlotStranded>,
    /// What a rescue Scout read of a stranded slot, while it reads and after.
    /// Since 23.29.
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
    /// Who held the slot last, where Fleet can say.
    /// Absent where the record names none, as for a terminal Fleet does not
    /// host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_holder: Option<SlotLastHolder>,
    /// When Fleet told that holder to commit, push and release. Fleet tells
    /// once a stranding, and only a holder that is `open`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub told: Option<Instant>,
}

/// The Session a stranded slot was last held for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotLastHolder {
    pub kind: SlotHolderKind,
    /// The Session's id.
    pub id: String,
    /// Its title, where the ledger or the store still has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A Session still live. Fleet tells only an open Session; a holder that
    /// is not open is gone, and a rescue Scout reads
    /// the slot without a press.
    pub open: bool,
}

/// What a slot's last holder was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotHolderKind {
    Session,
}

/// One commit on a stranded slot's branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotCommit {
    pub sha: String,
    pub subject: String,
    /// Whether it exists anywhere but this slot. Since 23.29.
    pub home: CommitHome,
}

/// Where else a commit on a stranded slot exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitHome {
    /// On no remote branch and not on the local base.
    OnlyHere,
    /// On a remote branch.
    OnRemote,
    /// On the local base.
    OnMain,
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

/// What a rescue Scout concluded of a stranded slot's work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotVerdict {
    /// The work has a part left to do.
    Unfinished,
    /// Leftovers that need no more work.
    Scraps,
}

/// What a rescue Scout read of a stranded slot: the Finding, kept against the
/// slot. Since 23.29.
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
    /// Whether the work still has a part left to do, or is leftovers. Absent
    /// while the Scout has not answered, and where its answer was not the
    /// shape asked for. Since 23.29.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<SlotVerdict>,
    /// Under `unfinished`, what is left to do, a line each. Under `scraps`, one
    /// line saying what the leftovers are. Since 23.29.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
    /// What it said last, where that was not the shape asked for. Absent while
    /// it has said nothing, and where `verdict` carries the answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Why it failed, where it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// `rescue_slot`'s body: one act on a stranded slot. Since 23.29.
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
    /// Stash, then start a Session that continues from the branch.
    PickUp,
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
    /// For `release`: the holder the person was shown, as the slot named it
    /// (`nvim (pid 44698)`). A slot held by anyone else now is refused.
    /// Since 23.37.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
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
    /// Commit what an agent session holds uncommitted in this slot to its
    /// branch and give the slot back. Since 23.37.
    Release,
}

/// The slot `change_slot_pool` changed: the new one, for `add`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotPoolChanged {
    pub manifest_id: ManifestId,
    pub slot: u32,
    /// What a `release` did to the branch. Absent for every other act.
    /// Since 23.37.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released: Option<SlotReleased>,
}

/// What was committed to a slot's branch so that freeing the slot loses
/// nothing. Nothing is pushed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReclaimedSaved {
    /// The WIP commit, on the slot's branch.
    pub commit: String,
    /// The files it took, as git names them.
    pub files: Vec<String>,
}

/// The branch a released slot's work is on, and what was committed to it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotReleased {
    pub branch: String,
    /// The WIP commit and its files. Absent where the slot held nothing
    /// uncommitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved: Option<ReclaimedSaved>,
}

/// Who holds a slot, or why nothing can. Tagged on `state`.
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
    /// A process outside Fleet, as `ps` names it: `zsh (pid 4120)`.
    Session {
        holder: String,
        /// The Session Fleet hosts whose agent, or something it started, this
        /// process is: a subagent's bay names the Session it began from.
        /// Absent for a terminal Fleet does not host.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
        /// That Session's title, where it has one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_title: Option<String>,
    },
    /// A Session Fleet hosts, holding the slot under its own id.
    HostedSession {
        session_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_title: Option<String>,
    },
    /// Its holder is gone and it still holds work.
    Stranded {
        why: String,
    },
}
