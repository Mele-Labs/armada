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
    /// The turn was a subagent's, not the session's own.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub subagent: bool,
    /// What the turn's first call, when it was a shell command, was for. Left out where the first
    /// call was not one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bash_use: Option<BashUse>,
}

/// What a shell command is for, sorted by the first command it runs. A label, never the command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BashUse {
    /// Reading and searching: grep, cat, ls, `git show` and the like.
    ReadSearch,
    /// An interpreter run on a script.
    Scripts,
    /// Any other `git`.
    Git,
    /// Building and testing.
    BuildTest,
    /// Loops, sleeps, polling and Armada's own commands other than a check.
    LoopsWaits,
    /// The forge's command line.
    Forge,
    Other,
}

impl BashUse {
    /// The name it is kept and served under.
    pub const fn label(self) -> &'static str {
        match self {
            BashUse::ReadSearch => "read_search",
            BashUse::Scripts => "scripts",
            BashUse::Git => "git",
            BashUse::BuildTest => "build_test",
            BashUse::LoopsWaits => "loops_waits",
            BashUse::Forge => "forge",
            BashUse::Other => "other",
        }
    }

    /// The label back, for a name read from the store.
    pub fn from_label(label: &str) -> Option<BashUse> {
        [
            BashUse::ReadSearch,
            BashUse::Scripts,
            BashUse::Git,
            BashUse::BuildTest,
            BashUse::LoopsWaits,
            BashUse::Forge,
            BashUse::Other,
        ]
        .into_iter()
        .find(|one| one.label() == label)
    }
}

/// `get_session_usage`: a session's turns in the window, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTurnUsage {
    pub turns: Vec<TurnUsage>,
}
