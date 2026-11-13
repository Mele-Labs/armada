//! A Job's sketches, drafts and pages as one record: `get_job_artifacts`.
//! `crates/ipc/operations/get_job_artifacts.toml` is the contract, `docs/concepts/artifact.md` the rules.

use serde::{Deserialize, Serialize};

use crate::{Instant, SessionId};

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
    /// Who folded it away, and when. Only `restore_artifact` lifts it; a new revision does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived: Option<ArtifactArchived>,
}

/// An artifact the agent or he archived as stale. `archive_artifact`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactArchived {
    pub by: ArtifactBy,
    pub at: Instant,
}

/// `archive_artifact` and `restore_artifact`: one artifact of a Job, by its [`JobArtifact::id`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// The artifact's id: `sketch:<title>`, the page's path, or `draft:<title>`.
    pub artifact: String,
    /// Which Job holds it. **Read only where the connection places no hosted session**, as `WithdrawSketch`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// On a sketch or page's attachment: `agent` where the Job took it back with `withdraw_sketch` or
/// `withdraw_document`, so it reads `withdrawn` and his Dismiss reads `dismissed`.
pub const ARTIFACT_WITHDRAWN_BY: &str = "withdrawn_by";

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
