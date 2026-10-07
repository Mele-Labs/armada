//! Saving a workflow definition. Since 23.28.
//!
//! **Its own surface, and not a method of [`Commands`](super::Commands)**, which
//! was at the line the gate refuses a file over when this arrived. The reason
//! that stands without the line count is the one [`Retros`](super::Retros)
//! gives: this is whole on its own, one act with one body, and a handler that
//! takes only this cannot reach for anything else a command does.

use std::future::Future;

use ipc::{
    ChooseTriggerFix, JobId, ManifestId, RemoveTrigger, SaveTrigger, SaveWorkflow,
    TriggerFixChosen, TriggerRemoved, TriggerSaved, WorkflowSaved,
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
}
