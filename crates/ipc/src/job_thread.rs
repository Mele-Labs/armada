//! A Job's thread as a screen draws it, worked out by Fleet: `get_job_thread` and `job_thread.tail`.
//!
//! **Fleet folds, every client draws.** The rows a thread keeps (`SessionRow`) are what happened, one
//! at a time; a screen draws a run of tool calls as one line, a Fleet notice with the work it set off
//! under it, and a sketch, a file or a document as a card. That reading was Bridge's in TypeScript,
//! and it is here so the native Bridge and Pocket draw the same thread without each working it out.
//!
//! **States, not words.** An item carries what a row is and how it stands, never a sentence written
//! for a screen. A sentence Fleet itself wrote, such as a notice or a Fleet act's `said`, passes
//! through.
//!
//! **The fold is by order alone.** A tool row has no call id and no result row, so a run of calls is
//! the tool rows that came one after another, and any other row ends it.

use serde::{Deserialize, Serialize};

use crate::call::{AskedQuestion, CallInFlight};
use crate::hosted_sessions::{SessionRow, ToolSubagent};
use crate::ids::Instant;
use crate::sketch::{RawScene, SketchHow};
use crate::SessionId;

/// What `get_job_thread` answers when it is asked for no `limit`.
pub const THREAD_ITEMS_SERVED: u32 = 200;
/// The most items one read answers; a larger `limit` is served this many.
pub const THREAD_ITEMS_MOST: u32 = 1000;

/// `get_job_thread`: the newest items of a Job's thread, or the ones before an item a client holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobThreadRead {
    pub session_id: SessionId,
    /// The id of an item, or of any row inside one: the answer holds the items before it. Absent is
    /// the newest. **A row folded into another item since is found in that item**, so a client's
    /// cursor survives a fold that moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// How many items, at most. Absent is [`THREAD_ITEMS_SERVED`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// The answer to `get_job_thread`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobThread {
    pub session_id: SessionId,
    pub head: ThreadHead,
    /// Oldest first. **An item is never cut at the window's edge**: Fleet folds the whole thread and
    /// then takes whole items.
    pub items: Vec<ThreadItem>,
    /// Older items exist: pass this as `before` to read them. Absent where `items` starts the thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub older: Option<String>,
}

/// `job_thread.tail`: what changed in a thread a client has read.
///
/// **`items` replace everything a client holds from the item `from` to the end.** A row appended,
/// or one replaced by id (an ask answered, a Check run ended, a tool call's subagent settled),
/// re-folds the thread, and `from` is the first item that differs. A client that does not hold
/// `from`, because it is older than the window it read, keeps the items it holds older than the
/// first one in `items` and takes the rest. `from` absent is the head alone changing, with `items`
/// empty. A client that missed one reads `get_job_thread` again on the resync after a `Missed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobThreadTail {
    pub session_id: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default)]
    pub items: Vec<ThreadItem>,
    pub head: ThreadHead,
}

/// What stands over the whole thread rather than in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadHead {
    /// The call the Job waits on the person for now, hosted or in a terminal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asked: Option<ThreadAsk>,
    /// The tool call the running turn is in: set while a turn runs and the thread's last row is a
    /// tool row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_call: Option<OpenCall>,
    /// Questions nobody answered in time, that were skipped or that were kept for the person,
    /// oldest first. Absent for an ended Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lapsed: Option<Lapsed>,
    /// What Fleet did, does and will do for the Job, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fleet_acts: Vec<FleetAct>,
    pub trail: ThreadTrail,
}

/// The call the Job waits on, with the command a person reads: the argument whole where `detail`
/// was cut, else `detail`, else the tool's name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadAsk {
    pub command: String,
    pub call: CallInFlight,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenCall {
    pub text: String,
    pub since: Instant,
}

/// Questions that lapsed, under the id of the newest ask that held one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lapsed {
    pub id: String,
    pub questions: Vec<AskedQuestion>,
}

/// One thing Fleet did, does or will do for a Job, read from what is kept and never from the agent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetAct {
    pub id: String,
    pub at: Instant,
    pub state: FleetActState,
    pub kind: FleetActKind,
    /// Fleet's own words: the first line of its note, or what it is about to do.
    pub said: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetActState {
    Over,
    UnderWay,
    Coming,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetActKind {
    Nudge,
    Step,
    Slot,
    Restart,
    Told,
}

/// Where the Job has been: when it started, the slots it held, and when each pull request opened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadTrail {
    pub started: Instant,
    /// Oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bays: Vec<TrailBay>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opened: Vec<TrailOpened>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrailBay {
    pub slot: u32,
    pub from: Instant,
    /// Absent while the Job still holds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrailOpened {
    pub number: u64,
    pub at: Instant,
}

/// One thing a thread draws.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ThreadItem {
    /// A row drawn as it came. **Never a `tool` or a `check` row**, which fold, a waiting `ask`,
    /// which is the head's `asked`, or a `browser_reminder`, which the Job's browser draws.
    Row { row: SessionRow },
    /// Tool calls that ran one after another, without a subagent: `tool` rows only. A lone call
    /// is a run of one. `names` are the tools it holds, each once, in the order first called.
    Calls {
        id: String,
        names: Vec<String>,
        rows: Vec<SessionRow>,
    },
    /// A call that started a subagent, alone.
    Subagent {
        id: String,
        at: Instant,
        subagent: ToolSubagent,
    },
    /// Runs of one Check that sat side by side: `check` rows of one `name` only.
    Checks { id: String, rows: Vec<SessionRow> },
    /// A turn Fleet started: its notice, a `message` row from Fleet, then the agent's tool rows and
    /// messages up to the next row of any other kind. **Its last agent message is the next item**,
    /// as words meant for the person. `handled` is the agent having done anything after it.
    FleetTurn {
        id: String,
        notice: SessionRow,
        rows: Vec<SessionRow>,
        handled: bool,
    },
    /// A `show_sketch` call whose sketch the ledger holds.
    Sketch(ThreadSketch),
    /// A `show_file` call whose file the ledger holds, opened by the ledger's own path.
    File {
        id: String,
        at: Instant,
        path: String,
        name: String,
    },
    /// A document revision the Job kept, placed at when it was kept.
    Document(ThreadDocument),
    /// The person changed a value of the Job, read from the timeline.
    Change {
        id: String,
        at: Instant,
        what: ChangedValue,
        from: String,
        to: String,
    },
    /// The Job's agent never started.
    NotStarted {
        id: String,
        at: Instant,
        cause: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        log: Vec<String>,
    },
}

/// A sketch card. `drawing` is the scene the call drew, with the `kept` hash of each mock node the
/// ledger's kept scene draws alike.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadSketch {
    pub id: String,
    pub at: Instant,
    /// The ledger's id, `sketch:<title>`.
    pub sketch: String,
    pub title: String,
    pub drawing: RawScene,
    /// The title of the sketch it is read as a change to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub against: Option<String>,
    /// That sketch as it was drawn then: the latest earlier card of the title, else the scene the
    /// ledger kept at the call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub against_drawing: Option<RawScene>,
    /// A later card holds the same sketch. **Only the latest card carries how it was put and answered.**
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replaced: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub asked: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub withdrawn: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<CardSettled>,
}

/// A document card, one per revision kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadDocument {
    /// `document:<page>:r<revision>`.
    pub id: String,
    pub at: Instant,
    pub page: String,
    pub title: String,
    pub revision: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub asked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<CardSettled>,
}

/// How the person answered a sketch or a document put to him.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardSettled {
    pub how: SketchHow,
    pub at: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
}

/// A value of a Job the person can change from its panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangedValue {
    Workflow,
    Model,
    Effort,
}

impl ThreadItem {
    /// What a client holds the item by, and what `before` and `from` name: its first row's id.
    pub fn id(&self) -> &str {
        match self {
            ThreadItem::Row { row } => row.id(),
            ThreadItem::Calls { id, .. }
            | ThreadItem::Subagent { id, .. }
            | ThreadItem::Checks { id, .. }
            | ThreadItem::FleetTurn { id, .. }
            | ThreadItem::File { id, .. }
            | ThreadItem::Change { id, .. }
            | ThreadItem::NotStarted { id, .. } => id,
            ThreadItem::Sketch(sketch) => &sketch.id,
            ThreadItem::Document(document) => &document.id,
        }
    }
}
