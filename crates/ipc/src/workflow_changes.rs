//! A change to a workflow that an agent drafts for the owner to approve: asked for in his words, or
//! suggested when one step of a workflow keeps drifting. Protocol `docs/practices/protocol/the-workflows-page.md`.
//!
//! **Nothing here saves a definition.** A change holds the text as it was and as drafted; approving it
//! is what writes, through the path `save_workflow` checks.

use serde::{Deserialize, Serialize};

use crate::{Instant, SessionId, StepId, WorkflowId, WorkflowSaved};

/// Where a change came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChangeOrigin {
    /// The owner typed it. `text` is his words.
    Asked { text: String },
    /// Armada drafted it, or he pressed Suggest a change, from the nudges kept for one step. `nudges` are
    /// their `StepNudge::id`s.
    Drift { nudges: Vec<i64> },
}

/// Where a change stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeState {
    /// An agent is writing it.
    Drafting,
    /// The draft fits and waits on Approve, Dismiss or a send back.
    Ready,
    /// Written to the definition's file. Not listed.
    Approved,
    /// Put aside by the owner. Not listed.
    Dismissed,
    /// No draft fit, and `reason` says why.
    Failed,
    /// The agent answered that Armada has no way to do what was asked, and `missing` says what it lacks.
    /// Listed, and settled by Dismiss or by `build_workflow_support`.
    Cannot,
}

/// One step of either side of a change, as Fleet's rail reads it: its words, and what ends it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeStep {
    pub id: StepId,
    pub label: String,
    /// What the agent is told on entering it. Absent where the step says nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// What ends the step, to the person, as `StepReading::ends_when` says it. Absent where nothing does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_when: Option<String>,
    /// When the step is passed over instead of walked. Absent on a step that never is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passed_over_when: Option<String>,
}

/// One change to one workflow in one place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowChange {
    /// What approve, dismiss and send back take.
    pub id: i64,
    pub workflow_id: WorkflowId,
    /// `repository` or `kit`.
    pub source: String,
    /// The one step a suggestion may change. Absent on a change the owner asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<StepId>,
    pub origin: ChangeOrigin,
    /// The definition's text when the change began. Approve is refused if the file no longer holds it.
    pub before: String,
    /// The drafted definition, the whole of it. Present while the change is ready.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    pub state: ChangeState,
    /// Why no draft fit. Present on a failed change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// What Armada lacks to do what was asked, one sentence each. Present on a `cannot` change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<String>,
    /// `before`, step by step. Absent where the text no longer reads as a definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_steps: Option<Vec<ChangeStep>>,
    /// `after`, step by step, present with it where it reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_steps: Option<Vec<ChangeStep>>,
    /// What the owner said when he sent it back, the latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// When it last moved.
    pub at: Instant,
}

/// `ask_workflow_change`'s body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskWorkflowChange {
    pub workflow_id: WorkflowId,
    /// `repository` or `kit`. `armada` is refused: Armada's rows are read-only.
    pub source: String,
    /// What he wants changed, in his words.
    pub text: String,
}

/// `suggest_workflow_change`'s body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestWorkflowChange {
    pub workflow_id: WorkflowId,
    /// `repository` or `kit`.
    pub source: String,
    /// The step whose words are to be redrafted.
    pub step_id: StepId,
}

/// `list_workflow_changes`' answer: the open changes to one workflow in one place, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowChanges {
    pub changes: Vec<WorkflowChange>,
}

/// The body of `approve_workflow_change` and `dismiss_workflow_change`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowChangeRef {
    pub change: i64,
}

/// `send_back_workflow_change`'s body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendBackWorkflowChange {
    pub change: i64,
    /// What was wrong with the draft, in his words.
    pub notes: String,
}

/// What `approve_workflow_change` did: the change, approved, and the save it made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowChangeApproved {
    pub change: WorkflowChange,
    pub saved: WorkflowSaved,
}

/// What `build_workflow_support` started: the Job on Armada's own repository that builds what a
/// `cannot` change lacks. Its body is a [`WorkflowChangeRef`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowSupportBuilding {
    pub job_id: SessionId,
}

/// `propose_workflow_to_armada`'s body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposeToArmada {
    pub workflow_id: WorkflowId,
    /// The definition's text, as it is to be carried.
    pub definition: String,
}

/// The Job started to land a definition in Armada.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmadaProposed {
    pub workflow_id: WorkflowId,
    pub session_id: SessionId,
    /// The brief it was sent, or why it was not sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_failed: Option<String>,
}
