//! What happened across the Fleet, one entry each, with the instant it did —
//! `get_fleet_timeline` and `timeline.appended` in `crates/ipc/operations/`.
//!
//! **The Cockpit's timeline reads this, and nothing else draws from it.** The
//! facts existed already, as a Session row replaced whole, a merge line list
//! replaced whole, and a command whose answer left no trace. None had an
//! instant a window could be cut by, and a pull request or a slot lease had no
//! event at all. Fleet writes an entry as each happens and serves the kept ones.
//!
//! **An entry is the fact as it read then.** The title is the one the subject
//! had when it happened, so a Session forgotten since still has a history a
//! window can draw.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};
use crate::sessions::SessionId;

/// What happened. **Strict**: Bridge picks a mark and a row from it.
///
/// Each names the fact it is read from, so a reader can tell a kind Fleet
/// records from one it only could.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineKind {
    /// A Session row seen for the first time.
    #[serde(rename = "session.started")]
    SessionStarted,
    /// A Session began waiting on the person for something Fleet raised: a question or permission card, a
    /// question in a walk window, a reply or a done to read. What the agent listed itself is `session.waiting`.
    #[serde(rename = "session.asked")]
    SessionAsked,
    /// A person's answer to a Session's ask or waiting item.
    #[serde(rename = "session.answered")]
    SessionAnswered,
    /// A Session's turn ended. The fact is how long it took.
    #[serde(rename = "session.turn")]
    SessionTurn,
    /// A Session wrote files, whichever tool did: `Edited 3 files`. **One entry for a burst**, `detail`
    /// the latest path, whole and from its worktree's root, and never content.
    #[serde(rename = "session.edited")]
    SessionEdited,
    /// A Session read files: `Read 14 files`. One entry for a burst, as `session.edited` is.
    #[serde(rename = "session.read")]
    SessionRead,
    /// A Session ran shell commands: `Ran 5 commands`. One entry for a burst, `detail` the latest
    /// command, cut and with what looks like a secret hidden.
    #[serde(rename = "session.ran")]
    SessionRan,
    /// A subagent of a Session started, or finished.
    #[serde(rename = "session.subagent")]
    SessionSubagent,
    /// An item the Session's agent put on its `waiting_for` list.
    #[serde(rename = "session.waiting")]
    SessionWaiting,
    /// An item left the Session's `waiting_for` list, answered or cleared.
    #[serde(rename = "session.resolved")]
    SessionResolved,
    /// A Session's agent started another Session, or forked its own.
    #[serde(rename = "session.spawned")]
    SessionSpawned,
    /// A Session's message was delivered to another Session's thread. The subject sent it and `to` received
    /// it; `says` is `Messaged: {text}`, cut. Written where Fleet delivers it, and never for a person's or Helm's.
    #[serde(rename = "session.messaged")]
    SessionMessaged,
    /// A Session showed the person a walk. `detail` is the page's title. Written when the page is shown,
    /// and apart from the `walk` item on its `waiting_for` list, which this timeline does not enter.
    #[serde(rename = "session.walked")]
    SessionWalked,
    /// A Session ran a `git commit` that made a commit. `detail` is the branch.
    #[serde(rename = "session.committed")]
    SessionCommitted,
    /// A Session ran a `git push` that updated a ref. `detail` is the branch.
    #[serde(rename = "session.pushed")]
    SessionPushed,
    /// A Session called a service outside the repository: its forge, an issue tracker, the web, another
    /// tool's server. **One entry for each call**, `says` `Called {name}`, `detail` and `call.line` the one
    /// line that says what was asked, and never what it was given. `call` names the service.
    #[serde(rename = "session.called")]
    SessionCalled,
    /// A person changed a Session's workflow in a panel. `says` is `workflow: old → new`.
    #[serde(rename = "session.workflow")]
    SessionWorkflow,
    /// A Session moved from one step of its workflow to another, as Fleet observed. `says` is
    /// `step: old → new`, naming a step by its label, none before the first as `start` and past the
    /// last as `done`.
    #[serde(rename = "session.step")]
    SessionStep,
    /// A Session's turn started on a different model than the turn before. `says` is `model: old → new`.
    #[serde(rename = "session.model")]
    SessionModel,
    /// A Session's turn started on a different effort than the turn before. `says` is `effort: old → new`.
    #[serde(rename = "session.effort")]
    SessionEffort,
    /// A Session went from live to ended.
    #[serde(rename = "session.ended")]
    SessionEnded,
    /// A Session was given a pool slot.
    #[serde(rename = "slot.leased")]
    SlotLeased,
    /// A Session gave its slot back.
    #[serde(rename = "slot.released")]
    SlotReleased,
    /// A pull request appeared in a merge line's hub.
    #[serde(rename = "pr.opened")]
    PrOpened,
    /// A pull request's CI came to a different state.
    #[serde(rename = "pr.checks")]
    PrChecks,
    /// A pull request's place in the merge queue came to a different state.
    #[serde(rename = "pr.queued")]
    PrQueued,
    /// A pull request left the open list and is among the merged.
    #[serde(rename = "pr.merged")]
    PrMerged,
    /// Main's CI came to red.
    #[serde(rename = "main.red")]
    MainRed,
    /// Main's CI came to green after red.
    #[serde(rename = "main.green")]
    MainGreen,
}

impl TimelineKind {
    /// The dotted name it travels under, which is also what Fleet keeps.
    /// **Read out of the serialisation**, as `Event::kind` is, so there is one
    /// spelling and not a `match` beside the `rename`s above.
    pub fn as_wire(self) -> String {
        spelt(&self)
    }

    /// Read a kept name back. `None` where it is not one this build has.
    pub fn from_wire(spelling: &str) -> Option<TimelineKind> {
        unspelt(spelling)
    }
}

/// What an entry is a state of, as a colour reads it. **Strict.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineTone {
    /// Working.
    Run,
    /// Needs the person.
    Ask,
    /// Failing.
    Bad,
    /// Passed.
    Ok,
    /// Waiting, or only a fact.
    Idle,
}

impl TimelineTone {
    /// The name it travels under, which is also what Fleet keeps.
    pub fn as_wire(self) -> String {
        spelt(&self)
    }

    /// Read a kept name back. `None` where it is not one this build has.
    pub fn from_wire(spelling: &str) -> Option<TimelineTone> {
        unspelt(spelling)
    }
}

fn spelt<T: Serialize>(one: &T) -> String {
    match serde_json::to_value(one) {
        Ok(serde_json::Value::String(spelling)) => spelling,
        // Unreachable: both are unit-variant enums, which serialise as a string.
        _ => String::new(),
    }
}

fn unspelt<T: serde::de::DeserializeOwned>(spelling: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(spelling.to_string())).ok()
}

/// What it happened to. **Strict**, tagged by `kind`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineSubject {
    Session { id: SessionId },
    PullRequest {
        number: u64,
        /// Absent where Fleet had no address for it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    /// The repository's main branch, which has no id of its own: which one is
    /// `manifest_id`.
    Main,
}

/// One thing that happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// The number Fleet kept it under. **Rises with the order Fleet wrote
    /// entries, not with `at`**, and is how a client that read a window and
    /// then hears `timeline.appended` knows it already holds one.
    pub id: u64,
    /// When it happened. A Session's start is written when Fleet first sees
    /// the row, with the instant the row says.
    pub at: Instant,
    pub kind: TimelineKind,
    pub subject: TimelineSubject,
    /// What the subject was called when this happened. Absent where nothing
    /// named it: main.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub tone: TimelineTone,
    /// The fact, bare: `Started`, `Asked: split or wrap?`.
    pub says: String,
    /// What it opens to: a Check's name, a branch, a command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// What a burst counted: the files or commands behind `Read 14 files` or `Ran 5 commands`, newest
    /// first, at most [`MOST_TIMELINE_ITEMS`]: a file whole and from its worktree's root, a command one
    /// line cut to the length of `detail`. **Its first is
    /// `detail`.** Absent for every other kind, and for an entry written before Fleet kept them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
    /// The call a `session.called` entry records. Absent for every other kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<ServiceCall>,
    /// The Session a `session.messaged` entry's message was delivered to, so either end's Field finds it.
    /// Absent for every other kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<SessionId>,
    /// The repository it belongs to. Absent where Fleet could not place it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    /// The subject is still working after this.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub live: bool,
    /// The subject is over after this: landed, failed, merged, ended.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub over: bool,
}

/// One call to a service outside the repository. **Open**: Bridge draws a service it has no word for as
/// a plug beside the rest, so a new server needs no change here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceCall {
    /// Which service: `forge` for the forge, `tracker` for an issue tracker, `web` for a fetch or a
    /// search, and any other tool's server by its own lowercase name.
    pub service: String,
    /// What the service is called, as a row says it: the forge's own name, `the issue tracker`, `the web`, or the
    /// server's own name capitalised. Bridge labels the service's outpost with the newest call's.
    pub name: String,
    /// What was asked of it, in a few words: the `gh` subcommand, the host of a page, a tool's name.
    /// **Never an argument, a query, a path or a body.**
    pub line: String,
}

/// `timeline.appended`: the entry Fleet just kept.
///
/// **Wrapped, not the entry itself**: the stream tags its events with `kind`, and an entry
/// carries a `kind` of its own, so the entry rides under `event`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineAppended {
    pub event: TimelineEvent,
}

/// What `get_fleet_timeline` is asked, as its query string.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineAsk {
    /// Entries at or after this. Absent is from the beginning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<Instant>,
    /// Entries before this. Absent is up to now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Instant>,
    /// At most this many, the newest. Absent is [`DEFAULT_TIMELINE_LIMIT`]; Fleet cuts a larger one to
    /// [`MOST_TIMELINE_LIMIT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// How many entries an unlimited ask gets.
pub const DEFAULT_TIMELINE_LIMIT: u32 = 500;

/// The most one ask is given.
pub const MOST_TIMELINE_LIMIT: u32 = 2000;

/// The most files or commands an entry lists, the newest. 25 lines bound an entry at a few KB (a file
/// is kept whole, and a repository's paths are rarely past 200 characters), so a window of 2000 stays
/// within a few MB if every one were a burst, and the list opens to about a screen. The count in `says`
/// is the whole burst's.
pub const MOST_TIMELINE_ITEMS: usize = 25;

/// `get_fleet_timeline`: the window, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetTimeline {
    pub events: Vec<TimelineEvent>,
    /// Older entries inside the window were left out to keep to the limit.
    /// **Stated rather than inferred from a full list**, so a window that
    /// holds exactly the limit is not read as cut.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
}
