//! `dispatch_job`: the person's work for a repository, handed to Fleet before any Job exists. With the
//! Operator on and seated it goes to the Operator, which starts a Job, folds the work into a live one, keeps it
//! for later or asks the person. `docs/concepts/operator.md`.

use serde::{Deserialize, Serialize};

use crate::hosted_sessions::{SendSessionMessage, SentSketch, SessionTag, SessionUpload, StartSession};
use crate::session_notes::SessionNote;
use crate::sessions::{SessionId, SessionRecord};

/// What the person marks the work he dispatches as: a hint to the Operator and nothing more. It picks no way and
/// no workflow, and a dispatch with none is as valid as one with any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchKind {
    /// He takes it for a loose thought.
    Idea,
    /// He takes it for work to build.
    Feature,
    /// He takes it for something broken.
    Bug,
}

impl DispatchKind {
    /// The word on the wire.
    pub fn as_wire(self) -> &'static str {
        match self {
            DispatchKind::Idea => "idea",
            DispatchKind::Feature => "feature",
            DispatchKind::Bug => "bug",
        }
    }
}

/// `dispatch_job`'s body: a `StartSession` and what Bridge sends into the Job once it exists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchJob {
    #[serde(flatten)]
    pub start: StartSession,
    /// What he marks it as, which the Operator is told as a hint. Left out, the Operator reads it from the words and
    /// pictures. It is kept with the dispatch, so it rides onto the Job or the parked idea.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<DispatchKind>,
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
/// the Operator holds the dispatch, which is its id (`operator.dispatched` says what became of it). `parked` is
/// no longer answered: an idea is kept only by the Operator's ruling, which `operator.dispatched` names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDispatched {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parked: Option<String>,
}

/// One idea kept for later: the dispatch whole, so starting it is `dispatch_job` with `dispatch`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedIdea {
    /// The id `operator.dispatched` named it by.
    pub id: String,
    pub dispatch: DispatchJob,
    /// RFC 3339.
    pub at: String,
}

/// `list_parked_ideas`' answer, and `remove_parked_idea`'s: oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedIdeas {
    pub ideas: Vec<ParkedIdea>,
}

/// `remove_parked_idea`: one press.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveParkedIdea {
    pub id: String,
}
