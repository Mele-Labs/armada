//! A Job's Commander on the wire: what the owner says to it, the socket its
//! thread comes back on, and the event that says whether it is at work. `#2166`.
//!
//! **`crate::helm`'s shape, one subject over.** `opened`, the thread so far and
//! then what follows is the Helm socket's, with Helm's words and rows. What
//! differs is the subject, a Job, and `woke`, the one message only this socket
//! has: Fleet started a turn, and the fact that did. Nothing a Commander says
//! advances a Job, so nothing here is a claim.

use serde::{Deserialize, Serialize};

use crate::event::Missed;
use crate::helm::{HelmAsked, HelmFresh, HelmText, HelmUnanswered};
use crate::ids::{Instant, JobId};
use crate::protocol_id::ProtocolId;
use crate::turn::Shown;

/// `POST /jobs/:job_id/commander/ask` — what the owner says to a Job's
/// Commander. Blank does not decode, as a message to Helm does not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskCommander {
    pub text: HelmText,
}

/// What `ask_commander` answers with. **Not the reply**, which arrives on the
/// socket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderConversation {
    pub job_id: JobId,
    /// A turn is being written, or the words are waiting for the next one.
    pub replying: bool,
    /// The next turn resumes a stored session. `false` starts a new one.
    pub resumes: bool,
}

/// One message on `GET /jobs/:job_id/commander/observe`'s socket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum CommanderMessage {
    /// The first message on every connection.
    Opened(CommanderOpened),
    /// Fleet started a turn, and the fact that did.
    Woke(CommanderWoke),
    /// What the owner said, as Fleet took it. **Said when it is taken**, not
    /// when its turn starts, so a message sent mid-turn is on the thread at
    /// once and answered after.
    Asked(HelmAsked),
    /// What the session wrote: its prose, its calls, and `ended` with what the
    /// turn cost.
    Row(Shown),
    /// The stored session could not be resumed, so this turn starts a new one.
    Fresh(HelmFresh),
    /// No turn came of it, and why.
    Unanswered(HelmUnanswered),
    /// This viewer fell behind and lost messages.
    Missed(Missed),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderOpened {
    pub protocol_id: ProtocolId,
    pub job_id: JobId,
    /// Whether a turn was being written, or one was waiting, when this opened.
    pub replying: bool,
    /// Older messages the thread left out, because the backfill is bounded.
    pub skipped: u64,
}

/// A turn started for a reason Fleet can state in a line. **Several wakes that
/// arrived while a turn was running are one turn**, and this names each.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderWoke {
    pub ts: Instant,
    pub why: String,
}

/// Whether a Job's Commander is at work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommanderState {
    /// No turn is being written and none is waiting.
    Idle,
    /// A turn is being written.
    Working,
}

/// `commander.changed`: a Job's Commander started or finished a turn.
///
/// **A fact about the Job, not a row of its thread.** A client that draws a
/// Job's card reads this; the thread is the socket's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderChanged {
    pub job_id: JobId,
    pub state: CommanderState,
}
