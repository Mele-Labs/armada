//! The owner's notes on a pull request's lines, kept by Fleet so the Job a note was sent to can
//! resolve it with a tool. `docs/concepts/pull-review.md`.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// Which side of the diff a note's line is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteSide {
    Left,
    Right,
}

/// Whether a Job has dealt with the note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteStatus {
    Open,
    Done,
}

/// One note on one line of a pull request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestNote {
    /// Fleet-made.
    pub id: String,
    /// `owner/name`, as the forge names the repository.
    pub repository: String,
    pub number: u32,
    pub path: String,
    pub line: u32,
    pub side: NoteSide,
    /// Never blank, and trimmed.
    pub body: String,
    pub status: NoteStatus,
    /// The Job the note was sent to. Absent until it is sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent_to: Option<SessionId>,
    /// That Job's title when the note was sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
    pub created_at: String,
}

/// `list_pull_request_notes`, and what the commands that change the notes answer: every note on
/// one pull request, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestNotes {
    pub notes: Vec<PullRequestNote>,
}

/// `save_pull_request_note`'s body. Without `id` it makes a note, and `path`, `line` and `side` are
/// required. With `id` it edits that note's `body`, and only while the note is open and unsent;
/// the other fields are ignored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavePullRequestNote {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<NoteSide>,
    pub body: String,
}

/// `send_pull_request_notes`'s body: these notes were sent to this Job. It records; Bridge sends the
/// message itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendPullRequestNotes {
    pub note_ids: Vec<String>,
    pub session_id: SessionId,
    /// The Job's title now, kept so a note still says who has it after the Job is renamed.
    pub title: String,
}

/// `resolve_pull_request_note`'s body: the Job a note was sent to marks it done.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvePullRequestNote {
    pub note_id: String,
    /// Which session. **Read only where the connection places no hosted session**, as `FinishUp`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}
