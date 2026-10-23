//! The Logbook on the wire: what Armada learned, read, and the acts a person has on an entry.
//! `get_logbook`, `edit_logbook_entry`, `pin_logbook_entry`, `forget_logbook_entry` and
//! `teach_logbook_entry` in `crates/ipc/operations/`. `docs/concepts/logbook.md`.
//!
//! **Bridge's mirror is `packages/protocol/src/logbook.ts`**, field for field.

use serde::{Deserialize, Serialize};

use crate::Instant;

/// Whose entry it is: one repository's, the owner's across every repository, or this Mac's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogbookSection {
    Repo,
    Habits,
    Machine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogbookKind {
    Command,
    Gotcha,
    Convention,
    Environment,
    Habit,
}

/// A candidate is not handed to agents, a standing entry is, and a faded one is kept and searchable only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogbookState {
    Candidate,
    Standing,
    Faded,
}

/// One record row an entry cites, and the Job whose record holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogbookEvidence {
    pub job_id: String,
    /// The Job's title, or its id where it has none.
    pub job_title: String,
    /// The row's own cite in that record, as Distil named it.
    pub cite: String,
    /// What the row shows, in a line. **Absent**: Fleet does not read the record back for a list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    pub at: Instant,
}

/// Who replaced a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogbookReplacedBy {
    Distil,
    Person,
}

/// Text an entry held before.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogbookEarlier {
    pub text: String,
    /// When it was replaced.
    pub until: Instant,
    pub by: LogbookReplacedBy,
}

/// One entry. **An id is a number for an entry Fleet learnt or a person taught, and a word and a
/// hash for one folded in from run memory or the machine note**, which the acts below refuse.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogbookEntry {
    pub id: String,
    pub section: LogbookSection,
    /// The repository's root, on a `repo` entry only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub kind: LogbookKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    pub evidence: Vec<LogbookEvidence>,
    /// How many distinct Jobs confirmed it.
    pub confirmed_by: u32,
    pub first_seen: Instant,
    pub last_confirmed: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_served: Option<Instant>,
    /// When a Job last met its warning anyway. Absent where none has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_tripped: Option<Instant>,
    pub state: LogbookState,
    pub pinned: bool,
    pub edited: bool,
    /// Earlier text, newest first.
    pub history: Vec<LogbookEarlier>,
}

/// `get_logbook`: the entries asked for, forgotten ones left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Logbook {
    pub entries: Vec<LogbookEntry>,
}

/// `edit_logbook_entry`' body: the text the entry now says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditLogbookEntry {
    pub text: String,
}

/// `pin_logbook_entry`' body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinLogbookEntry {
    pub pinned: bool,
}

/// `forget_logbook_entry`' answer: the entry is on the deny list and gone from every read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogbookForgotten {
    pub id: String,
}

/// `teach_logbook_entry`' body: an entry a person wrote, which stands at once.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachLogbookEntry {
    pub section: LogbookSection,
    /// The repository's root, **required for a `repo` entry** and ignored for the others.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub kind: LogbookKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}
