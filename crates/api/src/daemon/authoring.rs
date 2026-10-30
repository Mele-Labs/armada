//! Saving a workflow definition. Since 23.28.
//!
//! **Its own surface, and not a method of [`Commands`](super::Commands)**, which
//! was at the line the gate refuses a file over when this arrived. The reason
//! that stands without the line count is the one [`Retros`](super::Retros)
//! gives: this is whole on its own, one act with one body, and a handler that
//! takes only this cannot reach for anything else a command does.

use std::future::Future;

use std::sync::Arc;

use ipc::{
    ArmadaProposed, AskWorkflowChange, ManifestId, MarkNotADrift, NotADriftMarked, ProposeNewWorkflow, ProposeToArmada, ProposeWorkflowChange, SaveWorkflow,
    SendBackWorkflowChange, SuggestWorkflowChange, WorkflowChange, WorkflowChangeApproved, WorkflowChangeRef, WorkflowChanges, WorkflowCopy, WorkflowCopyDeleted,
    WorkflowId, WorkflowSaved, WorkflowSupportBuilding,
};

use crate::daemon::Refusal;

pub trait Authoring: Send + Sync + 'static {
    /// `save_workflow` — check one workflow definition against the repository's
    /// Checks and this machine's models, and write it in the scope named.
    ///
    /// **Checked before anything is written**, with the rules a loader applies,
    /// so a definition that is saved is one that loads. [`Refusal::Unacceptable`]
    /// with the loader's reason where it does not fit, where its id cannot be a
    /// file's name, and where the scope already holds the id and `overwrite` was
    /// not set; nothing is written in any of them. [`Refusal::Fault`] where the
    /// file will not be written. The definition is held when this answers.
    fn save_workflow(
        &self,
        save: SaveWorkflow,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<WorkflowSaved, Refusal>> + Send;

    /// `delete_workflow_copy` — delete the file in a repository's or Kit's folder that defines
    /// one workflow. [`Refusal::Unacceptable`] for Armada's own, which is carried in the binary,
    /// and where that place holds no such definition. The three places are read again first.
    fn delete_workflow_copy(
        &self,
        copy: WorkflowCopy,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<WorkflowCopyDeleted, Refusal>> + Send;

    /// `reset_workflow_copy` — rewrite a copy's file with Armada's definition of the same id,
    /// checked as [`Authoring::save_workflow`] checks. [`Refusal::Unacceptable`] for Armada's own,
    /// where Armada carries no such id, and where that place holds no copy.
    fn reset_workflow_copy(
        &self,
        copy: WorkflowCopy,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<WorkflowSaved, Refusal>> + Send;

    /// `mark_not_a_drift` — mark one kept nudge so it stops counting.
    /// [`Refusal::Unacceptable`] where Fleet kept none by that id.
    fn mark_not_a_drift(
        &self,
        marked: MarkNotADrift,
    ) -> impl Future<Output = Result<NotADriftMarked, Refusal>> + Send;

    /// `ask_workflow_change` — begin a change an agent drafts from the owner's words, and answer it
    /// `drafting` at once. **Takes the `Arc`** because the draft is a task of its own.
    /// [`Refusal::Unacceptable`] for Armada's own definition, one this place does not hold, blank words,
    /// and where no model could be asked.
    fn ask_workflow_change(
        self: Arc<Self>,
        asked: AskWorkflowChange,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<WorkflowChange, Refusal>> + Send;

    /// `suggest_workflow_change` — the same, for new words for one step, from the nudges that count.
    /// [`Refusal::Unacceptable`] as `ask_workflow_change`'s, for a step the definition lacks, for a step
    /// that already has an open change, and where no nudge counts there.
    fn suggest_workflow_change(
        self: Arc<Self>,
        asked: SuggestWorkflowChange,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<WorkflowChange, Refusal>> + Send;

    /// `propose_new_workflow` — keep a workflow a Job proposes as a change that is ready at once and
    /// writes nothing until approved. [`Refusal::Unacceptable`] for a blank definition and for an id
    /// the scope already holds.
    fn propose_new_workflow(
        &self,
        proposed: ProposeNewWorkflow,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<WorkflowChange, Refusal>> + Send;

    /// `propose_workflow_change` — keep a Job's new text for a workflow that exists as a change that is
    /// ready at once and writes nothing until approved. [`Refusal::NotFound`] for a workflow the place does
    /// not hold, [`Refusal::Unacceptable`] for a definition that does not read.
    fn propose_workflow_change(
        &self,
        proposed: ProposeWorkflowChange,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<WorkflowChange, Refusal>> + Send;

    /// `list_workflow_changes` — the open changes to one workflow in one place.
    fn list_workflow_changes(
        &self,
        workflow_id: WorkflowId,
        source: String,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<WorkflowChanges, Refusal>> + Send;

    /// `approve_workflow_change` — write a ready change through the save path, with overwrite.
    /// [`Refusal::NotFound`] for an unknown change, [`Refusal::IllegalMove`] for one that is not ready or
    /// whose file changed since it began.
    fn approve_workflow_change(
        &self,
        change: WorkflowChangeRef,
    ) -> impl Future<Output = Result<WorkflowChangeApproved, Refusal>> + Send;

    /// `dismiss_workflow_change` — put an open change aside.
    fn dismiss_workflow_change(
        &self,
        change: WorkflowChangeRef,
    ) -> impl Future<Output = Result<WorkflowChange, Refusal>> + Send;

    /// `send_back_workflow_change` — draft a ready or failed change again with the owner's notes.
    fn send_back_workflow_change(
        self: Arc<Self>,
        back: SendBackWorkflowChange,
    ) -> impl Future<Output = Result<WorkflowChange, Refusal>> + Send;

    /// `propose_workflow_to_armada` — start a Job on Armada's own repository that lands a definition.
    /// [`Refusal::Unacceptable`] where this Fleet does not serve it.
    fn propose_workflow_to_armada(
        self: Arc<Self>,
        proposed: ProposeToArmada,
    ) -> impl Future<Output = Result<ArmadaProposed, Refusal>> + Send;

    /// `build_workflow_support` — start a Job on Armada's own repository that builds what a `cannot`
    /// change lacks, and close the change. [`Refusal::IllegalMove`] for a change that is not `cannot`,
    /// [`Refusal::Unacceptable`] where this Fleet does not serve Armada's own repository.
    fn build_workflow_support(
        self: Arc<Self>,
        change: WorkflowChangeRef,
    ) -> impl Future<Output = Result<WorkflowSupportBuilding, Refusal>> + Send;






}
