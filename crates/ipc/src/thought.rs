//! The words sent on to a live Job.
//!
//! `tell_session` is a Job on your words passing words to a live Job: Fleet
//! sends them as Bridge's own send does and publishes `operator.words_sent`. The thread of the Job
//! that sent them keeps a `sent` row only in the old threads the Helm Job kept
//! (`SessionRow::Sent`), which stay readable.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};
use crate::sessions::SessionId;

/// What a thought may go to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThoughtKind {
    Session,
}

/// The Session a thought goes to, named so a person can tell which.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThoughtTarget {
    pub kind: ThoughtKind,
    /// A Session's id.
    pub id: String,
    /// What a person calls it.
    pub title: String,
    /// The repository it belongs to. Absent for a Session Fleet places in none it serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    /// The slot it holds, where it holds one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
}

/// `tell_session` (`POST /operator/tell_session?manifest_id=`), an `On your words` act: words a person asked
/// a Job to pass to a live Job. Fleet sends them as Bridge's own send does, so a terminal
/// Job that is not listening is refused in the same words, and on success publishes
/// `operator.words_sent`. Answers the [`ThoughtTarget`] the words went to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TellSession {
    pub session_id: SessionId,
    pub text: String,
}

/// `operator.words_sent`: a person's words were sent to a running Job. It names where and never the
/// words, which are the Job's thread's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordsSent {
    /// The repository the call was made in.
    pub manifest_id: ManifestId,
    pub to: ThoughtKind,
    /// The Session's id.
    pub id: String,
    pub at: Instant,
}
