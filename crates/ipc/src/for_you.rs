//! For you: one Job's tab of what waits on him, what it is building and what is landing.
//! `docs/practices/protocol/for-you.md`.
//!
//! **Derived, never filled in.** `waiting` is read off the Job's `waiting_for`, `building` off the Jobs it
//! started and `landing` off its own pull requests. Fleet keeps only what an agent or he says on top: the
//! order, a line, an item Fleet cannot see, and a withdrawal.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::{SessionId, SessionStanding};

/// `get_for_you`: one Job's own items, never another Job's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForYou {
    /// The Job asked for.
    pub job: SessionId,
    /// What it waits on him for, in order: placed first in the order set, then the rest oldest first.
    pub waiting: Vec<ForYouItem>,
    /// The live Jobs it started, oldest first.
    pub building: Vec<ForYouBuild>,
    /// Its own pull requests still open.
    pub landing: Vec<ForYouLanding>,
}

/// What an item is, for the glyph and the press.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForYouKind {
    Walk,
    Document,
    Plan,
    Draft,
    Sketch,
    Question,
    Run,
    Approve,
}

/// Who put the item on the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForYouBy {
    Fleet,
    Agent,
}

/// What a press on the item opens.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ForYouOpens {
    /// A sketch, page or draft, by its `JobArtifact.id`.
    Artifact { id: String },
    /// A walk window, by its address.
    Walk { url: String },
    /// A `waiting_for` item, by its id.
    Waiting { id: String },
    /// The Job's plan.
    Plan,
}

/// One thing the Job waits on him for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForYouItem {
    /// The same across reads: a Fleet item's own id (`walk:<url>`, `sketch:<title>`, `document:<path>`,
    /// `draft:<row>`, `plan:<version>`, `ask:<call>`), `waiting:<id>` for one the agent stated with `waiting_for`,
    /// and `added:<n>` for one `add_for_you` added.
    pub id: String,
    pub kind: ForYouKind,
    pub title: String,
    /// Why it is there, or what to decide, from `line_for_you`. Absent where none was set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    /// `order_for_you` placed it.
    pub placed: bool,
    /// When it began to wait.
    pub since: Instant,
    pub added_by: ForYouBy,
    pub opens: ForYouOpens,
    /// Settled in place with *Approve* or *Ask for changes*.
    pub approves: bool,
}

/// One live Job this Job started.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForYouBuild {
    /// The started Job, for the client to draw its row's state as its Jobs list does, and to open it.
    pub job: SessionId,
    pub title: String,
    /// What it is doing. Absent for a Job Fleet has read nothing of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing: Option<SessionStanding>,
    /// The step of its workflow it is on. Absent for a Job on no workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
}

/// One of the Job's own pull requests still open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForYouLanding {
    pub pr: u64,
    /// Empty where the forge has not been read for it yet.
    pub title: String,
}

/// `withdraw_for_you`: take a row off, whoever added it. It stays off through later reads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithdrawForYou {
    /// Which Job. **Read only where the connection places no hosted session**, as `SetWaitingFor`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub id: String,
}

/// `order_for_you`: the rows placed first, in this order. Replaces the order set before.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderForYou {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub ids: Vec<String>,
}

/// `line_for_you`: the line under one row. A blank line clears it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineForYou {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub id: String,
    pub line: String,
}

/// `add_for_you`: a row for something Fleet cannot see. One whose `opens` names a row already listed is that row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddForYou {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub title: String,
    pub kind: ForYouKind,
    pub opens: ForYouOpens,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
}

/// What `add_for_you` answers: the row's id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForYouAdded {
    pub id: String,
}

/// `session.for_you`: the Job's For you moved. Read `get_for_you` again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForYouChanged {
    pub session_id: SessionId,
}

/// `fleet.for_you_unknown`: an id that names no row on the Job's For you.
pub const FOR_YOU_UNKNOWN: &str = "fleet.for_you_unknown";
/// `fleet.for_you_blank`: `add_for_you` with a blank title.
pub const FOR_YOU_BLANK: &str = "fleet.for_you_blank";
