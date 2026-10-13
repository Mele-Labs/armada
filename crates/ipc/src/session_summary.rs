//! A Session as an agent lists it: who it is and what it holds now, with none of
//! the ledger behind it. `docs/practices/protocol.md`, *An agent lists Sessions
//! in a line each*.
//!
//! **A narrow row beside the wide one, not a cut of it.** [`SessionRecord`]
//! carries every attachment the Session ever took, each pull request's body
//! among them, and answered whole it ran to 696 KB on a live Fleet, past the
//! 64 KB the agent's door carries. The row here has no field that can grow with
//! a Session's history, so how many Sessions fit is a count and not a hope.

use serde::{Deserialize, Serialize};

use crate::hosted_sessions::SessionTurn;
use crate::ids::{Instant, ManifestId};
use crate::sessions::{
    AttachmentState, SessionId, SessionOrigin, SessionRecord, SessionState,
};

/// Which row `list_sessions` answers with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionView {
    /// **The default**: [`SessionList`](crate::SessionList), as Bridge reads it.
    #[default]
    Full,
    /// [`SessionSummaries`]. The agent's door asks for this one and no other.
    Summary,
}

/// The longest a title is carried at, in characters. A title is the first line
/// of a prompt, which has no length of its own.
const TITLE_AT_MOST: usize = 160;

/// Whether a hosted Session's turn is running. Absent on a terminal Session,
/// which reports where a turn ended and not where one began.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnState {
    Idle,
    Working,
}

/// A pull request a Session holds that is still open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldPullRequest {
    pub number: u64,
    /// Empty where the forge has not been read for it yet.
    pub title: String,
    /// `open` or `draft`; absent where the forge has not been read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// One Session, in a line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Absent for a Session in no repository Fleet serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    pub origin: SessionOrigin,
    pub state: SessionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<TurnState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_turn_at: Option<Instant>,
    pub last_seen_at: Instant,
    /// The slot it stands in, as the ledger holds it. Absent when it holds none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pull_requests: Vec<HeldPullRequest>,
    /// The Jobs it holds that have not finished, by id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jobs: Vec<String>,
}

/// `list_sessions` with `view=summary`: the most recently seen first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummaries {
    pub sessions: Vec<SessionSummary>,
}

impl SessionSummary {
    /// The line for `record`. **What it holds is what is standing**: a merged
    /// pull request or a slot let go is history, and `get_session` has it.
    pub fn of(record: &SessionRecord) -> SessionSummary {
        let standing = |kind: &'static str| {
            record
                .attachments
                .iter()
                .filter(move |one| one.kind == kind && one.state == AttachmentState::Standing)
        };
        let held = |kind: &'static str| standing(kind).next().map(|one| one.target.clone());
        SessionSummary {
            id: record.id.clone(),
            title: record.title.as_deref().map(shortened),
            manifest_id: record.manifest_id.clone(),
            origin: record.origin,
            state: record.state,
            turn: record.hosted.as_ref().map(|hosted| match hosted.turn {
                SessionTurn::Idle => TurnState::Idle,
                SessionTurn::Working { .. } => TurnState::Working,
            }),
            last_turn_at: record.last_turn_at.clone(),
            last_seen_at: record.last_seen_at.clone(),
            slot: held("slot"),
            branch: held("branch"),
            pull_requests: standing("pr")
                .filter_map(|one| {
                    Some(HeldPullRequest {
                        number: one.target.trim_start_matches('#').parse().ok()?,
                        title: one.detail.get("title").map_or_else(String::new, |t| shortened(t)),
                        state: one.detail.get("state").cloned(),
                    })
                })
                .collect(),
            jobs: standing("job").map(|one| one.target.clone()).collect(),
        }
    }
}

impl SessionSummaries {
    pub fn of(list: &crate::SessionList) -> SessionSummaries {
        SessionSummaries {
            sessions: list.sessions.iter().map(SessionSummary::of).collect(),
        }
    }
}

fn shortened(text: &str) -> String {
    match text.char_indices().nth(TITLE_AT_MOST) {
        None => text.to_string(),
        Some((at, _)) => format!("{}…", &text[..at]),
    }
}
