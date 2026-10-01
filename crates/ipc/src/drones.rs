//! The processes Fleet is holding, and what one of them has been doing.
//!
//! **Read off the roster, never off the Jobs.** A Job that escalated keeps its
//! Drone alive and idle so a redirect costs no respawn, so a list derived from
//! statuses would omit exactly the Drone somebody is asking about.
//!
//! What drifted outside a declaration is not here: it rides on `get_diff`'s
//! file list, where the whole Job's work is measured rather than one slot's.

use serde::{Deserialize, Serialize};

use crate::ids::{DroneId, Instant, JobId, StepId};
use crate::turn::TranscriptRow;

/// One Drone in a working slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneSummary {
    pub drone_id: DroneId,
    pub job_id: JobId,
    /// What a person calls the Job, and what the Drone's transcript file is
    /// named under.
    pub handle: String,
    /// The step it was put on. **It never moves**: a slot does not outlive a
    /// step boundary, so this is both where it started and where it is.
    pub step_id: StepId,
    /// The checkout it is writing in. **Absent where the worktree is gone** —
    /// a Drone outliving its checkout is a real state and not a blank string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
    /// The process Fleet is holding. **The fact a Doctor probe asked for** —
    /// which process is working which Job.
    pub pid: u32,
    /// When the Drone arrived on the step, off the Job's own log. **Absent
    /// where the log has no arrival for it**, which a reclaimed record gives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<Instant>,
}

/// Every Drone Fleet is holding.
///
/// **Read off the process register, never off the slots.** A slot is held for
/// the length of a Check, so a read that took one would block behind a gate;
/// what this walks is the pid map, which nothing holds across an await.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneList {
    pub drones: Vec<DroneSummary>,
}

/// One Drone, what it promised, and what it has said.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroneDetail {
    pub drone: DroneSummary,
    /// The paths the Drone said this step's work would be in, as the record
    /// kept them. **`None` until it declares**, which is a different answer
    /// from an empty declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared: Option<Vec<String>>,
    /// When that declaration was taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_at: Option<Instant>,
    /// This Drone's own rows, oldest last, narrowed for a viewer. **A window,
    /// not the transcript**: see [`DroneDetail::older`].
    pub turns: Vec<TranscriptRow>,
    /// How many older rows this window left out. **Non-zero means the answer
    /// is a tail**, and a caller that drew a conclusion about when something
    /// started from a tail would be reading the window rather than the Drone.
    pub older: u64,
}

/// Where one of a Job's Drones is, as `list_job_drones` reads it off the Job's
/// history.
///
/// **`killed` and `failed` are two answers, because they want two responses.**
/// `killed` is a person ending it: the step it was on stopped under
/// `drone_killed`, or the Job it worked was killed, as it left. `failed` is a
/// Drone that left without its step passing, on its own. `done` is one whose
/// step passed its advance gate, or reached a person's gate, while it was the
/// one on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DroneState {
    Running,
    Done,
    Failed,
    Killed,
}

/// One Drone a Job has had, running or not.
///
/// **Off the record, never the roster**, which is [`DroneSummary`]'s source
/// and the reason that list loses a Drone the moment it exits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDrone {
    pub drone_id: DroneId,
    /// The step it was put on. A slot does not outlive a step boundary, so a
    /// Drone has one.
    pub step_id: StepId,
    pub state: DroneState,
    /// When it was spawned onto the step, off `drone_spawned`.
    pub since: Instant,
    /// When it left, off `drone_exited`. **Absent while it runs.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// How many turns it has taken, summed across every terminating line of
    /// its session. **Absent where none has been seen**: a running Drone in its
    /// first invocation has taken turns the harness has not counted yet, and
    /// nought would say otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<u64>,
    /// What it has cost, in millionths of a dollar, as of its last terminating
    /// line, which carries the session's running total. **Absent is a Drone
    /// that never named a price**, which is not a price of nothing.
    ///
    /// Once it has stopped this is the row the Job's spend is summed from.
    /// While it runs it is read off its transcript, so it trails the Drone by
    /// whatever it has done since that line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// Every Drone a Job has had, in the order they were spawned.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDrones {
    pub job_id: JobId,
    pub drones: Vec<JobDrone>,
}
