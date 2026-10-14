//! What happened across the Fleet, one entry each, with the instant it did —
//! `get_fleet_timeline` and `timeline.appended` in `crates/ipc/operations/`.
//!
//! **The Cockpit's timeline reads this, and nothing else draws from it.** The
//! facts existed already, as a Job's own moves, a Session row replaced whole,
//! a merge line list replaced whole, and a command whose answer left no trace.
//! None had an instant a window could be cut by, and a pull request or a slot
//! lease had no event at all. Fleet writes an entry as each happens and serves
//! the kept ones.
//!
//! **An entry is the fact as it read then.** The title is the one the subject
//! had when it happened, so a Job forgotten since still has a history a window
//! can draw.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, JobId, ManifestId};
use crate::sessions::SessionId;

/// What happened. **Strict**: Bridge picks a mark and a row from it.
///
/// Each names the fact it is read from, so a reader can tell a kind Fleet
/// records from one it only could.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineKind {
    /// `job.created`.
    #[serde(rename = "job.created")]
    JobCreated,
    /// `job.state_changed` to `queued`.
    #[serde(rename = "job.queued")]
    JobQueued,
    /// `job.state_changed` to `running`.
    #[serde(rename = "job.started")]
    JobStarted,
    /// `job.step_advanced`.
    #[serde(rename = "job.step")]
    JobStep,
    /// `job.checking`, when a gate's Checks begin.
    #[serde(rename = "job.check")]
    JobCheck,
    /// `job.judging`, when a Judge begins to read.
    #[serde(rename = "job.judged")]
    JobJudged,
    /// `job.files_changed`.
    #[serde(rename = "job.files")]
    JobFiles,
    /// A Job's Drone read files. **One entry for a burst**, `says` the count: `Read 14 files`.
    #[serde(rename = "job.read")]
    JobRead,
    /// A Job's Drone ran shell commands. One entry for a burst, as `job.read` is.
    #[serde(rename = "job.ran")]
    JobRan,
    /// `job.asking`, when a Drone asks.
    #[serde(rename = "job.asked")]
    JobAsked,
    /// A person's answer to a Drone's question, a command or a Judge.
    #[serde(rename = "job.answered")]
    JobAnswered,
    /// `job.landed`.
    #[serde(rename = "job.landed")]
    JobLanded,
    /// `job.state_changed` to a failed or stopped end.
    #[serde(rename = "job.failed")]
    JobFailed,
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
    /// the latest path, and never content.
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
    /// A Session went from live to ended.
    #[serde(rename = "session.ended")]
    SessionEnded,
    /// A Job or a Session was given a pool slot.
    #[serde(rename = "slot.leased")]
    SlotLeased,
    /// A Job or a Session gave its slot back.
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
    Job { id: JobId },
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
    /// named it: main, or a Job Fleet had no handle for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub tone: TimelineTone,
    /// The fact, bare: `Started`, `Asked: split or wrap?`.
    pub says: String,
    /// What it opens to: a Check's name, a branch, a command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// What a burst counted: the files or commands behind `Read 14 files` or `Ran 5 commands`, newest
    /// first, at most [`MOST_TIMELINE_ITEMS`], each a line cut to the length of `detail`. **Its first is
    /// `detail`.** Absent for every other kind, and for an entry written before Fleet kept them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
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

/// The most files or commands an entry lists, the newest. 25 lines of 120 characters bound an entry at
/// about 3 KB, so a window of 2000 stays within a few MB if every one were a burst, and the list opens
/// to about a screen. The count in `says` is the whole burst's.
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
