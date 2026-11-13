//! What the editor asks of Fleet, through the host that mounts it. Each answer is a task the editor awaits off its
//! own update, so a host that keeps its wire on an entity of its own can answer without being updated under the
//! editor.

use gpui::{App, Task};
use ipc::{RepositoryAutonomy, RepositoryLifecycleRules, WatchRecipeList, AffectedJobs, DeleteLifecyclePiece, JobLanesView, LifecyclePieceDeleted, LifecyclePieceKind, MigrateJobs, MigratedJobs, PieceDraft, PieceDraftDiscarded, PieceScope, PiecePublished, PublishPieceDraft, PublishWorkflowDraft, PutPieceDraft, PutWorkflowDraft, RepositoryLifecycles, RevertWorkflow, WorkflowDefinition, WorkflowDraft, WorkflowDraftDiscarded, WorkflowPublished};

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
    /// The project the host is editing, by id and name, offered first where an edit asks where it applies. A
    /// project's own copy is that project's.
    pub project: Option<(String, String)>,
}

impl Subject {
    /// Where its draft is kept: the source and, where that is a project's, the project's id.
    pub fn place(&self) -> (&'static str, Option<String>) {
        match (self.source, &self.project) {
            ("project", Some((id, _))) => ("project", Some(id.clone())),
            (source, _) => (source, None),
        }
    }

    /// Where an edit applies without asking: the place the workflow is kept, where it is a copy.
    pub fn own_target(&self) -> Option<Target> {
        match (self.shipped, self.source) {
            (true, _) => None,
            (false, "repository") => Some(Target::Repository(self.held_under.clone())),
            (false, "project") => self.project.as_ref().map(|(id, _)| Target::Project(id.clone())),
            (false, _) => Some(Target::EveryProject),
        }
    }
}

/// The place a publish writes to: `kit`, `repository` or `project`, the project where it is one, and the Manifest the
/// publish is made under (Fleet reads it only for a repository's own).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub source: String,
    pub project: Option<String>,
    pub manifest: String,
}

pub trait EditorWire {
    /// `GET /workflows/lanes`.
    fn lanes(&self, at: &Subject, cx: &mut App) -> Task<Result<JobLanesView, Refused>>;
    /// `GET /workflows/definition`: the file's text.
    fn definition(&self, at: &Subject, cx: &mut App) -> Task<Result<WorkflowDefinition, Refused>>;
    /// `GET /workflows/draft`: Fleet's draft of it, or `fleet.workflow_draft_none` where none is kept.
    fn workflow_draft(&self, at: &Subject, cx: &mut App) -> Task<Result<WorkflowDraft, Refused>>;
    /// `PUT /workflows/draft` under a Manifest: the draft text as it is now.
    fn put_workflow_draft(&self, manifest: &str, put: PutWorkflowDraft, cx: &mut App) -> Task<Result<WorkflowDraft, Refused>>;
    /// `POST /workflows/revert` under a Manifest: a version's text put in the draft as a change of his.
    fn revert_workflow(&self, manifest: &str, revert: RevertWorkflow, cx: &mut App) -> Task<Result<WorkflowDraft, Refused>>;
    /// `DELETE /workflows/draft`.
    fn discard_workflow_draft(&self, at: &Subject, cx: &mut App) -> Task<Result<WorkflowDraftDiscarded, Refused>>;
    /// `POST /workflows/draft/publish` under a Manifest.
    fn publish_workflow_draft(&self, manifest: &str, publish: PublishWorkflowDraft, cx: &mut App) -> Task<Result<WorkflowPublished, Refused>>;
    /// `GET /workflows/affected`: the live Jobs that follow the workflow at `place`, and where each stands against the draft.
    fn affected_jobs(&self, at: &Subject, place: &Place, cx: &mut App) -> Task<Result<AffectedJobs, Refused>>;
    /// `POST /workflows/migrate` under a Manifest: the Jobs picked, moved to the published version.
    fn migrate_jobs(&self, manifest: &str, migrate: MigrateJobs, cx: &mut App) -> Task<Result<MigratedJobs, Refused>>;
    /// `GET /lifecycles`: the lifecycles that apply to a repository, by its Manifest id, and where each was kept.
    fn lifecycles(&self, manifest: &str, cx: &mut App) -> Task<Result<RepositoryLifecycles, Refused>>;
    /// `GET /lifecycles/rules`: the rules that apply to a repository, by its Manifest id. A host that keeps no rules answers
    /// with a refusal, and the lanes' rule chips are read and not opened.
    fn lifecycle_rules(&self, _manifest: &str, _cx: &mut App) -> Task<Result<RepositoryLifecycleRules, Refused>> {
        Task::ready(Err(Refused::said("This host reads no rules")))
    }
    /// `GET /workflows/lanes` at Armada's own level: the lanes of the workflow as Armada ships it, which a copy is compared with.
    fn armada_lanes(&self, _at: &Subject, _cx: &mut App) -> Task<Result<JobLanesView, Refused>> {
        Task::ready(Err(Refused::said("This host reads no Armada lanes")))
    }
    /// `GET /workflows/definition` with `source=armada`: the text Armada carries now.
    fn armada_definition(&self, _at: &Subject, _cx: &mut App) -> Task<Result<WorkflowDefinition, Refused>> {
        Task::ready(Err(Refused::said("This host reads no Armada text")))
    }
    /// `GET /workflows/definition` with `source=base`: the text the copy began from; `fleet.workflow_base_none` where it has none.
    fn base_definition(&self, _at: &Subject, _cx: &mut App) -> Task<Result<WorkflowDefinition, Refused>> {
        Task::ready(Err(Refused::said("This host reads no base text")))
    }
    /// `GET /autonomy`: Armada's default for each kind of act, and the layers over it.
    fn autonomy_defaults(&self, _manifest: &str, _cx: &mut App) -> Task<Result<RepositoryAutonomy, Refused>> {
        Task::ready(Err(Refused::said("This host reads no autonomy defaults")))
    }
    /// `GET /watches/recipes`: what a rule's start of a watch may name.
    fn watch_recipes(&self, _cx: &mut App) -> Task<Result<WatchRecipeList, Refused>> {
        Task::ready(Err(Refused::said("This host reads no watch recipes")))
    }
    /// The workflows Fleet runs, `(id, name)`: what a condition on the workflow a Job follows may name.
    fn workflow_names(&self, _cx: &mut App) -> Vec<(String, String)> {
        Vec::new()
    }
    /// `GET /lifecycles/pieces/draft`: `fleet.piece_draft_none` where none is kept.
    fn piece_draft(&self, kind: LifecyclePieceKind, id: &str, scope: &PieceScope, cx: &mut App) -> Task<Result<PieceDraft, Refused>>;
    /// `PUT /lifecycles/pieces/draft`: a state renamed in place is kept as its lifecycle's draft.
    fn put_piece_draft(&self, put: PutPieceDraft, cx: &mut App) -> Task<Result<PieceDraft, Refused>>;
    /// `DELETE /lifecycles/pieces/draft`.
    fn discard_piece_draft(&self, kind: LifecyclePieceKind, id: &str, scope: &PieceScope, cx: &mut App) -> Task<Result<PieceDraftDiscarded, Refused>>;
    /// `POST /lifecycles/pieces/draft/publish`.
    fn publish_piece_draft(&self, publish: PublishPieceDraft, cx: &mut App) -> Task<Result<PiecePublished, Refused>>;
    /// `POST /lifecycles/pieces/delete`.
    #[allow(dead_code)] // For the edits that take a piece out.
    fn delete_piece(&self, delete: DeleteLifecyclePiece, cx: &mut App) -> Task<Result<LifecyclePieceDeleted, Refused>>;
}
