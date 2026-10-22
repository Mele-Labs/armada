//! What a Session Fleet hosts is doing, as Fleet saw it. `docs/concepts/session.md`, *Working on*.
//!
//! **Detected, not reported.** `now` comes from the tool calls Fleet's gate sees, and no tool exists
//! for the agent to say it. `todos` is the one thing the agent says, as its own `TodoWrite` list, kept
//! as Fleet saw it last. `account` is the one thing a model writes, and says so.

use serde::{Deserialize, Serialize};

use crate::Instant;

/// Where one item of the agent's own list stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTodoState {
    Done,
    Doing,
    Todo,
}

/// One item of the agent's own `TodoWrite` list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTodo {
    pub text: String,
    pub state: SessionTodoState,
}

/// What one subagent of a hosted Session is doing, by the gate's `agent_id` on its calls.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentActivity {
    pub id: String,
    /// The subagent's latest call as one short action, as the Session's own `now` is.
    pub now: String,
}

/// What a small model wrote of the turn so far, in the person's own words: no file names, paths,
/// code identifiers or tool names. Rewritten while the Session works and kept until his next message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionAccount {
    /// The id of the person's message the account answers, the last one he sent when it was written.
    #[serde(rename = "for")]
    pub for_message: String,
    /// What the Session has done for that message, oldest first. Empty is nothing yet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done: Vec<String>,
    /// What it is on now. Absent where the writer named nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
    /// What it does next. Absent where the writer named nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// The id of the approved plan group it is on, `SessionPlanGroup::id`. Absent where the Session's
    /// plan is not approved or the account names no group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// When the account was written.
    pub at: Instant,
}

/// The page the Session is building, for a live view beside its account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLive {
    pub url: String,
    /// What the Session called the page it showed, or the name of the server it started.
    pub title: String,
}

/// What a hosted Session is doing. Absent from the row until its first tool call, and again once
/// it ends. Memory only: a Fleet that restarts starts it empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionActivity {
    /// The latest tool call as one short action, such as `Editing gate.rs` or `Running cargo check`.
    /// Absent until a call Fleet can name has been made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
    /// The first line of the agent's latest text this turn, at most 160 characters. Absent until it
    /// has said something, and again when a turn begins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
    /// The agent's own list as its last `TodoWrite` left it. Empty is no list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub todos: Vec<SessionTodo>,
    /// What each subagent at work is doing, by its own calls. An entry goes when the subagent ends
    /// and when a turn begins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subagents: Vec<SubagentActivity>,
    /// What the Session has done, is on and does next, for the person's last message. Written by a
    /// small model, at most every few seconds; kept after the turn ends and until his next message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<SessionAccount>,
    /// The newest page the Session showed or served since the person's last message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live: Option<SessionLive>,
}
