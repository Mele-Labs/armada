//! `api::Commands`, implemented over a real Fleet: every act a person takes.
//!
//! **The write half of the seam `serving` holds the read half of**, split out
//! because `api::Daemon` is three traits and Rust takes one impl block per
//! trait. Nothing here decides: each method converts the request, calls the
//! `Fleet` method that moves it, and maps a refusal through `Fleet::refusal`.
//!
//! `#712`: most commands race their work against [`CommandBudget`] through
//! [`budgeted`](crate::budget::budgeted). **The race itself moved to
//! `crate::budget` at the 900-line refusal, `#897`** — a method added here is
//! a delegating line into whichever module already holds its Job's logic.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Commands, Refusal};
use ipc::{JobForgotten, JobId, JobSummary, WorktreeReclaimed};

use crate::budget::budgeted_for;
use crate::daemon::Fleet;
use crate::reporting::Filed;
use crate::wire::reported;

impl<H, V, W> Commands for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// A corrected Manifest, written and nothing more —
    /// [`editing`](mod@crate::editing). Where a form's edit is pending, the
    /// text goes into its pull request instead.
    ///
    /// **Not `Arc`, because nothing outlives the call.** The bytes are on disk
    /// when this answers; what the re-read makes of them is the watch's, and
    /// arrives as `manifest.reread`.
    async fn save_manifest_file(
        &self,
        asked: ipc::SaveManifestFile,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ManifestSaved, Refusal> {
        self.write_manifest_file(asked, &self.served_named(manifest_id.as_ref())?)
            .await
    }

    /// A form's edits, placed and sent — [`amending`](mod@crate::amending).
    ///
    /// **Not `Arc`**, for [`Commands::save_manifest_file`]'s reason: the edit
    /// is on disk or on the remote when this answers.
    async fn edit_manifest(
        &self,
        asked: ipc::EditManifest,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ManifestEdited, Refusal> {
        self.edit_manifest_file(asked, &self.served_named(manifest_id.as_ref())?)
            .await
    }

    /// One edit to a Setup proposal. Held in memory, so nothing is written.
    async fn edit_manifest_proposal(
        &self,
        asked: ipc::EditManifestProposal,
        repository: Option<String>,
    ) -> Result<ipc::ManifestProposal, Refusal> {
        self.edit_manifest_proposal(
            asked,
            self.repository_named(repository.as_deref())?.as_ref(),
        )
    }

    /// A Setup proposal, created on disk and never over a file that is there.
    async fn write_manifest_proposal(
        &self,
        asked: ipc::WriteManifestProposal,
        repository: Option<String>,
    ) -> Result<ipc::ManifestProposal, Refusal> {
        self.write_manifest_proposal(
            asked,
            self.repository_named(repository.as_deref())?.as_ref(),
        )
    }

    /// A person starting a server, for a Job or the main checkout. **The `Arc`
    /// is handed on**, so the server is a task of its own — `crate::servers`.
    ///
    /// **Not [`budgeted`]**, for the reason `save_limits` states: a server
    /// answers through its own refusal.
    async fn start_server(
        self: std::sync::Arc<Self>,
        asked: ipc::StartServer,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ServerState, Refusal> {
        Fleet::start_server_answered(self, asked, manifest_id).await
    }

    /// A restart onto `main` or the preview, started detached —
    /// [`crate::building`]. **Not `budgeted`**, `save_limits`' reason: no Job is
    /// touched, and the answer is that the wrapper has started.
    async fn change_fleet_build(
        &self,
        asked: ipc::ChangeFleetBuild,
    ) -> Result<ipc::FleetBuildChanging, Refusal> {
        self.change_fleet_build_now(&asked)
    }

    /// The one pull a held checkout is offered — `crate::catching_up`. **Not
    /// `budgeted`**, `change_fleet_build`'s reason: no Job is touched.
    async fn catch_up_checkout(
        &self,
        asked: ipc::CatchUpCheckout,
    ) -> Result<ipc::CheckoutCaughtUp, Refusal> {
        self.caught_up_checkout(asked).await
    }

    /// A held checkout's files sent up as a pull request —
    /// `crate::catching_up`. **Not `budgeted`**, for the same reason.
    async fn checkout_to_pull_request(
        &self,
        asked: ipc::CheckoutToPullRequest,
    ) -> Result<ipc::CheckoutSentAsPullRequest, Refusal> {
        self.checkout_sent_as_pull_request(asked).await
    }

    /// Take back a rule a person always-allowed for this Manifest's
    /// repository — `crate::allowed::repository`. **Not `budgeted`**, for
    /// `save_limits`' reason: no Job is touched.
    async fn remove_repository_allowed_command(
        &self,
        removing: ipc::RemoveRepositoryAllowedCommand,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::RepositoryAllowedCommands, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        Fleet::remove_repository_allowed_command(self, &served, &removing.run)
            .await
            .map_err(|why| self.repository_allow_refusal(why))?;
        Ok(ipc::RepositoryAllowedCommands {
            commands: self
                .repository_allowed(&served)
                .await
                .iter()
                .map(ipc::AllowedCommandRow::from)
                .collect(),
        })
    }

    /// Put an MCP server in Kit — `crate::kit`. It reaches no Drone until a
    /// person says so on one of the two acts below.
    async fn add_kit_server(
        &self,
        adding: ipc::AddKitServer,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::KitServers, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        Fleet::add_kit_server(self, adding)
            .await
            .map_err(|why| self.kit_refusal(why))?;
        Ok(self.kit_servers_listed(&served).await)
    }

    /// Take one out of Kit, and every Manifest's word about it with it.
    async fn forget_kit_server(
        &self,
        forgetting: ipc::ForgetKitServer,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::KitServers, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        Fleet::forget_kit_server(self, forgetting)
            .await
            .map_err(|why| self.kit_refusal(why))?;
        Ok(self.kit_servers_listed(&served).await)
    }

    /// Kit's own tier for one server.
    async fn set_kit_server_reach(
        &self,
        setting: ipc::SetKitServerReach,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::KitServers, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        Fleet::set_kit_server_reach(self, setting)
            .await
            .map_err(|why| self.kit_refusal(why))?;
        Ok(self.kit_servers_listed(&served).await)
    }

    /// This Manifest's own word over one Kit server.
    async fn set_manifest_server_reach(
        &self,
        setting: ipc::SetManifestServerReach,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::KitServers, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        Fleet::set_manifest_server_reach(self, &served, setting)
            .await
            .map_err(|why| self.kit_refusal(why))?;
        Ok(self.kit_servers_listed(&served).await)
    }

    /// Take a command out of Kit's allowlist: `crate::kit_allowlist`.
    async fn remove_kit_allowed_command(
        &self,
        removing: ipc::RemoveKitAllowedCommand,
    ) -> Result<ipc::KitAllowedCommands, Refusal> {
        self.remove_from_kit_allowlist(&removing.run).await
    }

    /// Serve one more repository, from a folder — `crate::repositories`.
    async fn add_repository(
        &self,
        asked: ipc::AddRepository,
    ) -> Result<ipc::RepositorySummary, Refusal> {
        self.added_repository(asked).await
    }

    /// Clone from a URL, then serve it — `crate::repositories`.
    async fn clone_repository(
        self: Arc<Self>,
        asked: ipc::CloneRepository,
    ) -> Result<ipc::RepositorySummary, Refusal> {
        self.cloned_repository(asked).await
    }

    /// **Not [`budgeted`]**, for the reason `save_limits` states.
    async fn stop_server(&self, named: ipc::NamedServer) -> Result<ipc::ServerState, Refusal> {
        self.stopped_server(&named.id)
            .await
            .map_err(|why| self.server_refusal(why, None))
    }

    /// The unit of work, not the process.
    async fn kill_job(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::kill_job(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The record, gone. **Nothing is redacted here** — there is no Job left
    /// to redact, only the id it used to name.
    async fn forget_job(self: Arc<Self>, job_id: JobId) -> Result<JobForgotten, Refusal> {
        budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            let id = job_id.clone();
            async move { Fleet::forget_job(&fleet, &id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        Ok(JobForgotten { job_id })
    }

    /// The disk, given back — `crate::reclaiming`, which says why nothing is redacted.
    async fn reclaim_worktree(
        self: Arc<Self>,
        job_id: JobId,
    ) -> Result<WorktreeReclaimed, Refusal> {
        let reclaimed = Fleet::reclaim_answered(Arc::clone(&self), job_id).await?;
        self.worktrees_changed().await;
        Ok(reclaimed)
    }

    /// The pool reshaped on this machine — `crate::leasing`.
    async fn change_slot_pool(
        &self,
        change: ipc::ChangeSlotPool,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::SlotPoolChanged, Refusal> {
        let changed = self.slot_pool_changed(change, manifest_id.as_ref()).await?;
        self.worktrees_changed().await;
        Ok(changed)
    }

    /// A stranded slot read, scrapped or stashed — `crate::rescuing`.
    async fn rescue_slot(
        self: Arc<Self>,
        asked: ipc::RescueSlot,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::SlotRescued, Refusal> {
        let rescued = Fleet::rescue_slot(Arc::clone(&self), asked, manifest_id.as_ref()).await?;
        self.worktrees_changed().await;
        Ok(rescued)
    }

    async fn delete_branch(
        self: Arc<Self>,
        job_id: JobId,
        asked: ipc::DeleteBranch,
    ) -> Result<ipc::BranchDeleted, Refusal> {
        Fleet::delete_branch_answered(self, job_id, asked).await
    }

    /// A person says this Job failed in error, and the record is filed with
    /// what they said.
    ///
    /// **Everything here is in `crate::reporting`**, including the emptiness and
    /// the two refusals it has: `crate::reporting::NotFiled` names each cause
    /// and says it, and this only carries one out as the 422 it is.
    async fn file_report(
        self: Arc<Self>,
        job_id: JobId,
        filing: ipc::FileReport,
    ) -> Result<ipc::Report, Refusal> {
        let id = job_id.to_domain();
        let filed = Filed::saying(
            filing.claim,
            &filing.said,
            filing.step_id,
            filing.criterion_id,
        )
        .map_err(|cause| self.refusal(cause.about(&id)))?;
        let filed = budgeted_for(self.command_budget(), job_id, {
            let fleet = Arc::clone(&self);
            async move { Fleet::file_report(&fleet, &id, &filed).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        reported(&filed).map_err(|why| self.refusal(why))
    }

}
