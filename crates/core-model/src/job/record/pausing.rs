//! A Job's pause marker, and the writers that set and clear it.
//!
//! **A pause is a stored marker and not a status.** A Job parked at a gate
//! keeps the status it reads at, and a running one goes `queued`, so the
//! marker is the only thing that says a person asked for it. No event carries
//! it, so the column is its own authority, as `worktree_slot` is. A child of
//! `record` for its private fields, as `approving` is.

use crate::envelope::Timestamp;

use super::Job;

/// Who paused a Job, and so who may lift it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PausedBy {
    Person,
    Fleet,
}

impl PausedBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            PausedBy::Person => "person",
            PausedBy::Fleet => "fleet",
        }
    }

    pub fn from_stored(text: &str) -> Option<PausedBy> {
        match text {
            "person" => Some(PausedBy::Person),
            "fleet" => Some(PausedBy::Fleet),
            _ => None,
        }
    }
}

/// The marker on a paused Job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pause {
    pub by: PausedBy,
    pub at: Timestamp,
    /// A resume found the pool full: the Job is waiting for a slot and takes
    /// the first one that frees. Only a Job that kept its gate status sets it;
    /// a queued one waits in admission's line instead.
    pub resuming: bool,
}

impl Job {
    /// The Job as paused: marker set, and its slot given up. **Both together,
    /// because a Job whose slot is gone and that does not say why reads as one
    /// whose slot was lost.**
    pub fn paused(&self, by: PausedBy, at: Timestamp) -> Job {
        let mut job = self.clone();
        job.pause = Some(Pause {
            by,
            at,
            resuming: false,
        });
        job.worktree_slot = None;
        job
    }

    /// The marker lifted. The slot is not touched: it is `in_slot`'s, written
    /// when the lease is.
    pub fn unpaused(&self) -> Job {
        let mut job = self.clone();
        job.pause = None;
        job
    }

    /// A resume that found no slot free, waiting for one. No marker, no change.
    pub fn resuming(&self) -> Job {
        let mut job = self.clone();
        if let Some(pause) = job.pause.as_mut() {
            pause.resuming = true;
        }
        job
    }

    /// The marker as stored, for a row read back.
    pub fn with_pause(&self, pause: Pause) -> Job {
        let mut job = self.clone();
        job.pause = Some(pause);
        job
    }

    pub fn pause(&self) -> Option<&Pause> {
        self.pause.as_ref()
    }

    /// Whether the Job has a worktree on disk that is parked on its branch: it
    /// has been dispatched, and holds no slot.
    pub fn is_parked(&self) -> bool {
        self.pause.is_some() && self.branch.is_some() && self.worktree_slot.is_none()
    }
}
