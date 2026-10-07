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

use crate::helm_call::{HelmCallAnswer, HelmCallInFlight};
use crate::ids::{Instant, JobId, ManifestId};
use crate::piloting::{DroneNarrative, PilotOutcome};
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
    /// Take a Job over and start the session on its worktree. Since 23.51.
    ///
    /// **One call is the whole act**: Fleet marks the Job, ends its Drone,
    /// hands the Job's slot and branch to the new session and writes the
    /// handoff row first in its thread. The session's repository is the Job's,
    /// so `manifest_id` is not read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pilot: Option<PilotFrom>,
}

/// The Job a session starts by taking over. Since 23.51.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PilotFrom {
    pub job_id: JobId,
    pub outcome: PilotOutcome,
}

/// What a message tags with `@`, so the agent knows what is meant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    Session,
    Job,
    PullRequest,
    Branch,
}

/// Something a message names: another session, a Job, a pull request or a
/// branch. **Fleet writes the line that tells the agent**; the person's words
/// are kept as typed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTag {
    pub kind: TagKind,
    /// A session's id, a Job's id, a pull request's number or a branch's name.
    pub id: String,
    pub title: String,
    /// What a Job tag knows of the Job, so the ledger can hold it as one the
    /// session is looking at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<TaggedJob>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaggedJob {
    pub number: u32,
    pub branch: String,
    pub slot: u32,
    pub state: String,
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

/// `answer_session_ask`: one of the ask's own offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerSessionAsk {
    pub session_id: SessionId,
    /// [`HelmCallInFlight::call`] of the ask the row carries.
    pub call: String,
    pub answer: HelmCallAnswer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
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

/// `close_session`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseSession {
    pub session_id: SessionId,
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
    Refused,
    /// Nobody answered inside the hold, so it was refused.
    Unanswered,
    /// The process went before the answer did.
    SessionGone,
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
    },
    /// What a piloted session starts with: the Job's own worktree, handed over,
    /// and what Fleet knew when its Drone stopped, as the handoff bundle's
    /// structured fields. **Written once, first in the thread.** Since 23.51.
    Handoff {
        id: String,
        at: Instant,
        job_id: JobId,
        number: u32,
        title: String,
        /// `take_over` or `restart_step`.
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slot: Option<u32>,
        branch: String,
        /// The step the Job stopped on. Absent where none did.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step: Option<HandoffStep>,
        /// How many times that step was worked.
        attempts: u32,
        /// What the Judge refused on it, in its own words.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        refusals: Vec<String>,
        plan: HandoffPlan,
        /// Absent where no Drone said what it was stuck on.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        narrative: Option<DroneNarrative>,
    },
    /// A tool call, one line.
    Tool {
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
    /// A call put to the person. **The row is replaced by id** as the ask is
    /// answered, so a client holds one row per ask.
    Ask {
        id: String,
        at: Instant,
        ask: HelmCallInFlight,
        state: SessionAskState,
    },
}

/// The step a Job stopped on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffStep {
    pub id: String,
    pub label: String,
}

/// The step's declared plan against the worktree. **A mark and no judgement**:
/// both lists restate a comparison Fleet already made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffPlan {
    /// Whether the step declared one. False is no plan, and `outside` is then
    /// empty rather than everything.
    pub declared: bool,
    /// Files the worktree holds changed that the plan did not cover.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outside: Vec<String>,
    /// Paths the plan named that nothing changed under.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unwritten: Vec<String>,
}

impl SessionRow {
    pub fn id(&self) -> &str {
        match self {
            SessionRow::Message { id, .. }
            | SessionRow::Handoff { id, .. }
            | SessionRow::Tool { id, .. }
            | SessionRow::Lease { id, .. }
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
}

/// What the `armada` mod in a terminal session asks on a timer: has anyone
/// written to this session from Bridge. Since 23.53.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakeHeld {
    pub session_id: String,
}

/// What a person sent a terminal session, oldest first, handed over once. The
/// mod submits each as the person's own prompt.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagesHeld {
    pub messages: Vec<String>,
    /// Commands to run as if typed, in order with the messages' own turns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<HeldCommand>,
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
}
