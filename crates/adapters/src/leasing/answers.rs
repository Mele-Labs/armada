//! What the pool answers: a lease, a full pool, a slot's state, and why a
//! take or a release refused.

use std::path::{Path, PathBuf};

use super::record::Holder;

/// What a lease came to.
#[derive(Debug)]
pub enum Leased {
    Took(Lease),
    /// No slot could be taken now; each is named with why.
    Full(Full),
}

/// Every slot, when none was free.
#[derive(Debug)]
pub struct Full {
    pub slots: Vec<Slot>,
}

/// A slot that was taken.
#[derive(Debug)]
pub struct Lease {
    pub(super) slot: usize,
    pub(super) path: PathBuf,
    pub(super) reclaimed_from: Option<String>,
    pub(super) seeded_from: Option<String>,
    pub(super) unfetched: Option<String>,
}

impl Lease {
    pub fn slot(&self) -> usize {
        self.slot
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The branch a dead holder left on the slot, where this lease took it
    /// back.
    pub fn reclaimed_from(&self) -> Option<&str> {
        self.reclaimed_from.as_deref()
    }

    /// The base commit whose warm build a new slot was cloned from.
    pub fn seeded_from(&self) -> Option<&str> {
        self.seeded_from.as_deref()
    }

    /// Why the base was not fetched first, where it was not. The lease went
    /// ahead from what this machine last fetched.
    pub fn unfetched(&self) -> Option<&str> {
        self.unfetched.as_deref()
    }
}

/// One slot, as a lease or `--status` found it.
#[derive(Debug)]
pub struct Slot {
    pub number: usize,
    pub path: PathBuf,
    pub state: SlotState,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SlotState {
    /// Never made yet. The next lease makes it.
    Unmade,
    Free,
    /// A take or a release is under way on it this moment.
    Busy,
    Held {
        branch: String,
        holder: Holder,
        since: u64,
    },
    /// Its holder is gone and it is clean with nothing unlanded: the next
    /// lease takes it.
    Abandoned {
        branch: String,
        since: u64,
    },
    /// The directory is there and is not a checkout of its own. Nothing
    /// leases it until a person removes it.
    NotACheckout,
    /// Its holder is gone and it still holds work, so it stays held until a
    /// person lands or removes it.
    Stranded {
        branch: String,
        since: u64,
        why: String,
    },
}

/// Why a lease took nothing, other than every slot being held.
#[derive(Debug)]
pub enum LeaseRefused {
    /// The branch exists and carries commits on neither the remote nor the
    /// base. Resetting it to the base would orphan them.
    BranchHoldsWork { branch: String, commits: usize },
    /// The version-control command refused, and this is what it said.
    Vcs(String),
}

/// Why a release let nothing go.
#[derive(Debug)]
pub enum ReleaseRefused {
    /// The path is not one of this pool's slots.
    NotASlot(PathBuf),
    NotLeased(PathBuf),
    Dirty {
        path: PathBuf,
        files: Vec<String>,
    },
    Unlanded {
        branch: String,
        commits: usize,
    },
    Vcs(String),
}

/// What a release gave back.
#[derive(Debug)]
pub struct Released {
    pub slot: usize,
    pub branch: String,
}
