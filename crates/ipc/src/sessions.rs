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

use crate::ids::{Instant, ManifestId};
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
    /// A turn ended. `said` is the close of the last thing the agent said to the person (its last 1,500
    /// characters from a line start; a mod before 0.3.39 sent the first line), which
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
    /// A turn ended because the account's sign-in ran out and would not renew. Fleet signs itself out
    /// for every Job, as a hosted Job's harness line does, and wakes the session once the account
    /// works again: a later `turn_completed` from it is that.
    SignedOut,
    /// Files the session wrote, relative to its `cwd`. **Paths only**, and only those under it.
    /// Fleet reads them for write clashes (`docs/concepts/fleet.md`, *Write clashes*). Since 23.81.
    /// `by` is the id of the subagent that wrote them, absent for the session's own agent, and `act`
    /// whether the call made the file or changed it; both feed the Field (`docs/concepts/session.md`,
    /// *Who touched which file*).
    Touched {
        paths: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        by: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        act: Option<crate::session_plan::TouchAct>,
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
    /// The agent's task list, whole, as its last `TodoWrite`, `TaskCreate` or `TaskUpdate` left it.
    /// An empty list clears it. Fleet keeps it and records when each item was first seen completed
    /// (`docs/concepts/session.md`, *The build record*).
    Tasks {
        tasks: Vec<crate::session_build::ReportedTask>,
    },
    /// The agent left plan mode (`ExitPlanMode`).
    PlanLeft,
    /// A `git commit` ran and its output showed a commit made. `branch` is the one that output named;
    /// `commit` its short hash and `subject` its first line, empty from a mod before 0.3.46.
    Committed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        commit: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        subject: String,
    },
    /// The session's own agent made `calls` more tool calls since the last of these, not its subagents'.
    /// Told every ten by a terminal session's mod; Fleet counts a hosted session's itself and reads
    /// none of these from it. Since mod 0.3.46.
    Calls {
        calls: u32,
    },
    /// A `git push` ran and its output showed a ref updated. `branch` is the one that output named.
    Pushed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
    },
    /// The harness opened its own permission prompt for `tool`, and the turn waits on a person at the
    /// terminal. `detail` is the command or the path, one line and cut, empty where the tool has
    /// none. **The harness's fact, told by the mod**, never the agent's. Since mod 0.3.22.
    PermissionAsked {
        tool: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        detail: String,
    },
    /// A turn was answered: its model, tokens and the tools it called. Fleet keeps one row a message
    /// id, so a turn reported twice is one.
    TurnMeasured {
        turn: crate::turn_usage::TurnUsage,
    },
    /// The prompt `permission_asked` told of is closed: the call ran, was refused, or was abandoned.
    PermissionSettled,
    /// What the agent knows and was told, whole, replacing the last. The mod sends it at the start
    /// and whenever it changes. Fleet keeps the latest.
    Equipped {
        loadout: crate::loadout::LoadoutItems,
    },
    /// The agent used a skill, a tool or a subagent the mod reports. Fleet keeps the last fifty turns.
    Used {
        #[serde(rename = "use")]
        used: crate::loadout::LoadoutUse,
    },
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

/// `name_session`: Armada names a hosted session from its first message, on request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameSession {
    pub session_id: SessionId,
}

/// `start_session_run`: the person ran a shell block from a Session's thread. `POST /sessions/run/start`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartSessionRun {
    pub session_id: SessionId,
    pub command: String,
    /// The terminal's size in cells. Absent is 80 by 24.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cols: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<u16>,
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

/// `write_session_run`: keys typed into a run's terminal. `POST /sessions/run/write`. Answered 200 with
/// `SessionRunUnderway`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteSessionRun {
    pub run_id: String,
    /// Written to the terminal as it is: Ctrl-C is the byte 3.
    pub data: String,
}

/// `resize_session_run`: the run's terminal changed size. `POST /sessions/run/resize`. Answered 200 with
/// `SessionRunUnderway`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResizeSessionRun {
    pub run_id: String,
    pub cols: u16,
    pub rows: u16,
}

/// One message on `observe_session_run`'s socket, `GET /sessions/run/:run_id/observe`. What the run
/// printed so far and then what it prints next as `output`, then `exited` once, then the socket closes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionRunMessage {
    Opened {
        protocol_id: ProtocolId,
        run_id: String,
    },
    /// Terminal output with its escape sequences intact. A character is never split across two.
    Output {
        text: String,
    },
    /// This many messages were dropped before this viewer read them: it must read the run again.
    Missed {
        dropped: u64,
    },
    /// `code` is `null` where the process group was stopped or killed by a signal, and `stopped`
    /// is true where it was Stop that ended it: a null code with `stopped` false is a signal's.
    Exited {
        #[serde(deserialize_with = "stated")]
        code: Option<i32>,
        stopped: bool,
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    /// A scripted walk of the page, played by Bridge from outside it: a ring, a caption and the
    /// acts, on any page. **The same steps the mock walks use**, `crate::WalkStep`. Absent: the
    /// page is shown as it is, and a mock that plays its own walk keeps doing so.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<crate::WalkStep>,
    /// Other web origins the window may pass through, such as a single sign-on provider: `https://host`
    /// and an optional port, with no path. The page stays pinned to the shown origin and these.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub origins: Vec<String>,
    /// Addresses the window loads in order before the shown page, to sign in. **A secret**: held in
    /// memory until Bridge opens the window and written nowhere. Each is on the shown origin or one of `origins`.
    #[serde(default, skip_serializing_if = "SignIn::is_empty")]
    pub sign_in: SignIn,
}

/// The addresses a session signs in with. **Its `Debug` says how many and nothing else**, so a
/// log line that prints a request cannot print one.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SignIn(pub Vec<String>);

impl SignIn {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for SignIn {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "SignIn({} held back)", self.0.len())
    }
}

/// `take_window_sign_in`: Bridge asks for the sign-in addresses held for the window it is opening.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakeWindowSignIn {
    pub session_id: SessionId,
    /// The address the window is opened on, as `show_window` named it.
    pub url: String,
}

/// What `take_window_sign_in` answers: the addresses to load, once. Empty where none are held.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSignIn {
    pub sign_in: SignIn,
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

/// The origins a window may pass through as the window's artifact on the ledger keeps them, one
/// string in its `detail`. Bridge reads it back to reopen the window with them.
pub fn origins_as_detail(origins: &[String]) -> String {
    serde_json::to_string(origins).unwrap_or_default()
}

/// `window_closed`: the person closed the window a session showed. Bridge tells it; nothing is
/// closed by it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowClosed {
    pub session_id: SessionId,
    /// The address the window was opened on, as `show_window` named it.
    pub url: String,
}

/// `window_failed`: the window a session showed could not show its page. Bridge tells it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowFailed {
    pub session_id: SessionId,
    /// The address the window was loading.
    pub url: String,
    /// What Bridge saw, in one line: `ERR_CONNECTION_REFUSED`, `HTTP 500`, `the page crashed`.
    pub why: String,
}

/// What `window_failed` answers: whether the owner of the walk was told now. `false` where it was
/// told already, or nothing is there to hear it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalkTold {
    pub told: bool,
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

/// `propose_draft`: a session puts a draft to the person for approval, such as a pull request
/// description, a comment or a message. Fleet keeps a `draft` row and answers at once; the answer
/// comes back later as a message from the person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposeDraft {
    /// One short line naming what it is and where it goes.
    pub title: String,
    /// The draft, in markdown.
    pub text: String,
    /// The text it replaces, where there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// Which session proposes it. **Read only where the connection places no hosted session**, as `ShowWindow`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `withdraw_draft`: a session takes back a draft it proposed with `propose_draft`, so it leaves the
/// person's Needs you list and its row settles `withdrawn`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithdrawDraft {
    /// The id `propose_draft`'s receipt gave, `draft:<row id>`. The bare row id reads the same.
    pub draft: String,
    /// Which session withdraws it. **Read only where the connection places no hosted session**, as `ShowWindow`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `withdraw_window`: a session takes back a walk it showed with `show_window`, so it leaves the
/// person's Needs you list and stops holding the session's pending plan. Showing the page again
/// makes it wait again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithdrawWindow {
    /// The address the window was shown on, as `show_window` was given it.
    pub url: String,
    /// Which session withdraws it. **Read only where the connection places no hosted session**, as `ShowWindow`'s.
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
    /// Fleet: a draft the session proposed with `propose_draft` waits on the person. Read off its
    /// row as `draft:<row>`; settled by `answer_waiting`.
    Draft,
    /// Fleet: the Session waits on its start card for the person to press Start (`begin_session`).
    /// The item has no reply, since Start is the answer. Since 23.80; every held Session since 25 Oct.
    FirstMessage,
    /// Fleet: the turn ended on a reply whose close asks the person something, and the agent has asked
    /// nothing through a tool. Derived on every read, never kept: id `replied:<turn ended at>`, text the
    /// sentence that asks. A reply that only reports is `SessionStanding::Replied` and no item.
    Replied,
    /// Fleet: `done_since` stands. Derived on every read, never kept: id `done:<done_since>`, text
    /// `Done`. The agent's `all_done` states it; nothing the person sees depends on that call alone.
    Done,
    /// Fleet: the plan the agent recorded is not approved yet. Derived on every read: id
    /// `plan:<version>`. It is settled by `approve_plan` or `send_plan_changes`.
    Plan,
    /// Fleet: a judge raised a concern about the work. Kept, id `judge:<n>`, with `concern` beside the
    /// text. It is settled by `send_concern` or `dismiss_concern`.
    Judge,
    /// Fleet: the session's first write found every worktree slot held and it is in the line for one,
    /// and the person has not said whether it waits or Fleet makes a slot. Derived on every read,
    /// never kept: id `slot-choice`, text `Every slot is in use`, an `answer` act, and exactly two
    /// options, `Wait for a slot` (choice 0) and `Add a slot` (choice 1), labels only. Answering
    /// with a `choice` settles it and no message goes to the agent: it was told to end its turn and
    /// is moved in when a slot is granted. A free-text answer is refused.
    SlotChoice,
    /// Fleet: a bug or feature Job started and its first message did not say how the work is
    /// delivered. Derived on every read from the thread's `delivery` row while it has no choice: id
    /// `delivery`, text `How should this work be delivered?`, an `answer` act, and exactly three
    /// options, `Auto land` (choice 0), `Open PR only` (choice 1) and `Keep local` (choice 2), each
    /// with a description. The `choice` is kept on the row and the agent is told it.
    Delivery,
    /// Fleet: words a person taught open nowhere Fleet could find in this Session's repository. Derived
    /// on every read from the store's question and the mod, while the Words mod has no address for
    /// the repository: id `words:<mod>:<manifest id>`, text `Where does <name> open in <manifest id>?`,
    /// an `answer` act, and an option for each address any Words mod holds (label the address, a
    /// description naming the repositories that use it) then `Other`. A `choice` of an address
    /// writes it to the mod; `Other` takes the `text`, which needs `{match}` where the matched words go.
    /// Asked once for a mod and repository, and dismissing it does not ask again. No message goes to
    /// the agent.
    WordsAddress,
    /// Fleet: a Fleet mod's rule says this notice is a call, so the Session was told it and the person
    /// is asked to look. Derived on every read from the store's call: id `call:<n>`, text the words the
    /// Session was told, a `message` act that opens the Session. Dismissing it settles it and tells the
    /// agent nothing.
    FleetCall,
    /// Fleet: the Job finished, by `all_done` or by its pull request merging, with written project tasks
    /// it was started with, and whether they are done is not said. Derived on every read from the
    /// tasks, never kept in the Job's list: id `task_done:<project>`, one per project, text `Mark <title>
    /// done?` for one task or `Mark the <project> tasks done?` for several, an `answer` act, and exactly
    /// two options, `Mark done` (choice 0) and `Keep open` (choice 1), labels only. Answering with a
    /// `choice` settles every task asked in that project and no message goes to the agent. Not
    /// dismissed. Listed only while the Job is live: the ask stands on the tasks when it ends, and the
    /// Projects tab settles it.
    TaskDone,
    /// Fleet: a watch of the Job ended `not_ours`. Derived on every read from the store's watch: id
    /// `watch_not_ours:<watch id>`, text `<watch>: <parts> failed`, an `answer` act whose `target` is
    /// the watch id, and exactly two options, `Ignore` (choice 0) and `Fix it` (choice 1), labels only.
    /// `Ignore` settles it and tells the agent nothing; `Fix it` sends the agent the part, its
    /// sentence and its link, then settles it. Words alone go to the agent as a message and settle
    /// it. Dismissing settles it. Listed only while the Job is live.
    WatchNotOurs,
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
    /// The turn ended on a message of the agent's that only reports, and the person has not answered
    /// since. Under Idle: it is the unread mark, and raises no item. One that asks is an item instead.
    Replied,
    /// The agent said, through `all_done`, that nothing is left, and the person has not answered.
    Done,
    /// Nothing is owed. What a `done` becomes once the person dismisses it.
    Idle,
}

/// One thing a session is waiting on the person for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingItem {
    /// Stable across updates: the agent's own, or `ask:<call>`, `walk:<url>`, `perm:<call>`,
    /// `window_ask:<row>`, `draft:<row>`, `unsent:<session>`, `done:<done_since>`.
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
    /// The procedure the person follows, in order; its press opens them as steps.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<String>,
    /// Where a question asked in the walk window points. Present only on a `window_ask` item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<WindowPlace>,
    /// What a `judge` item carries beside its text. Present only on that source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concern: Option<crate::session_rail::JudgeConcern>,
    /// Where a `judge` item points: the files its quotation lands in, as the change leaves them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cites: Vec<crate::session_rail::WaitingCite>,
    /// A `replied` item whose sentence he said is a question (`confirm_waiting`). Absent where he has not.
    #[serde(default, skip_serializing_if = "is_false")]
    pub confirmed: bool,
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
    /// The procedure the person follows, in order; its press opens them as steps.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<String>,
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

/// A follow-up the agent said it keeps for itself, read from the close of a reply and open until it is
/// settled. The same sentence said again is the same one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLater {
    /// What `settle_later` names it by.
    pub id: String,
    /// As the agent said it.
    pub sentence: String,
    /// When the reply that said it ended.
    pub said_at: Instant,
    /// He said it is a follow-up, with thumbs up (`confirm_later`). Absent where he has not.
    #[serde(default, skip_serializing_if = "is_false")]
    pub confirmed: bool,
}

/// How a follow-up was settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaterHow {
    /// The work is done.
    Done,
    /// It is an issue now: `issue` names it.
    Filed,
    /// It will not be done: `reason` says why.
    Dropped,
}

/// `settle_later`: one open follow-up is settled, by the agent or by the person. Answered with
/// [`LaterSettled`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettleLater {
    /// The `SessionLater::id`.
    pub id: String,
    pub how: LaterHow,
    /// The issue it was filed as, where `how` is `filed`: the forge's number or a tracker's key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Why it was dropped, where `how` is `dropped`. **Required of an agent's call**; the person's Drop needs none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// He pressed thumbs down, Not deferred: the sentence is kept as no follow-up. **Only the person's
    /// call** (no connection places a job) may set it, and only with `how` `dropped`. Absent where false.
    #[serde(default, skip_serializing_if = "is_false")]
    pub misread: bool,
}

/// `confirm_later`: thumbs up on an open follow-up. Its sentence is kept as one, the row stays and
/// reads `confirmed`. The person's call; `session_id` names the Session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmLater {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `defer_later`: the agent keeps one follow-up for itself, as a sentence. Answered with [`LaterDeferred`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeferLater {
    /// What the job will do later, as it would say it.
    pub sentence: String,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `defer_later` answers: the follow-up kept, and the ones open with it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaterDeferred {
    pub id: String,
    pub sentence: String,
    pub open: Vec<SessionLater>,
}

/// What `settle_later` answers: the follow-up that was settled and the ones still open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaterSettled {
    pub id: String,
    pub sentence: String,
    pub open: Vec<SessionLater>,
}

/// How the agent is told to settle an item itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitingMode {
    /// Think it through and decide.
    Best,
    /// Just get it done: finish the work and land it, skipping the formalities.
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
    /// The person's corrected text for a `draft` item, which approves it with their words. Refused on
    /// an item of any other source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<String>,
}

/// `dismiss_waiting`: the person drops one item for good. An item the agent stated tells it so;
/// one Fleet derived is only dropped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DismissWaiting {
    pub session_id: SessionId,
    pub item_id: String,
    /// Why he removed it, sent to the agent after the item where it is non-blank. Absent where he gave none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// He pressed Not a question on an item Fleet read from a reply: its sentence is kept as one that
    /// asks nothing. Refused on an item of any other source. Absent where false.
    #[serde(default, skip_serializing_if = "is_false")]
    pub misread: bool,
}

/// `confirm_waiting`: thumbs up on an item Fleet read from a reply. Its sentence is kept as one that
/// asks, the item stays, and it reads `confirmed` from then on. Refused on an item of any other source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmWaiting {
    pub session_id: SessionId,
    pub item_id: String,
}

fn is_false(value: &bool) -> bool {
    !*value
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

/// One attachment as it stood from `at`, until the next version of the same `(kind, manifest_id,
/// target)` or for good. Not the row `Attachment` carries: that is the latest, this is the history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentVersion {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    pub target: String,
    pub state: AttachmentState,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub detail: BTreeMap<String, String>,
    pub at: Instant,
    /// The thread row id of the person's latest message when this version was written. Fleet works it out:
    /// absent for a version written before there was a message, or before this was kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_message: Option<String>,
    /// The plan task in progress when this version was written, absent where none was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_task: Option<String>,
}

/// `get_attachment_history`: every version of everything one session holds, oldest first. Read on
/// demand, never carried on a `SessionRecord`, which is published whole on every change.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentHistory {
    pub versions: Vec<AttachmentVersion>,
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
    /// The Loadout in brief, from the latest `equipped` and the latest turn's uses. Absent where no
    /// `equipped` was ever reported. The whole of it is `get_session_loadout`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loadout_summary: Option<crate::loadout::LoadoutSummary>,
    /// This hosted session is a repository's Helm: opening it from the Sessions list opens the
    /// dock, and at most one is live per `manifest_id`. Absent on every other session.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub helm: bool,
    /// A Mod Job, started from Mod for modding Armada itself. Absent on every other session.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub r#mod: bool,
    /// Auto land is on: Fleet turns on the forge's auto-merge for each pull request the Session
    /// holds as it attaches, and off again when this is turned off. Absent is off. Hosted only.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub auto_land: bool,
    /// What a hosted session is doing, as Fleet's gate saw its tool calls. Absent before the first
    /// call, on a session a terminal runs and once the session ends. Since 14 Oct 2026.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<crate::session_activity::SessionActivity>,
    /// What the Session has built, derived by Fleet on every read. Absent for a Session that has done
    /// nothing Fleet observed. `docs/concepts/session.md`, *The build record*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<crate::session_build::SessionBuild>,
    /// The workflow id the Session runs under. Absent is a custom Session, which is not a default
    /// workflow. Hosted Sessions only. Since 23.91. `docs/concepts/session.md`, *A Session's proposal*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<String>,
    /// What the proposer did for this Session. Absent where it was never asked. Since 23.91.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal: Option<SessionProposal>,
    /// The plan the agent recorded, with what Fleet saw of it carried out. Absent until it records
    /// one. `docs/concepts/session.md`, *The plan*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<crate::session_plan::SessionPlan>,
    /// The latest files the Session and its subagents wrote, newest last, for the Field. Absent
    /// until one is seen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touches: Vec<crate::session_plan::FileTouch>,
    /// The index of the workflow step the Session is on, read by Fleet from what it observed and never
    /// from the agent's word. Absent for a custom Session, and once the delivering step's pull request
    /// has merged with no step after it. `docs/concepts/session.md`, *The rail*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_step: Option<u32>,
    /// A judge is reading the work now. Held in memory: a Fleet that restarts reads nothing as reading.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub judging: bool,
    /// The last reading's outcome. Absent until a judge has read. `concerns` 0 is "No concerns".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge: Option<crate::session_rail::JudgeReading>,
    /// Concerns the person has settled, oldest first, a bounded window. Each is a `judge` item with its
    /// `concern.settled` set, for the thread to draw as one line. They are not in `waiting_for`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settled: Vec<WaitingItem>,
    /// Dispatched to wait on another Session and not started: it holds its brief, no slot and no
    /// process until the condition is met. Absent once it has started, and on a Session that never
    /// waited. `docs/concepts/session.md`, *A Session that waits on another*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waits: Option<crate::session_waits::SessionWait>,
    /// The follow-ups the agent said it keeps for itself, each open until it is settled, oldest first.
    /// `all_done` is refused while any is open.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub later: Vec<SessionLater>,
    /// The name of the live watch that started first, set while it is all that keeps the Job open: its
    /// turn has ended, nothing is in `waiting_for`, and nothing of its own runs in the background. Absent
    /// otherwise. Not a Needs you item. `docs/design/fleet-watches-ci-and-deploys-for-a-job.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_open_by: Option<String>,
    /// How the Job was started, as it stood when its process first ran, so a later change of model,
    /// effort or mode does not rewrite it. Absent for a Job started before Fleet kept it, and for a
    /// terminal session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<SessionStarted>,
    /// When each step of the workflow began, oldest first. A step the Job went back to appears again.
    /// Empty for a custom Session and for one that has entered no step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workflow_visits: Vec<StepVisit>,
    /// The steps the person skipped, by the index `workflow_step` uses, ascending. The rail counts each
    /// one passed. Empty where none is. `skip_session_step`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_steps: Vec<u32>,
}

/// How a Job was started. Every part is absent where it was not set then.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStarted {
    /// The workflow's name as it read then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<String>,
    /// The landing chosen by then. A Job that is asked later has none here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<crate::session_delivery::DeliveryChoice>,
    /// The model it ran on, the machine's own where none was picked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<crate::hosted_sessions::SessionMode>,
    /// The Job that started it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<crate::session_starts::SessionLink>,
}

/// The time a workflow step began.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepVisit {
    /// The step's index in the workflow.
    pub step: u32,
    pub since: Instant,
}

/// A value of a Session the proposer can pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposedField {
    Title,
    Workflow,
    Model,
    Effort,
}

impl ProposedField {
    /// The spelling Fleet keeps a list of them under.
    pub fn as_wire(self) -> &'static str {
        match self {
            ProposedField::Title => "title",
            ProposedField::Workflow => "workflow",
            ProposedField::Model => "model",
            ProposedField::Effort => "effort",
        }
    }

    pub fn from_wire(spelling: &str) -> Option<ProposedField> {
        [
            ProposedField::Title,
            ProposedField::Workflow,
            ProposedField::Model,
            ProposedField::Effort,
        ]
        .into_iter()
        .find(|field| field.as_wire() == spelling)
    }
}

/// The proposer's part in a Session. **Per field, and cleared by the person's own change**: a
/// field in `picked` is a value the proposer chose and nobody has touched since, so Bridge marks
/// it "Picked by the proposer"; changing it in a panel takes it out of the list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionProposal {
    /// The call is out. The Session exists and its process has not started, and Fleet starts it on
    /// the answer. Held in memory: a Fleet that restarts reads nothing as reading.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reading: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub picked: Vec<ProposedField>,
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

/// `claim_pull_request`: a Session takes an open pull request nobody holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimPullRequest {
    pub number: u64,
    /// Which session claims it. **Read only where the connection places no hosted session and no
    /// Drone**, as `ShowWindow::session_id` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// `take_main_red`: a Session takes a repository's red main, so no Job on another machine is
/// handed it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakeMainRed {
    /// Which session takes it. **Read only where the connection places no hosted session**, as
    /// `ClaimPullRequest::session_id` is. The red is the one in the session's own repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// Replace another machine's claim on the red with this session's. Asked for, never
    /// assumed: the call follows a refusal that named the holder.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub take_over: bool,
}

/// What `take_main_red` answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainRedTaken {
    /// The repository, as `get_merge_lines` names it.
    pub root: String,
    /// Whether a claim was written to the central store. `false` where the repository's
    /// *Pull request owners* switch is off or the machine is signed out: nothing was written
    /// and no other machine will be told.
    pub claimed: bool,
}

/// What `claim_pull_request` answers: the pull request now held, and by whom.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestClaimed {
    pub number: u64,
    pub branch: String,
    pub url: String,
    /// The Session's id.
    pub holder_id: String,
    /// Auto land is on for the Session and the forge would not turn on auto-merge: its words. The
    /// claim stands. Absent where Auto land is off or the forge took it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_merge_refused: Option<String>,
}
