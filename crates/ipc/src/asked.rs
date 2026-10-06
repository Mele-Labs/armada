//! A Drone's own run of a step's Checks, as a row of its own. Since 23.40.
//!
//! **Never a [`CheckRun`](crate::CheckRun).** A Check row is what a gate ruled
//! on, and an asked result readable as one is a pass nobody measured at the gate.
//! These ride on [`StepDetail::asked_runs`](crate::StepDetail::asked_runs) and
//! [`RunList::asked_runs`](crate::RunList::asked_runs), and a surface that draws
//! them beside the gate's rows says they are asked runs.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::Requester;

/// Where an asked run stands. `lost` is a run whose task died, or whose Fleet
/// went away while it was `running`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AskedRunState {
    Running,
    Passed,
    Failed,
    Stopped,
    Lost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskedRun {
    /// Fleet's own number for the row, unique across restarts.
    pub id: i64,
    /// A `drone_task` or `drone_step` requester: the Job, step, task and Drone.
    pub requester: Requester,
    /// Which run of the step it was asked in.
    pub attempt: u32,
    pub started_at: Instant,
    /// Absent while it runs, and on a run nobody saw end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<Instant>,
    pub state: AskedRunState,
    /// The Checks the run was about, in the step's order.
    pub checks: Vec<String>,
    pub narrowed: bool,
    /// The one Check the ask named, where it named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only_check: Option<String>,
    /// Each Check's log, relative to the repository's records root, **one per
    /// entry of `checks` and in its order**, empty where that Check kept none.
    /// Written when the run ends, so absent while it runs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<String>,
}
