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
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Authoring, Refusal};
use ipc::{SaveWorkflow, WireError, WireValue, WorkflowSaved};

use crate::daemon::Fleet;
use crate::repositories::{Served, WorkflowNotSaved};

/// The definition does not fit this repository. **A 422 carrying the loader's
/// own sentence**, and nothing is written.
pub(crate) const WORKFLOW_UNFIT: &str = "fleet.workflow_unfit";
/// The id is not usable as a file's name. A 422.
const WORKFLOW_NOT_A_NAME: &str = "fleet.workflow_id_not_a_name";
/// A definition of this id is already in that scope and `overwrite` was not
/// set. A 422 naming the file, so the caller can say what it would replace.
const WORKFLOW_EXISTS: &str = "fleet.workflow_exists";
/// The folder or the file could not be written. A 500.
pub(crate) const WORKFLOW_UNWRITABLE: &str = "fleet.workflow_unwritable";

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
    pub(crate) fn save_workflow_file(
        &self,
        asked: SaveWorkflow,
        served: &Served,
    ) -> Result<WorkflowSaved, Refusal> {
        let scope = asked.scope;
        self.lifecycle_ends_unnamed(served.manifest().id().as_str(), &asked.definition).map_err(|why| self.not_saved(why))?;
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

    /// Check without writing: what [`Self::save_workflow_file`] would refuse.
    pub(crate) fn check_workflow_file(&self, asked: &SaveWorkflow, served: &Served) -> Result<(), Refusal> {
        self.lifecycle_ends_unnamed(served.manifest().id().as_str(), &asked.definition).map_err(|why| self.not_saved(why))?;
        self.locating()
            .check_workflow(Path::new(served.root()), served.manifest(), asked)
            .map_err(|why| self.not_saved(why))
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

    pub(crate) fn read_workflows_again(&self, served: &Served) {
        let read = self.locating().workflows(
            Path::new(served.root()),
            served.manifest(),
            &served.workspaces(),
        );
        served.catalogued(self.lifecycle_ends_checked(served.manifest().id().as_str(), read));
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

    async fn delete_workflow_copy(
        &self,
        copy: ipc::WorkflowCopy,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowCopyDeleted, Refusal> {
        self.delete_workflow_copy_file(copy, &self.served_named(manifest_id.as_ref())?)
    }

    async fn reset_workflow_copy(
        &self,
        copy: ipc::WorkflowCopy,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<WorkflowSaved, Refusal> {
        self.reset_workflow_copy_file(copy, &self.served_named(manifest_id.as_ref())?)
    }

    async fn mark_not_a_drift(&self, marked: ipc::MarkNotADrift) -> Result<ipc::NotADriftMarked, Refusal> {
        self.nudge_marked(marked).await
    }

    async fn ask_workflow_change(
        self: Arc<Self>,
        asked: ipc::AskWorkflowChange,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowChange, Refusal> {
        self.workflow_change_asked(asked, manifest_id).await
    }

    async fn propose_new_workflow(
        &self,
        proposed: ipc::ProposeNewWorkflow,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowChange, Refusal> {
        self.new_workflow_proposed(proposed, manifest_id).await
    }

    async fn propose_workflow_change(
        &self,
        proposed: ipc::ProposeWorkflowChange,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowChange, Refusal> {
        self.workflow_change_proposed(proposed, manifest_id).await
    }

    async fn suggest_workflow_change(
        self: Arc<Self>,
        asked: ipc::SuggestWorkflowChange,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowChange, Refusal> {
        self.workflow_change_suggested(asked, manifest_id).await
    }

    async fn list_workflow_changes(
        &self,
        workflow_id: ipc::WorkflowId,
        source: String,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowChanges, Refusal> {
        self.workflow_changes_listed(&workflow_id, &source, manifest_id.as_ref()).await
    }

    async fn approve_workflow_change(
        &self,
        change: ipc::WorkflowChangeRef,
    ) -> Result<ipc::WorkflowChangeApproved, Refusal> {
        self.workflow_change_approved(change).await
    }

    async fn dismiss_workflow_change(&self, change: ipc::WorkflowChangeRef) -> Result<ipc::WorkflowChange, Refusal> {
        self.workflow_change_dismissed(change).await
    }

    async fn send_back_workflow_change(
        self: Arc<Self>,
        back: ipc::SendBackWorkflowChange,
    ) -> Result<ipc::WorkflowChange, Refusal> {
        self.workflow_change_sent_back(back).await
    }

    async fn propose_workflow_to_armada(
        self: Arc<Self>,
        proposed: ipc::ProposeToArmada,
    ) -> Result<ipc::ArmadaProposed, Refusal> {
        self.workflow_proposed_to_armada(proposed).await
    }

    async fn build_workflow_support(
        self: Arc<Self>,
        change: ipc::WorkflowChangeRef,
    ) -> Result<ipc::WorkflowSupportBuilding, Refusal> {
        self.workflow_support_built(change).await
    }
}
