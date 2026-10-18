//! What a session's turns cost: the model, the four token fields and the names of the tools each
//! turn called. `docs/practices/protocol.md`, *Usage a turn at a time*.
//!
//! **Names and counts only**: a tool is its name, never its input or output.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// One turn, as the `turn_measured` fact carries it and `get_session_usage` returns it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnUsage {
    /// The id the harness gave the turn's message: a session reports each turn under one.
    pub message_id: String,
    /// When the turn was answered.
    pub at: Instant,
    pub model: String,
    pub input_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    /// The tools the turn called, by name, once for each call.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
}

/// `get_session_usage`: a session's turns in the window, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTurnUsage {
    pub turns: Vec<TurnUsage>,
}
