//! What the Workflows page reads beside a definition's text: each step's words and what ends it;
//! and the acts on a copy. Protocol `docs/practices/protocol/the-workflows-page.md`.
//!
//! **A step is read, not edited.** The words are changed by `save_workflow` with the whole definition;
//! nothing here takes a step back.

use serde::{Deserialize, Serialize};

use crate::{StepId, WorkflowId};

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
}

/// The body of `delete_workflow_copy` and `reset_workflow_copy`: the copy, by id and place.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowCopy {
    pub workflow_id: WorkflowId,
    /// `repository`, `kit` or `project`. `armada` is refused: Armada's own is not a copy.
    pub source: String,
    /// **The project whose copy it is, required where `source` is `project`** and ignored otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// The file of a copy that was left out for not fitting, as `list_left_out_workflows` names it. A
    /// copy left out is found by its file and not by the id it declares, which may be none Fleet can read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// What `delete_workflow_copy` removed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowCopyDeleted {
    pub workflow_id: WorkflowId,
    /// The place it was removed from.
    pub source: String,
    /// The project it was removed from, where `source` is `project`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// The file that was deleted, absolute.
    pub file: String,
    /// Whose definition of this id runs now: `armada`, `kit`, `repository` or `project`. Absent where none does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runs_from: Option<String>,
}
