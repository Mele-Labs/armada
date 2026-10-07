//! Saving a workflow definition, and reading the folders again when one moves.
//!
//! **What a save is, to Fleet**: the checks and the write are the composition
//! root's, behind `Locating::save_workflow`, because that is where the roster
//! and Kit's folder are known. Fleet asks, turns a refusal into the wire's
//! words, and then reads the folders again so the definition is held the moment
//! the answer arrives rather than at the next poll.
//!
//! **The same re-read serves a file somebody edited by hand.** The watch lives
//! above this crate, and calls [`Fleet::workflows_changed`] once a burst of
//! writes has settled.

use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Authoring, Refusal};
use ipc::{SaveWorkflow, WireError, WireValue, WorkflowSaved};

use crate::daemon::Fleet;
use crate::repositories::{Served, WorkflowNotSaved};

/// The definition does not fit this repository. **A 422 carrying the loader's
/// own sentence**, and nothing is written.
const WORKFLOW_UNFIT: &str = "fleet.workflow_unfit";
/// The id is not usable as a file's name. A 422.
const WORKFLOW_NOT_A_NAME: &str = "fleet.workflow_id_not_a_name";
/// A definition of this id is already in that scope and `overwrite` was not
/// set. A 422 naming the file, so the caller can say what it would replace.
const WORKFLOW_EXISTS: &str = "fleet.workflow_exists";
/// The folder or the file could not be written. A 500.
const WORKFLOW_UNWRITABLE: &str = "fleet.workflow_unwritable";

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Check, write, and hold.
    fn save_workflow_file(
        &self,
        asked: SaveWorkflow,
        served: &Served,
    ) -> Result<WorkflowSaved, Refusal> {
        let scope = asked.scope;
        let saved = self
            .locating()
            .save_workflow(Path::new(served.root()), served.manifest(), &asked)
            .map_err(|why| self.not_saved(why))?;
        self.read_workflows_again(served);
        let runs_from = served
            .workflows()
            .get(&saved.id)
            .map(|held| held.source().as_wire().to_string());
        Ok(WorkflowSaved {
            workflow_id: ipc::WorkflowId::from(&saved.id),
            scope,
            file: saved.file,
            replaced: saved.replaced,
            runs_from,
        })
    }

    /// The workflow folders of the repository at `root` were written to: read
    /// them again, and hold what fits. A root no longer served is nothing.
    ///
    /// `pub` for the watch, which is the composition root's.
    pub fn workflows_changed(&self, root: &str) {
        if let Some(served) = self
            .repositories()
            .served()
            .into_iter()
            .find(|one| one.root() == root)
        {
            self.read_workflows_again(&served);
        }
    }

    fn read_workflows_again(&self, served: &Served) {
        let read = self.locating().workflows(
            Path::new(served.root()),
            served.manifest(),
            &served.workspaces(),
        );
        served.catalogued(read);
    }

    fn not_saved(&self, why: WorkflowNotSaved) -> Refusal {
        match why {
            WorkflowNotSaved::Unfit { why } => Refusal::Unacceptable(
                WireError::raised(WORKFLOW_UNFIT, why.clone(), self.run_id())
                    .with_field("reason", WireValue::Str(why)),
            ),
            WorkflowNotSaved::NotAName { id } => Refusal::Unacceptable(
                WireError::raised(
                    WORKFLOW_NOT_A_NAME,
                    format!(
                        "`{id}` cannot be a file's name: a workflow_id here is letters, digits, \
                         `-` and `_`"
                    ),
                    self.run_id(),
                )
                .with_field("workflow_id", WireValue::Str(id)),
            ),
            WorkflowNotSaved::Exists { id, file } => Refusal::Unacceptable(
                WireError::raised(
                    WORKFLOW_EXISTS,
                    format!(
                        "{file} already defines `{id}` and nothing was written; save again with \
                         overwrite to replace it"
                    ),
                    self.run_id(),
                )
                .with_field("workflow_id", WireValue::Str(id))
                .with_field("file", WireValue::Str(file)),
            ),
            WorkflowNotSaved::Unwritable { file, cause } => Refusal::Fault(
                WireError::raised(
                    WORKFLOW_UNWRITABLE,
                    format!("{file} could not be written: {cause}"),
                    self.run_id(),
                )
                .with_field("file", WireValue::Str(file)),
            ),
        }
    }
}

impl<H, V, W> Authoring for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// **Not `Arc`**, for `Commands::save_manifest_file`'s reason: the file is on
    /// disk, and held, when this answers.
    async fn save_workflow(
        &self,
        asked: SaveWorkflow,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<WorkflowSaved, Refusal> {
        self.save_workflow_file(asked, &self.served_named(manifest_id.as_ref())?)
    }

    async fn choose_trigger_fix(
        self: std::sync::Arc<Self>,
        job_id: ipc::JobId,
        choose: ipc::ChooseTriggerFix,
    ) -> Result<ipc::TriggerFixChosen, Refusal> {
        self.fix_chosen(job_id, choose).await
    }

    async fn rerun_trigger(
        self: std::sync::Arc<Self>,
        job_id: ipc::JobId,
        act: ipc::HoldAct,
    ) -> Result<ipc::HoldSettled, Refusal> {
        self.hold_rerun(job_id, act).await
    }

    async fn skip_trigger(
        &self,
        job_id: ipc::JobId,
        act: ipc::HoldAct,
    ) -> Result<ipc::HoldSettled, Refusal> {
        self.hold_skip(job_id, act).await
    }

    async fn save_trigger(
        &self,
        asked: ipc::SaveTrigger,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::TriggerSaved, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        // Asked before the file is written, so a refusal leaves nothing behind.
        let kept = match &asked.kept_from {
            Some(from) => Some(self.addition_to_keep(from).await?),
            None => None,
        };
        let saved = self.save_trigger_file(asked, &served)?;
        if let Some((job, added)) = kept {
            self.addition_kept(&job, &added, saved.scope).await;
        }
        Ok(saved)
    }

    async fn add_job_step(
        &self,
        job_id: ipc::JobId,
        add: ipc::AddStep,
    ) -> Result<ipc::AddedStep, Refusal> {
        self.step_added(&job_id.to_domain(), &add).await
    }

    async fn remove_job_step(
        &self,
        job_id: ipc::JobId,
        remove: ipc::RemoveAddedStep,
    ) -> Result<ipc::AddedStepRemoved, Refusal> {
        self.step_removed(&job_id.to_domain(), &remove.id).await
    }

    async fn remove_trigger(
        &self,
        asked: ipc::RemoveTrigger,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::TriggerRemoved, Refusal> {
        self.remove_trigger_file(asked, &self.served_named(manifest_id.as_ref())?)
    }
}
