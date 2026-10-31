//! The Logbook on the wire: what Armada learned, read, and the acts a person has on an entry.
//! `get_logbook`, `edit_logbook_entry`, `lock_logbook_entry`, `forget_logbook_entry` and
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

/// What an entry is about, as the sorter places it: Git and PRs, Slots and worktrees, Testing and checks,
/// Fleet and Bridge tools, Mac and shell. **No `other`**: an entry the sorter has not placed has no topic, and
/// Bridge draws it under Other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogbookTopic {
    Git,
    Slots,
    Testing,
    Tools,
    Mac,
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
    /// Tidy folded a near-duplicate into this entry and wrote the one line for both.
    Merged,
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
    /// Where the sorter placed it. Absent until it has, and then Other.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<LogbookTopic>,
    /// A few words naming it, as its line in the log reads. Absent on an entry written before titles were.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
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
    pub locked: bool,
    pub edited: bool,
    /// Earlier text, newest first.
    pub history: Vec<LogbookEarlier>,
}

/// `get_logbook`: the entries asked for, forgotten ones left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Logbook {
    pub entries: Vec<LogbookEntry>,
    pub tidy: LogbookTidy,
}

/// Whether a Tidy pass is running, and what the last one did. Kept in memory: a restart forgets `last`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogbookTidy {
    pub running: bool,
    /// The last pass that ended since Fleet started, failed or not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<LogbookTidyLast>,
}

/// What a finished Tidy pass did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogbookTidyLast {
    /// Entries folded into another and archived.
    pub merged: u32,
    pub at: Instant,
    /// A repository's call failed: `merged` counts what was archived before it did.
    pub failed: bool,
}

/// `edit_logbook_entry`' body: the text the entry now says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditLogbookEntry {
    pub text: String,
}

/// `lock_logbook_entry`' body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockLogbookEntry {
    pub locked: bool,
}

/// `forget_logbook_entry`' answer: the entry is on the deny list and gone from every read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgottenLogbookEntry {
    pub id: String,
}

/// `tidy_logbook`' body: the repository whose Logbook to tidy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TidyLogbook {
    /// A served repository's root. **Absent**: every served repository, with the habits and this Mac's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

/// `tidy_logbook`' answer, given at once: a pass is running, whether this call started it or one was already going.
/// What it merged is in `get_logbook`'s `tidy`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TidiedLogbook {
    pub running: bool,
}

/// The three looks Organize takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizeAction {
    /// Entries that say one thing in different words, folded into one.
    Merge,
    /// Entries nobody has confirmed in 45 days.
    Stale,
    /// Entries that read as workarounds for a fault in Armada itself.
    Bugs,
}

/// `organize_logbook`' body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizeLogbook {
    pub action: OrganizeAction,
    /// A served repository's root. **Absent**: every served repository, with the habits and this Mac's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

/// One thing Organize proposes. Nothing has changed when it is read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizeProposal {
    /// Names the proposal: the entry kept for a merge, the entry for the others.
    pub id: String,
    /// The entries it concerns. A merge lists the entry it keeps first, then those it absorbs.
    pub entry_ids: Vec<String>,
    /// A merge: the kept entry as it would read. The others: the entry's own text.
    pub text: String,
    /// Stale: how long since it was confirmed. Bugs: the one-line fault. A merge: empty.
    pub note: String,
}

/// `organize_logbook`' answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizedLogbook {
    pub action: OrganizeAction,
    pub proposals: Vec<OrganizeProposal>,
    /// A repository's model call failed, so its proposals are missing. The others are here.
    pub failed: bool,
}

/// `apply_organize_logbook`' body: the proposals to carry out, as `organize_logbook` answered them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyOrganizeLogbook {
    pub action: OrganizeAction,
    pub proposals: Vec<OrganizeProposal>,
}

/// `apply_organize_logbook`' answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedOrganizeLogbook {
    pub action: OrganizeAction,
    /// Merge: entries folded into another. Stale: entries archived. Bugs: 0, since nothing is written.
    pub applied: u32,
    /// Bugs: the proposals as reported, which is the whole of reporting for now. Empty for the others.
    pub reported: Vec<OrganizeProposal>,
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
    /// What the person pressed Teach on. It becomes the entry's one evidence row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<TeachSource>,
}

/// What a taught entry was taught from: the Session, and the message, notice or Check in its record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachSource {
    pub session_id: String,
    pub cite: String,
}

/// `recall`: the entries that match, best first. `GET /logbook/recall?query=&section=&manifest_id=`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recalled {
    pub entries: Vec<RecalledEntry>,
}

/// One Logbook entry as `recall` answers it. Faded and candidate entries are in it, and `state`
/// says which.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecalledEntry {
    /// `repo`, `habits` or `machine`.
    pub section: String,
    /// `command`, `gotcha`, `convention`, `environment` or `habit`.
    pub kind: String,
    /// `candidate`, `standing` or `faded`.
    pub state: String,
    pub text: String,
    /// How many distinct Sessions bore it out.
    pub confirmed_by: u32,
}

/// `get_logbook_for_subagent`'s body: the Session whose subagent is starting, and the prompt it is
/// given. `POST /logbook/subagent`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentAsk {
    pub session_id: String,
    pub prompt: String,
}

/// What a subagent is handed. `text` is empty where the Logbook holds nothing for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForSubagent {
    pub text: String,
}
