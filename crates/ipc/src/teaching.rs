//! Teaching on the wire: a person presses Teach on something in a Session or in Bridge, says what
//! they mean, and Fleet reads that into a proposal they check before it is kept. `read_teaching`,
//! `save_teaching` and `list_word_rules` in `crates/ipc/operations/`. `docs/concepts/mods.md`.
//!
//! **A teaching becomes a Logbook entry for what agents should know, and mods by what it changes**:
//! Words, Fleet, a theme tweak or a layout. Save makes the entry through `teach_logbook_entry` and
//! writes the mods, so Bridge files nothing itself. A teaching on Bridge's own interface has no
//! agent note and lives in Mods alone.
//!
//! **Bridge's mirror is `packages/protocol/src/teaching.ts`**, field for field.

use serde::{Deserialize, Serialize};

use crate::logbook::{LogbookKind, LogbookSection};
use crate::taught_mods::FleetRule;

/// How far a teaching reaches: the Session's repository, or all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeachingReach {
    Repository,
    Everywhere,
}

/// What Teach was pressed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeachingSourceKind {
    /// Words selected in a message.
    Words,
    /// A command a Session ran.
    Command,
    /// A line Fleet sent a Session. `ids` holds the notice's word first.
    Notice,
    /// A task in a Session's plan.
    Task,
    /// A Check's result.
    Check,
    PullRequest,
    /// A panel of Bridge's own interface. `ids` holds the panel's id.
    Panel,
    /// A row of a list in Bridge.
    Row,
    /// The title bar.
    TitleBar,
}

impl TeachingSourceKind {
    /// Whether it is Bridge's own interface, where no agent was in the room: such a teaching has no
    /// agent note.
    pub fn is_bridge(self) -> bool {
        matches!(self, TeachingSourceKind::Panel | TeachingSourceKind::Row | TeachingSourceKind::TitleBar)
    }
}

/// The thing Teach was pressed on, as a short text and the ids that name it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachingSource {
    pub kind: TeachingSourceKind,
    /// What it shows, in a few words. A mod made from this records it as `taught_on`.
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
}

/// `read_teaching`' body: what was selected or pressed, where it came from and what the person said.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadTeaching {
    /// The selected words. Empty for a teaching on Bridge's interface, where `source.text` says it.
    #[serde(default)]
    pub words: String,
    /// The message they were selected from.
    #[serde(default)]
    pub message: String,
    /// What the person wrote: what the thing is and what to do.
    pub said: String,
    /// The Session's repository, a root `list_repositories` names. **Required for the
    /// `repository` reach.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    pub reach: TeachingReach,
    /// The Session it was taught in, which a rule for one Job is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// What Teach was pressed on. Absent is selected words, as before there were others.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<TeachingSource>,
}

/// What an agent is to be told from now on: an entry for the Logbook.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachingNote {
    pub section: LogbookSection,
    pub kind: LogbookKind,
    pub text: String,
}

/// A pattern Bridge links, with the address for the repository it was taught in where the person
/// gave one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachingWords {
    /// What the pattern finds, as a person would say it: `Jira key`.
    pub name: String,
    /// A regular expression Bridge runs as a JavaScript one, held to the subset both engines read.
    pub pattern: String,
    /// The selected words, which the pattern matches whole.
    pub example: String,
    /// An `http` or `https` address with `{match}` where the matched words go. **Absent when the
    /// person did not give one**: Fleet looks for it in the repository, and asks if it finds none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

/// One token a theme tweak sets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakChange {
    /// A design token, `--bg-raised`.
    pub token: String,
    /// The value the shipped theme gives it. Fleet fills this in.
    pub was: String,
    /// The value to set.
    pub now: String,
    /// The tokens that are aliases of this one, so they move with it. **The places a token is used
    /// that Fleet can name**: the tokens file lists no component that reads one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub used_in: Vec<String>,
    /// The token's description in the tokens file, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A layout change, written as the `layout.json` text a layout mod holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachingLayout {
    /// What the mod is called, which Fleet makes a name of.
    pub name: String,
    pub layout: String,
}

/// `read_teaching`' answer: **at least one part is present**. Nothing is kept.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachingProposal {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<TeachingNote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<TeachingWords>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fleet: Option<FleetRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tweak: Vec<TweakChange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<TeachingLayout>,
}

/// `save_teaching`' body: the proposal as the person left it, and where it was taught. Every part
/// is checked again as `read_teaching` checks it; a tweak's `was`, `used_in` and `note` are not read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveTeaching {
    #[serde(flatten)]
    pub proposal: TeachingProposal,
    pub reach: TeachingReach,
    /// The Session's repository, a root `list_repositories` names. **Required for the `repository`
    /// reach and for a `repo` note.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// The Session it was taught in, which is told the note and cited as the entry's evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Where in that Session's record it was taught: a message, a notice or a Check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cite: Option<String>,
    /// What Teach was pressed on, which each mod records as `taught_on`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<TeachingSource>,
}

/// `save_teaching`' answer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedTeaching {
    /// The Logbook entry made from the note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// The mods written or added to, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mods: Vec<String>,
}

/// One pattern Bridge links, with where it opens for the repository asked about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordRule {
    /// The mod's name.
    pub mod_name: String,
    pub name: String,
    pub pattern: String,
    pub example: String,
    /// An address with `{match}`, the repository's own or the one for every repository.
    pub address: String,
}

/// `list_word_rules`' answer: the patterns that have an address for the repository asked for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordRules {
    pub rules: Vec<WordRule>,
}
