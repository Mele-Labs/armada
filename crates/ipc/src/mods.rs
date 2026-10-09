//! Mods: a directory on this machine that changes how Bridge looks.
//! `docs/concepts/mods.md`.
//!
//! **A mod that is broken is a row with `valid: false`, never a failed read.**
//! Every field Fleet could not read is left out and `reason` says why, so a list
//! with one bad mod in it is still a list.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};

/// What a mod changes. The kinds this build knows; a `mod.toml` naming another
/// is invalid, so a valid row's kind is always one Bridge can act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModKind {
    Theme,
    /// A `layout.json` that reorders and hides Bridge's tabs, panels and rail rows.
    Layout,
}

/// One mod on disk, as `list_mods` answers it and `mods.changed` carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModSummary {
    /// The directory's name, which `mod.toml` must repeat.
    pub name: String,
    /// Absent where `mod.toml` could not be read or names a kind this build
    /// does not have.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ModKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
}

/// A mod made. `path` is the directory, where `theme.css` or `layout.json` is edited.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModScaffolded {
    pub name: String,
    pub path: String,
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
}

/// `promote_mod`: put the mod on a branch of a repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromoteMod {
    pub name: String,
    /// The repository to promote into. Absent is the first one Fleet serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
}

/// A mod on a branch of the repository, pushed nowhere.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModPromoted {
    pub name: String,
    pub branch: String,
    pub commit: String,
}
