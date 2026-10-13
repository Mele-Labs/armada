//! One repository's Helm conversation: what a person sends, and the socket its
//! replies come back on. `#939`.
//!
//! **`TurnMessage`'s shape, one subject over.** `opened`, the thread so far,
//! then what follows, with `missed` where a viewer fell behind — and the rows
//! the session writes are [`Shown`] rows, so a reply is read with the
//! vocabulary a Drone's turn already is. **Deliberately not `/events`**, for
//! the second socket's reason in `docs/practices/protocol.md`.
//!
//! **A conversation has no end to close on.** The socket carries every reply
//! for as long as a viewer holds it, and `closed` is sent only when a person
//! starts fresh and the thread it was showing is gone.

use serde::{Deserialize, Serialize};

use crate::event::Missed;
use crate::hosted_sessions::{SentFile, SessionTag, SessionUpload};
use crate::ids::{Instant, JobId, ManifestId, StudioId, StudioNodeId};
use crate::protocol_id::ProtocolId;
use crate::turn::Shown;

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
    /// arrived. `crates/fleet/src/helm/serving.rs` reads it; the thread's
    /// `asked` row never does, and keeps only what was typed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<HelmContext>,
}

/// Where the person is in Bridge, as one ask carries it — `#1075`'s three
/// decisions. A snapshot of the moment the message was sent, not a
/// subscription: the picker, the chip and the cursor may all have moved by
/// the time the next message goes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmContext {
    pub screen: HelmScreen,
    /// The repository the rail has picked. Absent for All repositories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picked: Option<ManifestId>,
    /// The Job chipped above Helm's message box — present only while the
    /// chip stands, gone the moment its `×` is pressed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chip: Option<JobId>,
    /// The row the cursor is on, in the Board or in Overview. Neither always
    /// has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<JobId>,
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
    Board,
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
    JobDetail,
}

/// `POST /helm/tune` — what Helm's next reply runs on. An absent model or effort is the
/// machine's own, and a blank string counts as absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuneHelm {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// `low`, `medium` or `high`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

/// What Helm's message box offers and runs on. Sent whole on `opened` and again as `facts`
/// whenever it changes; **never kept in the thread**, so a replay carries none.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmBoxFacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// The commands the agent said it has, for `/` to offer: Helm's own once it has spoken,
    /// otherwise the last any Session read.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
}

/// What `ask_helm` and `start_helm_fresh` answer with. **Not the reply**, which
/// arrives on the socket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmConversation {
    pub manifest_id: ManifestId,
    /// A reply is being written, or a message is waiting for one.
    pub replying: bool,
    /// The next message resumes a stored session. `false` starts a new one.
    pub resumes: bool,
}

/// One message on a Helm conversation's socket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "message", rename_all = "snake_case")]
pub enum HelmMessage {
    /// The first message on every connection.
    Opened(HelmOpened),
    /// What the person said, as Fleet took it.
    Asked(HelmAsked),
    /// What the session wrote: its prose, its calls, and `ended` with what the
    /// reply cost. `by` is `drone` on these, the decoder's word for a session's
    /// own output; which session is the socket's.
    Row(Shown),
    /// The stored session could not be resumed, so this reply starts a new one.
    Fresh(HelmFresh),
    /// No reply came, and why.
    Unanswered(HelmUnanswered),
    /// These words went to a Session or a Job and never to Helm's session. Since a thought was
    /// routed: `crate::thought`.
    Sent(crate::thought::HelmSent),
    /// What the message box offers and runs on, now. **Live only**: offered to whoever is
    /// watching, never held in the backfill and never written to the thread's file.
    Facts(HelmBoxFacts),
    /// This viewer fell behind and lost messages.
    Missed(Missed),
    /// Nothing more on this connection, and why.
    Closed(HelmClosed),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmOpened {
    pub protocol_id: ProtocolId,
    pub manifest_id: ManifestId,
    /// Whether a reply was being written when this opened.
    pub replying: bool,
    /// Older messages the thread left out, because the backfill is bounded.
    pub skipped: u64,
    /// What the message box offers and runs on as this opened. Always present, and `{}` when
    /// there is nothing to say.
    pub facts: HelmBoxFacts,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmAsked {
    pub ts: Instant,
    pub text: String,
    /// What the message carried, kept by Fleet and read with `get_helm_file`.
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmClosed {
    pub because: HelmSilence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelmSilence {
    /// A person started fresh. The thread this connection showed is gone.
    StartedFresh,
}

/// `helm.changed_checkout`: Helm wrote a file in a repository's own checkout.
///
/// **Helm's act as its own event type** — `docs/concepts/helm.md`, *Audit
/// trail*, the rule `studio.helm_acted` already follows one subject over. A
/// person who finds a file changed and did not change it reads this to see that
/// Helm did.
///
/// **A write, not a run.** `tool` is one of the built-ins that edits a file, so
/// the path is known. A shell line Helm ran may also have written something and
/// nothing in the session's stream says whether it did, so a `Bash` call is on
/// the conversation's thread like every other call and produces none of these.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelmChangedCheckout {
    pub manifest_id: ManifestId,
    /// The tool that wrote it, as the session's own stream named it.
    pub tool: String,
    /// What it wrote, relative to the checkout where it is inside it.
    pub path: String,
    pub at: Instant,
}
