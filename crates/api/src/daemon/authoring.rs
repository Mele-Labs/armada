//! Saving a workflow definition. Since 23.28.
//!
//! **Its own surface, and not a method of [`Commands`](super::Commands)**, which
//! was at the line the gate refuses a file over when this arrived. The reason
//! that stands without the line count is the one [`Retros`](super::Retros)
//! gives: this is whole on its own, one act with one body, and a handler that
//! takes only this cannot reach for anything else a command does.

use std::future::Future;

use ipc::{
    AddStep, AddedStep, AddedStepRemoved, JobId, ManifestId, RemoveAddedStep, RemoveTrigger,
    SaveTrigger, SaveWorkflow, TriggerRemoved, TriggerSaved, WorkflowSaved,
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

    /// `add_job_step` — one step added to a Job that is underway, for this Job
    /// only. **Refused where its moment has already come**: a 409
    /// `fleet.added_step_behind` for a gap behind the current step or a Job
    /// that is over, `fleet.added_step_before_approval` where the approval
    /// should carry it, and a 422 `fleet.unacceptable_addition` for nothing to
    /// run or a place the workflow lacks. Since 23.59.
    fn add_job_step(
        &self,
        job_id: JobId,
        add: AddStep,
    ) -> impl Future<Output = Result<AddedStep, Refusal>> + Send;

    /// `remove_job_step` — take an added step off the Job **before it fires**. A
    /// 409 `fleet.added_step_fired` once its moment has come. Since 23.59.
    fn remove_job_step(
        &self,
        job_id: JobId,
        remove: RemoveAddedStep,
    ) -> impl Future<Output = Result<AddedStepRemoved, Refusal>> + Send;
}
