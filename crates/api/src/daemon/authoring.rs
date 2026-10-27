//! Saving a workflow definition. Since 23.28.
//!
//! **Its own surface, and not a method of [`Commands`](super::Commands)**, which
//! was at the line the gate refuses a file over when this arrived. The reason
//! that stands without the line count is the one [`Retros`](super::Retros)
//! gives: this is whole on its own, one act with one body, and a handler that
//! takes only this cannot reach for anything else a command does.

use std::future::Future;

use ipc::{
    ManifestId, MarkNotADrift, NotADriftMarked, RemoveTrigger, SaveTrigger, SaveWorkflow,
    TriggerRemoved, TriggerSaved, WorkflowCopy, WorkflowCopyDeleted, WorkflowSaved,
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

    /// `save_trigger` — check one Trigger against the repository's Commands and
    /// write it in the scope named. **Checked before anything is written**, with
    /// the loader's rules. A repository's is refused for a Command it does not
    /// declare; a machine's is not, since the same file is right in the next
    /// repository, and the answer says where it would be skipped.
    fn save_trigger(
        &self,
        save: SaveTrigger,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<TriggerSaved, Refusal>> + Send;

    /// `remove_trigger` — delete the file in a scope that holds one identity.
    fn remove_trigger(
        &self,
        remove: RemoveTrigger,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<TriggerRemoved, Refusal>> + Send;






}
