//! `dispatch_job`: the person's work for a repository, handed to Fleet before any Job exists. With the
//! Operator on and seated it goes to the Operator, which starts a Job, folds the work into a live one, or
//! asks the person. `docs/concepts/operator.md`.

use serde::{Deserialize, Serialize};

use crate::hosted_sessions::{SendSessionMessage, SentSketch, SessionTag, SessionUpload, StartSession};
use crate::session_notes::SessionNote;
use crate::sessions::{SessionId, SessionRecord};

/// `dispatch_job`'s body: a `StartSession` and what Bridge sends into the Job once it exists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchJob {
    #[serde(flatten)]
    pub start: StartSession,
    /// Sent into the Job in order after it starts, or into the live Job the work was folded into. Left out, nothing is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then: Vec<DispatchThen>,
}

/// One send made into the Job a dispatch became: the notes of a walk, or a message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchThen {
    /// As `send_session_notes`, in the order he took them.
    Notes(Vec<SessionNote>),
    /// As `send_session_message`.
    Message(DispatchMessage),
}

/// A `SendSessionMessage` whose Job is not known yet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchMessage {
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<SessionUpload>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mentions: Vec<SessionTag>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sketches: Vec<SentSketch>,
}

impl DispatchMessage {
    /// The message as sent to `session`.
    pub fn to(self, session: SessionId) -> SendSessionMessage {
        SendSessionMessage { session_id: session, text: self.text, attachments: self.attachments, mentions: self.mentions, sketches: self.sketches }
    }
}

/// What `dispatch_job` answers. **Exactly one is present**: `session` where a Job was started, `operator` where
/// the Operator holds the dispatch, which is its id: `operator.dispatched` says what became of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDispatched {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
}
