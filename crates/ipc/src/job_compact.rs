//! Whether a Job offers Compact, and if not why, read by Fleet so a client does not work it out.
//! **States only**: the words for each reason stay with the client. Carried on `SessionRecord::compact`.

use serde::{Deserialize, Serialize};

/// How Compact reaches the Job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactHow {
    /// Typed in the Job's terminal, as a command (`run_session_command`).
    Terminal,
    /// Sent as a message to the agent Fleet hosts (`send_session_message`).
    Message,
}

/// Why Compact is off, in the order Bridge checks them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactOff {
    /// The Job ended.
    Ended,
    /// A terminal Job whose mod is not listening.
    NotListening,
    /// A terminal Job whose mod did not list `compact`.
    TerminalNotListed,
    /// A hosted Job whose agent did not list `compact`.
    AgentNotListed,
}

/// Compact offered, or off with the reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CompactOffer {
    Offered { how: CompactHow },
    Off { why: CompactOff },
}
