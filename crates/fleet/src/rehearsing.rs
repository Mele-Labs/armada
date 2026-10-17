//! A person's run of one Manifest entry — Journey 9, *Running one* and
//! *Running one inside a Job* — and what it leaves behind.
//!
//! **A rehearsal, never a verdict.** Nothing here writes Evidence or a
//! `job_step_checks` row, and nothing moves a Job: the run's own directory
//! under `.armada/runs` is its only record ([`records`]).
//!
//! **Off the turn loop**, for `crate::showing_again`'s reason: the run is a
//! task of its own, and one run at a time per owner is [`Rehearsals`]'s.
//!
//! **Keyed on an owner — a Job, or the main checkout.** The act itself is
//! [`shared`]'s and takes a [`Place`](owner::Place), never a `JobId`. What the
//! owners do not share is at the edges — the entrances here and in
//! [`checkout`], and the shapes [`record`] projects.

mod checkout;
mod entries;
mod in_flight;
mod owner;
mod record;
/// `pub(crate)` because a server's log is a run's log one directory over, read
/// and followed the same way — `crate::servers`.
pub(crate) mod records;
mod running;
mod served;
mod shared;
mod unrehearsable;
/// Verify: setup and every Check once, as a sequence of checkout runs.
mod verifying;
/// Which file a Verify runs: the root's, or one workspace's below it.
mod workspace;

use std::time::Duration;

use in_flight::Held;
pub(crate) use in_flight::Rehearsals;
pub use unrehearsable::{Unrehearsable, Whose};
pub use verifying::verify_steps;
#[cfg(test)]
pub(crate) use workspace::claimant as workspace_claimant;

/// How long a stop waits for a stopped run's record. The group is ended with
/// `SIGKILL`, so what is left is one tree read and one file write.
const STOPPING: Duration = Duration::from_secs(30);

/// What one viewer of a run's socket is answered with, before either owner's
/// opening message is built from it.
pub(crate) struct Seen {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) live: Option<api::RunWatch>,
    pub(crate) history: Vec<String>,
    pub(crate) skipped: u64,
    pub(crate) read_to: u64,
    pub(crate) unreadable: bool,
}
