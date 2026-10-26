//! A Session's agent starts a Session, or forks its own conversation into one, and the new one
//! waits for the person. Since 23.80. `docs/concepts/session.md`, *A Session an agent started*.
//!
//! **Neither call names the Session it is for.** Fleet places the caller by its connection, as
//! `show_window` does, so an agent cannot fork another's conversation or start one in another's
//! name. A call that places no hosted Session is refused.

use serde::{Deserialize, Serialize};

use crate::ids::ManifestId;
use crate::sessions::SessionId;

/// `start_another_session`: a new Session in `manifest_id`'s repository, with the agent's first message
/// held for the person. Answered with [`SessionHeld`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartAnotherSession {
    pub manifest_id: ManifestId,
    /// What the Session is called. Blank is refused: the thread names it in a row that opens it.
    pub title: String,
    /// The first message. **Held on the new Job's start card**: nothing runs until the person presses Start.
    pub message: String,
    /// The group of the caller's plan this Session is handed. Since the linked Sessions.
    ///
    /// **Present, the new Session is a linked Session the caller follows**: it is titled with the
    /// group's title, its first message is the group's tasks word for word and the caller's handoff
    /// with `message` after them, and both ledgers keep the group id (`detail.group` on the
    /// `started_by` and `started_to` rows). Refused where the caller's plan holds no such group, or
    /// has handed it already.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// A Session this one waits on, and when it starts. **Present, the new Session starts when the
    /// condition is met** and its first message, the other Session's handoff then `message`, is held
    /// for the person as any start's is. The Session waited on is told what `after.needs` says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<crate::session_waits::StartAfter>,
}

/// `hand_to_session`: the person hands one group of a Session's plan to a Session of its own, from
/// Bridge. Answered with [`SessionHeld`]. **Unlike `start_another_session` the first message is
/// sent**, since the press is the person's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandToSession {
    /// The Session whose plan holds the group. It follows the new one.
    pub session_id: SessionId,
    /// The group's id in that plan.
    pub group: String,
}

/// `fork_this_session`: a new Session that starts as a copy of the caller's own conversation, with the
/// agent's first message held for the person. Answered with [`SessionHeld`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkThisSession {
    /// What the fork is called. Blank is refused.
    pub title: String,
    /// The first message. **Held on the start card**, as [`StartAnotherSession::message`] is.
    pub message: String,
    /// The row of the caller's thread the copy ends at, **included**: a message. Absent, the copy
    /// ends at the caller's latest message of its own that the transcript holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// What both calls answer. The Session exists and is listed; no process has started.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionHeld {
    pub session_id: SessionId,
    pub title: String,
    /// Whether it is a fork of the caller's conversation.
    pub fork: bool,
}

/// A Session as another's thread names it, to open it by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLink {
    pub id: SessionId,
    pub title: String,
}
