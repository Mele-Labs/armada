//! Saving a workflow definition. Since 23.28.
//!
//! **Its own surface, and not a method of [`Commands`](super::Commands)**, which
//! was at the line the gate refuses a file over when this arrived. The reason
//! that stands without the line count is the one [`Retros`](super::Retros)
//! gives: this is whole on its own, one act with one body, and a handler that
//! takes only this cannot reach for anything else a command does.

use std::future::Future;

use ipc::{
    ManifestId, RemoveTrigger, SaveTrigger, SaveWorkflow, TriggerRemoved, TriggerSaved,
    WorkflowSaved,
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
}
