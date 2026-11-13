//! The versions a workflow has had in one place, going back to one, and what a copy of one of Armada's began from. Protocol
//! `docs/practices/protocol/protocol-23-96-a-workflow-with-versions-and-a-base.md`.
//!
//! **A version is a whole text**, written by a publish. Going back is `revert_workflow`, which puts that text in the draft
//! and publishes nothing. **A base is what a copy began from**, so Bridge can show what Armada's changed since and the owner
//! chooses what to take.

use serde::{Deserialize, Serialize};

use crate::{Instant, WorkflowId};

/// How a version came to be kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionOrigin {
    /// A publish wrote it.
    #[default]
    Published,
    /// It was the text in the place when the first publish came, kept as version 1 before the publish's own.
    Found,
}

/// One version of a workflow in one place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowVersion {
    /// From 1, in the order they were written.
    pub version: u32,
    /// The definition, whole.
    pub text: String,
    /// The sha256 of `text`, in hex.
    pub hash: String,
    pub at: Instant,
    pub origin: VersionOrigin,
}

/// `get_workflow_versions`' answer: newest first. Empty where nothing was published in that place.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowVersions {
    pub versions: Vec<WorkflowVersion>,
}

/// `revert_workflow`'s body. `POST /workflows/revert`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevertWorkflow {
    pub workflow_id: WorkflowId,
    /// `kit`, `repository` or `project`: the place the versions were kept in.
    pub source: String,
    /// Required where `source` is `project`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub version: u32,
}

/// What a copy of one of Armada's workflows began from: the `version` Armada's definition declared then, and the sha256 of its
/// text. Absent on a copy made before Fleet kept it, which is never guessed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasedOn {
    pub version: u32,
    /// The sha256 of the carried text, in hex.
    pub hash: String,
}
