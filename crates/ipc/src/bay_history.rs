//! What happened to one slot of a repository's pool: who held it, and what was checked out in it.
//! `get_slot_history` in `crates/ipc/operations/`.
//!
//! **The side panel's Bay reads this, and nothing else draws from it.** Leases come from the pool,
//! which writes a line whenever it writes or clears a lease record, so a slot leased with `armada
//! worktree lease` is in it though that never reaches Fleet. Checkouts come from the slot's own
//! `HEAD` reflog. Both are kept thirty days.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};

/// What held a slot. **Strict**: Bridge picks a mark from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BayHolderKind {
    /// A Session Fleet hosts.
    Session,
    /// A process that took the lease itself, with `armada worktree lease`: an agent in a terminal, or
    /// a person.
    Agent,
}

/// Who held the slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BayHolder {
    pub kind: BayHolderKind,
    /// The Session's id. For an `agent`, the id of the agent Session its lease was taken
    /// for, else the process's pid.
    pub id: String,
    /// What it is called now, where Fleet still has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// One thing that happened to the slot. **Strict**, tagged by `kind`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BayHistoryEntry {
    /// The slot was leased.
    Leased {
        at: Instant,
        holder: BayHolder,
        /// The branch it was leased onto.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
    },
    /// The holder no longer holds it: it gave the slot back, or the lease was taken from a holder
    /// that had gone.
    Released { at: Instant, holder: BayHolder },
    /// `HEAD` moved from one branch or commit to another.
    Checkout {
        at: Instant,
        /// A branch, or the commit a detached head stood at.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<String>,
        /// A branch, or the commit it was detached at.
        to: String,
        /// The commit `HEAD` stood at once it moved.
        sha: String,
    },
}

/// What `get_slot_history` is asked, as its query string.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BayHistoryAsk {
    pub manifest_id: ManifestId,
    pub slot: u32,
}

/// The most entries one answer holds.
pub const MOST_BAY_HISTORY: usize = 300;

/// `get_slot_history`: the slot's entries, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BayHistory {
    pub manifest_id: ManifestId,
    pub slot: u32,
    pub entries: Vec<BayHistoryEntry>,
    /// Older entries inside the thirty days were left out to keep to [`MOST_BAY_HISTORY`]. **Stated
    /// rather than inferred from a full list**, so a slot with exactly that many is not read as cut.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
}
