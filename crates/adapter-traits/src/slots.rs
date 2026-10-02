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

/// Why a Job's slot was not given back, in a sentence. The slot stays held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotKept(pub String);
