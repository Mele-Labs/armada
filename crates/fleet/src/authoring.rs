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
use core_model::ScopeId;
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
/// A project's copy was asked for a project Fleet holds none of. A 404.
pub(crate) const WORKFLOW_PROJECT_UNKNOWN: &str = "fleet.workflow_project_unknown";
/// A project's copy was asked for a project that does not hold the repository, so the repository
/// would never run it. A 422.
pub(crate) const WORKFLOW_PROJECT_NOT_MEMBER: &str = "fleet.workflow_project_not_member";

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
        self.workflow_scope_held(&asked.scope, served)?;
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
            scope: asked.scope,
            file: saved.file,
            replaced: saved.replaced,
            runs_from,
        })
    }

    /// Check without writing: what [`Self::save_workflow_file`] would refuse.
    pub(crate) fn check_workflow_file(&self, asked: &SaveWorkflow, served: &Served) -> Result<(), Refusal> {
        self.workflow_scope_held(&asked.scope, served)?;
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

    /// A project's copy is for a project Fleet holds, and one that holds this repository. **Checked
    /// where it is read**, so a copy is never written that this repository would not run.
    pub(crate) fn workflow_scope_held(&self, scope: &ipc::WorkflowScope, served: &Served) -> Result<(), Refusal> {
        let Some(id) = scope.project() else { return Ok(()) };
        let held = self.projects_held();
        let Some(project) = ScopeId::new(id).ok().and_then(|scope| held.get(&scope).cloned()) else {
            return Err(Refusal::NotFound(
                WireError::raised(WORKFLOW_PROJECT_UNKNOWN, format!("Fleet holds no project {id}, so no copy is kept for it"), self.run_id())
                    .with_field("project", WireValue::Str(id.to_string())),
            ));
        };
        if !project.holds(served.manifest().id()) {
            return Err(Refusal::Unacceptable(
                WireError::raised(
                    WORKFLOW_PROJECT_NOT_MEMBER,
                    format!(
                        "this repository is not a member of the project {}, so a copy kept for it would not run here",
                        project.name().as_str()
                    ),
                    self.run_id(),
                )
                .with_field("project", WireValue::Str(id.to_string())),
            ));
        }
        Ok(())
    }

    /// The projects that hold this repository, whose copies of a workflow it runs.
    pub(crate) fn projects_of(&self, served: &Served) -> Vec<ScopeId> {
        self.projects_held().holding(served.manifest().id()).into_iter().map(|project| project.id().clone()).collect()
    }

    /// [`Self::read_workflows_again`] for a repository whose projects changed, **taken only where a project's copy
    /// is held now or was before**: a project that keeps no workflow changes nothing the repository runs.
    pub(crate) fn project_copies_read_again(&self, served: &Served) {
        let project = |source: &str| source == core_model::WorkflowSource::Project.as_wire();
        let read = self.locating().workflows(
            Path::new(served.root()),
            served.manifest(),
            &served.workspaces(),
            &self.projects_of(served),
        );
        let held = served.files().iter().any(|file| file.source() == core_model::WorkflowSource::Project)
            || served.left_out().iter().any(|one| project(&one.source));
        let reads = read.files.iter().any(|file| file.source() == core_model::WorkflowSource::Project)
            || read.left_out.iter().any(|one| project(&one.source));
        if held || reads {
            served.catalogued(self.lifecycle_ends_checked(served.manifest().id().as_str(), read));
            self.events().publish(ipc::Event::RepositoriesChanged(self.repository_list()));
        }
    }

    pub(crate) fn read_workflows_again(&self, served: &Served) {
        let read = self.locating().workflows(
            Path::new(served.root()),
            served.manifest(),
            &served.workspaces(),
            &self.projects_of(served),
        );
        served.catalogued(self.lifecycle_ends_checked(served.manifest().id().as_str(), read));
        // Bridge reads its workflow pickers again on the list, so a workflow saved to the Kit or a repository is offered at once.
        self.events().publish(ipc::Event::RepositoriesChanged(self.repository_list()));
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

    async fn list_workflow_changes(
        &self,
        workflow_id: ipc::WorkflowId,
        source: String,
        project: Option<String>,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowChanges, Refusal> {
        self.workflow_changes_listed(&workflow_id, &source, project.as_deref(), manifest_id.as_ref()).await
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

    async fn get_workflow_draft(
        &self,
        workflow_id: ipc::WorkflowId,
        source: String,
        project: Option<String>,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowDraft, Refusal> {
        self.workflow_draft_read(&workflow_id, &source, project.as_deref(), &self.served_named(manifest_id.as_ref())?).await
    }

    async fn put_workflow_draft(
        &self,
        put: ipc::PutWorkflowDraft,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowDraft, Refusal> {
        self.workflow_draft_put(put, &self.served_named(manifest_id.as_ref())?).await
    }

    async fn discard_workflow_draft(
        &self,
        workflow_id: ipc::WorkflowId,
        source: String,
        project: Option<String>,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowDraftDiscarded, Refusal> {
        self.workflow_draft_discarded(workflow_id, source, project, &self.served_named(manifest_id.as_ref())?).await
    }

    async fn publish_workflow_draft(
        &self,
        publish: ipc::PublishWorkflowDraft,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowPublished, Refusal> {
        self.workflow_draft_published(publish, &self.served_named(manifest_id.as_ref())?).await
    }
}
