//! What the Workflows page reads beside a definition's text: each step's words, what ends it, and
//! the Jobs Fleet nudged there; and the acts on a copy. Protocol `docs/practices/protocol/the-workflows-page.md`.
//!
//! **A step is read, not edited.** The words are changed by `save_workflow` with the whole definition;
//! nothing here takes a step back.

use serde::{Deserialize, Serialize};

use crate::{Instant, SessionId, StepId, WorkflowId};

/// `get_workflow_steps`' answer: one definition of one workflow, step by step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowSteps {
    pub workflow_id: WorkflowId,
    /// `armada`, `kit` or `repository`.
    pub source: String,
    /// In order. Order is the semantics.
    pub steps: Vec<StepReading>,
}

/// One step of a definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepReading {
    pub id: StepId,
    pub label: String,
    /// The words the agent is told on entering the step. Absent where the step has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// What ends the step, as a short fact. **Fleet's own reading of the rail's rules**, spelled once.
    /// Absent where nothing Fleet observes ends it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_when: Option<String>,
    /// Every end of the step, in order, as the page says them: any one of them ends it. Empty where
    /// it has none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ends_any: Vec<String>,
    /// When the step is passed over instead of walked. Absent on a step that is always done.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passed_over_when: Option<String>,
    /// The newest nudges sent for this step in this place's definition, newest first. **Those marked
    /// not a drift are not here.** Empty for a step nobody drifted from.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nudges: Vec<StepNudge>,
}

/// One time Fleet nudged a Job off a step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepNudge {
    /// What `mark_not_a_drift` takes.
    pub id: i64,
    pub job_id: SessionId,
    /// The Job's title. Absent where it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_title: Option<String>,
    /// What happened, as a short fact.
    pub fact: String,
    pub at: Instant,
}

/// The body of `delete_workflow_copy` and `reset_workflow_copy`: the copy, by id and place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowCopy {
    pub workflow_id: WorkflowId,
    /// `repository` or `kit`. `armada` is refused: Armada's own is not a copy.
    pub source: String,
    /// The file of a copy that was left out for not fitting, as `list_left_out_workflows` names it. A
    /// copy left out is found by its file and not by the id it declares, which may be none Fleet can read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// What `delete_workflow_copy` removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowCopyDeleted {
    pub workflow_id: WorkflowId,
    /// The place it was removed from.
    pub source: String,
    /// The file that was deleted, absolute.
    pub file: String,
    /// Whose definition of this id runs now: `armada`, `kit` or `repository`. Absent where none does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runs_from: Option<String>,
}

/// `mark_not_a_drift`' body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkNotADrift {
    /// A `StepNudge::id`.
    pub nudge: i64,
}

/// A nudge marked not a drift: it stays kept and stops counting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotADriftMarked {
    pub nudge: i64,
}
