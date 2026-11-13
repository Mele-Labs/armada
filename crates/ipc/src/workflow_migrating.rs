//! Moving running Jobs to a workflow's new version. Protocol
//! `docs/practices/protocol/protocol-23-97-moving-running-jobs-to-a-new-workflow-version.md`.
//!
//! **A publish moves no Job.** It pins each live Job it would touch to the text that Job was running
//! (`WorkflowPublished::pinned`), and the person then picks which Jobs move: `get_affected_jobs` lists them and
//! `migrate_jobs` moves the ones he picks.

use serde::{Deserialize, Serialize};

use crate::{ManifestId, SessionId, WorkflowId};

/// One live Job that follows the workflow, and where it stands against the new text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedJob {
    pub session_id: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub manifest_id: ManifestId,
    /// The step the rail stands on now, as the Job reads its own steps. Absent where it stands on none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_label: Option<String>,
    /// The step's id is one of the target's steps, so the Job keeps its place. False: a move puts it on the next step
    /// that survives. True also where the Job stands on no step.
    pub survives: bool,
    /// A publish pinned the Job to the text it was running.
    pub pinned: bool,
    /// The Job holds a copy of its own that it edited. It is listed so he sees it, and a move refuses it.
    pub edited: bool,
    /// The version in the place's log whose hash is the Job's copy text. Absent where none matches and where the Job
    /// holds no copy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_version: Option<u32>,
}

/// `get_affected_jobs`' answer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedJobs {
    pub jobs: Vec<AffectedJob>,
}

/// `migrate_jobs`' body. `POST /workflows/migrate`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrateJobs {
    pub workflow_id: WorkflowId,
    /// `kit`, `repository` or `project`: the place that was published.
    pub source: String,
    /// Required where `source` is `project`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub session_ids: Vec<SessionId>,
}

/// What `migrate_jobs` did for one Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrateResult {
    pub session_id: SessionId,
    /// The Job was moved. False for a refusal and for a Job that already reads the published text.
    pub moved: bool,
    /// Where the Job stands now, where it was moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_label: Option<String>,
    /// The Fleet code of a refusal. Absent on a move, and on a Job that needed none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// `migrate_jobs`' answer: one result for each id asked, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratedJobs {
    pub results: Vec<MigrateResult>,
}
