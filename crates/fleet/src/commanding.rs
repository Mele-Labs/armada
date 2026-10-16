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
//!
//! **A trait impl is one block, so the 1200-line rule was met by moving bodies out**:
//! `answering`, `job_settings`, `proposing` and `steering` hold the long ones as inherent
//! methods, and each is one line here.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Commands, Refusal};
use ipc::{
    AddTask, CapRaise, CaptureWalkNote, ChangesRequested, DropTask, FindingDismissed,
    FindingQueued, IssueFiled, JobExamined, JobForgotten, JobId, JobSummary, Overruled,
    ProposeJob, RaisedBy, Redirection, Redispatched, RemarksTakenUp, RemoveWalkNote,
    TurnRaise, WalkNotes, WorkPlan, WorktreeReclaimed,
};

use crate::budget::{budgeted, budgeted_for};
use crate::daemon::Fleet;
use crate::overruling::Overruling;
use crate::reporting::Filed;
use crate::wire::reported;

mod answering;
mod job_settings;
mod proposing;
mod steering;

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
    /// Draft a Job onto the approval gate. **Creation publishes `job.created`**
    /// — not a state change, because a created Job has no status it moved from.
    ///
    /// `by` is the transport's word: `Redirector::Helm` only where the door
    /// placed the call in a Helm session, `#943`'s reason for existing.
    async fn propose_job(
        self: Arc<Self>,
        proposal: ProposeJob,
        by: api::Redirector,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted(self.command_budget(), {
            let fleet = Arc::clone(&self);
            async move { fleet.propose_as(proposal, by).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// `proposing` has it.
    async fn propose_from_request(
        self: Arc<Self>,
        request: ipc::JobRequest,
        manifest_id: Option<ipc::ManifestId>,
        by: api::Redirector,
    ) -> Result<ipc::ProposedPlan, Refusal> {
        self.proposed_from_request(request, manifest_id, by).await
    }

    async fn fix_main(self: Arc<Self>, fix: ipc::FixMain) -> Result<JobSummary, Refusal> {
        self.fixed_main(fix).await
    }

    /// Stop a proposal that is out. **Answers rather than refuses on a
    /// proposal that has gone** — see the trait's own note.
    ///
    /// It touches no store and moves no Job, which is why it is the one command
    /// here that does not end in `summarised`: there is nothing to summarise.
    async fn stop_proposal(
        &self,
        proposal_id: ipc::ProposalId,
    ) -> Result<ipc::ProposalStopped, Refusal> {
        Ok(ipc::ProposalStopped {
            stopped: Fleet::stop_proposal(self, &proposal_id),
        })
    }

    /// Approve, as the person left the proposal where they sent one.
    /// `approving` has it.
    async fn approve_dispatch(
        self: Arc<Self>,
        job_id: JobId,
        left: Option<ipc::ApproveDispatch>,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                match left {
                    Some(left) => fleet.approve_as_left(&job_id.to_domain(), &left).await,
                    None => fleet.approve_as_proposed(&job_id.to_domain()).await,
                }
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// Save a proposal's words without releasing it. `approving` has it.
    async fn edit_job(
        self: Arc<Self>,
        job_id: JobId,
        edit: ipc::EditJob,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { fleet.edit_proposal(&job_id.to_domain(), &edit).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// Approve an Epic's plan and release its wave. `waving` has it.
    async fn approve_wave(
        self: Arc<Self>,
        job_id: JobId,
        wave: ipc::ApproveWave,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::approve_wave(&fleet, &job_id.to_domain(), &wave).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The person takes the work, and the Job goes on or is finished.
    /// **Included even though the last step's answer commits and delivers the
    /// branch inside it.** [`CommandBudget`] is sized to cover an ordinary
    /// local commit and push for that reason.
    async fn approve_review(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::approve_review(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The person merges the work, and then takes it.
    ///
    /// **The merge is Fleet's to perform and never Fleet's to decide.** What
    /// arrives here is a press, and what it buys over merging on the forge is
    /// the Checks that run against the tree the merge left.
    ///
    /// **Not [`budgeted`].** The forge call inside writes into a repository
    /// Fleet did not make, over the network this crate has not measured.
    async fn merge_pull_request(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::merge_pull_request(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// Merge pressed while the forge's checks run. **Not [`budgeted`]**, for
    /// [`Commands::merge_pull_request`]'s reason.
    async fn enable_job_auto_merge(&self, job_id: JobId) -> Result<ipc::PullRequestState, Refusal> {
        Fleet::enable_job_auto_merge(self, &job_id.to_domain()).await
    }

    /// The pull request's failed CI runs, started again. #905. **Not [`budgeted`]**, for
    /// [`Commands::merge_pull_request`]'s reason: the forge write inside is over a network.
    async fn rerun_failed_checks(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::rerun_failed_checks(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The branch, sent back for a Drone to find out why CI failed. #905.
    async fn investigate_failed_checks(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::investigate_failed_checks(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// `steering` has it.
    async fn request_changes(
        self: Arc<Self>,
        job_id: JobId,
        note: ChangesRequested,
    ) -> Result<JobSummary, Refusal> {
        self.changes_requested(job_id, note).await
    }

    /// **Nothing is decoded into a note here**, unlike `request_changes`: the
    /// body carries handles and the words come off the forge inside the act. An
    /// empty list is refused there rather than here, because it is a fact about
    /// the Job's pull request and not about the request's shape.
    ///
    /// **Not [`budgeted`]**, for [`Commands::merge_pull_request`]'s reason: the
    /// forge is read for the comments' own words before a Drone ever sees them.
    async fn take_up_remarks(
        &self,
        job_id: JobId,
        picked: RemarksTakenUp,
    ) -> Result<JobSummary, Refusal> {
        let job = Fleet::take_up_remarks(self, &job_id.to_domain(), &picked.remarks)
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    async fn dismiss_finding(&self, id: JobId, d: FindingDismissed) -> Result<JobSummary, Refusal> {
        self.dismissing(id, d).await
    }

    async fn capture_walk_note(&self, id: JobId, c: CaptureWalkNote) -> Result<WalkNotes, Refusal> {
        self.capturing_walk_note(id, c).await
    }

    async fn remove_walk_note(&self, id: JobId, r: RemoveWalkNote) -> Result<WalkNotes, Refusal> {
        self.removing_walk_note(id, r).await
    }

    async fn walk_failed(
        self: Arc<Self>,
        id: JobId,
        failed: ipc::WalkFailed,
    ) -> Result<ipc::WalkTold, Refusal> {
        self.walk_failed_told(id, failed).await
    }

    async fn queue_after_finding(
        &self,
        id: JobId,
        q: FindingQueued,
    ) -> Result<JobSummary, Refusal> {
        self.queueing_after(id, q).await
    }

    async fn file_finding_issue(&self, id: JobId, f: IssueFiled) -> Result<JobSummary, Refusal> {
        self.filing_issue(id, f).await
    }

    /// The Judge refused, a person disagrees, and the step advances anyway.
    ///
    /// **A blank reason is refused on a refusal and taken on a gaming flag**, and
    /// `Fleet::override_verdict` decides which, because only it reads what
    /// stopped the step. On a refusal an empty string loses the only account of
    /// why the Judge was wrong; a flag's own record already names what was
    /// disagreed with.
    ///
    /// **Included**, for [`Commands::approve_review`]'s reason: the step it
    /// advances may also be the workflow's last.
    async fn override_verdict(
        self: Arc<Self>,
        job_id: JobId,
        overruling: Overruled,
    ) -> Result<JobSummary, Refusal> {
        let said = Overruling::saying(&overruling.reason);
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::override_verdict(&fleet, &job_id.to_domain(), said.as_ref()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The gate could not decide, and a person asks it again on the evidence
    /// already submitted.
    ///
    /// **No reason is taken and none is refused**, which is
    /// `override_verdict`'s rule turned around: that act records why a person
    /// disagreed with a machine, and nothing here is disagreed with. What the
    /// second reading came to is written into the Job's own log by
    /// `crate::regating`, and it says more than a sentence would.
    ///
    /// **Not [`budgeted`].** This asks the Judge again, on its own budget
    /// paired with Bridge's own wait for this route.
    async fn rerun_gate(&self, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::rerun_gate(self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A person runs a failed step's Checks again. **The `Arc` is handed on**,
    /// so the run is a task of its own — `crate::rechecking`.
    ///
    /// **Not [`budgeted`].** The request waits for the Checks, however long
    /// they take.
    async fn rerun_checks(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::rerun_checks(Arc::clone(&self), &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A person asking a Job to show its work. **The `Arc` is handed on**, so
    /// the press runs on a task of its own — `crate::showing_again`.
    ///
    /// **Not [`budgeted`].** The request waits for the whole run, however long
    /// the repository's harness takes; Bridge sends `NO_WAIT` on this route.
    async fn show_again(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        picked: Option<String>,
    ) -> Result<ipc::ShownAgain, Refusal> {
        let refusing = std::sync::Arc::clone(&self);
        Fleet::show_again(self, &job_id.to_domain(), picked.as_deref())
            .await
            .map_err(|why| refusing.refusal(why))
    }

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
    /// **Not [`budgeted`]**, for [`Commands::start_run`]'s reason: a server
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

    /// Take back a rule a person always-allowed for this Manifest's
    /// repository — `crate::permitting::repository`. **Not `budgeted`**, for
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

    /// **Not [`budgeted`]**, for [`Commands::start_run`]'s reason.
    async fn stop_server(&self, named: ipc::NamedServer) -> Result<ipc::ServerState, Refusal> {
        self.stopped_server(&named.id)
            .await
            .map_err(|why| self.server_refusal(why, None))
    }

    /// More money for one Job, and nothing else changes.
    ///
    /// **The summary is re-read rather than folded from the raise.** Nothing
    /// moved, so the row a caller wants back is the row as it now stands — and
    /// the field it wants is `queued_reason`, which `summarised` computes from
    /// the board through the same predicate admission asks. A raise that reports
    /// its own success would be a second answer to *is this Job still held*.
    async fn raise_cost_cap(
        self: Arc<Self>,
        job_id: JobId,
        raise: CapRaise,
        by: RaisedBy,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::raise_cost_cap(&fleet, &job_id.to_domain(), &raise, by).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// More turns for one Job, and nothing else changes. The method above's
    /// shape and its reasons, on the other ceiling.
    async fn raise_turn_cap(
        self: Arc<Self>,
        job_id: JobId,
        raise: TurnRaise,
        by: RaisedBy,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::raise_turn_cap(&fleet, &job_id.to_domain(), &raise, by).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A verdict on the work, and the Job is over.
    async fn reject_job(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::reject(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The process, not the unit of work.
    async fn kill_drone(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::kill_drone(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
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

    /// Pause and resume: `crate::pausing`.
    async fn park_job(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        Fleet::pause_answered(self, job_id).await
    }

    async fn resume_job(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        Fleet::resume_answered(self, job_id).await
    }

    /// One Drone of the Job, by id: `crate::one_drone`.
    async fn kill_one_drone(
        self: Arc<Self>,
        job_id: JobId,
        drone_id: ipc::DroneId,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            let drone = drone_id.to_domain();
            async move { Fleet::kill_one_drone(&fleet, &job_id.to_domain(), &drone).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A person's hand on a running gate: `crate::check_acts`.
    async fn retry_check(
        self: Arc<Self>,
        job_id: JobId,
        named: ipc::CheckNamed,
    ) -> Result<JobSummary, Refusal> {
        let job = Fleet::retry_check(&self, &job_id.to_domain(), &named.check)
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    async fn skip_check(
        self: Arc<Self>,
        job_id: JobId,
        named: ipc::CheckNamed,
    ) -> Result<JobSummary, Refusal> {
        let job = Fleet::skip_check(&self, &job_id.to_domain(), &named.check)
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    async fn skip_checks(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = Fleet::skip_checks(&self, &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// `steering` has it.
    async fn redirect_one_drone(
        self: Arc<Self>,
        job_id: JobId,
        drone_id: ipc::DroneId,
        instruction: Redirection,
        by: api::Steerer,
    ) -> Result<JobSummary, Refusal> {
        self.one_drone_redirected(job_id, drone_id, instruction, by).await
    }

    /// One process and what it started; the Drone's own pid is `kill_drone`.
    async fn kill_process(self: Arc<Self>, job_id: JobId, pid: u32) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::kill_process(&fleet, &job_id.to_domain(), pid).await }
        })
        .await
        .map_err(|why| self.killing_refusal(why))?;
        self.summarised(&job).await
    }

    /// Every process: `kill_drone`, and what its group signal misses.
    async fn kill_processes(self: Arc<Self>, job_id: JobId) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::kill_processes(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// Go and look at this Job now. **The one act here that moves nothing** —
    /// what it leaves is a line in the Job's own log saying somebody asked and
    /// what was found, which is why it is a command rather than a read.
    ///
    /// **The redaction is `crate::resources`'s.** A process crosses as its
    /// executable's name and never its argument vector, which carries absolute
    /// paths and whatever a Check was invoked with.
    async fn examine_job(self: Arc<Self>, job_id: JobId) -> Result<JobExamined, Refusal> {
        budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let job = fleet.load(&job_id.to_domain()).await?;
                fleet.examined(&job).await
            }
        })
        .await
        .map_err(|why| self.refusal(why))
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

    /// **Two Jobs, redacted separately.** The failed one is now `killed`; the
    /// replacement carries `redispatched_from` and is what the caller opens
    /// next.
    async fn redispatch_job(self: Arc<Self>, job_id: JobId) -> Result<Redispatched, Refusal> {
        let both = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { fleet.redispatch(&job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        Ok(Redispatched {
            replaced: self.summarised(&both.replaced).await?,
            dispatched: self.summarised(&both.dispatched).await?,
        })
    }

    /// `steering` has it.
    async fn redirect_drone(
        self: Arc<Self>,
        job_id: JobId,
        instruction: Redirection,
        by: api::Steerer,
    ) -> Result<JobSummary, Refusal> {
        self.drone_redirected(job_id, instruction, by).await
    }

    /// `steering` has it.
    async fn restart_step(
        self: Arc<Self>,
        job_id: JobId,
        note: Option<ipc::RestartRequested>,
        by: api::Steerer,
    ) -> Result<JobSummary, Refusal> {
        self.step_restarted(job_id, note, by).await
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

    /// `answering` has it.
    async fn answer_question(
        self: Arc<Self>,
        job_id: JobId,
        answer: ipc::ChosenAnswer,
    ) -> Result<JobSummary, Refusal> {
        self.question_answered(job_id, answer).await
    }

    /// `answering` has it.
    async fn answer_command(
        self: Arc<Self>,
        job_id: JobId,
        answer: ipc::AnswerCommand,
    ) -> Result<JobSummary, Refusal> {
        self.command_answered(job_id, answer).await
    }

    /// `job_settings` has it.
    async fn set_when_blocked(
        self: Arc<Self>,
        job_id: JobId,
        setting: ipc::SetWhenBlocked,
    ) -> Result<JobSummary, Refusal> {
        self.when_blocked_set(job_id, setting).await
    }

    /// `answering` has it.
    async fn answer_judge(
        self: Arc<Self>,
        job_id: JobId,
        answered: ipc::JudgeAnswered,
    ) -> Result<JobSummary, Refusal> {
        self.judge_answered(job_id, answered).await
    }

    /// `job_settings` has it.
    async fn set_when_refused(
        self: Arc<Self>,
        job_id: JobId,
        setting: ipc::SetWhenRefused,
    ) -> Result<JobSummary, Refusal> {
        self.when_refused_set(job_id, setting).await
    }

    /// `job_settings` has it.
    async fn set_model(
        self: Arc<Self>,
        job_id: JobId,
        choice: ipc::SetModel,
    ) -> Result<JobSummary, Refusal> {
        self.model_set(job_id, choice).await
    }

    /// `job_settings` has it.
    async fn set_review_model(
        self: Arc<Self>,
        job_id: JobId,
        choice: ipc::SetModel,
    ) -> Result<JobSummary, Refusal> {
        self.review_model_set(job_id, choice).await
    }

    /// The model and effort this Job's Commander runs on. `job_settings` has it. #2166.
    async fn set_commander_model(
        self: Arc<Self>,
        job_id: JobId,
        choice: ipc::SetCommanderModel,
    ) -> Result<JobSummary, Refusal> {
        self.commander_model_chosen(job_id, choice).await
    }

    /// Which model each tier of this Job's tasks runs on. `task_edits` has it.
    async fn set_tiers(
        self: Arc<Self>,
        job: JobId,
        body: ipc::SetTiers,
    ) -> Result<JobSummary, Refusal> {
        Fleet::set_tiers_by_person(self, job, body).await
    }

    /// A proposal sent back to the proposer with a note. `reconsidering` has it.
    async fn to_proposer(
        self: Arc<Self>,
        job: JobId,
        note: ipc::ToProposer,
    ) -> Result<ipc::ProposedPlan, Refusal> {
        Fleet::to_proposer_by_person(self, job, note).await
    }

    /// Where an approved Job lands, set once. `aiming` has it.
    async fn set_landing_target(
        self: Arc<Self>,
        job: JobId,
        body: ipc::SetLandingTarget,
    ) -> Result<JobSummary, Refusal> {
        Fleet::set_landing_target_by_person(self, job, body).await
    }

    /// `job_settings` has it.
    async fn remove_allowed_command(
        self: Arc<Self>,
        job_id: JobId,
        removing: ipc::RemoveAllowedCommand,
    ) -> Result<JobSummary, Refusal> {
        self.allowed_command_removed(job_id, removing).await
    }

    /// A task added to the Job's plan, signed as `by`. `#897`; `work_plan` has it.
    async fn add_task(
        self: Arc<Self>,
        job: JobId,
        add: AddTask,
        by: api::Steerer,
    ) -> Result<WorkPlan, Refusal> {
        Fleet::add_task_by(self, job, add, by).await
    }

    /// A task dropped, with a reason, signed as `by`. `#897`; `work_plan` has it.
    async fn drop_task(
        self: Arc<Self>,
        job: JobId,
        body: DropTask,
        by: api::Steerer,
    ) -> Result<WorkPlan, Refusal> {
        Fleet::drop_task_by(self, job, body, by).await
    }

    /// Restart this task. `#1656`; `plan_acts` has it.
    async fn restart_task(
        self: Arc<Self>,
        job: JobId,
        task: String,
        body: ipc::RestartTask,
        by: api::Steerer,
    ) -> Result<JobSummary, Refusal> {
        let restarted = Fleet::restart_task_by(Arc::clone(&self), job, task, body, by).await?;
        self.summarised(&restarted).await
    }

    /// A task or a group moved, signed as `by`. `#1685`; `plan_acts` has it.
    async fn move_plan(
        self: Arc<Self>,
        job: JobId,
        body: ipc::MovePlan,
        by: api::Steerer,
    ) -> Result<WorkPlan, Refusal> {
        Fleet::move_plan_by(self, job, body, by).await
    }

    /// Edit this task. `#1657`; `task_edits` has it.
    async fn edit_task(
        self: Arc<Self>,
        job: JobId,
        task: String,
        body: ipc::EditTask,
        by: api::Steerer,
    ) -> Result<WorkPlan, Refusal> {
        Fleet::edit_task_by(self, job, task, body, by).await
    }
}
