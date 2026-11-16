//! A Kit piece's file: its text, a corrected text on its way back, and the press that opens it
//! elsewhere.
//!
//! **Fleet resolves the path and nothing here carries one**, as [`crate::SaveManifestFile`]'s
//! module says of `armada.yml`. A piece is named as `get_kit_inventory` names it, by `kind` and
//! `name`, and Fleet finds its file in an inventory read for the call, so a body can reach no file
//! the inventory does not list.
//!
//! **A save over a file that moved is refused**, as the Manifest's is: the disk at the moment of
//! the save must still hold [`SaveKitFile::read`].

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// The kinds of `get_kit_inventory` whose rows are a file: the words `SetupKindRow::kind` uses.
/// A plugin is a folder of pieces and has no file of its own, so it is [`KitOpenKind`]'s alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KitFileKind {
    Skills,
    SubAgents,
    Commands,
    AgentFile,
    OutputStyles,
}

/// What `open_kit_piece` opens: every [`KitFileKind`] and an installed plugin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KitOpenKind {
    Skills,
    SubAgents,
    Commands,
    AgentFile,
    OutputStyles,
    Plugins,
}

impl From<KitFileKind> for KitOpenKind {
    fn from(kind: KitFileKind) -> KitOpenKind {
        match kind {
            KitFileKind::Skills => KitOpenKind::Skills,
            KitFileKind::SubAgents => KitOpenKind::SubAgents,
            KitFileKind::Commands => KitOpenKind::Commands,
            KitFileKind::AgentFile => KitOpenKind::AgentFile,
            KitFileKind::OutputStyles => KitOpenKind::OutputStyles,
        }
    }
}

/// The request half of `get_kit_file`, as query parameters. `name` is `SetupItem::name`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetKitFile {
    pub kind: KitFileKind,
    pub name: String,
    /// The plugin that brought the piece, as `SetupItem::plugin` names it. Absent means the person's
    /// own or a repository's, so a piece of the same name from a plugin is told apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
}

/// A piece's file as it is on disk. **Whole and not windowed**: a partial text handed to an editor
/// would be saved back over the rest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitFile {
    /// The file Fleet resolved, for the view to name. A skill's is its `SKILL.md`.
    pub path: String,
    pub text: String,
    /// When Fleet read it. What the guard compares is [`SaveKitFile::read`], never this.
    pub at: Instant,
    /// The plugin that brought the piece, which is why [`SaveKitFile`] refuses it. Left out of the
    /// person's own pieces, so a view can draw the file read-only before a save is tried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
}

/// A corrected file, on its way to disk. **Every field is required**, so omitting
/// [`SaveKitFile::read`] fails to decode rather than overwriting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveKitFile {
    pub kind: KitFileKind,
    pub name: String,
    /// What the edit started from, [`KitFile::text`] unchanged and whole. Fleet refuses where the
    /// disk no longer holds it.
    pub read: String,
    /// The bytes to put on disk, exactly as they arrive.
    pub text: String,
    /// The plugin that brought the piece, as `SetupItem::plugin` names it. Absent means the person's
    /// own or a repository's, so a piece of the same name from a plugin is told apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
}

/// Where `open_kit_piece` opens a piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KitPlace {
    /// The person's editor: `editor.command`, then `$VISUAL`, then `$EDITOR`, then the system's.
    Editor,
    /// Finder, with the piece selected.
    Finder,
}

/// The request half of `open_kit_piece`. A plugin opens in Finder alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenKitPiece {
    pub kind: KitOpenKind,
    pub name: String,
    /// The plugin that brought the piece, as `SetupItem::plugin` names it. Absent means the person's
    /// own or a repository's, so a piece of the same name from a plugin is told apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    pub place: KitPlace,
}

/// What `open_kit_piece` opened, as Fleet resolved it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KitPieceOpened {
    pub path: String,
}
