//! What a Job's lease on a pool slot is asked with and answers.
//!
//! **A Job's lease is held by its id, never a process.** Fleet restarts often,
//! and a lease held for a pid would make every Job's slot read as abandoned
//! after one. A slot a Job holds is never taken back for a dead holder; it is
//! given back when the Job ends, or when a person clears a completed one, by
//! the pool's own rules.
//! `docs/concepts/fleet.md`, *Worktree slots*.

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
