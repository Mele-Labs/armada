//! What the fake answers a read with.
//!
//! **The `Queries` block, in its own file because the trait is three traits.**
//! `api::Daemon` stated every surface at once until #434, so one implementation
//! meant one impl block and one file; the fake was 856 lines of it. Nothing
//! here is new — it is the reads, moved.
//!
//! Every fixed value is [`shapes`](crate::tests::shapes)'s, still built out of
//! `ipc` alone.

use ipc::{
    FilesFound, FleetHealth, ManifestConfig,
    ManifestDrift, ManifestFile, ManifestId, ManifestReading, ManifestSummary, ModelChoices,
    WorkflowSummary, WorktreesHeld,
};

use super::FakeDaemon;
use crate::tests::shapes;
use crate::tests::shapes::run_id;
use crate::{Queries, Refusal};

impl FakeDaemon {


    /// The checkout's own, which name no Job to be missing.
    pub(super) fn runs_nothing_here<T>(&self) -> Result<T, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.cannot_run_here",
            "the fake daemon runs nothing in a worktree",
            run_id(),
        )))
    }
}

impl FakeDaemon {
}

impl FakeDaemon {
}

impl Queries for FakeDaemon {

    /// The Manifest named, where the fake holds it; the first where none was.
    async fn scope(&self, named: Option<ManifestId>) -> Result<ManifestId, Refusal> {
        let held = shapes::manifests();
        match named {
            None => Ok(held[0].id.clone()),
            Some(named) if held.iter().any(|one| one.id == named) => Ok(named),
            Some(named) => Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.no_such_manifest",
                format!("this Fleet serves no Manifest `{}`", named.as_str()),
                run_id(),
            ))),
        }
    }

    async fn get_health(&self) -> Result<FleetHealth, Refusal> {
        Ok(shapes::health())
    }

    /// A Manifest this fake does not hold is a 422 and never a 404: the request
    /// is well-formed and names something not in the record.
    async fn get_manifest(&self, manifest_id: ManifestId) -> Result<ManifestConfig, Refusal> {
        match manifest_id == shapes::manifests()[0].id {
            true => Ok(shapes::manifest_config()),
            false => Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.unacceptable_proposal",
                "this Fleet holds no such Manifest",
                run_id(),
            ))),
        }
    }


    /// The build [`shapes::fleet_build`] describes, restarting onto whatever the
    /// last change asked for.
    async fn get_fleet_build(&self) -> Result<ipc::FleetBuildReport, Refusal> {
        let mut report = shapes::fleet_build();
        report.restarting = self
            .builds
            .lock()
            .expect("not poisoned")
            .last()
            .map(|asked| asked.build);
        // A restart under way has reached some stage; the fake says the first build one.
        report.stage = report.restarting.map(|_| ipc::BuildStage::BuildingFleet);
        Ok(report)
    }

    /// Whatever a test planted, unfiltered — `#836`.
    async fn get_repository_allowed_commands(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::RepositoryAllowedCommands, Refusal> {
        Ok(ipc::RepositoryAllowedCommands {
            commands: self
                .repository_allowed
                .lock()
                .expect("not poisoned")
                .clone(),
        })
    }

    /// Whatever the fake's own acts have left — `#1275`.
    /// **A setup with nothing in it**, which is what a route test asserts the
    /// shape of. What a real read makes of a real home is `adapters`' own case.
    async fn get_flight_controls(&self) -> Result<ipc::FlightControlsRead, Refusal> {
        Ok(ipc::FlightControlsRead::Read { controls: ipc::FlightControls::default() })
    }

    async fn get_kit_inventory(&self) -> Result<ipc::KitInventory, Refusal> {
        Ok(ipc::KitInventory {
            harness: "a harness".to_string(),
            home: "/home/user/.setup".to_string(),
            present: true,
            kinds: Vec::new(),
        })
    }

    async fn get_kit_servers(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::KitServers, Refusal> {
        Ok(ipc::KitServers {
            servers: self.kit_servers.lock().expect("not poisoned").clone(),
        })
    }

    /// **Always a reading, and always one worth saying.** The fake exists so a
    /// route test has a shape to assert on; a `None` here would make the
    /// ordinary case a test of the empty answer.
    async fn get_manifest_reading(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<Option<ManifestReading>, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(Some(shapes::manifest_reading()))
    }

    /// **A file that does not parse**, matching the reading beside it: the two
    /// answers a surface draws together are about one `armada.yml`.
    async fn get_manifest_file(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ManifestFile, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::manifest_file())
    }

    /// **Always two rows, one of each verdict.** A fake answering all `current`
    /// or all `gone` would let a route test pass against a constant.
    async fn get_manifest_drift(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ManifestDrift, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::manifest_drift())
    }

    async fn get_repository_scan(
        &self,
        _repository: Option<String>,
    ) -> Result<ipc::RepositoryScan, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::repository_scan())
    }

    /// **No proposals and a stated cap**: what a proposal holds is
    /// `fleet::manifest_proposal`'s and tested there against a Scan.
    async fn get_manifest_proposals(
        &self,
        _repository: Option<String>,
    ) -> Result<ipc::ManifestProposals, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(ipc::ManifestProposals {
            checkout: "/repo".to_string(),
            proposals: Vec::new(),
        })
    }

    /// **The fake declares nothing**, so the sheet is empty rather than
    /// refused: `get_checkout_run_sheet` answers on a repository that has
    /// never had a Job in it, and an empty Manifest is that answer.
    async fn get_checkout_run_sheet(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunSheet, Refusal> {
        Ok(ipc::CheckoutRunSheet {
            setup: Vec::new(),
            checks: Vec::new(),
            commands: Vec::new(),
            manifest_edited_at: None,
            running: None,
            servers: Vec::new(),
            verify: None,
            workspaces: Vec::new(),
            seed: None,
        })
    }


    async fn list_checkout_runs(
        &self,
        manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunList, Refusal> {
        self.checkout_runs_named
            .lock()
            .expect("not poisoned")
            .push(manifest_id);
        Ok(ipc::CheckoutRunList {
            runs: Vec::new(),
            unreadable: Vec::new(),
        })
    }

    async fn get_checkout_run_output(
        &self,
        _run_id: String,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::RunOutput, Refusal> {
        self.runs_nothing_here()
    }

    async fn get_checkout_run_diff(
        &self,
        _run_id: String,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunDiff, Refusal> {
        self.runs_nothing_here()
    }

    async fn observe_checkout_run(
        &self,
        _run_id: String,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<crate::ObservedCheckoutRun, Refusal> {
        self.runs_nothing_here()
    }

    /// **The fake holds no server**, so the list is empty and every id names
    /// nothing. What holding one means is `fleet::servers`', tested there.
    async fn list_servers(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<ipc::ServerList, Refusal> {
        let servers = self.servers.lock().expect("not poisoned").clone();
        Ok(ipc::ServerList {
            servers: servers
                .into_iter()
                .filter(|one| manifest_id.is_none() || one.manifest_id == manifest_id)
                .collect(),
        })
    }

    async fn observe_server(&self, server_id: String) -> Result<crate::ObservedServer, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.no_such_server",
            format!("the fake daemon holds no server called `{server_id}`"),
            run_id(),
        )))
    }



    async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::workflows())
    }

    async fn get_workflow(
        &self,
        _workflow_id: ipc::WorkflowId,
        _source: Option<String>,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::WorkflowDefinition, Refusal> {
        Err(self.fault("the fake holds no definition files"))
    }

    async fn list_left_out_workflows(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<Vec<ipc::LeftOutWorkflow>, Refusal> {
        Ok(Vec::new())
    }

    async fn list_triggers(
        &self,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::TriggerList, Refusal> {
        Ok(ipc::TriggerList::default())
    }

    async fn get_trigger(
        &self,
        _when: ipc::TriggerMoment,
        _step: Option<ipc::StepId>,
        _name: String,
        _level: Option<ipc::TriggerLevel>,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::TriggerDefinition, Refusal> {
        Err(self.fault("the fake holds no Trigger files"))
    }

    async fn list_manifests(&self) -> Result<Vec<ManifestSummary>, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::manifests())
    }

    async fn list_models(&self) -> Result<ModelChoices, Refusal> {
        if *self.mute.lock().expect("not poisoned") {
            return Err(self.fault("the fake was told not to answer"));
        }
        Ok(shapes::models())
    }

    async fn list_worktrees(
        &self,
        _manifest_id: Option<ManifestId>,
    ) -> Result<WorktreesHeld, Refusal> {
        Ok(WorktreesHeld { slots: Vec::new() })
    }

    async fn list_repositories(&self) -> Result<ipc::RepositoryList, Refusal> {
        Ok(ipc::RepositoryList {
            repositories: Vec::new(),
        })
    }

    async fn read_request_repository(
        &self,
        words: String,
    ) -> Result<ipc::RequestRepository, Refusal> {
        Ok(shapes::request_repository(&words))
    }

    async fn get_fleet_timeline(
        &self,
        ask: ipc::TimelineAsk,
    ) -> Result<ipc::FleetTimeline, Refusal> {
        Ok(shapes::fleet_timeline(&ask))
    }

    async fn get_slot_history(&self, ask: ipc::BayHistoryAsk) -> Result<ipc::BayHistory, Refusal> {
        Ok(shapes::slot_history(&ask))
    }

    async fn checkout_diff(&self, ask: ipc::CheckoutDiffAsk) -> Result<ipc::CheckoutDiff, Refusal> {
        Err(Refusal::IllegalMove(ipc::WireError::raised(
            "fleet.checkout_not_held",
            format!("the fake holds no checkout for {}", ask.manifest_id.as_str()),
            crate::tests::shapes::run_id(),
        )))
    }

    async fn get_merge_lines(&self) -> Result<ipc::MergeLines, Refusal> {
        Ok(ipc::MergeLines::default())
    }

    async fn observe_land_check(
        &self,
        root: String,
        branch: String,
        check: String,
    ) -> Result<crate::LandOutput, Refusal> {
        let planted = self.land.lock().expect("not poisoned").clone();
        match planted {
            Some(land) if land.root == root && land.branch == branch && land.name == check => {
                Ok(land)
            }
            _ => Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.unacceptable",
                format!("no Check named `{check}` has a log for `{branch}`"),
                run_id(),
            ))),
        }
    }

    async fn search_files(
        &self,
        query: String,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<FilesFound, Refusal> {
        Ok(FilesFound {
            paths: shapes::files_found(&query),
        })
    }

}
