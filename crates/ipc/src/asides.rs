//! An aside: a side question put to a Job without interrupting it, answered by one read-only turn on a thrown-away
//! copy of its conversation, and kept beside its record where its agent never sees it.
//!
//! **The Job's own conversation is not touched.** The answer is not a row of its thread and not a hosted row; it
//! reaches the agent only when the owner tells it (`tell_aside`), as a normal message of his. Since 15 Nov 2026.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::sessions::SessionId;

/// How far an aside has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AsideState {
    /// The turn is running. `answer` holds what has arrived.
    Asking,
    /// The turn ended and `answer` is whole.
    Answered,
    /// It ended without an answer, and `failed` says why. **Never a result.**
    Failed,
}

/// One aside, going or kept. `session.aside` carries it whole, so a client replaces the one it holds by `aside_id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aside {
    /// Fleet's.
    pub aside_id: String,
    /// The Job it was asked of.
    pub session_id: SessionId,
    pub question: String,
    /// The text the owner selected in the thread when he asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
    /// What the copy has said so far, whole once `state` is `answered`. Empty until it says something.
    pub answer: String,
    pub state: AsideState,
    /// Why a failed aside failed, in Fleet's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    pub asked_at: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered_at: Option<Instant>,
    /// When the owner told it to the Job. Absent: not told.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub told_at: Option<Instant>,
}

/// `ask_aside`: put a side question to a Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskAside {
    pub session_id: SessionId,
    pub question: String,
    /// The text the owner selected in the thread, where he did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
}

/// `tell_aside`: send an answered aside to the Job as a message of the owner's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TellAside {
    pub session_id: SessionId,
    pub aside_id: String,
}

/// What `list_asides` answers: the Job's asides, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsideList {
    pub session_id: SessionId,
    pub asides: Vec<Aside>,
}
