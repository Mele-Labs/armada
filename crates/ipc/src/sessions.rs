//! The session ledger on the wire: what a harness reports about a session, and
//! what Fleet answers and publishes about one. `docs/concepts/session.md`.
//!
//! **Intake is Armada's own shape and names no harness's event.** A mod, a
//! Bridge-hosted session and a script all send the same [`SessionReport`], so
//! nothing past the adapter that sends it knows what a session was run in.
//!
//! **No instant is reported.** Fleet stamps each fact as it arrives, so a clock
//! on the sender's side is never compared with Fleet's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, JobId, ManifestId};
use crate::protocol_id::ProtocolId;

/// A session, by the id its harness knows it by.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    pub fn carried(value: impl Into<String>) -> Self {
        SessionId(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What started a session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionOrigin {
    /// Run by a person in a terminal, and reported by the harness it runs in.
    #[default]
    Terminal,
    /// Hosted by Bridge.
    Bridge,
}

/// Whether a session is still running.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Live,
    Ended,
}

/// Where an attachment stands, for every kind of one: a slot let go and a need
/// whose branch landed are different words and the same question.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentState {
    /// In force: the slot is held, the pull request is open.
    Standing,
    /// Used and done with: a merged pull request, a need whose branch landed.
    Spent,
    /// Let go before it was used: a released slot.
    GivenBack,
}

/// What a row of the ledger is held by. Not a foreign key to either table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HolderKind {
    Session,
    Job,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holder {
    pub kind: HolderKind,
    pub id: String,
}

/// The figures a session last reported. Each is absent until reported.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// One thing a harness says a session took or did. `kind` is open text: the
/// ledger keeps a word it has not met, and Fleet gives only `slot` and
/// `branch` a meaning of their own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentReport {
    pub kind: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub detail: BTreeMap<String, String>,
}

/// Which attachment a fact is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentNamed {
    pub kind: String,
    pub target: String,
}

/// One fact about a session, as it happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionFact {
    /// The session began, or a resumed one began again. Fleet resolves `cwd` to
    /// a repository and to the pool slot it stands in.
    Started {
        cwd: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default)]
        origin: SessionOrigin,
        /// The version of the `armada` mod reporting, so a session whose mod is older than the
        /// repository's can be marked. Absent from a mod that predates it. Since 23.62.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mod_version: Option<String>,
    },
    /// A title. **`named`** says a person chose it, in the terminal's `/rename`:
    /// it replaces whatever title there is. Otherwise it is the first prompt's
    /// line, kept only where the session has no title yet. `named` since 23.52.
    Titled {
        title: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        named: bool,
    },
    /// The session's directory changed.
    Moved {
        cwd: String,
    },
    Attached {
        attachment: AttachmentReport,
    },
    /// An attachment is over, as `spent` or `given_back`.
    Settled {
        attachment: AttachmentNamed,
        state: AttachmentState,
    },
    Measured {
        usage: SessionUsage,
        /// The account's plan windows as this session last read them; empty off a subscription. The
        /// windows are the account's, so Fleet keeps the newest from any session (`PlanUsage`).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        plan: Vec<crate::plan_usage::PlanWindow>,
        /// What fills the context window, estimated by the mod. Fleet keeps the latest.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context_breakdown: Option<crate::context_breakdown::ContextBreakdown>,
    },
    /// What the terminal runs on: its model, its effort, its permission mode
    /// and the commands it lists. **A field left out is unchanged**, and an
    /// empty `commands` is no new list. Since 23.53.
    Tuned {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        effort: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<crate::hosted_sessions::SessionMode>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        commands: Vec<TerminalCommand>,
        /// As on `started`. Since 23.62.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mod_version: Option<String>,
    },
    /// A turn ended. `said` is the first line of the last thing the agent said to the person, which
    /// is how Fleet reads a terminal session as having replied without the agent being asked to say
    /// so; absent where the turn ended on tools alone, and from a mod that predates it. Since 23.86.
    TurnCompleted {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        said: Option<String>,
    },
    /// A turn began: the person sent a prompt in the terminal, or something woke the session. It ends
    /// any reply or done Fleet read off the turn before. Since 23.86.
    TurnStarted,
    /// A turn ended on the plan's limit, which resets at `resets_at`. Fleet holds the session
    /// and wakes it then (`SessionRecord::held_until`).
    Limited {
        resets_at: Instant,
    },
    /// Files the session wrote, relative to its `cwd`. **Paths only**, and only those under it.
    /// Fleet reads them for write clashes (`docs/concepts/fleet.md`, *Write clashes*). Since 23.81.
    Touched {
        paths: Vec<String>,
    },
    /// A file the session read, relative to its `cwd`. **A path and nothing else**, and only one under
    /// it. Counted for the Fleet's timeline (`docs/practices/protocol.md`, *The Fleet's timeline*).
    Read {
        path: String,
    },
    /// A shell command the session ran, as it was typed. Fleet hides what looks like a secret in it,
    /// cuts it, and counts it for the timeline; it is kept nowhere else.
    Ran {
        command: String,
    },
    /// The session itself wrote a file, not a subagent: the windows it showed are being changed.
    Wrote,
    Ended {
        reason: String,
    },
}

/// A command a terminal session lists, for `/` to offer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCommand {
    pub name: String,
    #[serde(default)]
    pub says: String,
}

/// What a terminal session runs on, as its mod last said. **Read-only for the
/// mode**: the mods API cannot switch a live session's permission mode
/// (spike 27). Since 23.53.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalFacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<crate::hosted_sessions::SessionMode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<TerminalCommand>,
    /// Whether its mod asked within the last ten seconds, which is what a
    /// message sent to it needs. A live session that is not listening is one
    /// nothing can be said to. Since 23.69.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub listening: bool,
    /// The question the session put in its terminal and Bridge may answer
    /// first, while it waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asked: Option<crate::helm_call::HelmCallInFlight>,
}

/// What a harness sends Fleet. `POST /sessions/report`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionReport {
    pub harness: String,
    pub session_id: SessionId,
    pub fact: SessionFact,
}

/// `rename_session`: a person gave a session a name in Bridge. Since 23.52.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameSession {
    pub session_id: SessionId,
    pub title: String,
}

/// `start_session_run`: the person ran a shell block from a Session's thread. `POST /sessions/run/start`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartSessionRun {
    pub session_id: SessionId,
    pub command: String,
}

/// `start_session_run`, answered 202: the run is underway; what it prints is on `observe_session_run`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRunUnderway {
    pub run_id: String,
}

/// `stop_session_run`: end a run's process group. `POST /sessions/run/stop`. Answered 200 with the same body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopSessionRun {
    pub run_id: String,
}

/// One message on `observe_session_run`'s socket, `GET /sessions/run/:run_id/observe`. What the run
/// printed so far and then what it prints next as `lines`, then `exited` once, then the socket closes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionRunMessage {
    Opened {
        protocol_id: ProtocolId,
        run_id: String,
    },
    Lines {
        lines: Vec<String>,
    },
    /// This many messages were dropped before this viewer read them: it must read the run again.
    Missed {
        dropped: u64,
    },
    /// `code` is `null` where the process group was stopped or killed by a signal.
    Exited {
        #[serde(deserialize_with = "stated")]
        code: Option<i32>,
    },
}

/// An `Option` whose key must be present: `null` is the answer, a run a signal ended.
fn stated<'de, D>(input: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<i32>::deserialize(input)
}

/// `show_window`: a session shows the person a web page in a window of Bridge's own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowWindow {
    /// `http` or `https`; anything else is refused.
    pub url: String,
    /// What the ledger and the thread call it. Absent is the address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Which session shows it. **Read only where the connection places no hosted session**: a
    /// hosted session is the one whose process holds the call, whatever this says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// Several takes of one change, each a page of its own, which the window swaps between. **When
    /// present, `url` is the first take's.** Each is `http` or `https` as `url` is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variations: Vec<WindowTake>,
}

/// One take of a change the session shows in its window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowTake {
    /// What the bar calls it, beside its letter.
    pub name: String,
    /// `http` or `https`; anything else refuses the whole call.
    pub url: String,
}

impl WindowTake {
    /// The takes as the window's artifact on the ledger keeps them, one string in its `detail`,
    /// which holds strings only. Bridge reads it back to reopen the window with its takes.
    pub fn as_detail(takes: &[WindowTake]) -> String {
        serde_json::to_string(takes).unwrap_or_default()
    }
}

/// `window_closed`: the person closed the window a session showed. Bridge tells it; nothing is
/// closed by it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowClosed {
    pub session_id: SessionId,
    /// The address the window was opened on, as `show_window` named it.
    pub url: String,
}

/// What a press on a waiting item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitingActKind {
    /// Open the walk window at `target`, an address.
    Walk,
    /// Go to the question or permission card; `target` is its call.
    Answer,
    /// Open the pull request; `target` is its number.
    ApprovePr,
    /// Show a command; `target` is the command.
    Run,
    /// Send the Session a message: go to its box. `target` is the Session's id. Since 23.80.
    Message,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingAct {
    pub kind: WaitingActKind,
    pub target: String,
}

/// One choice of an `answer` item, in the order to draw and number them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingOption {
    pub label: String,
    /// What choosing it means, where the asker said. Drawn under the label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Where on a walked page a question points: the address, and the element as a note taken in the
/// window names one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowPlace {
    /// On the origin of a window the session showed.
    pub url: String,
    /// A CSS selector for the element.
    pub selector: String,
    /// Text the element shows, which picks one where the selector finds several.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// `ask_in_window`: a session asks the person about a place on the page its walk window shows. Fleet
/// answers at once; the answer comes back later as a message from the person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskInWindow {
    pub question: String,
    /// The choices to offer. The person may always answer in words of their own instead.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<WaitingOption>,
    /// The page to take the person to, on the origin of a window this session showed.
    pub url: String,
    /// The element to highlight there, as a CSS selector.
    pub selector: String,
    /// Text the element shows, to pick one where the selector finds several.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Which session asks. **Read only where the connection places no hosted session**, as `ShowWindow`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// Who put an item on the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitingSource {
    /// The agent, through `waiting_for`.
    Agent,
    /// Fleet: the agent's question card is open.
    AskCard,
    /// Fleet: a walk window the session showed has not been approved.
    Walk,
    /// Fleet: a permission card is open.
    Permission,
    /// Fleet: a question the session asked in its walk window, `ask_in_window`, is unanswered.
    WindowAsk,
    /// Fleet: another Session's agent started this one and its first message has not been sent.
    /// The item has no reply, since sending that message is the answer. Since 23.80.
    FirstMessage,
    /// Fleet: the Session's turn ended on a message of the agent's and the person has not answered it.
    /// Derived on every read from the turn and the thread, never kept, so it does not depend on the
    /// agent calling any tool. Id `replied:<turn end>`, text the first line of that message.
    Replied,
    /// Fleet: `done_since` stands. Derived on every read, never kept: id `done:<done_since>`, text
    /// `Done`. The agent's `all_done` states it; nothing the person sees depends on that call alone.
    Done,
}

/// What a Session is doing, read from what happened and never from what the agent said. One answer,
/// for every surface that draws a Session: the Sessions list, the Cockpit's tiles, lanes and calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStanding {
    /// A turn is running.
    Working,
    /// A turn is running and nothing has happened for the quiet threshold.
    Stuck,
    /// The turn ended on a message of the agent's and the person has not answered since.
    Replied,
    /// The agent said, through `all_done`, that nothing is left, and the person has not answered.
    Done,
    /// Nothing is owed. What a `replied` or `done` becomes once the person dismisses it.
    Idle,
}

/// One thing a session is waiting on the person for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingItem {
    /// Stable across updates: the agent's own, or `ask:<call>`, `walk:<url>`, `perm:<call>`,
    /// `window_ask:<row>`, `unsent:<session>`, `replied:<turn end>`, `done:<done_since>`.
    pub id: String,
    pub text: String,
    /// When it began to wait.
    pub since: Instant,
    pub source: WaitingSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act: Option<WaitingAct>,
    /// Present only on an `answer` item that has choices.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<WaitingOption>,
    /// Where a question asked in the walk window points. Present only on a `window_ask` item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<WindowPlace>,
}

/// One item as the agent states it. Fleet stamps `since` and `source`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingInput {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act: Option<WaitingAct>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<WaitingOption>,
}

/// `waiting_for`: the agent sets the whole list of what it needs from the person. An empty list clears it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetWaitingFor {
    pub items: Vec<WaitingInput>,
    /// Which session. **Read only where the connection places no hosted session**, as `ShowWindow`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `all_done`: the agent says it has nothing left to work on and it is safe to end the session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllDone {
    /// Which session. **Read only where the connection places no hosted session**, as `SetWaitingFor`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `finish_up`: the agent is finished with its slots, branches and servers, and asks Fleet to give
/// them back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinishUp {
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `finish_up` gave back and what it kept.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinishedUp {
    /// The slots released.
    pub released: Vec<u32>,
    /// The local branches deleted, each already in the base.
    pub deleted_branches: Vec<String>,
    /// What stayed, one sentence each, with why.
    pub kept: Vec<String>,
}

/// How the agent is told to settle an item itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitingMode {
    /// Think it through and decide.
    Best,
    /// Take the reasonable path and keep moving.
    Quick,
}

/// `answer_waiting`: the person settles one item. `choice` is an index into the item's `options`, `text`
/// is their own words, and `mode` leaves it to the agent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerWaiting {
    pub session_id: SessionId,
    pub item_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<WaitingMode>,
}

/// `dismiss_waiting`: the person drops one item for good. Nothing is sent to the agent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DismissWaiting {
    pub session_id: SessionId,
    pub item_id: String,
}

/// One row of the ledger as the wire carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub kind: String,
    /// The repository a slot or a branch is of; absent where it is of none
    /// Fleet serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    pub target: String,
    pub state: AttachmentState,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub detail: BTreeMap<String, String>,
    pub since: Instant,
    pub changed_at: Instant,
}

/// A session and everything it holds. Answered by `report_session`, listed by
/// `list_sessions`, and published whole as `session.changed`, so a client
/// replaces the row rather than patching it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: SessionId,
    pub harness: String,
    pub origin: SessionOrigin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub state: SessionState,
    pub started_at: Instant,
    pub last_seen_at: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_turn_at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<String>,
    pub usage: SessionUsage,
    pub attachments: Vec<Attachment>,
    /// What a session Bridge hosts carries beyond a terminal's. Absent on one
    /// a harness reports. Since 23.49.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted: Option<crate::hosted_sessions::HostedFacts>,
    /// What a session run in a terminal runs on, once its mod has said. Since 23.53.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<TerminalFacts>,
    /// A session run in a terminal whose `armada` mod is older than the one the repository holds,
    /// or reported no version at all. Absent otherwise, and where Fleet cannot read the
    /// repository's. Since 23.62.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mod_out_of_date: bool,
    /// What the session waits on the person for: the agent's list with Fleet's own items merged in, the
    /// agent's first. Empty for an ended session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waiting_for: Vec<WaitingItem>,
    /// When the agent said, through `all_done`, that it has nothing left to work on. Cleared by the
    /// person's next message, by a non-empty `waiting_for`, and by a hosted turn starting again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_since: Option<Instant>,
    /// What the Session is doing, derived by Fleet on every read from its turn, its thread and
    /// `done_since`. Absent from a Fleet that predates it, and Bridge then draws the Session as it
    /// drew it before. Since 23.86.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standing: Option<SessionStanding>,
    /// The plan's limit stopped this session, and Fleet wakes it with `Continue` a minute after this
    /// instant, when the limit has reset. Cleared by a turn, a message from the person, the wake
    /// and the session ending. Absent on a session that is not held.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_until: Option<Instant>,
    /// What fills the session's context window, as `/context` shows it, from the latest report. Absent
    /// until a mod reports one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_breakdown: Option<crate::context_breakdown::ContextBreakdown>,
    /// This hosted session is a repository's Helm: opening it from the Sessions list opens the
    /// dock, and at most one is live per `manifest_id`. Absent on every other session.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub helm: bool,
}

/// `list_sessions`: the most recently seen first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionList {
    pub sessions: Vec<SessionRecord>,
}

/// One holder of what `who_owns` was asked about, and its row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ownership {
    pub holder: Holder,
    pub attachment: Attachment,
    /// The holder's title, where it is a session that has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// `who_owns`: every holder of the thing asked about, those still holding it
/// first. **Empty is an answer**: nothing the ledger knows holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owners {
    pub holders: Vec<Ownership>,
}

/// `claim_pull_request`: a Session or Job takes an open pull request nobody holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimPullRequest {
    pub number: u64,
    /// Which session claims it. **Read only where the connection places no hosted session and no
    /// Drone**, as `ShowWindow::session_id` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// Or which Job, for a person attaching the pull request to one they picked. Exactly one of
    /// the two, read where `session_id` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<JobId>,
}

/// What `claim_pull_request` answers: the pull request now held, and by whom.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestClaimed {
    pub number: u64,
    pub branch: String,
    pub url: String,
    /// `session` or `job`, and its id.
    pub holder_kind: String,
    pub holder_id: String,
}
