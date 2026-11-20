//! What the Job's right column and its strip draw for the workflow step the Job is on
//! (the design page *The Job's right column, by workflow step*). **Fleet's reading, never the agent's.**
//!
//! One field of `SessionRecord`, `workflow_strip`, so a client reads the step's tabs, its usual time,
//! what ends it and whether it is stuck from one place. The time on the step is in
//! `SessionRecord::workflow_visits`: the last visit to `workflow_step`.

use serde::{Deserialize, Serialize};

use crate::Instant;

/// The step the Job is on, as the column draws it. Set for a live Job on a workflow that resolves.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStrip {
    /// The tabs the step opens, the front one first, as ids of the native Bridge's zones (`now` and
    /// `walks` beside them). Empty where the step lists none, and where the Job is on no step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tabs: Vec<String>,
    /// The step's 9-in-10 time in seconds: the 90th percentile of how long Jobs on this workflow stayed
    /// on this step before the next began, from the visits Fleet kept. **Absent where fewer than five
    /// visits are known.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usual_secs: Option<u64>,
    /// What ends the step, to the person, any one of them: `you approve the plan`, `the Job ends`.
    /// Empty where nothing Fleet observes does.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ends_when: Vec<String>,
    /// The step has stood past `usual_secs` with no move. Absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stalled: Option<StepStalled>,
    /// The steps the rail moved past because work that belongs to a later step was seen (a push or a
    /// pull request), though the person never gave the Approve they wait on, by the index
    /// `workflow_step` uses, ascending. Drawn `passed without your Approve`. Empty where none is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passed_unapproved: Vec<u32>,
}

/// A step that stands past its usual time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepStalled {
    /// When the step went past `usual_secs`: the step's start and the usual time after it.
    pub since: Instant,
    /// Why, in a sentence: what the step waits on, and whose it is.
    pub cause: String,
}
