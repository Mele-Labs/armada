//! Mods: a directory on this machine that changes how Bridge looks.
//! `docs/concepts/mods.md`.
//!
//! **A mod that is broken is a row with `valid: false`, never a failed read.**
//! Every field Fleet could not read is left out and `reason` says why, so a list
//! with one bad mod in it is still a list.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};
use crate::session_notes::SessionNote;
use crate::sessions::SessionId;
use crate::taught_mods::{CommandsFile, FleetFile, WordsFile};

/// What a mod changes. The kinds this build knows; a `mod.toml` naming another
/// is invalid, so a valid row's kind is always one Bridge can act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModKind {
    Theme,
    /// A `layout.json` that reorders and hides Bridge's tabs, panels and rail rows.
    Layout,
    /// A `words.toml`: a pattern Bridge links in a Session, and where it opens in each repository.
    Words,
    /// A `fleet.toml`: what Fleet does with a notice it would send, by Job, repository or everywhere.
    Fleet,
    /// A `tweak.css`: token values layered over whichever theme is chosen, several at once.
    Tweak,
    /// A `tracker.toml` that teaches Fleet a tracker this build has no adapter for.
    Tracker,
    /// A `commands.toml`: rows added to Bridge's command palette.
    Commands,
}

/// Where a mod comes from. Ordered so a local mod is listed ahead of the built-in one of its name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModOrigin {
    /// A folder in this machine's `mods/`.
    #[default]
    Local,
    /// A folder under the repository's `packages/mods/`, compiled into this Fleet.
    BuiltIn,
}

impl ModOrigin {
    fn is_local(&self) -> bool {
        *self == ModOrigin::Local
    }
}

/// One mod on disk, as `list_mods` answers it and `mods.changed` carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModSummary {
    /// The directory's name, which `mod.toml` must repeat. A built-in mod's is `built-in:` and
    /// its folder's name, which no directory's can be, so every operation names one or the other.
    pub name: String,
    /// Left out where local, so a local mod's row is the one it always was.
    #[serde(default, skip_serializing_if = "ModOrigin::is_local")]
    pub origin: ModOrigin,
    /// Absent where `mod.toml` could not be read or names a kind this build
    /// does not have.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ModKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// What Teach was pressed on, where a person taught this mod. Absent for a mod written by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taught_on: Option<String>,
    /// The Logbook entry a teaching from a Job made this mod with. Absent for a mod written by hand
    /// or taught somewhere no entry was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    /// The Job whose `scaffold_mod` made this mod. Absent for a mod made by hand, by a teaching, or
    /// by a call that named no Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub made_by: Option<SessionId>,
    /// This machine's switch. A mod nobody switched is enabled.
    pub enabled: bool,
    /// Whether Bridge may load it. Anything wrong with `mod.toml` or the files
    /// it names makes this false.
    pub valid: bool,
    /// The first thing wrong, and how many more. Present only where `valid` is
    /// false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// When the newest of the mod's own files was written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_at: Option<Instant>,
    /// A tracker mod's last try, present only where it tried the `tracker.toml` that is there now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tried: Option<TrackerTry>,
    /// A tracker mod that was tried, and whose `tracker.toml` has been edited since, so `tried` is absent.
    /// Absent, like `tried`, for a mod never tried.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stale: bool,
    /// The pull request a promote opened for this mod, while it is open, and the Job that holds it.
    /// Absent for a mod never promoted, and once that pull request merges or closes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<ModPullRequest>,
}

/// A promoted mod's open pull request, as the Job of its own holds it on its row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModPullRequest {
    pub address: String,
    pub number: u64,
    /// What the promote titled the pull request and its Job: `Add the calm theme mod`.
    pub title: String,
    /// The Job of its own that holds it.
    pub session_id: SessionId,
}

/// Every mod on this machine, by name. The answer to `list_mods` and the body
/// of `mods.changed`, which a client replaces its list with.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModList {
    pub mods: Vec<ModSummary>,
}

/// `scaffold_mod`: make a mod with a starter `theme.css`, or a `layout.json` for a layout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScaffoldMod {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Absent is a theme, which is all there was before layouts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ModKind>,
    /// The Job calling, which the mod records as `made_by`. Absent records none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// A mod made. `path` is the directory, where `theme.css` or `layout.json` is edited.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModScaffolded {
    pub name: String,
    pub path: String,
}

/// `start_mod_session`: a Mod Job, a hosted Job on Armada's own repository for modding Armada
/// itself, started on what the person pointed at, or on his words where he pointed at nothing.
/// **Not both `note` and `said`**, nor a blank `said`: a 422 `fleet.unacceptable_mod_start`.
/// Neither is an empty Mod Job, whose brief rides ahead of his first message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartModSession {
    /// Absent is the first line of the note's words, or of `said`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Absent is the machine's own default, as for `start_session`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// What he pointed at, as `send_session_note` takes it: the Job's first message. Neither: an empty Mod Job, whose brief rides ahead of his first message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<SessionNote>,
    /// His words, where Mod was pressed on nothing: the Job's first message after the brief.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
}

/// What he picked in an empty Mod Job: the kind of mod he is aiming for, `source` for a change to Armada's code. Read with his first message only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModAim {
    Theme,
    Layout,
    Words,
    Fleet,
    Tracker,
    Commands,
    Source,
}

/// A Mod Job started, and sent its first message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModSessionStarted {
    pub session_id: SessionId,
    pub title: String,
    /// Why the first message was not sent, in Fleet's words, where the Job started and the send
    /// was refused. Bridge opens the Job and the person sends by hand. Absent when sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_failed: Option<String>,
}

/// `set_mod_enabled`: this machine's switch for one mod.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetModEnabled {
    pub name: String,
    pub enabled: bool,
}

/// What `validate_mod` found. `problems` is empty exactly when `valid`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModChecked {
    pub name: String,
    pub valid: bool,
    pub problems: Vec<String>,
    /// The stylesheet as it was checked, present only where it passed. Bridge
    /// injects this text and no other, so what it loads is what was validated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub css: Option<String>,
    /// A layout mod's `layout.json` as it was checked, present only where it passed. Bridge
    /// applies this text and no other, as it does `css`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    /// A Words mod's `words.toml` as it was checked, present only where it passed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<WordsFile>,
    /// A Fleet mod's `fleet.toml` as it was checked, present only where it passed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fleet: Option<FleetFile>,
    /// A Commands mod's `commands.toml` as it was checked, present only where it passed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<CommandsFile>,
}

/// `try_tracker_mod`: run a tracker mod's Open query, and read one issue it listed, against a
/// repository Fleet serves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TryTrackerMod {
    pub name: String,
    /// The repository to try it against. Absent is the first one served whose Manifest names this mod as its tracker, else the first served.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
}

/// How a tracker mod's try went. **Only a tracker that passed one is used** by a Manifest naming it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackerTry {
    pub at: Instant,
    /// The repository it was tried against.
    pub manifest_id: ManifestId,
    pub passed: bool,
    /// Why it did not pass. Present only where `passed` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// How many issues the Open query listed.
    pub listed: u32,
    /// The reference read back, where the Open query listed one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<String>,
}

/// `promote_mod`: put the mod into a repository by a Job of its own, with a pull request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromoteMod {
    pub name: String,
    /// The repository to promote into. Absent is the first one Fleet serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
}

/// A mod committed by a Job of its own, and the pull request it opened. Modelled on
/// `CommittedToRepository`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModPromoted {
    pub name: String,
    /// The Job made for this promote, which watches its pull request until it merges.
    pub session_id: SessionId,
    /// What the commit, the Job and the pull request are titled: `Add the calm theme mod`.
    pub title: String,
    /// The Job's own branch, which holds the one commit.
    pub branch: String,
    pub commit: String,
    /// The pull request's address, where one was opened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<String>,
    /// Why no pull request was opened, where none was. The commit stays on the Job's branch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request_not_opened: Option<String>,
}
