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
//!
//! **Electron Bridge's fold is the spec, item for item**: `itemsOf` and the rows its host hands it. What
//! Bridge reads off the record, such as what the plan says, stays on the record.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::call::{AskedQuestion, CallInFlight};
use crate::hosted_sessions::{SessionRow, ToolSubagent};
use crate::ids::Instant;
use crate::judge::JudgeArtifact;
use crate::sketch::{draws, RawScene, SketchHow};
use crate::SessionId;

/// What `get_job_thread` answers when it is asked for no `limit`.
pub const THREAD_ITEMS_SERVED: u32 = 200;
/// The most items one read answers; a larger `limit` is served this many.
pub const THREAD_ITEMS_MOST: u32 = 1000;

/// The most items from the end of a thread a change may reach for `job_thread.tail` to carry it. **A change
/// further back is only marked** (`job_thread.changed`): an old ask answered, or a long-finished Check gaining
/// a row, must not push a whole long thread to every client.
pub const THREAD_TAIL_MOST: usize = 200;

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
    /// The id of a row: the thread is read as it stood at that row, the rows after it left out before Fleet
    /// folds, so a run of calls that went on past it ends at it. **The thread is cut, not followed**: such a
    /// read is not held, and `job_thread.tail` never carries to it. `before` and `limit` page that cut
    /// thread, and its `head` is the cut fold's. A row the thread does not hold is refused. Absent is the
    /// whole thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upto: Option<String>,
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
/// **`items` replace what a client holds from the item `from` to the end.** A row appended, or one
/// replaced by id (an ask answered, a Check run ended, a tool call's subagent settled), re-folds the
/// thread, and `from` is the first item the client held that differs. A client that does not hold
/// `from` holds only items after it, so it replaces everything it holds with `items`. `from` absent
/// is `items` after the last item, and with `items` empty it is the head alone changing. **A change
/// more than [`THREAD_TAIL_MOST`] items back is [`JobThreadChanged`] and not a tail**, except an
/// append, which is always a tail. A client that missed one reads `get_job_thread` again on the
/// resync after a `Missed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobThreadTail {
    pub session_id: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default)]
    pub items: Vec<ThreadItem>,
    pub head: ThreadHead,
}

/// `job_thread.changed`: a change reached back further than [`THREAD_TAIL_MOST`] items from the end of a
/// thread, so Fleet sends the mark and not the tail. **A client that holds an item at or after `from`
/// reads its window again with `get_job_thread`**, which is every client that holds the end of the
/// thread; one that holds none ignores it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobThreadChanged {
    pub session_id: SessionId,
    /// The first item that changed, one the client held before the change.
    pub from: String,
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
    /// The plan put to the person, or set aside, drawn after the last item. **An approved plan is an item**,
    /// [`ThreadItem::Plan`]. What it says is `SessionRecord::plan`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<ThreadPlan>,
    /// The newest sketch put to the person and not answered or taken back, drawn after the plan. **Its
    /// newest card is no item** while it stands here. The scene is the one the ledger kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sketch: Option<ThreadSketch>,
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
    /// A row drawn as it came. **Never a `tool`, `check`, `ask` or `window_ask` row**, which fold; a
    /// `plan` row, whose card is [`ThreadItem::Plan`]; a `delivery` row, a `linked` row that
    /// `replaces`, or a `browser_reminder`, which no screen draws in the thread, or a
    /// `reasoning` row, held back until the native thread draws it. **A Fleet notice is
    /// a row only where it is kept**: one the agent disputed, and the last Fleet turn while the Job
    /// waits on the person (its `asked`, the head's `plan` waiting or set aside, or its `sketch`).
    Row { row: SessionRow },
    /// Calls that ran one after another, without a subagent: `tool` rows, settled `ask` rows and
    /// `window_ask` rows. A lone call is a run of one. `names` are the tools it holds, each once, in
    /// the order first called: an `ask` names the tool it asked about, a `window_ask` none.
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
    /// The approved plan, after the item of the last row written before its `plan` row. **Only where
    /// that item is one row drawn as it came**: after a run of calls, a Fleet turn or a Check run there
    /// is none, as Electron Bridge draws none.
    Plan(ThreadPlan),
    /// A Judge finished reading, after the last row written by then: `id` is `judged:<reading id>`.
    /// `clean` is a reading with no notes.
    Judged {
        id: String,
        at: Instant,
        artifact: JudgeArtifact,
        clean: bool,
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
    /// Each drawing Fleet kept under the title, oldest first, on the latest card only. **Empty until
    /// every one reads**, so a menu over them never offers one that will not draw.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revisions: Vec<RawScene>,
}

/// A document card: one per page, its newest revision. The page's earlier revisions are its own.
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

/// Where a plan's card stands. `id` is `plan:v<version>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadPlan {
    pub id: String,
    pub version: u32,
    pub state: PlanCardState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanCardState {
    /// Fleet puts it to the person.
    Waiting,
    /// He put it away to keep talking; nothing waits on him for it.
    SetAside,
    Approved,
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
            | ThreadItem::NotStarted { id, .. }
            | ThreadItem::Judged { id, .. } => id,
            ThreadItem::Sketch(sketch) => &sketch.id,
            ThreadItem::Document(document) => &document.id,
            ThreadItem::Plan(plan) => &plan.id,
        }
    }
}

impl SessionRow {
    /// When the row was written. A retro row's is when the press was accepted.
    pub fn at(&self) -> &Instant {
        match self {
            SessionRow::Message { at, .. }
            | SessionRow::Tool { at, .. }
            | SessionRow::Command { at, .. }
            | SessionRow::Compaction { at, .. }
            | SessionRow::Reasoning { at, .. }
            | SessionRow::Lease { at, .. }
            | SessionRow::Release { at, .. }
            | SessionRow::Window { at, .. }
            | SessionRow::Started { at, .. }
            | SessionRow::Linked { at, .. }
            | SessionRow::Waiter { at, .. }
            | SessionRow::Check { at, .. }
            | SessionRow::Ran { at, .. }
            | SessionRow::Requested { at, .. }
            | SessionRow::Saved { at, .. }
            | SessionRow::Retro { at, .. }
            | SessionRow::WindowAsk { at, .. }
            | SessionRow::Draft { at, .. }
            | SessionRow::Sent { at, .. }
            | SessionRow::Plan { at, .. }
            | SessionRow::PlanChanges { at, .. }
            | SessionRow::Delivery { at, .. }
            | SessionRow::Disputed { at, .. }
            | SessionRow::BrowserReminder { at, .. }
            | SessionRow::Ask { at, .. } => at,
        }
    }
}

// What follows reads the JSON a tool row's text and a ledger row's `detail` carry. It is here and
// not in `fleet` because the JSON stays in this crate.

/// A `show_sketch` call as a thread's tool row carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SketchCall {
    pub title: String,
    pub scene: RawScene,
    pub against: Option<String>,
}

/// The tool a row's text starts with, its arguments after it, and the harness's `server__` prefix
/// left off the name.
fn call_of(text: &str) -> Option<(&str, &str)> {
    let (name, rest) = text.split_once(char::is_whitespace)?;
    let name = name.rsplit("__").next().unwrap_or(name);
    Some((name, rest))
}

/// A `show_sketch` call: the tool's name, then its arguments as JSON. **`None` for any row that is not
/// one, and for one whose scene would not draw**, which stays the plain tool row it was.
pub fn sketch_call_of(text: &str) -> Option<SketchCall> {
    let (name, rest) = call_of(text)?;
    let args = rest.trim();
    if name != "show_sketch" || !args.starts_with('{') || !args.ends_with('}') {
        return None;
    }
    let Value::Object(mut args) = crate::decode::<Value>("a show_sketch call", args.as_bytes()).ok()? else {
        return None;
    };
    let title = args.get("title").and_then(Value::as_str).map(str::trim).filter(|title| !title.is_empty())?;
    let title = title.to_string();
    let scene = args.remove("scene").filter(draws)?;
    let against = args
        .get("against")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|against| !against.is_empty())
        .map(str::to_string);
    Some(SketchCall { title, scene: RawScene(scene), against })
}

/// The path a `show_file` call names, as the agent wrote it.
pub fn file_call_of(text: &str) -> Option<String> {
    let (name, rest) = call_of(text)?;
    let path = rest.trim();
    (name == "show_file" && !path.is_empty() && !path.contains(['\n', '\r'])).then(|| path.to_string())
}

/// The scene a ledger row keeps as JSON text, `None` where it would not draw.
pub fn scene_of_detail(text: &str) -> Option<RawScene> {
    let scene = crate::decode::<Value>("a kept scene", text.as_bytes()).ok()?;
    draws(&scene).then_some(RawScene(scene))
}

/// A ledger sketch's `detail.against`: the title of the sketch it is read against and that sketch's
/// scene, which is JSON text again inside the JSON.
pub fn against_of_detail(text: &str) -> Option<(String, RawScene)> {
    let Value::Object(earlier) = crate::decode::<Value>("a kept earlier sketch", text.as_bytes()).ok()? else {
        return None;
    };
    let title = earlier.get("title")?.as_str()?.to_string();
    let scene = match earlier.get("scene")? {
        Value::String(text) => scene_of_detail(text)?,
        other => draws(other).then(|| RawScene(other.clone()))?,
    };
    Some((title, scene))
}

/// `scene` with the `kept` hash of each `mock` node that `held`, the scene Fleet kept, carries for the
/// same node. Fleet writes `kept` into the sketch it keeps and not into the call a card is read from.
/// **Only a node the call and the kept scene draw alike takes it**: a hash is for one set of HTML and
/// rules, and a node redrawn since has none to give.
pub fn kept_from(mut scene: RawScene, held: &RawScene) -> RawScene {
    let Some(Value::Array(had)) = held.0.get("nodes") else { return scene };
    let Some(Value::Array(nodes)) = scene.0.get_mut("nodes") else { return scene };
    for node in nodes.iter_mut().filter_map(Value::as_object_mut) {
        if node.get("kind").and_then(Value::as_str) != Some("mock") || node.contains_key("kept") {
            continue;
        }
        let was = had.iter().filter_map(Value::as_object).find(|was| {
            was.get("id") == node.get("id")
                && was.get("kind").and_then(Value::as_str) == Some("mock")
                && ["html", "css", "styles"].iter().all(|field| was.get(*field) == node.get(*field))
        });
        if let Some(kept) = was.and_then(|was| was.get("kept")) {
            node.insert(String::from("kept"), kept.clone());
        }
    }
    scene
}

/// How many revisions a document's `detail.revisions` lists: JSON text of a list of paths, oldest
/// first. **Anything that is not a list of non-empty strings is none.**
pub fn revisions_of_detail(text: &str) -> usize {
    revision_paths_of_detail(text).len()
}

/// The paths a document's or a sketch's `detail.revisions` lists, oldest first, read as
/// [`revisions_of_detail`] reads them.
pub fn revision_paths_of_detail(text: &str) -> Vec<String> {
    match crate::decode::<Value>("a kept revisions list", text.as_bytes()) {
        Ok(Value::Array(paths)) if paths.iter().all(|path| path.as_str().is_some_and(|path| !path.is_empty())) => {
            paths.iter().filter_map(Value::as_str).map(str::to_string).collect()
        }
        _ => Vec::new(),
    }
}
