//! Saving and deleting one Workspace, lifecycle, rule or autonomy default in Kit, which is where
//! Bridge's Workflows page keeps what a person edits. The reads are `lifecycles`.
//!
//! **The DTOs a read answers are the input.** Bridge sends back what it was sent, changed, and
//! Fleet writes the piece's file syntax itself and refuses a piece its own parser would refuse, so
//! no client holds a second copy of the file format.
//!
//! **A piece is (kind, id, scope), and not its id alone.** `ticket` for every repository and
//! `ticket` within one repository are two files that rank against each other. A Workspace has no
//! scope: its id is its identity.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::lifecycles::{LifecycleDefinition, LifecycleRule, PieceScope};
use crate::ManifestId;

fn everywhere() -> PieceScope {
    PieceScope::Everywhere
}

/// The four kinds of piece a person keeps in Kit's folders.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecyclePieceKind {
    Lifecycle,
    Rule,
    Workspace,
    Autonomy,
}

impl LifecyclePieceKind {
    pub fn as_wire(self) -> &'static str {
        match self {
            LifecyclePieceKind::Lifecycle => "lifecycle",
            LifecyclePieceKind::Rule => "rule",
            LifecyclePieceKind::Workspace => "workspace",
            LifecyclePieceKind::Autonomy => "autonomy",
        }
    }
}

/// A Workspace as it is saved: a set of repositories under one id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceToSave {
    pub id: String,
    pub name: String,
    /// The repositories, in the order the file should list them.
    pub repositories: Vec<ManifestId>,
}

/// A layer of autonomy defaults as it is saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyToSave {
    pub id: String,
    #[serde(default = "everywhere")]
    pub scope: PieceScope,
    /// A kind of act (`tracker_write`, `forge_write`, `draft`, `start_job`, `start_watch`,
    /// `tell_agent`, `run_command` or `run_agent`) to `acts`, `asks` or `off`.
    pub defaults: BTreeMap<String, String>,
}

/// One piece, by kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LifecyclePiece {
    Lifecycle { lifecycle: LifecycleDefinition },
    Rule { rule: LifecycleRule },
    Workspace { workspace: WorkspaceToSave },
    Autonomy { autonomy: AutonomyToSave },
}

/// `save_lifecycle_piece`'s body. `POST /lifecycles/pieces/save`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveLifecyclePiece {
    pub piece: LifecyclePiece,
    /// **Required to replace the piece that already holds this kind, id and scope.** Absent is
    /// `false`, so replacing is something a caller says.
    #[serde(default)]
    pub overwrite: bool,
}

/// What `save_lifecycle_piece` wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecyclePieceSaved {
    pub kind: LifecyclePieceKind,
    pub id: String,
    /// `everywhere` for a Workspace, which has none.
    pub scope: PieceScope,
    /// The file as written, absolute.
    pub file: String,
    /// It took the place of a file that declared the same kind, id and scope.
    pub replaced: bool,
}

/// `delete_lifecycle_piece`'s body. `POST /lifecycles/pieces/delete`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteLifecyclePiece {
    pub kind: LifecyclePieceKind,
    pub id: String,
    /// Absent is everywhere. Ignored for a Workspace.
    #[serde(default = "everywhere")]
    pub scope: PieceScope,
}

/// What `delete_lifecycle_piece` removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecyclePieceDeleted {
    pub kind: LifecyclePieceKind,
    pub id: String,
    pub scope: PieceScope,
    /// The file that was deleted, absolute.
    pub file: String,
}
