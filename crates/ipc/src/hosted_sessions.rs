//! A session Bridge hosts: the commands that drive one, the thread it keeps and
//! the gate its first write goes through. Since 23.49.
//! `docs/concepts/session.md`, *A session Fleet hosts*.
//!
//! **The ledger's own row, and one thing more.** A hosted session is a
//! [`SessionRecord`](crate::SessionRecord) with `origin: bridge` and a
//! [`HostedFacts`] on it, so `list_sessions` and `session.changed` carry it
//! without a second row. Its thread is not on the row: rows are many and arrive
//! one at a time, so they are `get_session`'s answer and the `session.row`
//! event.

use serde::{Deserialize, Serialize};

use crate::helm_call::{HelmCallAnswer, HelmCallInFlight, QuestionAnswer};
use crate::ids::{Instant, ManifestId};
use crate::mods::ModAim;
use crate::sessions::{SessionId, SessionRecord};

/// The permission mode a hosted session runs in, as a terminal's are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    /// Every call the person's own settings do not cover is put to them.
    Ask,
    /// **The default.** Everything runs except what is destructive, pushes code
    /// to a shared space or writes off this machine, which is put to the person.
    #[default]
    Auto,
    AcceptEdits,
    Plan,
}

/// Start a session in one repository. It holds no slot and no branch until the
/// agent's first write.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartSession {
    pub manifest_id: ManifestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Absent is the machine's own default, as Dispatch's `Auto`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// Absent is [`SessionMode::Auto`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<SessionMode>,
    /// Start as a copy of a session's conversation. Since 23.69.
    ///
    /// The new session has its own id, ledger and slot, and holds none of the
    /// old one's. Refused where that session is unknown, or live and `at` is
    /// absent. `manifest_id` is read as for any start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork: Option<ForkFrom>,
    /// Start in a served repository that has no `armada.yml` yet, so the
    /// session can write one. The session runs in `root`, carries no
    /// Manifest, and `manifest_id` is not read. A `root` that has a Manifest
    /// by now starts as a plain start on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// The words the person dispatched with. Since 23.91.
    ///
    /// **Present, Fleet reads them once, with the proposer's model**, and fills in what the
    /// person left unset: the title, the workflow, the model and the effort. The call is not
    /// waited on here: the Session exists at once, carries `proposal.reading` while the call is
    /// out, and starts its process on the answer. A first message sent meanwhile is held until
    /// then. Whatever this start sets itself stands over the answer. Not read with `fork` or `root`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,
    /// A Session this one waits on, and when it starts. **Present, `request` is the Session's brief
    /// and nothing is sent**: it holds the brief, no slot and no process until the condition is met,
    /// and then starts on the other Session's handoff followed by the brief. The Session waited on
    /// is told. Not read with `fork` or `root`. `docs/concepts/session.md`, *A Session that
    /// waits on another*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<crate::session_waits::StartAfter>,
    /// Start on the start card: no agent runs and a message sent is kept, until `begin_session`.
    /// Not read with `fork`, `root` or `after`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub held: bool,
    /// Rows of the repository's projects this Job starts with. **Each written task is held by the
    /// Job** (`projects.changed`) and shows it; a linked issue row keeps no state and is only named,
    /// so the brief can carry it. Refused 404 for a project or row that is not there and 422 for one
    /// of another repository, before anything is made. Not read with `fork` or `root`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project_rows: Vec<crate::ProjectRowRef>,
}

/// The session a new one is forked from. Since 23.69.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkFrom {
    pub session_id: SessionId,
    /// The row of the old session's thread the copy ends at, **included**. Since 23.79.
    ///
    /// Present, the old session may be live and goes on untouched; absent, the
    /// whole conversation is copied and only an ended or dead one is. Refused
    /// 422 where no message of that thread has that id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// What a message tags with `@`, so the agent knows what is meant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    Session,
    PullRequest,
    Branch,
    /// A tracker issue. `id` is its number as a string.
    Issue,
}

/// Something a message names: another session, a pull request, an issue or a
/// branch. **Fleet writes the line that tells the agent**; the person's words
/// are kept as typed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTag {
    pub kind: TagKind,
    /// A session's id, a pull request's or an issue's number, or a branch's name.
    pub id: String,
    pub title: String,
    /// The forge's address for an issue or a pull request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A picture or a file sent with a message. `data` is base64.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionUpload {
    pub name: String,
    /// `image/png`, `text/plain`, and so on. A `image/*` type reaches the
    /// session as a picture; anything else as a path it reads.
    pub media_type: String,
    pub data: String,
}

/// `send_session_message`. It takes a turn.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendSessionMessage {
    pub session_id: SessionId,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<SessionUpload>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mentions: Vec<SessionTag>,
}

/// `send_session_message`'s body: the message, and what he picked in an empty Mod Job. The aim
/// rides beside the message, not in it, so a message built anywhere else needs no word of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendSessionMessageBody {
    #[serde(flatten)]
    pub message: SendSessionMessage,
    /// The kind of mod he is aiming for, `source` for a change to Armada's code. Read with his first message only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aim: Option<ModAim>,
}

/// `answer_session_ask`: one of the ask's own offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerSessionAsk {
    pub session_id: SessionId,
    /// [`HelmCallInFlight::call`] of the ask the row carries.
    pub call: String,
    pub answer: HelmCallAnswer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// One entry per question, for an ask that carries some. Since 23.71.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<QuestionAnswer>,
    /// [`crate::AnswerHelmCall::folder`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

/// `tune_session`: what the next turn runs on. An absent model or effort is the
/// machine's own, as Dispatch's `Auto`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuneSession {
    pub session_id: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    pub mode: SessionMode,
}

/// `set_session_workflow`: a person picked the Session's workflow in a panel, or cleared it. Since 23.91.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSessionWorkflow {
    pub session_id: SessionId,
    /// A workflow id the Session's repository holds. Absent clears it: a custom Session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<String>,
}

/// `begin_session`: Start pressed on the start card. Answers the `SessionRecord`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeginSession {
    pub session_id: SessionId,
    /// How the work lands. Required where `hosted.start.asks_delivery` holds; read nowhere else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<crate::session_delivery::DeliveryChoice>,
    /// The first message as he left it on the card. Present, it replaces the held first turn's
    /// words, what the agent reads and what the thread keeps, and its files stay with it. Absent,
    /// the first turn goes as it was held.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// `run_session_command`: a slash command, written whole (`/update-armada`, `/model opus`), that a
/// terminal session's mod runs as typed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSessionCommand {
    pub session_id: SessionId,
    pub command: String,
}

/// `close_session`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseSession {
    pub session_id: SessionId,
}

/// `move_session`: end a Session and continue its conversation in another served repository.
/// `docs/concepts/session.md`, *A moved session*.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveSession {
    pub session_id: SessionId,
    /// The repository the conversation continues in. Refused 404 where none is served by it, and
    /// 409 where it is the Session's own.
    pub manifest_id: ManifestId,
}

/// `stop_session`: end the turn that is running. Since 23.84.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopSession {
    pub session_id: SessionId,
}

/// `retry_handoff`: run again the hand off that failed on this Job. The person's press is the consent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryHandoff {
    pub session_id: SessionId,
}

/// `pause_session`: end the turn, park the slot's work on the branch and give the slot back.
/// Since 23.92.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseSession {
    pub session_id: SessionId,
}

/// `set_session_auto_land`: turn Auto land on or off for a Session. On, Fleet enables auto-merge on
/// each open pull request the Session holds and on each that attaches later; off, it disables it
/// on the open ones. The flag changes even where the forge refuses a pull request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSessionAutoLand {
    pub id: SessionId,
    pub on: bool,
}

/// `resume_session`: take a slot again at the branch tip. Since 23.92.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeSession {
    pub session_id: SessionId,
}

/// `release_session_holds`: the person gives back everything a Session holds on the ledger, or
/// hands it to another live Session of the same repository. `docs/concepts/session.md`,
/// *Releasing what a Session holds*.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseSessionHolds {
    pub session_id: SessionId,
    /// Absent gives every row back. Present, each row is written standing for this Session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_session_id: Option<SessionId>,
    /// The rows to act on, each the Session holds standing. Absent is every one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<ChosenHold>>,
}

/// One ledger row the person ticked, as the ledger keys it within the Session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChosenHold {
    /// `slot`, `branch` or `pr`.
    pub kind: String,
    pub target: String,
}

/// What `release_session_holds` moved, and what stayed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionHoldsReleased {
    /// The Session that was handed them. Absent where they were given back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_session_id: Option<SessionId>,
    /// Each row given back, or handed over.
    pub moved: Vec<HoldMoved>,
    /// Each row that stays the Session's, with why.
    pub kept: Vec<HoldKept>,
}

/// One ledger row `release_session_holds` moved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldMoved {
    /// `slot`, `branch` or `pr`.
    pub kind: String,
    pub target: String,
    /// For a slot given back: the branch its work is on, and what was committed to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released: Option<crate::SlotReleased>,
}

/// One ledger row `release_session_holds` left where it was.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldKept {
    pub kind: String,
    pub target: String,
    /// Why, in a sentence.
    pub why: String,
}

/// A turn running, or none. **A message from another session starts one**, so
/// `working` can have no author on the person's side.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SessionTurn {
    Idle,
    Working {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        woken_by: Option<SessionVoiceNamed>,
    },
}

/// A session named as a person reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionVoiceNamed {
    pub id: String,
    pub title: String,
}

/// What a hosted session carries beyond a terminal's row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedFacts {
    pub turn: SessionTurn,
    /// What the agent is held on, while it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asked: Option<HelmCallInFlight>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The model the CLI resolved for the running process, as its `init` line names it:
    /// `vendor-opus-5-5` where `model` is `opus` or absent. **Absent until the process has
    /// started, and again once it is replaced**, which is not the same as `model` being
    /// absent: a session with no pick still runs on something. Since 23.85.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    pub mode: SessionMode,
    /// Whether a process of Fleet's is running this session now. **False after
    /// a quiet timeout**: the next message resumes it.
    pub running: bool,
    /// The commands the agent said it has, for `/` to offer: its slash commands
    /// and its skills, by name. **Read off the stream's `init` line**, so a
    /// session whose process has not started is given the last one any session
    /// read, and a Fleet that has not run an agent yet gives none. Since 23.51.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
    /// Set while the person has paused the session: its slot is given back and its work is on
    /// its branch. `docs/concepts/session.md`, *Pausing a Session*. Since 23.92.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paused: Option<SessionPaused>,
    /// Present while the Job waits for `begin_session`. Absent once it has begun, and for a Job
    /// that was never held.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<crate::session_delivery::StartHeld>,
}

/// The marker on a paused hosted session. Not a state: the session stays live and `hosted.turn`
/// reads as it does for any idle one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPaused {
    pub at: Instant,
    /// A resume was asked and waits for a slot. The first one to free is the session's.
    pub resuming: bool,
}

/// Who said a row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionVoice {
    You,
    Agent,
    /// **Another session that wrote to this one is named**, never folded into
    /// the agent.
    Session {
        id: String,
        title: String,
    },
}

/// A file or picture a message carried, kept by Fleet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentFile {
    /// What `get_session_file` takes.
    pub id: String,
    pub name: String,
    pub media_type: String,
}

/// How an ask on the thread stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAskState {
    Waiting,
    /// Ran unasked: Fleet read the call as none of the three classes.
    RanUnasked,
    AllowedOnce,
    AllowedAndRemembered,
    /// Allowed, and the folder put on the list every session's guard reads.
    AlwaysAllowed,
    /// Refused, and the folder put on the list as blocked.
    AlwaysBlocked,
    /// Allowed, and the folder allowed for this session until it ends.
    AllowedForSession,
    Refused,
    /// Nobody answered inside the hold, so it was refused.
    Unanswered,
    /// Refused to the agent because its session's walk was open, and kept whole for the person to
    /// answer after it. His notes and his Approve do not clear it.
    Kept,
    /// The process went before the answer did.
    SessionGone,
}

/// What a Session that is followed did, for the parent's thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkedEvent {
    /// It waited on the person. **Fleet no longer writes it**; threads kept before that still hold the
    /// row, and a thread with one unread would not open.
    Needs,
    /// It ended.
    Done,
    /// A pull request it held merged.
    Merged,
}

/// One row of a hosted session's thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionRow {
    Message {
        id: String,
        at: Instant,
        from: SessionVoice,
        text: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        files: Vec<SentFile>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<SessionTag>,
        /// The Fleet notice this row was sent for, when it was one (`Notice::word`). A person can
        /// mute a notice by it. Absent on every row kept before it existed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notice: Option<String>,
    },
    /// A tool call, one line.
    Tool {
        id: String,
        at: Instant,
        text: String,
    },
    /// A command the person ran in the terminal, as typed: `/reload-plugins`,
    /// with its arguments. Its output is not drawn. Since 23.56.
    Command {
        id: String,
        at: Instant,
        text: String,
    },
    /// The summary the agent's CLI wrote in place of a conversation it
    /// compacted. **Not the person's words**, and not the agent's either.
    /// Since 23.56.
    Compaction {
        id: String,
        at: Instant,
        text: String,
    },
    /// The first write: the slot leased and the branch cut.
    Lease {
        id: String,
        at: Instant,
        slot: u32,
        branch: String,
    },
    /// The slot given back, with the branch it was on.
    Release {
        id: String,
        at: Instant,
        slot: u32,
        branch: String,
    },
    /// The session showed the person a page in a window.
    Window {
        id: String,
        at: Instant,
        title: String,
        url: String,
        /// The takes it shows, when it shows several; `url` is the first's. **Kept when the walk
        /// ends**, so the row reopens the window with them.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        variations: Vec<crate::sessions::WindowTake>,
        /// The scripted walk it carries, when it carries one. **Kept when the walk ends**, as the
        /// takes are.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        steps: Vec<crate::WalkStep>,
        /// The other web origins the window may pass through, such as a sign-in provider. **Kept
        /// when the walk ends**, as `variations` are.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        origins: Vec<String>,
        /// When the walk ended: the person approved the page or closed its window. Absent while
        /// it is open. **The row is replaced by id** when it is set.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ended_at: Option<Instant>,
    },
    /// This session's agent started a session, or forked its own conversation into one, and the
    /// new one waits for the person. Written in **this** session's thread; `session` is the new
    /// one, and the row is the press that opens it. Since 23.80.
    Started {
        id: String,
        at: Instant,
        session: crate::session_starts::SessionLink,
        fork: bool,
    },
    /// A Session this one follows needs the person, finished or had its pull request merged.
    /// Written in **this** session's thread where it happened, and the agent is told the same
    /// moment; `session` is the linked one, and the row is the press that opens it.
    Linked {
        id: String,
        at: Instant,
        session: crate::session_starts::SessionLink,
        event: LinkedEvent,
    },
    /// A Session waits on this one, its condition changed, or it started. Written in **this**
    /// session's thread (`side` `waited`, `session` the one that waits), and in the waiting
    /// session's thread where the condition changed (`side` `waiting`, `session` the one it waits
    /// on). `says` is what the waiting Session needs, or where it started from; `from` and `to` are
    /// the condition as it was and as it is; `by` is who changed it. The row is the press that opens
    /// `session`.
    Waiter {
        id: String,
        at: Instant,
        session: crate::session_starts::SessionLink,
        event: crate::session_waits::WaiterEvent,
        #[serde(default, skip_serializing_if = "crate::session_waits::WaiterSide::is_waited")]
        side: crate::session_waits::WaiterSide,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        says: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        by: Option<crate::session_waits::WaitBy>,
    },
    /// A question the session asked in its walk window. **Replaced by id once answered**, with
    /// `answer` set to what the person chose and said.
    WindowAsk {
        id: String,
        at: Instant,
        question: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        options: Vec<crate::sessions::WaitingOption>,
        place: crate::sessions::WindowPlace,
        /// The address of the window the session showed that the question is asked in.
        window: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        answer: Option<String>,
    },
    /// An `armada check` the agent ran in its slot. **The row is replaced by id** as the run ends,
    /// so a client holds one row per run. `run` is the id the Checks page opens it by. Since 23.75.
    Check {
        id: String,
        at: Instant,
        name: String,
        run: u64,
        state: crate::SessionCheckState,
        /// What the run was narrowed to. Absent where it ran whole.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<String>,
        /// Where a failed run's log says it failed. Absent while running, on a pass, and where
        /// the log names no file in the slot.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blame: Option<crate::CheckBlame>,
    },
    /// A command the person ran from the thread or the Needs-you band, ended, and what the
    /// session was told of it. `code` is absent where the run was stopped or a signal ended it;
    /// `tail` is the last lines it printed as plain text, the same Fleet sent the agent. Written
    /// instead of a message from Fleet. Since 23.89.
    Ran {
        id: String,
        at: Instant,
        command: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<i32>,
        tail: String,
    },
    /// An HTTP request sent from Requests by the owner with this Job open, or by the Job's agent,
    /// and how it was answered. **The request half is whole**, so Bridge reopens the row to edit
    /// and send again: `url` and `headers` as sent, filled. **No secret's value is on the row**:
    /// a secret reads `{{name}}` in every field. Since Requests.
    Requested {
        id: String,
        at: Instant,
        by: crate::RequestBy,
        method: String,
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        environment: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        headers: Vec<crate::RequestHeader>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        /// Absent where nothing answered, and `error` says why.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<u16>,
        ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        /// The answer's body, its start where it was cut.
        #[serde(default)]
        response: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        truncated: bool,
    },
    /// A request the Job's agent saved into a collection with `save_request_to_collection`.
    /// `collection_id` and `request` are the ids Requests opens it by; `name` and `collection`
    /// are what the row says. Since Requests.
    Saved {
        id: String,
        at: Instant,
        name: String,
        collection: String,
        collection_id: String,
        request: String,
    },
    /// A Session's retro being written. **The row is replaced by id** as the writing goes
    /// and stays when it is done. `items` settle while the model writes and are empty once
    /// `written`, when `retro` names the kept one for `get_session_retro`. Since 23.78.
    Retro {
        id: String,
        /// When the press was accepted.
        at: Instant,
        stage: crate::RetroStage,
        /// When it reached this stage.
        stage_since: Instant,
        /// How much of the model's answer has arrived.
        characters: u64,
        items: Vec<crate::PartialRetroItem>,
        /// The kept retro's number, on `written`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retro: Option<i64>,
        /// Fleet's words for why it was not kept, on `failed`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        why: Option<String>,
    },
    /// Words that went to another Session: a person's thought Helm's dock routed, or
    /// what Helm passed on with `tell_session`. **Written in a Helm Session's thread only, and never
    /// delivered to it**: it records where the words went and the agent is not told. `files` and
    /// `tags` are what went with them, read back with `get_session_file`. Since the Helm Session.
    Sent {
        id: String,
        at: Instant,
        text: String,
        target: crate::thought::ThoughtTarget,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        files: Vec<SentFile>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<SessionTag>,
    },
    /// A call put to the person. **The row is replaced by id** as the ask is
    /// answered, so a client holds one row per ask.
    Ask {
        id: String,
        at: Instant,
        ask: HelmCallInFlight,
        state: SessionAskState,
        /// The door the answer came through. Absent while the ask waits, and
        /// where nobody answered it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        via: Option<crate::Via>,
    },
    /// The agent recorded a plan, or recorded it again. The plan itself is `SessionRecord::plan`;
    /// this row is where in the thread it was recorded, which its card draws after.
    Plan {
        id: String,
        at: Instant,
        version: u32,
    },
    /// What the person sent the agent against a version of the plan, as Fleet worded it. The agent
    /// is sent the same lines, and the plan does not change until it records it again.
    PlanChanges {
        id: String,
        at: Instant,
        version: u32,
        /// The plan was approved with them.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        approved: bool,
        lines: Vec<crate::session_plan::PlanChangeLine>,
    },
    /// How the Job's work is to be delivered. **Replaced by id** once the owner answers, or written
    /// answered with `named` set where the first message already said. `choice` is absent while the
    /// question stands.
    Delivery {
        id: String,
        at: Instant,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        choice: Option<crate::DeliveryChoice>,
        /// The first message named it, so nothing was asked.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        named: bool,
    },
    /// The agent said a Fleet notice was wrong. `notice` is the Fleet message row's id; `review` the job started, or joined, to settle it, absent where none was.
    Disputed {
        id: String,
        at: Instant,
        notice: String,
        why: String,
        spotted: crate::DisputeSpotted,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        review: Option<String>,
    },
}

impl SessionRow {
    pub fn id(&self) -> &str {
        match self {
            SessionRow::Message { id, .. }
            | SessionRow::Tool { id, .. }
            | SessionRow::Command { id, .. }
            | SessionRow::Compaction { id, .. }
            | SessionRow::Lease { id, .. }
            | SessionRow::Release { id, .. }
            | SessionRow::Window { id, .. }
            | SessionRow::Started { id, .. }
            | SessionRow::Linked { id, .. }
            | SessionRow::Waiter { id, .. }
            | SessionRow::Check { id, .. }
            | SessionRow::Ran { id, .. }
            | SessionRow::Requested { id, .. }
            | SessionRow::Saved { id, .. }
            | SessionRow::Retro { id, .. }
            | SessionRow::WindowAsk { id, .. }
            | SessionRow::Sent { id, .. }
            | SessionRow::Plan { id, .. }
            | SessionRow::PlanChanges { id, .. }
            | SessionRow::Delivery { id, .. }
            | SessionRow::Disputed { id, .. }
            | SessionRow::Ask { id, .. } => id,
        }
    }
}

/// `get_session`: the row and its thread, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionThread {
    pub session: SessionRecord,
    pub rows: Vec<SessionRow>,
}

/// `get_session_subagent`: one subagent's own thread, as it stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSubagent {
    pub rows: Vec<SessionRow>,
    /// Its last turn ended and asked for nothing more.
    pub finished: bool,
    /// What it said last, once it has finished.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<String>,
    /// The model it runs on, as its transcript writes the id. Absent until it has answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// What its last answer had in its window, and the window that is measured against.
    /// Absent until it has answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
}

/// `session.row`: one row appended or replaced, by `row.id()`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRowChanged {
    pub session_id: SessionId,
    pub row: SessionRow,
}

/// What a session's `PreToolUse` hook sends Fleet before a tool runs. The
/// harness's own fields, named as it names them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionGate {
    pub session_id: String,
    #[serde(default)]
    pub cwd: String,
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: serde_json::Value,
    /// Present only when a subagent made the call, and absent for the session's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

/// The Bash call's field that runs it in the background; the harness wakes the session when it ends.
const IN_BACKGROUND: &str = "run_in_background";

impl SessionGate {
    /// Whether the call already runs in the background.
    pub fn in_background(&self) -> bool {
        self.tool_input.get(IN_BACKGROUND).and_then(serde_json::Value::as_bool) == Some(true)
    }

    /// The call's input as it was asked, set to run in the background.
    pub fn backgrounded(&self) -> serde_json::Value {
        let mut input = self.tool_input.clone();
        if let Some(fields) = input.as_object_mut() {
            fields.insert(IN_BACKGROUND.into(), serde_json::Value::Bool(true));
        }
        input
    }
}

/// What the `armada` mod in a terminal session asks: has anyone written to this
/// session from Bridge. Since 23.53.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakeHeld {
    pub session_id: String,
    /// How long Fleet may hold the ask while nothing is held, in milliseconds,
    /// answering the moment something is. **Left out, it is answered at once.**
    /// Fleet caps it at 25 seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_ms: Option<u64>,
}

/// What the `armada` mod in a terminal session tells Fleet about one
/// `AskUserQuestion`, or one permission prompt: that it is open, then that the
/// terminal's own prompt ended first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminalAsk {
    /// The question, as the tool was called. Answered at once with its `call`;
    /// the mod then asks `Wait` until a person answers in Bridge or the
    /// terminal's own prompt ends.
    Asks {
        session_id: String,
        input: serde_json::Value,
        /// The tool whose call this is, where it is not `AskUserQuestion`: the terminal's own
        /// permission prompt for it is open, and Bridge may allow or refuse the call first. Since
        /// mod 0.3.22.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool: Option<String>,
    },
    /// The terminal's prompt ended: `answered` is false where it was dismissed.
    Settled { session_id: String, answered: bool },
    /// Is the question `call` answered yet. **Held for a bounded time**, so a
    /// request that ends, or a Fleet that restarts, costs the next poll and not
    /// the question.
    Wait { session_id: String, call: String },
}

/// What Fleet answers a `TerminalAsk` with. `Answered` carries the tool's input
/// with the person's `answers` filled in, which the mod hands back as the
/// call's result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum TerminalAsked {
    /// The question is on Bridge's thread as `call`; ask `Wait` for its answer.
    Asked { call: String },
    /// Nobody has answered within this poll's hold: ask again.
    Waiting {},
    Answered { updated_input: serde_json::Value },
    Refused { message: String },
    /// The terminal answered first, or the card lapsed: nothing more to wait on.
    Gone {},
}

/// What a person sent a terminal session, oldest first, handed over once. The
/// mod submits each as the person's own prompt.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagesHeld {
    pub messages: Vec<String>,
    /// Commands to run as if typed, in order with the messages' own turns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<HeldCommand>,
    /// End the session: a person pressed End session in Bridge. The mod runs
    /// it after this answer's messages, as `/exit`. Left out when not asked.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub end: bool,
}

/// A slash command a person chose in Bridge for a terminal session: `model` or
/// `effort`, and its argument.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldCommand {
    pub command: String,
    pub args: String,
}

/// What the hook is answered with. **Spelled as the harness reads it**, as
/// [`RunOrNot`](crate::RunOrNot) is: an empty object lets the call go to the
/// rest of the permission path, and a deny stops it with a reason the model
/// reads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum GateAnswer {
    Pass {},
    Hold {
        #[serde(rename = "hookSpecificOutput")]
        held: GateHold,
    },
    /// The call runs, with this input in place of what was asked. #2415.
    Rewrite {
        #[serde(rename = "hookSpecificOutput")]
        rewritten: GateRewrite,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GateRewrite {
    #[serde(rename = "hookEventName")]
    pub event: &'static str,
    /// `allow`, or `ask` where the person is asked about every call.
    #[serde(rename = "permissionDecision")]
    pub decision: &'static str,
    #[serde(rename = "permissionDecisionReason")]
    pub reason: String,
    #[serde(rename = "updatedInput")]
    pub input: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GateHold {
    #[serde(rename = "hookEventName")]
    pub event: &'static str,
    #[serde(rename = "permissionDecision")]
    pub decision: &'static str,
    #[serde(rename = "permissionDecisionReason")]
    pub reason: String,
}

impl GateAnswer {
    pub fn pass() -> GateAnswer {
        GateAnswer::Pass {}
    }

    pub fn deny(reason: impl Into<String>) -> GateAnswer {
        GateAnswer::Hold {
            held: GateHold {
                event: "PreToolUse",
                decision: "deny",
                reason: reason.into(),
            },
        }
    }

    /// Run the call with `input` instead. `asks` keeps the person's say where
    /// they are asked about every call; otherwise it is allowed.
    pub fn rewrite(input: serde_json::Value, asks: bool, reason: impl Into<String>) -> GateAnswer {
        GateAnswer::Rewrite {
            rewritten: GateRewrite {
                event: "PreToolUse",
                decision: if asks { "ask" } else { "allow" },
                reason: reason.into(),
                input,
            },
        }
    }
}
