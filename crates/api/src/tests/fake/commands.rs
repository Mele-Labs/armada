//! Every command, in one list, each delegating to its subject.
//!
//! **A trait implementation cannot be split across files**, so this is the whole
//! `Commands` block and the bodies are inherent methods next door. What that
//! buys is the roster: every command Fleet answers, in the order the inventory
//! names them, on one screen — and three files sized by subject rather than one
//! file sized by the trait.
//!
//! What is asserted through it is that the route reached the daemon, that the
//! body arrived, and that a refusal comes back as the refusal the transport is
//! supposed to map.

use super::FakeDaemon;
use crate::{Commands, Refusal};

impl FakeDaemon {


    fn no_such_workspace(&self, dir: &str) -> Refusal {
        let said = format!("the fake holds no proposal for `{dir}`");
        Refusal::Unacceptable(ipc::WireError::raised(
            "fake.no_such_workspace",
            said,
            crate::tests::shapes::run_id(),
        ))
    }
}

impl Commands for FakeDaemon {
    /// The bytes echoed back as a path so a route test can tell the body
    /// arrived. Nothing is written: what a save does to a file is
    /// `fleet::editing`'s and is tested there against a real one.
    async fn save_manifest_file(
        &self,
        save: ipc::SaveManifestFile,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ManifestSaved, Refusal> {
        Ok(ipc::ManifestSaved {
            path: format!("armada.yml ({} bytes)", save.text.len()),
            at: ipc::Instant::carried("2026-09-12T09:00:00.000Z"),
        })
    }
    /// The edits counted into the answer so a route test can tell the body
    /// arrived. Nothing is placed: what edits do to a file is
    /// `config::amend`'s, and `fleet::amending`'s against a real one.
    async fn edit_manifest(
        &self,
        edit: ipc::EditManifest,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ManifestEdited, Refusal> {
        Ok(ipc::ManifestEdited {
            path: "armada.yml".to_string(),
            at: ipc::Instant::carried("2026-09-12T09:00:00.000Z"),
            text: format!("{}# {} edits\n", edit.read, edit.edits.len()),
            declared: Some(crate::tests::shapes::manifest_declared()),
        })
    }

    /// The fake holds no proposals, so every workspace is one Scan did not
    /// find — the refusal a route test can tell from a missing route.
    async fn edit_manifest_proposal(
        &self,
        asked: ipc::EditManifestProposal,
        _repository: Option<String>,
    ) -> Result<ipc::ManifestProposal, Refusal> {
        Err(self.no_such_workspace(&asked.dir))
    }
    async fn write_manifest_proposal(
        &self,
        asked: ipc::WriteManifestProposal,
        _repository: Option<String>,
    ) -> Result<ipc::ManifestProposal, Refusal> {
        Err(self.no_such_workspace(&asked.dir))
    }

    /// Recorded and answered; nothing restarts.
    async fn change_fleet_build(
        &self,
        asked: ipc::ChangeFleetBuild,
    ) -> Result<ipc::FleetBuildChanging, Refusal> {
        let build = asked.build;
        self.builds.lock().expect("not poisoned").push(asked);
        Ok(ipc::FleetBuildChanging { build })
    }

    /// The named row goes, by its exact text; one absent is the 409 every
    /// fake give a name that names nothing.
    async fn add_repository(
        &self,
        asked: ipc::AddRepository,
    ) -> Result<ipc::RepositorySummary, Refusal> {
        Ok(ipc::RepositorySummary {
            root: asked.path,
            records_root: String::from("/records"),
            manifest: None,
            implicit_id: None,
        })
    }

    /// Clones nothing: served under `parent`, at the folder a URL of `shop` names.
    async fn clone_repository(
        self: std::sync::Arc<Self>,
        asked: ipc::CloneRepository,
    ) -> Result<ipc::RepositorySummary, Refusal> {
        Ok(ipc::RepositorySummary {
            root: format!("{}/shop", asked.parent),
            records_root: String::from("/records"),
            manifest: None,
            implicit_id: None,
        })
    }

    /// Makes nothing: served under `parent`, at the folder `name` names.
    async fn create_project(
        self: std::sync::Arc<Self>,
        asked: ipc::CreateProject,
    ) -> Result<ipc::RepositorySummary, Refusal> {
        Ok(ipc::RepositorySummary {
            root: format!("{}/{}", asked.parent, asked.name),
            records_root: String::from("/records"),
            manifest: None,
            implicit_id: None,
        })
    }

    /// Installs nothing: says it did.
    async fn install_agent_mod(&self, _asked: ipc::InstallAgentMod) -> Result<ipc::ModInstalled, Refusal> {
        Ok(ipc::ModInstalled::Installed { said: String::from("installed") })
    }

    async fn set_run_memory_place(&self, chosen: ipc::RunMemoryPlace) -> Result<ipc::RunMemoryPlace, Refusal> {
        Ok(chosen)
    }

    async fn set_kept_on_this_mac(&self, asked: ipc::SetKeptOnThisMac) -> Result<ipc::KeptOnThisMac, Refusal> {
        Ok(asked)
    }

    async fn set_notice_mute(&self, asked: ipc::SetNoticeMute) -> Result<ipc::SetNoticeMute, Refusal> {
        Ok(asked)
    }

    async fn drop_run_fact(&self, dropping: ipc::DropRunFact) -> Result<ipc::RunMemory, Refusal> {
        Ok(ipc::RunMemory { repository: dropping.repository, place: ipc::RunPlace::ThisMac, workspaces: Vec::new() })
    }

    async fn remove_repository_allowed_command(
        &self,
        removing: ipc::RemoveRepositoryAllowedCommand,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::RepositoryAllowedCommands, Refusal> {
        let mut allowed = self.repository_allowed.lock().expect("not poisoned");
        let before = allowed.len();
        allowed.retain(|row| row.run != removing.run);
        if allowed.len() == before {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fake.no_such_repository_allow",
                format!(
                    "nothing always-allowed for the repository is spelled `{}`",
                    removing.run
                ),
                crate::tests::shapes::run_id(),
            )));
        }
        Ok(ipc::RepositoryAllowedCommands {
            commands: allowed.clone(),
        })
    }

    /// A server joins reaching no Drone, which is the property the real one
    /// holds by construction — `core_model::KitServer::added`. `#1275`.
    async fn add_kit_server(
        &self,
        adding: ipc::AddKitServer,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::KitServers, Refusal> {
        let mut servers = self.kit_servers.lock().expect("not poisoned");
        servers.retain(|row| row.name != adding.name);
        servers.push(ipc::KitServerRow {
            name: adding.name,
            address: adding.address,
            drones: reaches("no"),
            manifest: None,
            resolves: false,
            added_at: ipc::Instant::carried("2026-09-17T09:00:00.000Z"),
            by: ipc::Actor::from_wire("human").expect("a spelling the registry has"),
        });
        Ok(ipc::KitServers {
            servers: servers.clone(),
        })
    }

    async fn forget_kit_server(
        &self,
        forgetting: ipc::ForgetKitServer,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::KitServers, Refusal> {
        let mut servers = self.kit_servers.lock().expect("not poisoned");
        let before = servers.len();
        servers.retain(|row| row.name != forgetting.name);
        if servers.len() == before {
            return Err(no_such_server(&forgetting.name));
        }
        Ok(ipc::KitServers {
            servers: servers.clone(),
        })
    }

    /// **`resolves` is left where it was.** The resolution is Fleet's, over
    /// `core_model::a_drone_resolves`, and `fleet::kit` is where it is tested;
    /// a second rule here would be a second answer.
    async fn set_kit_server_reach(
        &self,
        setting: ipc::SetKitServerReach,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::KitServers, Refusal> {
        let mut servers = self.kit_servers.lock().expect("not poisoned");
        let Some(row) = servers.iter_mut().find(|row| row.name == setting.name) else {
            return Err(no_such_server(&setting.name));
        };
        row.drones = setting.drones;
        Ok(ipc::KitServers {
            servers: servers.clone(),
        })
    }

    async fn set_manifest_server_reach(
        &self,
        setting: ipc::SetManifestServerReach,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::KitServers, Refusal> {
        let mut servers = self.kit_servers.lock().expect("not poisoned");
        let Some(row) = servers.iter_mut().find(|row| row.name == setting.name) else {
            return Err(no_such_server(&setting.name));
        };
        row.manifest = setting.reach;
        Ok(ipc::KitServers {
            servers: servers.clone(),
        })
    }

    /// Holds nothing, and answers with the one command a removal was asked
    /// for gone: enough for a route test to tell the body arrived.
    async fn remove_kit_allowed_command(
        &self,
        removing: ipc::RemoveKitAllowedCommand,
    ) -> Result<ipc::KitAllowedCommands, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fake.no_such_kit_allow",
            format!(
                "Kit's allowlist holds no command spelled `{}`",
                removing.run
            ),
            crate::tests::shapes::run_id(),
        )))
    }

    /// Answers the file it was sent, so a route test can tell the body arrived.
    async fn save_flight_controls(
        &self,
        controls: ipc::FlightControls,
    ) -> Result<ipc::FlightControlsRead, Refusal> {
        Ok(ipc::FlightControlsRead::Read { controls })
    }

    /// Answers what it was sent, for [`super::SERVED_ROOT`] only.
    async fn save_repository_flight_controls(
        &self,
        saving: ipc::SaveRepositoryFlightControls,
    ) -> Result<ipc::RepositoryFlightControls, Refusal> {
        super::served_root(&saving.repository)?;
        Ok(ipc::RepositoryFlightControls {
            repository: saving.repository,
            place: ipc::RunPlace::ThisMac,
            read: ipc::FlightControlsRead::Read { controls: saving.controls },
            in_repository: ipc::FlightControlsRead::Read { controls: ipc::FlightControls::default() },
            asking: Vec::new(),
        })
    }

    /// For [`super::SERVED_ROOT`] only: answers the length of the text it was handed as the file's
    /// address, so a route test can tell the body arrived whole.
    async fn save_kit_manifest(&self, _caller: Option<crate::Caller>, saving: ipc::SaveKitManifest) -> Result<ipc::KitManifestSaved, Refusal> {
        super::served_root(&saving.repository)?;
        Ok(ipc::KitManifestSaved { repository: saving.repository, file: format!("{} bytes", saving.text.len()) })
    }

    /// For [`super::SERVED_ROOT`] only: answers the tile id it was handed as the repository's
    /// `in_repository` refusal, so a route test can tell the body arrived whole.
    async fn approve_repository_tile(
        &self,
        approving: ipc::ApproveRepositoryTile,
    ) -> Result<ipc::RepositoryFlightControls, Refusal> {
        super::served_root(&approving.repository)?;
        Ok(ipc::RepositoryFlightControls {
            repository: approving.repository,
            place: ipc::RunPlace::ThisMac,
            read: ipc::FlightControlsRead::Read { controls: ipc::FlightControls::default() },
            in_repository: ipc::FlightControlsRead::NotRead { why: format!("approved {} as {}", approving.tile_id, approving.command_hash) },
            asking: Vec::new(),
        })
    }

    /// A package of the one text a route test sends, named for the first repository it chose.
    async fn share_package(&self, asked: ipc::SharePieces) -> Result<ipc::SharedPackage, Refusal> {
        for chosen in &asked.pieces {
            super::served_root(&chosen.repository)?;
        }
        Ok(ipc::SharedPackage { file_name: String::from("specs.armada.toml"), text: format!("version = 1\n# {} pieces\n", asked.pieces.len()) })
    }

    /// One tile piece, whatever the text, so a route test can tell the body arrived.
    async fn read_package(&self, asked: ipc::PackageText) -> Result<ipc::PackageRead, Refusal> {
        Ok(ipc::PackageRead {
            pieces: vec![ipc::PackagePiece {
                kind: ipc::PieceKind::Tile,
                id: asked.text,
                name: String::from("Specs"),
                list: Some(ipc::TileList::Job),
                action: Some(ipc::FlightControlKind::Shell),
                commands: vec![String::from("bin/rspec")],
                remote: Some(String::from("acme/widgets")),
                repository: None,
                exists: false,
            }],
        })
    }

    /// Answers every piece unbound, and `replace` as the id it was sent under.
    async fn add_package(&self, asked: ipc::AddPackage) -> Result<ipc::PackageAdded, Refusal> {
        Ok(ipc::PackageAdded {
            pieces: vec![ipc::PieceAdded {
                kind: ipc::PieceKind::Tile,
                id: asked.replace.to_string(),
                name: asked.text,
                list: Some(ipc::TileList::Deck),
                remote: None,
                repository: None,
                landed: ipc::Landed::Unbound,
                unbound_id: Some(String::from("0123456789abcdef")),
            }],
        })
    }

    /// Binds to [`super::SERVED_ROOT`] only.
    async fn bind_unbound_piece(&self, asked: ipc::BindUnboundPiece) -> Result<ipc::PieceAdded, Refusal> {
        super::served_root(&asked.repository)?;
        Ok(ipc::PieceAdded {
            kind: ipc::PieceKind::Tile,
            id: asked.unbound_id,
            name: String::from("Specs"),
            list: Some(ipc::TileList::Job),
            remote: None,
            repository: Some(asked.repository),
            landed: ipc::Landed::Added,
            unbound_id: None,
        })
    }

    /// For [`super::SERVED_ROOT`] only: refused without `overwrite`, and else answers a Job of
    /// its own on a branch named for the piece's path, so a route test can tell the body arrived whole.
    async fn commit_to_repository(
        &self,
        asked: ipc::CommitToRepository,
    ) -> Result<ipc::CommittedToRepository, Refusal> {
        super::served_root(&asked.repository)?;
        if !asked.overwrite {
            return Err(Refusal::Unacceptable(ipc::WireError::raised("fleet.commit_exists", "main already holds it.", super::run_id())));
        }
        let path = match asked.piece {
            ipc::PieceToCommit::FlightControls { .. } => ".armada/flight-controls.toml".to_string(),
            ipc::PieceToCommit::Workflow { workflow_id } => format!(".armada/workflows/{}", workflow_id.as_str()),
            ipc::PieceToCommit::ReviewTool { id } => format!(".armada/review-tools/{id}.md"),
            ipc::PieceToCommit::Manifest => "armada.yml".to_string(),
        };
        Ok(ipc::CommittedToRepository {
            repository: asked.repository,
            session_id: ipc::SessionId::carried("s1"),
            branch: String::from("session-s1"),
            paths: vec![path],
            commit: String::from("4f2c9a1"),
            pull_request: None,
            pull_request_not_opened: Some(String::from("No pull request was opened.")),
        })
    }

    /// Refused, naming what was asked for, so a route test can tell the body
    /// arrived. Holding a server is `fleet::servers`' and tested there.
    async fn start_server(
        self: std::sync::Arc<Self>,
        asked: ipc::StartServer,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::ServerState, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.not_a_server",
            format!(
                "the fake daemon starts no server, `{}` included",
                asked.name
            ),
            crate::tests::shapes::run_id(),
        )))
    }
    async fn stop_server(&self, named: ipc::NamedServer) -> Result<ipc::ServerState, Refusal> {
        Err(Refusal::IllegalMove(ipc::WireError::raised(
            "fleet.no_such_server",
            format!("the fake daemon holds no server called `{}`", named.id),
            crate::tests::shapes::run_id(),
        )))
    }
    async fn catch_up_checkout(
        &self,
        asked: ipc::CatchUpCheckout,
    ) -> Result<ipc::CheckoutCaughtUp, Refusal> {
        Err(Refusal::IllegalMove(ipc::WireError::raised(
            "fleet.checkout_not_held",
            format!("the fake holds no checkout for {}", asked.manifest_id.as_str()),
            crate::tests::shapes::run_id(),
        )))
    }
    async fn checkout_to_pull_request(
        &self,
        asked: ipc::CheckoutToPullRequest,
    ) -> Result<ipc::CheckoutSentAsPullRequest, Refusal> {
        Err(Refusal::IllegalMove(ipc::WireError::raised(
            "fleet.checkout_not_held",
            format!("the fake holds no checkout for {}", asked.manifest_id.as_str()),
            crate::tests::shapes::run_id(),
        )))
    }
    async fn change_slot_pool(
        &self,
        _change: ipc::ChangeSlotPool,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::SlotPoolChanged, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.no_such_slot",
            String::from("the fake daemon serves no pool"),
            crate::tests::shapes::run_id(),
        )))
    }
    async fn rescue_slot(
        self: std::sync::Arc<Self>,
        _asked: ipc::RescueSlot,
        _manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ipc::SlotRescued, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised(
            "fleet.no_such_slot",
            String::from("the fake daemon serves no pool"),
            crate::tests::shapes::run_id(),
        )))
    }
}

/// Kit's reach, through the one mapping `core-model` owns.
fn reaches(spelling: &str) -> ipc::ReachesDrones {
    ipc::ReachesDrones::from_wire(spelling).expect("a spelling the registry has")
}

fn no_such_server(name: &str) -> Refusal {
    Refusal::Unacceptable(ipc::WireError::raised(
        "fake.no_such_kit_server",
        format!("Kit holds no server called `{name}`"),
        crate::tests::shapes::run_id(),
    ))
}
