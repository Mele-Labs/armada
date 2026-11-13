//! What the editor asks of Fleet, through the host that mounts it. Each answer is a task the editor awaits off its
//! own update, so a host that keeps its wire on an entity of its own can answer without being updated under the
//! editor.

use gpui::{App, Task};
use ipc::{DeleteLifecyclePiece, JobLanesView, LifecyclePieceDeleted, LifecyclePieceSaved, RepositoryLifecycles, SaveLifecyclePiece, SaveWorkflow, WorkflowDefinition, WorkflowSaved};

pub use crate::settings::wire::Refused;

use super::draft::Target;

/// The workflow an editor is on, and the place it is read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    pub workflow: String,
    /// The level its lanes are read at (`default`, or a repository's Manifest id), and the Manifest they are read under.
    pub level: String,
    pub manifest: String,
    /// The Manifest its definition is read under: the row's own.
    pub held_under: String,
    /// Where its definition is read from: `armada`, `repository` or `kit`.
    pub source: &'static str,
    /// Armada's own, or running in a repository that keeps no copy of it: an edit asks where it applies, and a
    /// copy is made where that says, when the draft is published.
    pub shipped: bool,
    /// The repository "This repository" means: the one it runs in, else the one it is read under.
    pub repository: String,
}

impl Subject {
    /// Where an edit applies without asking: the place the workflow is kept, where it is a copy.
    pub fn own_target(&self) -> Option<Target> {
        match (self.shipped, self.source) {
            (true, _) => None,
            (false, "repository") => Some(Target::Repository(self.held_under.clone())),
            (false, _) => Some(Target::EveryProject),
        }
    }
}

pub trait EditorWire {
    /// `GET /workflows/lanes`.
    fn lanes(&self, at: &Subject, cx: &mut App) -> Task<Result<JobLanesView, Refused>>;
    /// `GET /workflows/definition`: the file's text.
    fn definition(&self, at: &Subject, cx: &mut App) -> Task<Result<WorkflowDefinition, Refused>>;
    /// `POST /workflows/save` under a Manifest.
    fn save_workflow(&self, manifest: &str, save: SaveWorkflow, cx: &mut App) -> Task<Result<WorkflowSaved, Refused>>;
    /// `GET /lifecycles`: the lifecycles that apply to a repository, by its Manifest id, and where each was kept.
    fn lifecycles(&self, manifest: &str, cx: &mut App) -> Task<Result<RepositoryLifecycles, Refused>>;
    /// `POST /lifecycles/pieces/save`: a lifecycle, rule or autonomy default. A state renamed in place is written as
    /// its lifecycle.
    fn save_piece(&self, save: SaveLifecyclePiece, cx: &mut App) -> Task<Result<LifecyclePieceSaved, Refused>>;
    /// `POST /lifecycles/pieces/delete`.
    #[allow(dead_code)] // For the edits that take a piece out.
    fn delete_piece(&self, delete: DeleteLifecyclePiece, cx: &mut App) -> Task<Result<LifecyclePieceDeleted, Refused>>;
}
