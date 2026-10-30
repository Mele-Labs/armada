//! Teaching on the wire: a person presses Teach on something in a Session or in Bridge, says what
//! they mean, and Fleet reads that into a proposal they check before it is kept. `read_teaching`,
//! `save_teaching` and `list_word_rules` in `crates/ipc/operations/`. `docs/concepts/mods.md`.
//!
//! **A teaching becomes a Logbook entry for what agents should know, and mods by what it changes**:
//! Words, Fleet, a theme tweak or a layout. Save makes the entry through `teach_logbook_entry` and
//! writes the mods, so Bridge files nothing itself. A teaching on Bridge's own interface has no
//! agent note and lives in Mods alone. **A lifecycle rule is not a mod**: it is a rule piece in Kit,
//! written as the Workflows page writes one (`save_lifecycle_piece`).
//!
//! **Bridge's mirror is `packages/protocol/src/teaching.ts`**, field for field.

use serde::{Deserialize, Serialize};

use crate::lifecycles::LifecycleRule;
use crate::logbook::{LogbookKind, LogbookSection};
use crate::piece_saving::LifecyclePieceSaved;
use crate::taught_mods::{AddressFound, FleetRule};

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
    /// A ticket in a Job's lanes. `ids` holds its key.
    Ticket,
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

/// Where a pattern opens in the repository it was taught in, and how that was known.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedAddress {
    /// An `http` or `https` address with `{match}` where the matched words go.
    pub template: String,
    /// Where it came from: `settings`, `pull_request #N` or `person`. **A save keeps it as it is**.
    pub found: AddressFound,
}

/// A pattern Bridge links, with the address for the repository it was taught in where one is known.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachingWords {
    /// What the pattern finds, as a person would say it: `Jira key`.
    pub name: String,
    /// A regular expression Bridge runs as a JavaScript one, held to the subset both engines read.
    pub pattern: String,
    /// The selected words, which the pattern matches whole.
    pub example: String,
    /// The address for the repository the teaching is in, where the person gave one or Fleet found
    /// one: a Words mod that already has it, the Manifest's tracker, a recent merged pull request.
    /// **`save_teaching` writes the one it carries.** Absent where none is known, and a save then
    /// looks in the background and asks the person once if it finds none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<ProposedAddress>,
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

/// The Fleet part of a proposal: the rule as a Fleet mod would keep it, and what the person said
/// that Fleet cannot test. **Only the rule is ever written to a file**; `untestable` is for Bridge
/// to say so, and a rule saved with some is saved without them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedFleet {
    #[serde(flatten)]
    pub rule: FleetRule,
    /// The person's own words for each condition they named that is not one of
    /// [`Condition`](crate::Condition). Fleet invents none in its place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub untestable: Vec<String>,
}

/// The rule part of a proposal: a lifecycle rule as the Workflows page edits it, and what the person
/// said that Fleet cannot test. **Only the rule is ever written to a file**; `untestable` is for
/// Bridge to say so, and a rule saved with some is saved without them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedRule {
    #[serde(flatten)]
    pub rule: LifecycleRule,
    /// The person's own words for each condition they named that is not one a rule's `if` can hold.
    /// Fleet invents none in its place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub untestable: Vec<String>,
}

/// `read_teaching`' answer: **at least one part is present**. Nothing is kept.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeachingProposal {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<TeachingNote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<TeachingWords>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fleet: Option<ProposedFleet>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tweak: Vec<TweakChange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<TeachingLayout>,
    /// A lifecycle rule: when, if and do, with an autonomy. **Its scope is the reach the person
    /// chose**, so a save puts it there whatever scope this carries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<ProposedRule>,
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
    /// The rule piece written in Kit, which is not a mod.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<LifecyclePieceSaved>,
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
    /// Where that address came from, so Bridge can say.
    pub found: AddressFound,
}

/// `list_word_rules`' answer: the patterns that have an address for the repository asked for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordRules {
    pub rules: Vec<WordRule>,
}
