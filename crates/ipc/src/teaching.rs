//! Teaching on the wire: a person selects words in a Session, says what they are and what to do, and
//! Fleet reads that into a proposal they check before it is kept. `read_teaching`, `save_teaching`,
//! `list_taught_links` and `forget_taught_link` in `crates/ipc/operations/`.
//!
//! **A link rule is Fleet's to keep; an agent note is the Logbook's**, which Bridge sends itself to
//! `teach_logbook_entry`. Nothing here reads or writes the Logbook, so a note names its section and
//! kind with the words that route takes and no more.
//!
//! **Bridge's mirror is `packages/protocol/src/teaching.ts`**, field for field.

use serde::{Deserialize, Serialize};

use crate::Instant;

/// How far a teaching reaches: the Session's repository, or all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeachingReach {
    Repository,
    Everywhere,
}

/// `read_teaching`' body: what the person selected, where it came from and what they said about it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadTeaching {
    /// The selected words.
    pub words: String,
    /// The message they were selected from.
    pub message: String,
    /// What the person wrote: what the words are and what to do.
    pub said: String,
    /// The Session's repository, a root `list_repositories` names. **Required for the
    /// `repository` reach.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub reach: TeachingReach,
}

/// Where an agent note goes: one repository's, the person's habits, or this Mac's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteSection {
    Repo,
    Habits,
    Machine,
}

/// What an agent note says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    Command,
    Gotcha,
    Convention,
    Environment,
    Habit,
}

/// A rule that turns words into a link. Bridge runs `pattern` as a JavaScript regular expression,
/// so Fleet holds it to the subset both read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachingLink {
    pub pattern: String,
    /// An `http` or `https` address, with `{match}` where the matched words go.
    pub address: String,
    /// The selected words, which the pattern matches whole.
    pub example: String,
}

/// What an agent is to be told from now on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachingNote {
    pub section: NoteSection,
    pub kind: NoteKind,
    pub text: String,
}

/// `read_teaching`' answer: **at least one of the two is present**. Nothing is kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachingProposal {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<TeachingLink>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<TeachingNote>,
}

/// `save_teaching`' body: the link rule to keep, and the note to tell a live Session about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveTeaching {
    /// The rule to keep, held to the rules `read_teaching` holds a proposal to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<TeachingLink>,
    /// The repository the rule is for, a root `list_repositories` names. Absent is every repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// The Session to tell, with `note`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// The text of the agent note the person kept, which that Session is told now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One kept link rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaughtLink {
    pub id: String,
    pub pattern: String,
    pub address: String,
    pub example: String,
    /// The repository's root. Absent: it applies to every repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub made_at: Instant,
}

/// `save_teaching`' answer: the rule as kept, where there was one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedTeaching {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<TaughtLink>,
}

/// `list_taught_links`' answer: the rules for the repository asked for, and those for every one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaughtLinks {
    pub links: Vec<TaughtLink>,
}

/// `forget_taught_link`' answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgottenTaughtLink {
    pub id: String,
}
