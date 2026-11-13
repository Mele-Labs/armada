//! A Job's sketches, drafts and pages as one record: `get_job_artifacts`.
//! `crates/ipc/operations/get_job_artifacts.toml` is the contract, `docs/concepts/artifact.md` the rules.

use serde::{Deserialize, Serialize};

use crate::Instant;

/// One artifact a Job made, across every revision of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobArtifact {
    /// Stable across revisions, the ledger target it is kept under: `sketch:<title>`, the page's path, or
    /// `draft:<title>`.
    pub id: String,
    /// The newest revision's title.
    pub title: String,
    /// The newest revision's form.
    pub form: ArtifactForm,
    /// Oldest first. Never empty.
    pub revisions: Vec<ArtifactRevision>,
    /// Where the newest revision stands.
    pub state: ArtifactState,
    /// His note on a send back, his words with an approval, or his reason with a draft's dismissal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    /// The Needs you item that answers it now: `sketch:<title>`, `document:<path>` or `draft:<row id>`.
    /// Absent where nothing waits on him.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
}

/// One revision of an artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRevision {
    /// 1 and on, in the order they were kept.
    pub number: u32,
    /// When it was kept.
    pub at: Instant,
    pub by: ArtifactBy,
    pub form: ArtifactForm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactForm {
    Sketch,
    Draft,
    /// A document: an HTML page under `.armada/documents/`.
    Page,
}

/// Who wrote a revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactBy {
    Agent,
    Owner,
}

/// Where an artifact's newest revision stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactState {
    /// For reading: nothing was put to him.
    Reading,
    /// Put to him and not answered.
    Asking,
    Approved,
    SentBack,
    /// Taken back by the Job, replaced by a draft of the same title, or a page whose file is gone.
    Withdrawn,
    /// He passed on it with no verdict.
    Dismissed,
}
