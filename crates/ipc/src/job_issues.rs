//! What is wrong with a Job, and what a quiet one is waiting on: the two
//! lists a Job's Now draws under Issues and Waiting.
//!
//! **Pointers, not copies.** An issue names the thing it is about and the
//! reader finds the rest where it already lives: a breakage in
//! [`JobDetail::breakages`](crate::JobDetail::breakages), an overlap in
//! `write_scope_overlaps`, a verdict on its step. Restating them here would
//! be a second place for the same fact to disagree with itself.
//!
//! **A closed set Bridge draws one row from, per kind.** Both are tagged
//! enums, so a kind added later is a change every reader is told about.

use serde::{Deserialize, Serialize};

use crate::ids::{DroneId, Instant, JobId, StepId};

/// One thing wrong with a Job right now, once each.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobIssue {
    /// A test broken on main that this Job reported or is fixing. The claim is
    /// [`ClaimedBreakage`](crate::ClaimedBreakage), found by these two names.
    Breakage { check: String, test: String },
    /// Another Job claiming to write where this one does. The paths are
    /// [`ScopeOverlap`](crate::ScopeOverlap), found by the Job.
    WriteScopeOverlap { job_id: JobId },
    /// The latest ruling on a step was `failed`, with the trigger it carried.
    FailedVerdict {
        step_id: StepId,
        attempt: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        trigger: Option<String>,
    },
    /// A working Drone that has written nothing for longer than Fleet's quiet
    /// threshold. **Never a Drone at rest or waiting on an answer**, which are
    /// quiet by design.
    StuckDrone {
        drone_id: DroneId,
        step_id: StepId,
        /// When it last wrote anything.
        quiet_since: Instant,
    },
}

/// What a Job with nothing running is waiting on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobWaiting {
    /// A named resource: the capacity cap, a budget hold, or whatever the
    /// queued reason names. The same spelling `JobSummary::queued_reason`
    /// carries, which is rendered and never matched on.
    Resource { name: String },
    /// Another Job it depends on that has not ended.
    Job {
        job_id: JobId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// A step between two states, with no Drone on it yet.
    Transition { step_id: StepId },
}
