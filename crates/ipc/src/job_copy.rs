//! A Job's own copy of its workflow, lifecycles and rules, and the changes made to it. Protocol
//! `docs/practices/protocol.md`, *A Job's own copy*.
//!
//! **A change applies to the one Job and nothing else.** The repository's files, Kit's and every other
//! Job's copy are untouched; `keep_job_workflow` is what proposes a change for them, and it is saved only on
//! the owner's Approve.

use serde::{Deserialize, Serialize};

use crate::{Instant, SessionId, WorkflowId};

/// Who made a change to a Job's copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangedBy {
    /// The Job's agent, with a tool.
    Job,
    /// The owner, from the Job's lanes or its rail.
    You,
}

/// One edit to the steps of a Job's copy of its workflow. `change_session_steps`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepChange {
    /// A step of its own, after the step `after` names, or first where it names none.
    Add {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<String>,
        /// What the step is called in the copy. Absent takes it from `label`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        label: String,
        /// What the step does for the Job, in a line for the person.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        about: Option<String>,
        /// What the agent is told on entering the step.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instructions: Option<String>,
        /// What ends it: a word such as `replied`, or a lifecycle and its state such as `job: landed`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        ends_on: Vec<String>,
    },
    /// A step after another, or first where `after` is absent.
    Move {
        step: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<String>,
    },
    /// New words for a step: any of its label, its line for the person and what the agent is told.
    Reword {
        step: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        about: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instructions: Option<String>,
    },
    /// A step taken out of the copy. The step that delivers cannot be.
    Drop { step: String },
}

/// `change_session_steps`: a step added, moved, reworded or dropped in the Job's own copy of its workflow.
/// Answers the Job's `JobCopy`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSessionSteps {
    pub session_id: SessionId,
    pub change: StepChange,
}

/// A kind of piece a Job's copy holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyPieceKind {
    Lifecycle,
    Rule,
}

/// `change_job_copy`: a lifecycle or a rule put in the Job's own copy, or taken out of it. Answers the
/// Job's `JobCopy`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeJobCopy {
    pub session_id: SessionId,
    pub kind: CopyPieceKind,
    /// The id the piece's file declares. A piece already in the copy under it is replaced. Blank with a `text`
    /// takes the id the file declares.
    #[serde(default)]
    pub id: String,
    /// The whole file, as the loader reads one. Absent takes the piece out of the copy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// `get_job_copy`'s question is the Job; this is one step of its workflow as the Job holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobCopyStep {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// What ends the step, one word or lifecycle state each, as `ends_on` writes them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ends_on: Vec<String>,
    /// The person or the Job skipped it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
}

/// A lifecycle or a rule the Job's copy holds, as the file it was kept from says it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobCopyPiece {
    pub id: String,
    pub text: String,
}

/// One change to the Job's copy that the lanes mark: `lane` is `workflow` with a step, a lifecycle's id
/// with a state (or none, for the lifecycle as a whole) or `rule` with a rule's id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobCopyChange {
    pub lane: String,
    pub item: String,
    /// What is marked, in a line that names who made it: `Added after Approve plan by the Job`.
    pub text: String,
    pub by: ChangedBy,
    pub at: Instant,
}

/// `get_job_copy`: what a Job holds of its workflow, lifecycles and rules, and what was changed in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobCopy {
    pub session_id: SessionId,
    /// The workflow the Job follows. Absent for a custom Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowId>,
    /// The steps are the Job's own, because it changed one. Otherwise they are the workflow as the
    /// repository holds it now.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,
    pub steps: Vec<JobCopyStep>,
    /// The lifecycles the copy holds as files: those Armada carries are not among them.
    pub lifecycles: Vec<JobCopyPiece>,
    pub rules: Vec<JobCopyPiece>,
    pub changes: Vec<JobCopyChange>,
}

/// `keep_job_workflow`: propose the Job's copy of its workflow for every Job, as a change to the workflow it
/// came from. Answers the `WorkflowChange` kept, which waits on the owner's Approve on the Workflows page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeepJobWorkflow {
    pub session_id: SessionId,
    /// Where the change would be saved: `repository` or `kit`. Absent takes the place the workflow came
    /// from, and `repository` for one Armada carries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// `propose_workflow_change`: a Job proposes new text for a workflow, for the owner to approve.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposeWorkflowChange {
    pub workflow_id: WorkflowId,
    /// Where the change would be saved: `repository` or `kit`.
    pub source: String,
    /// The whole definition, as JSON text.
    pub definition: String,
}
