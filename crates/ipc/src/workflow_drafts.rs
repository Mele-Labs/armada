//! A draft of a workflow that the owner and an agent edit, and Publish, which writes it. Protocol
//! `docs/practices/protocol/protocol-23-95-a-workflow-edited-as-a-draft-and-published.md`.
//!
//! **A draft is not a save.** `save_workflow` still writes at once, for the Classic Bridge that uses it; a draft
//! is kept by Fleet, edited as often as is wanted, and written only by `publish_workflow_draft`, through the same
//! save and held to the same loader. Each change to a draft keeps the whole text after it, so the editor steps
//! back by putting an earlier change's text again.

use serde::{Deserialize, Serialize};

use crate::{Instant, WorkflowId, WorkflowSaved, WorkflowScope};

/// Who made a change to a draft.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftBy {
    /// The owner, in the editor.
    #[default]
    You,
    /// An agent, asked for a change in his words (`ask_workflow_change`).
    Agent,
}

/// One change to a draft: the whole draft after it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftChange {
    /// Fleet's number for it, rising with each change to any draft.
    pub id: i64,
    pub by: DraftBy,
    pub at: Instant,
    /// The whole draft after this change.
    pub text: String,
    /// What the change did, in a line. An agent's change carries the words it was asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The `WorkflowChange` an agent's change came from. Absent on one the owner made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<i64>,
}

/// One draft of one workflow in one place, as `get_workflow_draft` answers it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDraft {
    pub workflow_id: WorkflowId,
    /// `armada`, `kit`, `repository` or `project`. **A draft of Armada's has no scope**: the copy is made when it is
    /// published, at the scope the owner picks.
    pub source: String,
    /// The project, where `source` is `project`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// The draft definition, whole: the last change's text.
    pub text: String,
    /// The published text the draft began from; for Armada's, the carried text. Empty where the place held none.
    pub base: String,
    /// Why `text` would not load, in the loader's own sentence. Absent where it would.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unfit: Option<String>,
    /// Oldest first. There is always at least one.
    pub changes: Vec<DraftChange>,
    pub started_at: Instant,
    pub updated_at: Instant,
}

/// `put_workflow_draft`'s body: the draft text as the owner has it now. `PUT /workflows/draft`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutWorkflowDraft {
    pub workflow_id: WorkflowId,
    pub source: String,
    /// Required where `source` is `project`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// **Any text**: a draft may be half-edited, and `WorkflowDraft.unfit` says where it would not load.
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// What `discard_workflow_draft` removed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDraftDiscarded {
    pub workflow_id: WorkflowId,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

/// `publish_workflow_draft`'s body. `POST /workflows/draft/publish`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishWorkflowDraft {
    pub workflow_id: WorkflowId,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// **Required for a draft of Armada's workflow**, which may be any of the three places, and 422
    /// `fleet.workflow_draft_scope_needed` without it. For any other source it is absent or names the draft's own
    /// place, and 422 `fleet.workflow_draft_scope_differs` where it names another.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<WorkflowScope>,
    /// **Replace the definition the scope already holds.** Read only for a draft of Armada's, whose copy is a new file: without
    /// it, a scope that holds the id is 422 `fleet.workflow_exists`, naming the file, with nothing written and the draft kept.
    /// A draft of any other source is a copy of what its own place holds, and a publish always replaces that.
    #[serde(default)]
    pub overwrite: bool,
}

/// What `publish_workflow_draft` wrote. A struct and not the save alone, since what a publish adds to it is its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPublished {
    pub saved: WorkflowSaved,
}
