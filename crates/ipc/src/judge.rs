//! A Judge on demand: one read-only model call that reads one artifact of a Job when a person presses
//! Judge on it, and answers with notes pinned to the parts they are about.
//!
//! **Only a press starts one, and a note reaches the agent only if the person sends it.** The Judge
//! replaces the old Session judge (concerns in `waiting_for`) and the plan's adversarial review (findings
//! on `SessionPlan`): a result is a note, whatever was read.
//!
//! A Job's readings ride on [`SessionRecord::judge_readings`](crate::SessionRecord), one per artifact with
//! the latest replacing the one before, so they arrive with every `session.changed` and no event of
//! their own is needed.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::sessions::SessionId;

/// What kind of thing was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgeKind {
    Job,
    Plan,
    PullRequest,
    Commit,
    File,
    Page,
    Doc,
    Sketch,
    Command,
    Subagent,
    Issue,
    Note,
}

/// What is read. `id` is the kind's own: a pull request's number, a commit's row, a path, a page's
/// address, the plan's version. One reading is kept per `(kind, id)` of a Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgeArtifact {
    pub kind: JudgeKind,
    pub id: String,
    /// Names it in a line.
    pub title: String,
}

/// How far a reading has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgeState {
    Reading,
    /// It read the artifact. No notes is an answer: it found nothing.
    Done,
    /// It ended without notes that held up, and `failed` says why. **Never a result.**
    Failed,
}

/// How the person settled a note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgeNoteHow {
    /// Sent to the agent as his message.
    Sent,
    /// Dropped. The agent was told nothing.
    Dismissed,
}

/// A note once settled.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgeNoteSettled {
    pub how: JudgeNoteHow,
    pub at: Instant,
}

/// One thing the Judge found, in the parts a refusal has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgeNote {
    /// Unique within its reading.
    pub id: String,
    /// The part it is about, in the artifact's own terms: a plan group or task id, `path:line` of a
    /// diff, `p:` and the words of a document's block. Absent, the artifact as a whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// What is wrong, in a line. Heads the note.
    pub claim: String,
    /// Where it is, as a person reads it: `auth/middleware.rs:88`, `Plan, task Check the token`.
    #[serde(default, rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub found: Option<String>,
    /// What a person using the thing sees or cannot do because of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
    /// `path` or `path:line`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cites: Vec<String>,
    /// Absent while the note waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<JudgeNoteSettled>,
}

/// One reading of one artifact of a Job, going or kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgeReading {
    /// Fleet's. A reading asked again for the same artifact is a new id.
    pub id: String,
    pub artifact: JudgeArtifact,
    /// What the person asked. Empty asks what was missed.
    pub prompt: String,
    pub state: JudgeState,
    pub asked_at: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<Instant>,
    /// Why a failed reading failed, in Fleet's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    /// The notes were opened. Fleet keeps it with the reading.
    pub read: bool,
    pub notes: Vec<JudgeNote>,
    /// What the vendor billed, in millionths of a dollar. Absent where the call ended without
    /// saying, which is not a cost of zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// `ask_judge`'s body. `POST /sessions/judge`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskJudge {
    pub session_id: SessionId,
    pub artifact: JudgeArtifact,
    /// Empty asks what was missed.
    #[serde(default)]
    pub prompt: String,
}

/// `send_judge_note`'s body. `POST /sessions/judge/send`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendJudgeNote {
    pub session_id: SessionId,
    pub reading: String,
    pub note: String,
}

/// `dismiss_judge_note`'s body. `POST /sessions/judge/dismiss`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DismissJudgeNote {
    pub session_id: SessionId,
    pub reading: String,
    pub note: String,
}

/// `mark_judge_read`'s body. `POST /sessions/judge/read`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkJudgeRead {
    pub session_id: SessionId,
    pub reading: String,
}
