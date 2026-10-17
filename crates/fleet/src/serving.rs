//! `api::Queries`, implemented over a real Fleet: every read a client makes.
//!
//! **The dependency points this way and never back.** The traits are stated in
//! `api`, where the transport is, and implemented here. `cargo tree -p api`
//! names no `fleet`, the daemon core is drivable with no socket and no process,
//! and `api`'s own tests were written against a fake before this existed.
//!
//! **This is the redaction, and it is a visible step.** Every signature below
//! speaks `ipc` DTOs, `JobSummary::of` is called here by hand, and a field added
//! to `core_model::Job` reaches the wire only when somebody writes the line that
//! puts it there — `api` never sees a domain type.
//!
//! **One of three files, because `api::Daemon` is three traits** — #434. The
//! writes are [`commanding`](mod@crate::commanding)'s and the tools are
//! [`tooling`](mod@crate::tooling)'s, each whole, so no file delegates.
//!
//! **Which refusal a failure is, and the code it carries, is
//! [`refusing`](mod@crate::refusing)'s** — every `WireError` below is raised
//! through `Fleet::refusal`.
//!
//! **The reason costs a second read, and is not derived.** `JobSummary` carries
//! the reason its last transition stored, which is in `job_events` and not on
//! the `jobs` row, so each summary below reads the Job's log for it. N reads for
//! N Jobs is the honest shape at M1: a status-to-reason mapping here would be a
//! second vocabulary that agrees with the log only until something changes.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{ObservedCheckoutRun, Queries, Refusal, Resolved};
use core_model::JobReference;
use ipc::{
    AlertList, FleetHealth,
    FleetUsage, JobDetail, JobDiff, JobHistory, JobId, JobList, ManifestConfig, ManifestDrift, ManifestFile, ManifestId, ManifestReading,
    ManifestSummary, ModelChoices, Work, WorkflowSummary, WorktreesHeld,
};
use store::{LoadJobError, ResolveJobError};

/// One Job in full. **Its own file**, because this one crossed the 900-line
/// rule and that read is a quarter of it.
mod detail;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::wire::{
    manifest_summary, recorded, reported, workflow_summary,
};

impl<H, V, W> Queries for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// What a caller said, turned into the Job they meant.
    ///
    /// **Three reads and not one, deliberately.** `store::Store::resolve_job`
    /// answers with the id and the handle together, so nothing downstream loads
    /// the Job a second time to derive a path from it.
    ///
    /// **A number counts within a Manifest**, and Fleet serves several: one
    /// that more than one of them holds is refused rather than guessed.
    async fn resolve_job(
        &self,
        named: String,
        within: Option<ManifestId>,
    ) -> Result<Resolved, Refusal> {
        let Some(reference) = JobReference::read(&named) else {
            return Err(self.refusal(Adrift::Unresolvable(ResolveJobError::NoSuchJob { named })));
        };
        let served = match &within {
            None => self.repositories().served(),
            Some(manifest_id) => vec![self.served_named(Some(manifest_id))?],
        };
        let found = self
            .resolve_job_across(&reference, &served)
            .await
            .map_err(|why| self.refusal(Adrift::Unresolvable(why)))?;
        // A ULID counts within nothing, so its owner is compared here.
        if let Some(manifest_id) = &within {
            let owner = self.names().owner_of(&found.job_id);
            if owner.as_deref() != Some(manifest_id.as_str()) {
                return Err(
                    self.refusal(Adrift::Unresolvable(ResolveJobError::ElsewhereOwned {
                        named,
                        within: manifest_id.as_str().to_string(),
                    })),
                );
            }
        }
        Ok(Resolved::of(JobId::from(&found.job_id), found.handle))
    }

    async fn list_jobs(&self, manifest_id: Option<ManifestId>) -> Result<JobList, Refusal> {
        let owned = self.owned_by(manifest_id.as_ref())?;
        let (mut loaded, unreadable) = self.every_job().await.map_err(|why| self.refusal(why))?;
        loaded.jobs.retain(|job| owned(job));
        // **One read for the whole list**, filled in afterwards rather than
        // passed into `JobSummary::of`. What became of a Job's pull request is
        // not on `core_model::Job` — it is in the delivery columns beside the
        // row — and reading it per Job would be a query per row on a list that
        // redraws on every event. Ordinarily an empty map: it holds only the
        // Jobs somebody has merged or closed.
        let landed = self
            .store()
            .lock()
            .await
            .landed_by_job()
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        // When each merged, for the same reason and in the same one read.
        let merged_at = self
            .store()
            .lock()
            .await
            .merged_at_by_job()
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        let mut jobs = Vec::with_capacity(loaded.jobs.len());
        for job in &loaded.jobs {
            let mut summary = self.summarised(job).await?;
            summary.landed = landed.get(job.id()).and_then(crate::attention::settled);
            summary.merged_at = merged_at.get(job.id()).map(ipc::Instant::from);
            jobs.push(summary);
        }
        Ok(JobList {
            jobs,
            unreadable: unreadable
                .into_iter()
                .map(|fault| ipc::UnreadableJob {
                    job_id: None,
                    fault,
                })
                .collect(),
        })
    }

    /// The Manifest a door session is answered inside: the one its relay
    /// named, refused where Fleet does not serve it. Absent is the first.
    async fn scope(&self, named: Option<ManifestId>) -> Result<ManifestId, Refusal> {
        let served = self.served_named(named.as_ref())?;
        Ok(ManifestId::from(served.manifest().id()))
    }

    /// The four narrowings, each one rule, stated in `crate::attention`.
    async fn list_job_board(&self, manifest_id: Option<ManifestId>) -> Result<JobList, Refusal> {
        self.job_board(manifest_id.as_ref()).await
    }

    async fn list_reviews(&self, manifest_id: Option<ManifestId>) -> Result<JobList, Refusal> {
        self.reviews(manifest_id.as_ref()).await
    }

    async fn get_activity_feed(&self, manifest_id: Option<ManifestId>) -> Result<JobList, Refusal> {
        self.activity_feed(manifest_id.as_ref()).await
    }

    async fn list_alerts(&self, manifest_id: Option<ManifestId>) -> Result<AlertList, Refusal> {
        self.alerts(manifest_id.as_ref()).await
    }

    async fn owned_jobs(&self, manifest_id: ManifestId) -> Result<Vec<JobId>, Refusal> {
        self.jobs_owned_by(manifest_id)
    }

    /// The probes Fleet can run on itself — `crate::probing`.
    async fn get_health(&self) -> Result<FleetHealth, Refusal> {
        self.health().await
    }

    /// The spend, and what the ceilings hold — `crate::spending`.
    async fn get_usage(&self) -> Result<FleetUsage, Refusal> {
        self.usage().await
    }

    /// The costliest and the longest Job against this Manifest — the budget
    /// form's warning, read beside the caps it is set against.
    async fn get_manifest_spend(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ManifestSpend, Refusal> {
        let past = self
            .store()
            .lock()
            .await
            .past_spend_for(self.served_named(manifest_id.as_ref())?.manifest().id())
            .map_err(|why| self.refusal(crate::adrift::Adrift::Reading(why)))?;
        Ok(ipc::ManifestSpend {
            jobs: past.jobs,
            most_cost_micros: past.most_cost_micros,
            most_turns: past.most_turns,
        })
    }

    /// What this repository declares — `crate::configured`.
    async fn get_manifest(&self, manifest_id: ManifestId) -> Result<ManifestConfig, Refusal> {
        self.manifest_config(manifest_id)
    }

    /// The build Fleet runs on — [`crate::building`].
    async fn get_fleet_build(&self) -> Result<ipc::FleetBuildReport, Refusal> {
        self.fleet_build().await
    }

    /// Every rule a person always-allowed for this Manifest's repository —
    /// `crate::allowed::repository`.
    async fn get_repository_allowed_commands(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::RepositoryAllowedCommands, Refusal> {
        Ok(ipc::RepositoryAllowedCommands {
            commands: self
                .repository_allowed(&self.served_named(manifest_id.as_ref())?)
                .await
                .iter()
                .map(ipc::AllowedCommandRow::from)
                .collect(),
        })
    }

    /// The setup a person already works with — `crate::kit`. Machine-wide, so
    /// no Manifest is named, and read fresh: a person who installs a skill and
    /// comes back expects to see it.
    async fn get_kit_inventory(&self) -> Result<ipc::KitInventory, Refusal> {
        Ok(self.kit_inventory().await)
    }

    /// Every MCP server in Kit, both tiers, and what a Drone dispatched here
    /// resolves — `crate::kit`.
    async fn get_kit_servers(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::KitServers, Refusal> {
        Ok(self
            .kit_servers_listed(&self.served_named(manifest_id.as_ref())?)
            .await)
    }

    /// What the last re-read of `armada.yml` came to, straight off what Fleet
    /// is holding.
    ///
    /// **No lock but its own and no work at all.** The reading was assembled
    /// when the file was read; this hands back the copy. `None` is a Fleet that
    /// has not re-read the file since it started with it, which is a real
    /// answer rather than a gap — the configuration in force is then the one it
    /// booted on.
    async fn get_manifest_reading(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<Option<ManifestReading>, Refusal> {
        Ok(self
            .served_named(manifest_id.as_ref())?
            .repository()
            .reading())
    }

    /// `armada.yml` as it is on disk, for the view that edits it —
    /// [`editing`](mod@crate::editing).
    ///
    /// **Unparsed, and whole.** A file that does not parse is the case a
    /// person opening this is most likely to be in, and what the parse came to
    /// is `get_manifest_reading` beside it.
    async fn get_manifest_file(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ManifestFile, Refusal> {
        self.read_manifest_file(&self.served_named(manifest_id.as_ref())?)
    }

    /// Whether the repository still has what `armada.yml` names —
    /// `crate::drifting`, which carries the whole argument.
    ///
    /// **Against the main checkout, not a Job's worktree**, and it takes no
    /// lock and starts no process.
    async fn get_manifest_drift(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ManifestDrift, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let checkout = std::path::Path::new(served.root());
        Ok(crate::drifting::drift(served.manifest(), checkout))
    }

    /// What Scan finds in the checkout Fleet was started in —
    /// [`scanning`](mod@crate::scanning). **It never reads `self.manifest()`**:
    /// Scan is for a repository with none, and the one Fleet holds is beside
    /// the point until Locate (#821) names another checkout.
    async fn get_repository_scan(
        &self,
        repository: Option<String>,
    ) -> Result<ipc::RepositoryScan, Refusal> {
        let repository = self.repository_named(repository.as_deref())?;
        let root = repository.root();
        let tree = crate::scanning::Checkout::at(root);
        Ok(crate::scanning::scan(
            root,
            &tree,
            &**self.ci_configuration(),
        ))
    }

    /// A proposal per workspace — `crate::manifest_proposal`, which holds
    /// them between calls.
    async fn get_manifest_proposals(
        &self,
        repository: Option<String>,
    ) -> Result<ipc::ManifestProposals, Refusal> {
        Ok(self.manifest_proposals(self.repository_named(repository.as_deref())?.as_ref()))
    }

    /// One Job in full — [`detail`](mod@detail), which is a quarter of this
    /// file's length and the read made on every open of a Job.
    async fn get_job(&self, job_id: JobId) -> Result<JobDetail, Refusal> {
        self.job_detail(job_id).await
    }

    /// Every move one Job made, oldest first. **The log, read — not folded.**
    ///
    /// # The Job is loaded first, and that is not a wasted read
    ///
    /// It is what makes an id that names nothing a 404 rather than an empty
    /// history — a lie about a Job that exists and has not moved — and it
    /// keeps this read behind the same fold every other read is behind: a log
    /// the machine would not admit refuses to load, so a history that reaches
    /// the wire is one `Job::transition` accepted. **This read cannot show a
    /// state the fold rejected**, and does not replay to avoid it —
    /// `crates/store/src/fold.rs` is still the only caller.
    ///
    /// # The rows come back whole
    ///
    /// One query, in `seq` order, over the one table both machines write to.
    /// A step move ordered against the status transitions around it is what a
    /// separately keyed second log could not have offered.
    async fn get_job_events(&self, job_id: JobId) -> Result<JobHistory, Refusal> {
        let id = job_id.to_domain();
        self.load(&id).await.map_err(|why| self.refusal(why))?;
        let (events, coords, vias) = {
            let store = self.store().lock().await;
            let events = store
                .events_for(&id)
                .map_err(|cause| self.refusal(Adrift::Reading(LoadJobError::Unreadable(cause))))?;
            let coords = store
                .group_coords(&id)
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let vias = store
                .vias_for(&id)
                .map_err(|cause| self.refusal(Adrift::Reading(LoadJobError::Unreadable(cause))))?;
            (events, coords, vias)
        };
        Ok(JobHistory {
            job_id,
            moves: events
                .iter()
                .map(|event| ipc::Recorded {
                    // The door a request came through, kept beside the row.
                    via: vias
                        .iter()
                        .find(|(seq, _)| *seq == event.seq())
                        .map(|(_, via)| (*via).into()),
                    ..recorded(event, &coords)
                })
                .collect(),
        })
    }

    /// One Job's whole patch, with the file list beside it.
    ///
    /// **The expensive read, and the only place Fleet spends it for a person.**
    /// `WorkProduct` keeps the patch behind its own call because the bytes are
    /// large and most steps ask no semantic question; the two calls are made
    /// together here because this is the one caller that wants both.
    ///
    /// A Job with no worktree answers `work: None` rather than an empty
    /// reading, and a worktree that will not open is a 500 rather than a patch
    /// nobody read. `plan_declared` is always false: a declaration belonged to a
    /// Drone, and none works now.
    async fn get_diff(&self, job_id: JobId) -> Result<JobDiff, Refusal> {
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let Some(worktree) = self.worktree_of(&job).map_err(|why| self.refusal(why))? else {
            return Ok(JobDiff { job_id, work: None });
        };
        // **The whole branch, which is what a person opening a Job is reading.**
        let changed = self
            .work()
            .changed_files(&worktree)
            .map_err(|cause| self.unreadable(job.id(), cause))?;
        let patch = self
            .work()
            .patch(&worktree)
            .map_err(|cause| self.unreadable(job.id(), cause))?;
        // Which ref the two readings above were taken against. **Carried rather
        // than assumed**: a patch measured from the branch's own tip is the
        // uncommitted remainder and not the Job's work, and until this field
        // existed the sheet said "the Job's patch" over either one.
        let measured = self.work().measured(&worktree);
        Ok(JobDiff {
            job_id,
            work: Some(Work {
                files: crate::wire::seen(&changed),
                measured_from: measured.from,
                measured_whole: measured.whole,
                plan_declared: false,
                // Absent where there is nothing in it. An empty string reads as
                // a reading that broke, and a reading that broke is the refusal
                // above rather than a field.
                patch: Some(patch.as_str().to_string()).filter(|text| !text.is_empty()),
            }),
        })
    }

    /// The Manifest surface's five reads — `crate::rehearsing::checkout`.
    async fn get_checkout_run_sheet(
        &self,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> Result<ipc::CheckoutRunSheet, Refusal> {
        let checkout = self.checkout_named(manifest_id.as_ref(), repository.as_deref())?;
        self.checkout_run_sheet(checkout).await
    }

    async fn list_checkout_runs(
        &self,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> Result<ipc::CheckoutRunList, Refusal> {
        let checkout = self.checkout_named(manifest_id.as_ref(), repository.as_deref())?;
        self.checkout_rehearsal_history(checkout).await
    }

    async fn get_checkout_run_output(
        &self,
        run_id: String,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> Result<ipc::RunOutput, Refusal> {
        let checkout = self.checkout_named(manifest_id.as_ref(), repository.as_deref())?;
        self.checkout_rehearsal_output(run_id, checkout).await
    }

    async fn get_checkout_run_diff(
        &self,
        run_id: String,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> Result<ipc::CheckoutRunDiff, Refusal> {
        let checkout = self.checkout_named(manifest_id.as_ref(), repository.as_deref())?;
        self.checkout_rehearsal_diff(run_id, checkout).await
    }

    async fn observe_checkout_run(
        &self,
        run_id: String,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> Result<ObservedCheckoutRun, Refusal> {
        let checkout = self.checkout_named(manifest_id.as_ref(), repository.as_deref())?;
        self.observe_checkout_rehearsal(run_id, checkout).await
    }

    /// Every server Fleet holds — `crate::servers`.
    async fn list_servers(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<ipc::ServerList, Refusal> {
        self.servers_within(manifest_id)
    }

    async fn observe_server(&self, server_id: String) -> Result<api::ObservedServer, Refusal> {
        self.observed_server(&server_id)
            .map_err(|why| self.server_refusal(why, None))
    }

    /// Every workflow this Fleet holds, so a caller can name one that will not
    /// be refused.
    async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, Refusal> {
        let served = self.repositories().served();
        Ok(served
            .iter()
            .flat_map(|one| {
                let files = one.files();
                one.workflows()
                    .values()
                    .map(|workflow| workflow_summary(workflow, one.manifest().id(), &files))
                    .collect::<Vec<_>>()
            })
            .collect())
    }

    /// One definition as its file holds it, for an editor.
    async fn get_workflow(
        &self,
        workflow_id: ipc::WorkflowId,
        source: Option<String>,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowDefinition, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let files = served.files();
        let found = files.iter().find(|file| {
            file.id().as_str() == workflow_id.as_str()
                && match &source {
                    Some(source) => file.source().as_wire() == source,
                    None => file.overridden_by().is_none(),
                }
        });
        match found {
            Some(file) => Ok(crate::wire::workflow_definition(file)),
            None => {
                let said = match &source {
                    Some(source) => format!(
                        "no `{}` is held from {source} in this repository; a definition that \
                         was left out is on `list_left_out_workflows`",
                        workflow_id.as_str()
                    ),
                    None => format!(
                        "no workflow `{}` runs in this repository",
                        workflow_id.as_str()
                    ),
                };
                Err(Refusal::Unacceptable(
                    ipc::WireError::raised(
                        "fleet.no_such_workflow_definition",
                        said,
                        self.run_id(),
                    )
                    .with_field(
                        "workflow_id",
                        ipc::WireValue::Str(workflow_id.as_str().to_string()),
                    ),
                ))
            }
        }
    }

    /// The Kit and carried definitions one repository runs without, each with why.
    async fn list_left_out_workflows(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<Vec<ipc::LeftOutWorkflow>, Refusal> {
        Ok(self.served_named(manifest_id.as_ref())?.left_out().to_vec())
    }

    async fn list_triggers(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::TriggerList, Refusal> {
        Ok(self.list_triggers_of(&self.served_named(manifest_id.as_ref())?))
    }

    async fn get_trigger(
        &self,
        when: ipc::TriggerMoment,
        step: Option<ipc::StepId>,
        name: String,
        level: Option<ipc::TriggerLevel>,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::TriggerDefinition, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        self.get_trigger_of(&served, when, step, &name, level)
    }

    /// Every Manifest this Fleet serves, the one it was started in first.
    async fn list_manifests(&self) -> Result<Vec<ManifestSummary>, Refusal> {
        let served = self.repositories().served();
        Ok(served
            .iter()
            .map(|one| manifest_summary(one.manifest(), one.records_root()))
            .collect())
    }

    /// Every repository served, Manifest or none — `crate::repositories`.
    async fn list_repositories(&self) -> Result<ipc::RepositoryList, Refusal> {
        Ok(self.repository_list())
    }

    /// Which served repository a request's words are about —
    /// `crate::repositories::reading`.
    async fn read_request_repository(&self, words: String) -> Result<ipc::RequestRepository, Refusal> {
        Ok(self.request_repository(words).await)
    }

    /// Who held one slot and what was checked out in it — [`crate::bay_history`].
    async fn get_slot_history(&self, ask: ipc::BayHistoryAsk) -> Result<ipc::BayHistory, Refusal> {
        self.slot_history(ask).await
    }

    /// What happened inside a window — [`crate::timeline`].
    async fn get_fleet_timeline(
        &self,
        ask: ipc::TimelineAsk,
    ) -> Result<ipc::FleetTimeline, Refusal> {
        self.fleet_timeline(ask).await
    }

    /// Each served repository's merge line — [`crate::merge_lines`].
    async fn get_merge_lines(&self) -> Result<ipc::MergeLines, Refusal> {
        Ok(crate::merge_lines::answer(self).await)
    }

    /// One uncommitted file of a held checkout — [`crate::catching_up`].
    async fn checkout_diff(&self, ask: ipc::CheckoutDiffAsk) -> Result<ipc::CheckoutDiff, Refusal> {
        self.checkout_diff_of(ask).await
    }

    async fn merge_hubs(&self) -> Vec<(String, ipc::MergeLineHub)> {
        Fleet::merge_hubs(self).await
    }

    /// The Jobs as loaded, without the per-Job reads a summary makes.
    async fn job_branches(&self) -> Vec<(String, JobId, String)> {
        let Ok((loaded, _)) = self.every_job().await else {
            return Vec::new();
        };
        loaded
            .jobs
            .iter()
            .filter_map(|job| {
                let branch = job.branch()?;
                Some((branch.as_str().to_string(), JobId::from(job.id()), job.handle()))
            })
            .collect()
    }

    async fn land_checks(&self) -> Vec<(String, Vec<String>)> {
        Fleet::land_checks(self)
    }

    /// One Check's log on main or a commit of it; a branch's line has none to open.
    async fn observe_land_check(
        &self,
        root: String,
        branch: String,
        check: String,
    ) -> Result<api::LandOutput, Refusal> {
        let asked = (root.clone(), branch.clone(), check.clone());
        let on_main = format!("{}@", crate::main_hub::MAIN);
        let found = match branch.strip_prefix(&on_main) {
            Some(commit) => self.main_run_log(root, commit.to_string(), check).await,
            None if branch == crate::main_hub::MAIN => self.main_log(root, check).await,
            None => None,
        };
        found.ok_or_else(|| {
            let (root, branch, check) = asked;
            self.refusal(Adrift::NoSuchLandLog {
                root,
                branch,
                check,
            })
        })
    }

    /// What a Job may be spawned as: the roster the composition root resolved, and the default now.
    async fn list_models(&self) -> Result<ModelChoices, Refusal> {
        Ok(ModelChoices { default: self.default_model(), session: self.session_model(), ..self.models().clone() })
    }

    /// Every worktree Fleet is holding disk for, and the test each one failed,
    /// and every slot of each pool. From memory where `crate::worktrees_kept`
    /// holds a true list; the derivation is `worktrees_live`.
    async fn list_worktrees(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<WorktreesHeld, Refusal> {
        let held = self.worktrees_answer().await?;
        self.worktrees_within(held, manifest_id)
    }

    /// Every report filed, newest first, with the counts they are read beside.
    async fn list_reports(&self) -> Result<ipc::ReportList, Refusal> {
        let (filed, counted) = Fleet::reports(self)
            .await
            .map_err(|why| self.refusal(why))?;
        let mut reports = Vec::with_capacity(filed.len());
        for report in &filed {
            reports.push(reported(report).map_err(|why| self.refusal(why))?);
        }
        Ok(ipc::ReportList {
            reports,
            calibration: ipc::Calibration {
                refusals_recorded: counted.refusals_recorded,
                refusals_disputed: counted.refusals_disputed,
                passes_disputed: counted.passes_disputed,
                reports_filed: counted.reports_filed,
            },
        })
    }

    /// Paths under the checkout narrowed against typed text, for Bridge's `@`
    /// mention popup. `crate::files::search` is the walk; this only names the
    /// root it walks and cannot refuse.
    async fn search_files(
        &self,
        query: String,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::FilesFound, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let root = std::path::Path::new(served.root());
        Ok(ipc::FilesFound {
            paths: crate::files::search(root, &query),
        })
    }

    /// The repository's branches, through the Vcs adapter. #1605.
    async fn list_branches(
        &self,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::Branches, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let listed = self
            .vcs()
            .branches(served.root(), served.manifest().base())
            .map_err(|why| {
                self.refusal(Adrift::BranchesUnread {
                    job: None,
                    why: why.to_string(),
                })
            })?;
        Ok(ipc::Branches {
            branches: listed
                .into_iter()
                .map(|branch| ipc::BranchRow {
                    name: branch.name,
                    base: branch.base,
                })
                .collect(),
        })
    }
}
