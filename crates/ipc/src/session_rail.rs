//! A Session's workflow rail and its judges. `docs/concepts/session.md`, *The rail* and *The
//! judges*; `docs/concepts/judge.md`, *A Session's judge*.
//!
//! **The rail is soft and the judge is advisory.** Fleet reads the step a Session is on from what it
//! observed and nudges when one is skipped; nothing blocks. A judge's concern is a call in the
//! Session's thread, and reaches the agent only if the person sends it.
//!
//! The wire additions live here and in `SessionRecord::{workflow_step, judging, judge, settled}` and
//! `WaitingItem::{concern, cites}`, so a change to one does not touch the rest of the record.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::sessions::SessionId;

/// What produced a concern: the Judge tile, or the boundary after a step of the workflow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConcernFrom {
    /// The person pressed the tile.
    Tile,
    /// The Session moved past the step at `index` (counted from zero, as `SessionRecord::workflow_step`).
    Step { index: u32, label: String },
}

/// A place a concern points at: a file and, where the judge's quotation lands on a line of it, the line as
/// the change leaves the file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingCite {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// How the person settled a concern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConcernHow {
    /// Sent to the agent as his message.
    Sent,
    /// Dropped. The agent was told nothing.
    Dismissed,
}

/// A concern once settled.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcernSettled {
    pub how: ConcernHow,
    pub at: Instant,
}

/// What a `judge` waiting item carries beside its text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgeConcern {
    pub from: ConcernFrom,
    /// Absent while the concern waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<ConcernSettled>,
    /// The judge's refusal in its three parts, which `text` joins: what should be seen, what was seen
    /// instead, and what the difference does. Each is one short sentence. Absent on a concern kept
    /// before the parts were, which Bridge draws from `text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produced: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consequence: Option<String>,
}

/// The outcome of the last reading. **Zero concerns is how Bridge says "No concerns".**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgeReading {
    pub at: Instant,
    pub concerns: u32,
}

/// `judge_session`: run a judge now on the work so far.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgeSession {
    pub session_id: SessionId,
}

/// `send_concern`: the concern goes to the agent as the person's message, and is settled.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendConcern {
    pub session_id: SessionId,
    pub item_id: String,
}

/// `dismiss_concern`: the concern is settled and the agent is told nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DismissConcern {
    pub session_id: SessionId,
    pub item_id: String,
}
