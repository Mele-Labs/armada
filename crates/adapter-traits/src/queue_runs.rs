//! The merge queue's own CI runs, and what a failed job of one says went wrong.
//!
//! **The forge's facts and nothing else**, as [`main_ci`](crate::QueueEntry)'s are: the sentences a
//! forge writes for "no machine took the job" and "the job ran out of time" are the adapter's, and
//! arrive here as [`InfraCause`].

use alloc::vec::Vec;

use crate::{FromOutside, Oid};

/// One run the queue started and the forge has not finished: its id, the branch the queue made
/// for it, the commit it tests, and the pull request the branch names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueRun {
    /// The forge's id for the run, which cancelling it takes.
    pub id: u64,
    pub branch: FromOutside,
    /// The commit the run tests: the queue's group commit, which is the entry's own head while
    /// the entry stands.
    pub head: Option<Oid>,
    /// The pull request the branch is named for. `None` for a branch that names none.
    pub number: Option<u64>,
}

/// Why a job of a queue run failed, where the forge's annotations say. **`Other` is anything
/// else, including no annotation at all**: nothing is called the forge's fault that it did not
/// say was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InfraCause {
    /// No runner took the job.
    NoRunner,
    /// The job ran past the time the forge gives one.
    TimedOut,
    /// Something in the job itself, or nothing said.
    Other,
}

/// One failed job on a commit, with why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfraFailure {
    pub job: FromOutside,
    /// Where a person reads the job. `None` where the forge named none.
    pub log_url: Option<FromOutside>,
    pub cause: InfraCause,
}

/// The queue's unfinished runs of a base. `None` from the call that returns them is a forge that
/// would not answer; an empty list is a queue running nothing.
pub type QueueRuns = Vec<QueueRun>;
