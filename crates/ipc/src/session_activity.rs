//! What a Session Fleet hosts is doing, as Fleet saw it. `docs/concepts/session.md`, *Working on*.
//!
//! **Detected, not reported.** `now` and `last_call_at` come from the tool calls Fleet's gate sees,
//! and no tool exists for the agent to say them. `todos` is the one thing the agent says, as its own
//! `TodoWrite` list, kept as Fleet saw it last.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

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

/// What a hosted Session is doing. Absent from the row until its first tool call, and again once
/// it ends. Memory only: a Fleet that restarts starts it empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionActivity {
    /// The latest tool call as one short action, such as `Editing gate.rs` or `Running cargo check`.
    /// Absent until a call Fleet can name has been made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
    /// The agent's own list as its last `TodoWrite` left it. Empty is no list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub todos: Vec<SessionTodo>,
    /// When the Session last made any tool call. **It does not move the row**: it rides whatever
    /// publish the next change to `now` or `todos` causes, so it can be older than the Session's
    /// latest call by a quiet spell, and a reader treats it as no fresher than the row it is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_call_at: Option<Instant>,
}
