//! The writes: what a person tells Fleet to do, and what the Job looks like
//! afterwards.
//!
//! **One of the three surfaces `Daemon` is composed of.** What separates these
//! from [`Queries`](super::Queries) is not size: a command decodes a body, may
//! answer 201, and has refusals meaning the machine would not admit the move.
//! `crate::commands` drew that line through the transport already; this is the
//! same line through the trait.
//!
//! # Killing a Drone and killing a Job are two methods, not one
//!
//! They are different acts on different things, and the registry is what says
//! so: `awaiting_approval -> killed` and `queued -> killed` leave statuses no
//! Drone has been spawned under, so a Job ends there with no process to
//! terminate. One signature covering both would have to mean whichever the
//! caller happened to be looking at.

use std::future::Future;

use crate::daemon::Refusal;
use ipc::{ManifestSaved, SaveManifestFile};

/// Everything a client asks Fleet to do.
pub trait Commands: Send + Sync + 'static {






    /// `catch_up_checkout` — pull a served checkout that Fleet would not catch
    /// up level with its base, setting its uncommitted files aside and putting
    /// them back. **Reads the hold again first** and refuses unless a pull
    /// cannot lose or stop on anything: [`Refusal::IllegalMove`] where nothing
    /// holds the checkout back or the hold is not that one;
    /// [`Refusal::Unacceptable`] on a repository this Fleet does not serve.
    /// **A pull git refused or left conflicted is an answer**, not a refusal.
    fn catch_up_checkout(
        &self,
        asked: ipc::CatchUpCheckout,
    ) -> impl Future<Output = Result<ipc::CheckoutCaughtUp, Refusal>> + Send;

    /// `checkout_to_pull_request` — send a held checkout's uncommitted files up
    /// as a pull request with auto-merge on, **without touching the checkout**.
    /// Reads the hold again first and refuses on `catch_up_checkout`'s
    /// conditions, and where the hold already carries a pull request.
    /// [`Refusal::IllegalMove`] for those; [`Refusal::Unacceptable`] on a
    /// repository this Fleet does not serve; [`Refusal::Fault`] where git or the
    /// forge would not take it.
    fn checkout_to_pull_request(
        &self,
        asked: ipc::CheckoutToPullRequest,
    ) -> impl Future<Output = Result<ipc::CheckoutSentAsPullRequest, Refusal>> + Send;










    /// `change_slot_pool` — add a slot to the repository's pool, remove one,
    /// or close or reopen one, on this machine only: `armada worktree lease`
    /// honours it as Fleet does. [`Refusal::IllegalMove`] naming why a slot
    /// cannot go: held, stranded, busy, not a checkout, dirty, or the last.
    fn change_slot_pool(
        &self,
        change: ipc::ChangeSlotPool,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::SlotPoolChanged, Refusal>> + Send;

    /// `rescue_slot` — start the Scout that reads a stranded slot, stop it, or
    /// scrap or stash the slot's work and free it. [`Refusal::IllegalMove`]
    /// naming why a slot cannot: not stranded, busy, being read, on no branch,
    /// or with nowhere to push.
    fn rescue_slot(
        self: std::sync::Arc<Self>,
        asked: ipc::RescueSlot,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::SlotRescued, Refusal>> + Send;

    /// `save_manifest_file` — write a corrected `armada.yml` to disk, and stop
    /// there.
    ///
    /// **No staging, no commit, no formatting and no reserialisation.** The
    /// bytes the caller sends are the bytes on disk — Journey 9, *Editing*.
    ///
    /// **A write that does not parse is still a write.** Refusing invalid YAML
    /// would mean work in progress cannot be saved. What Fleet could not adopt
    /// arrives as `manifest.reread`, previous values still in force.
    ///
    /// **What comes back is not a reading.** The watch settles before it
    /// re-reads, so what the save moved is not known when this answers.
    ///
    /// [`Refusal::Fault`] where the file will not be written. Nothing about
    /// the text can refuse it.
    fn save_manifest_file(
        &self,
        save: SaveManifestFile,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ManifestSaved, Refusal>> + Send;

    /// `edit_manifest` — apply a form's edits to `armada.yml`, **changing only
    /// the lines they name**, and write the result.
    ///
    /// **What a form produces always loads**, unlike a save: the result is
    /// parsed before anything is written. [`Refusal::Unacceptable`] where it
    /// would not load, with every fault; where an edit names what the file
    /// does not hold; and where the file is written in a shape a form does not
    /// edit. [`Refusal::IllegalMove`] where the file moved under the edit, as
    /// for a save.
    fn edit_manifest(
        &self,
        edit: ipc::EditManifest,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::ManifestEdited, Refusal>> + Send;

    /// `edit_manifest_proposal` — one edit to one proposal. [`Refusal::Unacceptable`] for an
    /// unknown workspace or an edit that cannot apply; a bad value is Write's to refuse.
    fn edit_manifest_proposal(
        &self,
        asked: ipc::EditManifestProposal,
        repository: Option<String>,
    ) -> impl Future<Output = Result<ipc::ManifestProposal, Refusal>> + Send;

    /// `write_manifest_proposal` — create `armada.yml` from a proposal. Refused where it would
    /// not load or a file is already there, which is never written over.
    fn write_manifest_proposal(
        &self,
        asked: ipc::WriteManifestProposal,
        repository: Option<String>,
    ) -> impl Future<Output = Result<ipc::ManifestProposal, Refusal>> + Send;

    /// `start_server` — start a server the Manifest declares, in a Job's
    /// worktree on its span or in the main checkout on its own, **or answer
    /// with the instance already up**: one per Job per server.
    ///
    /// **By `Arc`, for [`Commands::show_again`]'s reason**: the server is a task
    /// of its own and outlives the request. It answers `starting`; the rest
    /// arrives as `server.*` events.
    fn start_server(
        self: std::sync::Arc<Self>,
        asked: ipc::StartServer,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::ServerState, Refusal>> + Send;

    /// `stop_server` — end a server's process group, and answer with the
    /// instance once it has ended. [`Refusal::IllegalMove`] on one not running.
    fn stop_server(
        &self,
        named: ipc::NamedServer,
    ) -> impl Future<Output = Result<ipc::ServerState, Refusal>> + Send;

    /// `change_fleet_build` — start `scripts/restart-build` detached, so that it
    /// outlives the Fleet it stops, and answer that it has begun.
    ///
    /// **Answers before the restart has done anything**; what it came to is the
    /// next `get_fleet_build`. [`Refusal::IllegalMove`] where one is already
    /// under way or this Fleet serves no checkout to restart from;
    /// [`Refusal::Fault`] where the wrapper would not start.
    fn change_fleet_build(
        &self,
        asked: ipc::ChangeFleetBuild,
    ) -> impl Future<Output = Result<ipc::FleetBuildChanging, Refusal>> + Send;

    /// `remove_repository_allowed_command` — take back a rule a person
    /// always-allowed for this Manifest's repository. **Since `#836`.**
    ///
    /// **Fleet-wide, and touches no running Job**: read by the next permission
    /// question and the next spawn, so a Drone already granted the rule keeps
    /// it for the step it is on. Refused where nothing is spelled `run`.
    /// `add_repository` — serve one more repository, from a folder.
    ///
    /// [`Refusal::Unacceptable`] where the folder is not a git repository's
    /// root or its `armada.yml` will not load; [`Refusal::IllegalMove`] where
    /// it, or its Manifest id, is already served. A folder with no `armada.yml`
    /// is served, for Scan to read.
    fn add_repository(
        &self,
        asked: ipc::AddRepository,
    ) -> impl Future<Output = Result<ipc::RepositorySummary, Refusal>> + Send;

    /// `clone_repository` — clone from a URL into a new folder under `parent`,
    /// then serve it as [`add_repository`](Commands::add_repository) does.
    /// Answers when git finishes; its refusals are `add_repository`'s and three
    /// more, in `crates/ipc/operations/`.
    fn clone_repository(
        self: std::sync::Arc<Self>,
        asked: ipc::CloneRepository,
    ) -> impl Future<Output = Result<ipc::RepositorySummary, Refusal>> + Send;

    /// `create_project` — make a new folder `name` under `parent`, start a git
    /// repository in it with an empty first commit, and serve it as
    /// [`add_repository`](Commands::add_repository) does.
    fn create_project(
        self: std::sync::Arc<Self>,
        asked: ipc::CreateProject,
    ) -> impl Future<Output = Result<ipc::RepositorySummary, Refusal>> + Send;

    fn remove_repository_allowed_command(
        &self,
        removing: ipc::RemoveRepositoryAllowedCommand,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::RepositoryAllowedCommands, Refusal>> + Send;

    /// `install_agent_mod` — install the Armada mod into the person's agent CLI. **Only on a
    /// request**: [`Refusal::Unacceptable`] where `requested` is false, before anything runs. A CLI
    /// that fails is an answer, `failed` with its own words.
    fn install_agent_mod(
        &self,
        asked: ipc::InstallAgentMod,
    ) -> impl Future<Output = Result<ipc::ModInstalled, Refusal>> + Send;

    /// `set_run_memory_place` — choose where a repository's remembered runs are written.
    /// [`Refusal::Unacceptable`] for `cloud`, which is not built, and for `synced` without a folder.
    fn set_run_memory_place(
        &self,
        chosen: ipc::RunMemoryPlace,
    ) -> impl Future<Output = Result<ipc::RunMemoryPlace, Refusal>> + Send;

    /// `set_kept_on_this_mac` — mark a repository Keep on this Mac, which moves its run memory
    /// place to `this_mac`, or clear the mark, which moves nothing.
    fn set_kept_on_this_mac(
        &self,
        asked: ipc::SetKeptOnThisMac,
    ) -> impl Future<Output = Result<ipc::KeptOnThisMac, Refusal>> + Send;

    /// `drop_run_fact` — drop one remembered run onto the repository's deny list, for good.
    fn drop_run_fact(&self, dropping: ipc::DropRunFact) -> impl Future<Output = Result<ipc::RunMemory, Refusal>> + Send;

    /// `add_kit_server` — put an MCP server in Kit. It reaches no Drone until
    /// a person says so, on one of the two below. `#1275`.
    fn add_kit_server(
        &self,
        adding: ipc::AddKitServer,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::KitServers, Refusal>> + Send;

    /// `forget_kit_server` — take one out of Kit, and every Manifest's word
    /// about it with it. **Read by the next spawn**, so nothing running moves.
    fn forget_kit_server(
        &self,
        forgetting: ipc::ForgetKitServer,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::KitServers, Refusal>> + Send;

    /// `set_kit_server_reach` — Kit's own tier, for every Manifest that has
    /// not said otherwise.
    fn set_kit_server_reach(
        &self,
        setting: ipc::SetKitServerReach,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::KitServers, Refusal>> + Send;

    /// `set_manifest_server_reach` — this Manifest's own word over a Kit
    /// server, or `null` to take it back and leave Kit's default answering.
    fn set_manifest_server_reach(
        &self,
        setting: ipc::SetManifestServerReach,
        manifest_id: Option<ipc::ManifestId>,
    ) -> impl Future<Output = Result<ipc::KitServers, Refusal>> + Send;

    /// `remove_kit_allowed_command` — take a command out of Kit's allowlist, a
    /// person's act. **Since 23.35.** Answers with what the allowlist holds now.
    /// **Read by the next permission question**, so a Drone already granted
    /// the command keeps it for the call it is on. Refused where the allowlist
    /// holds no command spelled `run`.
    fn remove_kit_allowed_command(
        &self,
        removing: ipc::RemoveKitAllowedCommand,
    ) -> impl Future<Output = Result<ipc::KitAllowedCommands, Refusal>> + Send;

    /// `save_flight_controls` — write the Cockpit's flight controls to Kit, whole.
    /// Answers with the file read again. [`Refusal::Unacceptable`] naming the field for a
    /// file that breaks a rule; [`Refusal::Fault`] where it cannot be written.
    fn save_flight_controls(
        &self,
        controls: ipc::FlightControls,
    ) -> impl Future<Output = Result<ipc::FlightControlsRead, Refusal>> + Send;

    /// `save_repository_flight_controls` — write a repository's own actions, whole, to the
    /// place its Armada configuration is kept. Answers with them read again.
    /// [`Refusal::Unacceptable`] naming the field, or for a place that cannot take them now;
    /// [`Refusal::NotFound`] for a repository Fleet does not serve; [`Refusal::Fault`] where
    /// a file cannot be written.
    fn save_repository_flight_controls(
        &self,
        saving: ipc::SaveRepositoryFlightControls,
    ) -> impl Future<Output = Result<ipc::RepositoryFlightControls, Refusal>> + Send;

    /// `approve_repository_tile` — keep the hash of a shell tile's current command as approved on
    /// this Mac, and answer with the repository's tiles read again. [`Refusal::Unacceptable`] for
    /// a hash that is no longer the tile's command, or a tile the repository does not hold;
    /// [`Refusal::NotFound`] for a repository Fleet does not serve; [`Refusal::Fault`] where the
    /// approval cannot be written.
    fn approve_repository_tile(
        &self,
        approving: ipc::ApproveRepositoryTile,
    ) -> impl Future<Output = Result<ipc::RepositoryFlightControls, Refusal>> + Send;

    /// `share_package` — the chosen pieces as one package's text. Writes nothing.
    /// [`Refusal::Unacceptable`] for no pieces or one the person does not keep;
    /// [`Refusal::NotFound`] for a repository Fleet does not serve.
    fn share_package(
        &self,
        asked: ipc::SharePieces,
    ) -> impl Future<Output = Result<ipc::SharedPackage, Refusal>> + Send;

    /// `read_package` — every piece and command a package would add, and whether Kit holds
    /// each. Writes nothing. [`Refusal::Unacceptable`] naming what is wrong with the text.
    fn read_package(
        &self,
        asked: ipc::PackageText,
    ) -> impl Future<Output = Result<ipc::PackageRead, Refusal>> + Send;

    /// `add_package` — every piece into the person's Kit, or unbound where its remote names no
    /// repository. [`Refusal::Unacceptable`] for text that is no package, an id Kit holds
    /// without `replace`, or tiles that break a rule; [`Refusal::Fault`] where a file cannot be
    /// written.
    fn add_package(
        &self,
        asked: ipc::AddPackage,
    ) -> impl Future<Output = Result<ipc::PackageAdded, Refusal>> + Send;

    /// `bind_unbound_piece` — the piece kept unbound, once, onto the repository chosen.
    /// [`Refusal::NotFound`] for an id nothing is kept under or a repository Fleet does not serve.
    fn bind_unbound_piece(
        &self,
        asked: ipc::BindUnboundPiece,
    ) -> impl Future<Output = Result<ipc::PieceAdded, Refusal>> + Send;

    /// `commit_to_repository` — copy one of the person's own pieces into `.armada/` on the
    /// branch of the Job the request names, and commit it there. The person's copy stays.
    /// [`Refusal::Unacceptable`] with no Job named that holds a branch in the repository, for a
    /// piece that is not theirs, or one the branch already holds without `overwrite`;
    /// [`Refusal::NotFound`] for a repository Fleet does not serve; [`Refusal::Fault`] where a
    /// file cannot be written or git refuses the commit.
    fn commit_to_repository(
        &self,
        asked: ipc::CommitToRepository,
    ) -> impl Future<Output = Result<ipc::CommittedToRepository, Refusal>> + Send;





}
