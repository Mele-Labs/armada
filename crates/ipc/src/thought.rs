//! Helm reading a thought for where it belongs, and recording where it went.
//! `docs/concepts/helm.md`, *Routing a thought*.
//!
//! **Three operations, the first two Bridge's.** `read_thought` takes the words a person typed into
//! Helm's box and answers whether they are a question for Helm or a thought for other running
//! work, and which. `record_thought_sent` is said after Bridge delivered the words to that work: Fleet
//! keeps a `sent` row in the Helm Session's thread (`SessionRow::Sent`) and publishes
//! `helm.thought_routed`. Neither sends anything to a Session, a Job or Helm's own process:
//! delivery is the acts that already exist. `tell_session` is Helm's own delivery to a live
//! Session, for the words that reach Helm as a request, and it keeps the same row and the same
//! event itself.

use serde::{Deserialize, Serialize};

use crate::hosted_sessions::{SessionTag, SessionUpload};
use crate::ids::{Instant, ManifestId};
use crate::sessions::SessionId;

/// `read_thought` (`POST /helm/read_thought`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadThought {
    pub text: String,
}

/// What a thought may go to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThoughtKind {
    Session,
    Job,
}

/// The Session or Job a thought goes to, named so a person can tell which.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThoughtTarget {
    pub kind: ThoughtKind,
    /// A Session's id, or a Job's.
    pub id: String,
    /// What a person calls it.
    pub title: String,
    /// The repository it belongs to. Absent for a Session Fleet places in none it serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    /// The slot a Session holds, where it holds one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
}

/// What `read_thought` answers: **a question is the answer whenever Fleet could not say
/// otherwise**, so the words go to Helm and are never lost.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ThoughtReading {
    /// For Helm itself.
    Question,
    /// For other work. Without a `target`, nothing running is about it.
    Thought {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<ThoughtTarget>,
    },
}

/// `record_thought_sent` (`POST /helm/thought_sent?manifest_id=`): the words were delivered. The
/// manifest names the Helm Session whose thread keeps the row, as `ask_helm`'s does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThoughtSent {
    pub text: String,
    pub target: ThoughtTarget,
    /// What was sent to the Session with the words. Fleet keeps it with the Helm Session's own
    /// uploads, as a message's, so the row can draw it after a restart.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<SessionUpload>,
    /// The Sessions, Jobs, pull requests, issues and branches the words named.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mentions: Vec<SessionTag>,
}

/// `tell_session` (`POST /helm/tell_session?manifest_id=`), a `Helm only` act: words a person asked
/// Helm to pass to a live Session. Fleet sends them as Bridge's own send does, so a terminal
/// Session that is not listening is refused in the same words, and on success keeps the same
/// `sent` row and `helm.thought_routed` as `record_thought_sent`. Answers the [`ThoughtTarget`]
/// the words went to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TellSession {
    pub session_id: SessionId,
    pub text: String,
}

/// `helm.thought_routed`: Helm's reading sent a person's words to running work. **Helm's act as
/// its own event type**, beside `studio.helm_acted`. It names where and never the words,
/// which are the thread's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmThoughtRouted {
    /// The Helm Session whose thread holds the words, by its repository.
    pub manifest_id: ManifestId,
    pub to: ThoughtKind,
    /// The Session's id, or the Job's.
    pub id: String,
    pub at: Instant,
}
