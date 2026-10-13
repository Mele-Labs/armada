//! The repair a Job's Commander sends onto the Job's own branch. `#2170`.
//!
//! **A closed set of what broke, and nothing else to say.** The Drone reads the
//! worktree itself; the body only names which of four faults the Commander
//! found, so the Job's log and the Commander's thread state it in a word and
//! the Drone is granted what that fault needs. There is no field for a branch,
//! a path or a command: the repair is the Job's, on the Job's own branch.

use serde::{Deserialize, Serialize};

use crate::ids::{DroneId, JobId, StepId};

/// What the Commander found broken in the Job's worktree or environment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Breakage {
    /// A merge or rebase left conflicts in the worktree.
    MergeConflict,
    /// A dependency is missing, will not install, or is at the wrong version.
    Dependencies,
    /// A binary a command needs is not on the machine or not on the path.
    MissingBinary,
    /// The worktree is unusable: a git lock, a half-finished operation, files
    /// the branch cannot check out.
    Worktree,
}

/// `POST /jobs/:job_id/commander/repair` — send a repair Drone onto this Job's
/// own branch and worktree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendRepairDrone {
    pub broke: Breakage,
}

/// What `send_repair_drone` answers with. **The Drone is on its way and has not
/// finished**: the step it was sent to unblock is restarted when it hands in,
/// and the Commander is woken where it does not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairSent {
    pub job_id: JobId,
    /// The step the breakage stopped, which restarts once the repair is in.
    pub step_id: StepId,
    pub drone_id: DroneId,
    pub broke: Breakage,
}
