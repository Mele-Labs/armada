//! What a Job's lease on a pool slot is asked with and answers.
//!
//! **A Job's lease is held by its id, never a process.** Fleet restarts often,
//! and a lease held for a pid would make every Job's slot read as abandoned
//! after one. A slot a Job holds is never taken back for a dead holder; it is
//! given back when the Job ends, or when a person clears a completed one, by
//! the pool's own rules.
//! `docs/concepts/fleet.md`, *Worktree slots*.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::worktree::Worktree;

/// One repository's pool, as its root Manifest sizes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotPool {
    repo_root: String,
    slots: u32,
    base: String,
    keep: Vec<String>,
}

impl SlotPool {
    /// `slots` slots under `repo_root`, cut from `base`. `keep` is
    /// `setup.seed.paths`, kept across leases with the build directories the
    /// pool always keeps.
    pub fn of(repo_root: &str, slots: u32, base: &str, keep: Vec<String>) -> SlotPool {
        SlotPool {
            repo_root: String::from(repo_root),
            slots: slots.max(1),
            base: String::from(base),
            keep,
        }
    }

    pub fn repo_root(&self) -> &str {
        &self.repo_root
    }

    pub fn slots(&self) -> u32 {
        self.slots
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn keep(&self) -> &[String] {
        &self.keep
    }
}

/// What asking for a slot came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotLeased {
    /// The Job holds `slot`, on its own branch. `reused` where the slot was
    /// already made, so its build directories are the last lease's.
    Took {
        slot: u32,
        worktree: Worktree,
        reused: bool,
    },
    /// Every slot is held, and none by this Job.
    Full,
}

/// Where a slot a Job's record names stands now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotStanding {
    /// The Job still holds it.
    Held,
    /// Something else holds it, named for a person.
    HeldBy(String),
    /// Nothing holds it: it was given back.
    Free,
    /// There is no such slot on disk.
    Gone,
}

/// One slot of the pool, as `armada worktree --status` reads it, with whether
/// it is warm and how far it is behind the base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotReading {
    pub slot: u32,
    pub path: String,
    pub held: SlotHeld,
    /// The branch it is on. `None` for a slot not made, and for one nothing
    /// recorded a branch for.
    pub branch: Option<String>,
    /// Seconds since the epoch its holder took it, where that was recorded.
    pub since: Option<u64>,
    /// Every `setup.seed.paths` entry is a directory in the slot. A Manifest
    /// that declares none has nothing to be warm with, so its slots are cold.
    pub warm: bool,
    /// Commits on the base its checkout does not have. `None` where git could
    /// not count them, or the slot is not a checkout.
    pub behind: Option<u32>,
    /// A person closed it: never leased until reopened. A holder keeps it
    /// until its lease ends.
    pub closed: bool,
    /// Why a Job's release was refused, for a slot a Job still holds after it
    /// ended. Its work is in the slot, so a person rescues it like a stranded
    /// one.
    pub kept: Option<String>,
    /// The Job holding it completed, and holds it until a person clears it.
    pub completed: bool,
}

/// Who holds a slot, or why nothing can.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotHeld {
    /// Never made. The next lease makes it.
    Unmade,
    /// A directory that is not a checkout, so nothing leases it until a person
    /// removes it.
    NotACheckout,
    /// A take or a release is under way on it.
    Busy,
    Free,
    /// One of Fleet's Jobs, by id.
    Job(String),
    /// A process, named for a person: `zsh (pid 4120)`.
    Session(String),
    /// Its holder is gone and it holds work, said in a phrase.
    Stranded(String),
}

/// Why a Job's slot was not given back, in a sentence. The slot stays held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotKept(pub String);

/// Why a repair's branch was not deleted, in a sentence. It stays where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchKept(pub String);

/// What parking a Job's slot kept and freed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotParked {
    /// The branch the work is on, and stays on.
    pub branch: String,
    /// The WIP commit, where there was uncommitted work to keep; `None` where
    /// the slot was clean and was only released.
    pub commit: Option<String>,
    /// The files that commit took, as git names them. Empty where `commit` is
    /// `None`.
    pub files: Vec<String>,
}

/// Why a park changed nothing, or committed and could not free the slot. The
/// Job keeps its slot in every case and the commit, where one was made, stays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotParkRefused {
    /// The checkout is detached, so there is no branch to park onto.
    OnNoBranch,
    /// The checkout is on the base itself.
    OnTheBase(String),
    /// The checkout is on a branch other than the one the lease names.
    OnAnotherBranch { leased: String, on: String },
    /// Somebody else holds the slot, named for a person.
    HeldByAnother(String),
    /// A lease or a release is under way on it.
    Busy,
    /// Released on behalf of a holder that no longer holds it: who does now,
    /// or `nobody`.
    HolderChanged(String),
    /// The slot is not one of the pool's, or nobody leased it.
    NotLeased,
    /// The work was committed and the release then refused.
    Release(String),
    /// git refused, and this is what it said.
    Vcs(String),
}

impl SlotParkRefused {
    /// One sentence, for a person.
    pub fn said(&self) -> String {
        match self {
            SlotParkRefused::OnNoBranch => String::from(
                "HEAD is detached, so there is no branch to commit the uncommitted files to",
            ),
            SlotParkRefused::OnTheBase(base) => format!(
                "the worktree is on {base}, the base branch, so the uncommitted files were not \
                 committed there"
            ),
            SlotParkRefused::OnAnotherBranch { leased, on } => format!(
                "the worktree is on branch {on}, not {leased}, so the uncommitted files were not \
                 committed"
            ),
            SlotParkRefused::HeldByAnother(who) => format!("{who} holds it"),
            SlotParkRefused::Busy => String::from("a lease or a release is under way on it"),
            SlotParkRefused::HolderChanged(now) => {
                format!("it is held by {now} now, not by the holder that was shown")
            }
            SlotParkRefused::NotLeased => {
                String::from("nothing leased it, so there is nothing to park")
            }
            SlotParkRefused::Release(why) => {
                format!("the uncommitted files were committed, but the slot stays held: {why}")
            }
            SlotParkRefused::Vcs(why) => why.clone(),
        }
    }
}

/// A person's change to the pool's shape on this machine. Kept beside the
/// slots and never committed, so `setup.worktrees` stays the default for a
/// fresh machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotChange {
    /// One more slot, not made until a lease makes it.
    Add,
    /// This slot, gone: refused unless it is free or not made.
    Remove(u32),
    /// Never leased until reopened.
    Close(u32),
    Open(u32),
}

/// Why a change to the pool's shape changed nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotRefused {
    NoSuchSlot(u32),
    /// Its holder, named for a person.
    Held(String),
    /// Its holder is gone and it holds work, said in a phrase.
    Stranded(String),
    /// A take or a release is under way on it.
    Busy,
    /// A directory that is not a checkout, which a person removes by hand.
    NotACheckout,
    /// Files git would lose, first ones first.
    Dirty(Vec<String>),
    /// The pool keeps one slot.
    LastSlot,
    Vcs(String),
}

/// What a stranded slot holds: what a Scrap would lose, and what a rescue
/// Scout is handed. `docs/concepts/fleet.md`, *Rescuing a stranded slot*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrandedWork {
    /// The branch it is on. `None` for a checkout left detached.
    pub branch: Option<String>,
    /// The commit checked out.
    pub commit: String,
    /// Every path `git status` reports, untracked included.
    pub uncommitted: Vec<String>,
    /// Commits the base does not have, newest first.
    pub commits: Vec<SlotCommit>,
    /// Of those, how many are on neither the remote nor the base.
    pub unpushed: u32,
}

/// One commit on a stranded slot's branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotCommit {
    pub sha: String,
    pub subject: String,
    /// Where else it exists.
    pub home: CommitHome,
}

/// Whether a commit exists anywhere but the slot holding it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitHome {
    /// On no remote branch and not on the local base.
    OnlyHere,
    /// On a remote branch.
    OnRemote,
    /// On the local base, pushed or not.
    OnMain,
}

/// What a person chose to do with a stranded slot's work. Each frees the slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotRescue {
    /// Discard the uncommitted files and put the slot back at the base. The
    /// branch is deleted only where the base or the remote has every commit
    /// on it.
    Scrap,
    /// Commit the uncommitted files to the branch with `message`, and push the
    /// branch to the remote under its own name.
    Stash { message: String },
}

/// What a rescue did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotRescued {
    /// The branch the slot was on.
    pub branch: Option<String>,
    /// A Scrap kept the branch, because it holds commits nothing else has.
    pub branch_kept: bool,
    /// The uncommitted files were committed: the commit, where a Stash made one.
    pub committed: Option<String>,
}

/// Why a rescue changed nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RescueRefused {
    NoSuchSlot(u32),
    /// The slot is not stranded, and what it is instead, in a word or two.
    NotStranded(String),
    /// A take or a release is under way on it.
    Busy,
    /// Stash names a branch, and the checkout is on none.
    OnNoBranch,
    /// The slot is on the base itself, which a Stash never pushes to.
    OnTheBase(String),
    /// Stash pushes, and the repository has no `origin`.
    NoRemote,
    Vcs(String),
}
