//! Saving a workflow definition. Since 23.28.
//!
//! **Its own surface, and not a method of [`Commands`](super::Commands)**, which
//! was at the line the gate refuses a file over when this arrived. The reason
//! that stands without the line count is the one [`Retros`](super::Retros)
//! gives: this is whole on its own, one act with one body, and a handler that
//! takes only this cannot reach for anything else a command does.

use std::future::Future;

use ipc::{
    AddStep, AddedStep, AddedStepRemoved, ChooseTriggerFix, HoldAct, HoldSettled, JobId,
    ManifestId, RemoveAddedStep, RemoveTrigger, SaveTrigger, SaveWorkflow, TriggerFixChosen,
    TriggerRemoved, TriggerSaved, WorkflowSaved,
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

    /// `choose_trigger_fix` — where a failed Trigger's held fix goes. **Fleet
    /// never chooses**: a firing stays `fix_ready` until this is called, and
    /// stays there when the choice cannot be carried out.
    ///
    /// **By `Arc`, for [`Commands::show_again`](super::Commands::show_again)'s
    /// reason**: placing a fix merges, pushes and runs the Command again, and a
    /// client that stops waiting must not stop it halfway.
    ///
    /// [`Refusal::IllegalMove`] where no firing of the Trigger is `fix_ready`,
    /// where the fix could not be placed, where the Job's branch moved and the
    /// fix conflicts, and where every worktree slot is held.
    fn choose_trigger_fix(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        choose: ChooseTriggerFix,
    ) -> impl Future<Output = Result<TriggerFixChosen, Refusal>> + Send;

    /// `rerun_trigger` — run a held Trigger's Command again, with no Drone,
    /// and let the Job go if it passes. **By `Arc`**, for
    /// `choose_trigger_fix`'s reason: it runs the Command, and a client that
    /// stops waiting must not stop it halfway. Since 23.63.
    ///
    /// [`Refusal::IllegalMove`] where nothing holds the Job under that name,
    /// where a repair is under way, where a fix waits on a choice, where there
    /// is nothing Fleet can run, and where the Job's branch could not be
    /// reached. [`Refusal::Unacceptable`] where the body names both or neither.
    fn rerun_trigger(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        act: HoldAct,
    ) -> impl Future<Output = Result<HoldSettled, Refusal>> + Send;

    /// `skip_trigger` — let a held Trigger go without its Command passing, and
    /// record it skipped by the owner. Since 23.63.
    fn skip_trigger(
        &self,
        job_id: JobId,
        act: HoldAct,
    ) -> impl Future<Output = Result<HoldSettled, Refusal>> + Send;

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
