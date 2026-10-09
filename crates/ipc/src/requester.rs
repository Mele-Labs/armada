//! Who asked for a Check run, as a typed value a client can follow to the place
//! that asked. Since 23.40.
//!
//! **`kind` is an opaque string**, the way a Record row's kind is: a new
//! kind of requester is then a minor bump, and a client that has not heard of
//! one draws the ids it carries. The ids each kind needs are beside it, and
//! absent on every kind that does not use them.
//!
//! | `kind` | Who | Ids |
//! |---|---|---|
//! | `gate` | A Job's step gate | `job_id`, `step` |
//! | `drone_task` | A Drone asking for the Checks on a plan task | `job_id`, `step`, `task_id`, `drone_id` |
//! | `drone_step` | A Drone asking on a step with no task | `job_id`, `step`, `drone_id` |
//! | `merge_line` | The merge line, for one branch | `branch`, and `job_id` with `handle` where a Job owns the branch. Since 23.64 |
//! | `session` | A Session's agent running `armada check` in the slot it holds. Since 23.74 | `session_id`, `slot` |
//! | `outside` | Nothing in Armada: a person's press, a bare `armada check` | none |
//!
//! **`outside` is a value and never an absence.** A record written before this
//! field existed reads as `outside`, which is also what it was.

use serde::{Deserialize, Serialize};

use crate::ids::{DroneId, JobId, StepId};

pub const GATE: &str = "gate";
pub const DRONE_TASK: &str = "drone_task";
pub const DRONE_STEP: &str = "drone_step";
pub const MERGE_LINE: &str = "merge_line";
pub const SESSION: &str = "session";
pub const OUTSIDE: &str = "outside";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requester {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<JobId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<StepId>,
    /// `T3` and on, as the plan names a task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drone_id: Option<DroneId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// What a person calls the Job, `1-a-job`: **a Drone's handle**, since the
    /// Drone's transcript is named under it and a Drone has no other name than
    /// its id. Present on every kind that names a Job, where Fleet knows it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
    /// The Session whose agent ran it. Since 23.74.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// The pool slot it ran in. Since 23.74.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
}

impl Requester {
    fn of(kind: &str) -> Requester {
        Requester {
            kind: kind.to_string(),
            job_id: None,
            step: None,
            task_id: None,
            drone_id: None,
            branch: None,
            handle: None,
            session_id: None,
            slot: None,
        }
    }

    /// The same requester, with the Job's handle on it.
    pub fn with_handle(self, handle: &str) -> Requester {
        Requester {
            handle: Some(handle.to_string()),
            ..self
        }
    }

    pub fn gate(job: &JobId, step: &StepId) -> Requester {
        Requester {
            job_id: Some(job.clone()),
            step: Some(step.clone()),
            ..Requester::of(GATE)
        }
    }

    pub fn drone_on_task(job: &JobId, step: &StepId, task: &str, drone: &DroneId) -> Requester {
        Requester {
            job_id: Some(job.clone()),
            step: Some(step.clone()),
            task_id: Some(task.to_string()),
            drone_id: Some(drone.clone()),
            ..Requester::of(DRONE_TASK)
        }
    }

    pub fn drone_on_step(job: &JobId, step: &StepId, drone: &DroneId) -> Requester {
        Requester {
            job_id: Some(job.clone()),
            step: Some(step.clone()),
            drone_id: Some(drone.clone()),
            ..Requester::of(DRONE_STEP)
        }
    }

    pub fn merge_line(branch: &str) -> Requester {
        Requester {
            branch: Some(branch.to_string()),
            ..Requester::of(MERGE_LINE)
        }
    }

    pub fn session(session_id: &str, slot: u32) -> Requester {
        Requester {
            session_id: Some(session_id.to_string()),
            slot: Some(slot),
            ..Requester::of(SESSION)
        }
    }

    pub fn outside() -> Requester {
        Requester::of(OUTSIDE)
    }
}

impl Default for Requester {
    fn default() -> Requester {
        Requester::outside()
    }
}
