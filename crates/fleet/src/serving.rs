//! `api::Queries`, implemented over a real Fleet: every read a client makes.
//!
//! **The dependency points this way and never back.** The traits are stated in
//! `api`, where the transport is, and implemented here. `cargo tree -p api`
//! names no `fleet`, the daemon core is drivable with no socket and no process,
//! and `api`'s own tests were written against a fake before this existed.
//!
//! **This is the redaction, and it is a visible step.** Every signature below
//! speaks `ipc` DTOs, and a field added to a `core_model` type reaches the wire
//! only when somebody writes the line that puts it there — `api` never sees a
//! domain type.
//!
//! **One of three files, because `api::Daemon` is three traits** — #434. The
//! writes are [`commanding`](mod@crate::commanding)'s and the tools are
//! [`tooling`](mod@crate::tooling)'s, each whole, so no file delegates.
//!
//! **Which refusal a failure is, and the code it carries, is
//! [`refusing`](mod@crate::refusing)'s** — every `WireError` below is raised
//! through `Fleet::refusal`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{ObservedCheckoutRun, Queries, Refusal};
use ipc::{
    FleetHealth, ManifestConfig, ManifestDrift, ManifestFile, ManifestId,
    ManifestReading, ManifestSummary, ModelChoices, WorkflowSummary, WorktreesHeld,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::wire::{manifest_summary, workflow_summary};

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

    /// The Manifest a door session is answered inside: the one its relay
    /// named, refused where Fleet does not serve it. Absent is the first.
    async fn scope(&self, named: Option<ManifestId>) -> Result<ManifestId, Refusal> {
        let served = self.served_named(named.as_ref())?;
        Ok(ManifestId::from(served.manifest().id()))
    }

    /// The probes Fleet can run on itself — `crate::probing`.
    async fn get_health(&self) -> Result<FleetHealth, Refusal> {
        self.health().await
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

    /// The folders a person's agent CLI kept sessions for — `crate::session_import`.
    async fn list_import_folders(&self) -> Result<ipc::ImportFolders, Refusal> {
        self.import_folders().await
    }

    /// How one of them was run — `crate::session_import`.
    async fn get_import_runs(&self, path: String) -> Result<ipc::ImportRuns, Refusal> {
        self.import_runs(path).await
    }

    /// The Cockpit's flight controls, from Kit's home - `crate::flight_controls`.
    async fn get_flight_controls(&self) -> Result<ipc::FlightControlsRead, Refusal> {
        Ok(self.flight_controls_read().await)
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
            .map_err(|why| self.server_refusal(why))
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

    /// What a Session may be spawned as: the roster the composition root resolved, and the default now.
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

}
