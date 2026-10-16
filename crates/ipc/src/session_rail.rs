//! A Session's workflow rail and its judges. `docs/concepts/session.md`, *The rail* and *The
//! judges*; `docs/concepts/judge.md`, *A Session's judge*.
//!
//! **The rail is soft and the judge is advisory.** Fleet reads the step a Session is on from what it
//! observed and nudges when one is skipped; nothing blocks. A judge's concern is a call in the
//! Session's thread, and reaches the agent only if the person sends it.
//!
//! The wire additions live here and in `SessionRecord::{step, judging, judge, settled}` and
//! `WaitingItem::concern`, so a change to one does not touch the rest of the record.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::sessions::SessionId;

/// What produced a concern: the Judge tile, or the boundary after a step of the workflow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConcernFrom {
    /// The person pressed the tile.
    Tile,
    /// The Session moved past the step at `index` (counted from zero, as `SessionRecord::step`).
    Step { index: u32, label: String },
}

/// Where a concern points. **The part of the brief and the lines of it**, as every Judge citation
/// is; `path` is the file when the part is the diff and the lines fall under one file's header.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcernCite {
    /// `request`, `plan`, `diff`, or `brief` for a line under no part. Rendered and never matched on.
    pub region: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub from_line: u32,
    pub to_line: u32,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cites: Vec<ConcernCite>,
    /// Absent while the concern waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<ConcernSettled>,
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
