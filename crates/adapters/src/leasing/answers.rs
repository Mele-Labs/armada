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
    pub(super) made: bool,
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

    /// Whether this lease made the slot, rather than taking one already made
    /// with the last lease's build in it.
    pub fn made(&self) -> bool {
        self.made
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
    /// A person closed it, so no lease takes it until it is reopened.
    pub closed: bool,
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
        /// Why the holder's release was refused, where a Job ended and could
        /// not give the slot back.
        kept: Option<String>,
        /// The holder is a Job that completed, which holds its slot until a
        /// person clears it.
        completed: bool,
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
    /// Leasing an existing branch: there is no such local branch.
    NoSuchBranch(String),
    /// Leasing an existing branch: another checkout has it, and git lets one
    /// checkout have a branch.
    CheckedOutElsewhere { branch: String, at: PathBuf },
    /// The version-control command refused, and this is what it said.
    Vcs(String),
}

impl LeaseRefused {
    /// One sentence, for a person.
    pub fn said(&self) -> String {
        match self {
            LeaseRefused::BranchHoldsWork { branch, commits } => format!(
                "{branch} already exists with {commits} commits on neither the remote nor the \
                 base, and a lease would reset it"
            ),
            LeaseRefused::NoSuchBranch(branch) => {
                format!("there is no branch {branch} here to lease")
            }
            LeaseRefused::CheckedOutElsewhere { branch, at } => format!(
                "{branch} is already checked out at {}. Give that checkout back first",
                at.display()
            ),
            LeaseRefused::Vcs(why) => why.clone(),
        }
    }
}

/// Why a release let nothing go.
#[derive(Debug)]
pub enum ReleaseRefused {
    /// The path is not one of this pool's slots.
    NotASlot(PathBuf),
    NotLeased(PathBuf),
    /// Somebody else holds it, named for a person.
    HeldByAnother(String),
    Dirty {
        path: PathBuf,
        files: Vec<String>,
    },
    /// HEAD carries commits the lease's branch lacks, and neither the remote
    /// nor the base has them: a release would leave them on no branch.
    OffTheBranch {
        branch: String,
        commits: usize,
    },
    Vcs(String),
}

impl ReleaseRefused {
    /// One sentence, for a person and for the slot's own record.
    pub fn said(&self) -> String {
        match self {
            ReleaseRefused::NotASlot(path) => format!(
                "{} is not one of this repository's worktree slots",
                path.display()
            ),
            ReleaseRefused::NotLeased(path) => format!(
                "{} is not leased, so there is nothing to give back",
                path.display()
            ),
            ReleaseRefused::HeldByAnother(holder) => format!("{holder} holds it"),
            ReleaseRefused::Dirty { path, files } => format!(
                "{} has {} uncommitted, first {}. Commit or remove them, then release",
                path.display(),
                files.len(),
                files.first().map(String::as_str).unwrap_or_default()
            ),
            ReleaseRefused::OffTheBranch { branch, commits } => format!(
                "the checkout has {commits} commits that {branch} lacks, and the remote and \
                 the base lack them too. Put them on {branch}, then release"
            ),
            ReleaseRefused::Vcs(why) => why.clone(),
        }
    }
}

/// What a release gave back.
#[derive(Debug)]
pub struct Released {
    pub slot: usize,
    pub branch: String,
}

/// What a park committed and freed.
#[derive(Debug)]
pub struct Parked {
    pub slot: usize,
    /// The branch the work is on, and stays on.
    pub branch: String,
    /// The WIP commit, with the paths it took; `None` where the slot was
    /// clean and only released.
    pub committed: Option<Committed>,
}

/// The commit a park made.
#[derive(Debug)]
pub struct Committed {
    pub commit: String,
    pub files: Vec<String>,
}

/// Why a park changed nothing, or committed and could not free the slot.
#[derive(Debug)]
pub enum ParkRefused {
    NotASlot(PathBuf),
    NotLeased(PathBuf),
    /// Somebody else holds it, named for a person.
    HeldByAnother(String),
    /// A take or a release is under way on it.
    Busy,
    /// The checkout is detached, so there is no branch to park onto.
    OnNoBranch,
    /// The checkout is on the base itself.
    OnTheBase(String),
    /// The checkout is on a branch other than the one the lease names.
    OnAnotherBranch {
        leased: String,
        on: String,
    },
    /// The work was committed and the release then refused. The commit stays.
    Release(ReleaseRefused),
    Vcs(String),
}

impl ParkRefused {
    /// One sentence, for a person.
    pub fn said(&self) -> String {
        match self {
            ParkRefused::NotASlot(path) => format!(
                "{} is not one of this repository's worktree slots",
                path.display()
            ),
            ParkRefused::NotLeased(path) => format!(
                "{} is not leased, so there is nothing to park",
                path.display()
            ),
            ParkRefused::HeldByAnother(holder) => format!("{holder} holds it"),
            ParkRefused::Busy => String::from("a lease or a release is under way on it"),
            ParkRefused::OnNoBranch => String::from(
                "the checkout is on no branch, so there is nothing to park the work onto",
            ),
            ParkRefused::OnTheBase(base) => {
                format!("the checkout is on {base}, the base itself. Park onto a branch of its own")
            }
            ParkRefused::OnAnotherBranch { leased, on } => format!(
                "the checkout is on {on}, and the lease names {leased}. Switch back to {leased}, \
                 then park"
            ),
            ParkRefused::Release(refused) => {
                format!(
                    "the work is committed, but the slot stays held: {}",
                    refused.said()
                )
            }
            ParkRefused::Vcs(why) => why.clone(),
        }
    }
}
