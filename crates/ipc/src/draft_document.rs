//! `convert_draft_to_document`: the person turns a draft into a Document, the page he reads and notes.

use serde::{Deserialize, Serialize};

use crate::sessions::SessionId;

/// `POST /sessions/draft/document`: the person presses Make it a Document on a draft.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertDraftToDocument {
    pub session_id: SessionId,
    /// The draft's `draft:<row id>`, the item's id. The bare row id reads the same.
    pub draft_id: String,
}

/// The page a draft became.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftDocument {
    /// The page's name under `.armada/documents/`, without `.html`.
    pub name: String,
    /// The page's path in the Job's worktree, which is its `target` on the Job's record.
    pub path: String,
    /// Whether the Job was sent its turn. False for a Job that is paused or has ended, which is told nothing.
    pub told: bool,
}
