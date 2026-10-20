//! One repository's Helm: what a person sends it, and what the dock reads back.
//!
//! **Helm is a hosted Session** (`docs/concepts/helm.md`), so its thread is a Session's: the same
//! [`SessionThread`] and the same `session.row` events a Session page draws. What is Helm's alone
//! is here: the message with the person's whereabouts, and the lookup that finds the Session.
//! `#939` made it a conversation on a socket of its own; that socket, its tuning and its files
//! are gone, and the Commander's conversation (`commander.rs`) is the one that keeps the shape.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::hosted_sessions::{SentFile, SessionTag, SessionThread, SessionUpload};
use crate::ids::{Instant, ManifestId, StudioId, StudioNodeId};

/// `POST /helm/ask` — what a person says to Helm.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskHelm {
    /// What the person typed. **May be blank** where `attachments` or `mentions` carry the
    /// message; Fleet refuses one with none of the three.
    pub text: String,
    /// Pictures and files sent with the message, as a Session's send carries them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<SessionUpload>,
    /// What `@` and `#` named. Fleet writes the line that tells Helm; the thread keeps the words.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mentions: Vec<SessionTag>,
    /// Where the person is in Bridge. **Optional**, so a Bridge built before
    /// `#1075` still asks — Fleet hands a session this line only where one
    /// arrived. `crates/fleet/src/helm_session.rs` reads it; the thread's
    /// row never does, and keeps only what was typed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<HelmContext>,
    /// What the Helm Session runs on, **read only where this message starts it**: a Session that
    /// is live keeps the tuning it has, and `tune_session` changes it. Absent is the successor's
    /// predecessor's, else the machine's own. A blank string counts as absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// `low`, `medium`, `high`, `xhigh` or `max`, read as `model` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

/// Where the person is in Bridge, as one ask carries it. A snapshot of the
/// moment the message was sent, not a subscription: the picker may have moved
/// by the time the next message goes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmContext {
    pub screen: HelmScreen,
    /// The repository the rail has picked. Absent for All repositories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picked: Option<ManifestId>,
    /// The Studio open on the Studios surface — absent on its list, where
    /// `screen` is still [`HelmScreen::Studio`]. Since 14.7, `#1287`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub studio: Option<StudioId>,
    /// The node selected on that Studio's whiteboard. Only beside `studio`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<StudioNodeId>,
}

/// Which screen is showing. `apps/desktop/src/renderer/src/App.tsx` is the
/// one place that decides between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelmScreen {
    Overview,
    Manifest,
    Cleanup,
    /// The Studios surface: a repository's list, or one Studio open. Since 14.7.
    Studio,
    /// Kit: the MCP servers a Drone is handed. Since 15.1, `#1275`.
    Kit,
    /// This machine's own settings. **Added with `Kit` and not before it**:
    /// `#1287` added `Studio` and left this one out, so a person on Settings
    /// was reported to Helm as being on the Board.
    Settings,
    /// The Workflow creator: the definitions Fleet holds, one open for editing. Since 23.34.
    Workflows,
}

/// A message with something in it. **Blank does not decode**, so an empty
/// message is the transport's 400 and never a session started for nothing.
/// `AskCommander` carries one; `AskHelm` does not, because its message may be
/// only a file or a tag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HelmText(String);

impl HelmText {
    /// `None` where the text is blank.
    pub fn said(text: &str) -> Option<HelmText> {
        (!text.trim().is_empty()).then(|| HelmText(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A message that was only whitespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blank;

impl fmt::Display for Blank {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("a message to Helm has nothing in it")
    }
}

impl TryFrom<String> for HelmText {
    type Error = Blank;

    fn try_from(text: String) -> Result<HelmText, Blank> {
        HelmText::said(&text).ok_or(Blank)
    }
}

impl From<HelmText> for String {
    fn from(text: HelmText) -> String {
        text.0
    }
}

/// `get_helm_session` (`GET /helm/session`) and `start_helm_fresh` (`POST /helm/start_fresh`)
/// answer with this: the repository's live Helm Session and its thread, or none.
///
/// **`session` is absent before the first message** and after a person starts fresh. The dock
/// reads the thread from here, then follows `session.row` and `session.changed` on `/events` by
/// the session's id, as a Session page does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmSession {
    pub manifest_id: ManifestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionThread>,
}

/// What the person typed, kept in a thread row beside the files and tags it carried. A Commander's
/// thread draws it as Helm's once did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmAsked {
    pub ts: Instant,
    pub text: String,
    /// What the message carried, kept by Fleet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<SentFile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<SessionTag>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmFresh {
    pub ts: Instant,
    pub because: Freshness,
}

/// Why a conversation started over without a person asking it to. A plain
/// enum for `Silence`'s reason: no registry declares the set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// The agent CLI had no session by the stored id — its transcript was
    /// cleaned up, or never written on this machine.
    SessionNotFound,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmUnanswered {
    pub ts: Instant,
    pub why: String,
}
