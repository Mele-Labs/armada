//! The reads: what a client asks Fleet, that moves nothing.
//!
//! **One of the three surfaces `Daemon` is composed of**, and the seam is the
//! one `crate::queries` already draws against `crate::commands`: a read never
//! decodes a body, never answers 201, and has no refusal meaning the machine
//! would not admit a move. The transport half of that split was there first;
//! this is the same line drawn through the trait, so an implementation writes
//! one block per surface and a handler names the one surface it uses.
//!
//! The expensive reads are here on purpose and each says why. `get_diff`
//! spends the patch and `get_check_output` a Check's streams, and both are their own
//! operation so that the reads made on every refresh do not pay for them.

use std::future::Future;

use crate::daemon::Refusal;
use ipc::{
    CheckoutRunList, CheckoutRunSheet, FilesFound, FleetHealth, ManifestConfig,
    ManifestDrift, ManifestFile, ManifestId, ManifestReading, ManifestSummary, ModelChoices,
    RunOutput, WorkflowSummary, WorktreesHeld,
};



/// Everything a client reads.
///
/// # No `list_jobs` variant that returns a bare list
///
/// [`ipc::JobList`] carries the Jobs that loaded *and* the rows that would not.
/// A method returning `Vec<JobSummary>` would let the transport layer be the
/// place a partial failure is quietly completed — the v1 bug that lost
/// twenty-one Jobs, one layer further out.
pub trait Queries: Send + Sync + 'static {

    /// The Manifest every answer to this caller is given inside.
    ///
    /// **Not an operation, and on no route of its own** — [`resolve_job`]'s
    /// shape one question over. Named, it is that Manifest where Fleet serves
    /// it and refused where not; absent is the first repository. The agent
    /// door names the one its session stands in.
    ///
    /// [`resolve_job`]: Queries::resolve_job
    fn scope(
        &self,
        named: Option<ManifestId>,
    ) -> impl Future<Output = Result<ManifestId, Refusal>> + Send;

    /// `get_health` — the probes Fleet can run on itself.
    ///
    /// **Not Doctor.** Doctor's grid is ten modules probed from three crates
    /// and is not built; what this cannot answer for is named in
    /// [`ipc::FleetHealth::not_probed`] rather than left out.
    ///
    /// It never refuses. A probe that could not read is a `fail` row with the
    /// reason in it — a 500 here would be the health check being the thing
    /// that is unhealthy.
    fn get_health(&self) -> impl Future<Output = Result<FleetHealth, Refusal>> + Send;

    /// `get_manifest` — one Manifest as Fleet resolved it.
    ///
    /// [`Refusal::NotFound`] is not the refusal here: a Manifest this Fleet
    /// does not serve is [`Refusal::Unacceptable`], a well-formed request
    /// naming something that is not in the record.
    fn get_manifest(
        &self,
        manifest_id: ManifestId,
    ) -> impl Future<Output = Result<ManifestConfig, Refusal>> + Send;


    /// `get_fleet_build` — which build Fleet runs on, the commit it came from,
    /// where that stands against `origin/main`, and the restart under way or
    /// the one that failed.
    ///
    /// **Never waits on the network**: the counts are as fresh as Fleet's last
    /// background fetch. [`Refusal::IllegalMove`] where this Fleet serves no
    /// checkout that can be restarted from, and a Fleet in that state has no
    /// build to choose.
    fn get_fleet_build(&self) -> impl Future<Output = Result<ipc::FleetBuildReport, Refusal>> + Send;

    /// `get_repository_allowed_commands` — every rule a person always-allowed
    /// for this Manifest's repository, oldest first. **Since `#836`.**
    ///
    /// **Fleet-wide, and unfiltered**: every row this Fleet ever wrote for
    /// this Manifest, whether or not `armada.yml` still leaves it grantable.
    /// `JobDetail.repository_allowed_commands` is the same list, for a person
    /// reading one Job. The only `Refusal` is a fault.
    fn get_repository_allowed_commands(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ipc::RepositoryAllowedCommands, Refusal>> + Send;

    /// `get_kit_inventory` — the setup a person already works with, read from
    /// their agent harness's own home. `#1491`.
    ///
    /// **Every kind is answered**, including the ones nothing reads yet: a kind
    /// left out is drawn as empty, and an empty Kit beside a full home
    /// directory is what this read exists to stop. The only `Refusal` is a
    /// fault — a home that is not there is an answer.
    fn get_kit_inventory(&self) -> impl Future<Output = Result<ipc::KitInventory, Refusal>> + Send;

    /// `get_flight_controls` — the Cockpit's tiles a person arranged, from
    /// `~/.armada/flight-controls.toml`. **An absent file is the defaults and a
    /// file that will not read is `not_read`**, never the defaults: the only
    /// `Refusal` is a fault.
    fn get_flight_controls(
        &self,
    ) -> impl Future<Output = Result<ipc::FlightControlsRead, Refusal>> + Send;

    /// `get_repository_flight_controls` — a person's own actions for one repository, read
    /// from the place its Armada configuration is kept, and which place that was. A file that
    /// will not read is `not_read`; [`Refusal::NotFound`] for a repository Fleet does not serve.
    /// `in_repository` is the repository's own file, read in the tree of the Job `session_id`
    /// names, or the checkout with none.
    fn get_repository_flight_controls(
        &self,
        repository: String,
        session_id: Option<ipc::SessionId>,
    ) -> impl Future<Output = Result<ipc::RepositoryFlightControls, Refusal>> + Send;

    /// `get_kit_servers` — every MCP server in Kit, Kit's own default for
    /// each, this Manifest's word over it, and whether a Drone dispatched here
    /// resolves it. `#1275`.
    ///
    /// **`resolves` is the resolution, not a hint.** The same call decides the
    /// document a Drone is spawned against, so a surface drawing this row and
    /// the file a Drone reads cannot disagree. The only `Refusal` is a fault.
    fn get_kit_servers(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ipc::KitServers, Refusal>> + Send;

    /// `get_manifest_reading` — what Fleet's last re-read of `armada.yml` came
    /// to, and whether it took.
    ///
    /// **Fleet-wide, and there is no Job to hang it off.** A Manifest reload
    /// belongs to no Job's transcript, which is why the only place it used to
    /// be said was the daemon's console.
    ///
    /// `None` is a real answer and not a miss: Fleet has not re-read the file
    /// since it started with it. The configuration in force is then simply the
    /// one it booted on, and there is nothing about it to say.
    ///
    /// It cannot refuse. Fleet either holds a reading or does not, and both are
    /// answers — `Result` is here to match the surface, not because a failure
    /// is reachable.
    fn get_manifest_reading(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<Option<ManifestReading>, Refusal>> + Send;

    /// `get_manifest_file` — `armada.yml` as it is on disk, for the view that
    /// edits it.
    ///
    /// **The half a person acts with**, where
    /// [`Queries::get_manifest_reading`] is the half that reports.
    ///
    /// **The bytes, not a document.** Nothing is parsed, so a file that does
    /// not parse reads back as well as one that does — the case a person
    /// opening this is most likely to be in.
    ///
    /// **No path in.** `manifest_id` names the repository, absent being the one
    /// Fleet was started in, and Fleet resolves that repository's own file.
    ///
    /// [`Refusal::Fault`] where the file will not open.
    fn get_manifest_file(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ManifestFile, Refusal>> + Send;

    /// `get_manifest_drift` — whether the repository still has what
    /// `armada.yml` names, line by line.
    ///
    /// **The other half of [`Queries::get_manifest_reading`], and a different
    /// question.** That one reads the *file* and reports what Fleet could not
    /// adopt. A `run` line naming a script somebody deleted parses perfectly
    /// and is adopted without complaint, so the reread says nothing about it
    /// and cannot; this reads the repository around the file.
    ///
    /// **Free, and it has to stay free.** It runs on opening a surface, beside
    /// a dry-run that runs a real test suite behind its own button. Nothing in
    /// the implementation may start a process to find out whether one exists.
    ///
    /// `manifest_id` names the repository, absent being the one Fleet was
    /// started in.
    ///
    /// It cannot refuse: every line gets a verdict, and a line naming nothing
    /// this read can look for is answered as such rather than as a failure.
    /// `Result` is here to match the surface.
    fn get_manifest_drift(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ManifestDrift, Refusal>> + Send;

    /// `get_repository_scan` — every workspace in the checkout, what each
    /// one's files declare, and the file each finding came from.
    ///
    /// **It reads no Manifest**, because it is for a repository that has none.
    /// It writes nothing and runs nothing. It cannot refuse: what would not
    /// read is said in the answer. `Result` is here to match the surface.
    fn get_repository_scan(
        &self,
        repository: Option<String>,
    ) -> impl Future<Output = Result<ipc::RepositoryScan, Refusal>> + Send;

    /// `get_manifest_proposals` — a proposal per workspace Scan finds, held between calls
    /// so edits persist. It cannot refuse; `Result` matches the surface.
    fn get_manifest_proposals(
        &self,
        repository: Option<String>,
    ) -> impl Future<Output = Result<ipc::ManifestProposals, Refusal>> + Send;

    /// `list_workflows` — the workflows Fleet holds, with their steps.
    ///
    /// **What makes a `workflow_id` checkable.** Nothing joined one to a
    /// workflow before this, so a proposal naming an invented id was stored and
    /// the Job sat on the board claiming a workflow Fleet had never heard of.
    /// The refusal at creation is the fix; this is what lets a caller name one
    /// that will not be refused.
    fn list_workflows(&self) -> impl Future<Output = Result<Vec<WorkflowSummary>, Refusal>> + Send;

    /// `get_workflow` — one definition as its file holds it, for an editor.
    ///
    /// `source` names which place's definition of the id: `armada`, `kit` or
    /// `repository`, so a definition a more specific place replaced can still be
    /// read. Absent is the one that runs. Absent `manifest_id` is the repository
    /// Fleet started in.
    fn get_workflow(
        &self,
        workflow_id: ipc::WorkflowId,
        source: Option<String>,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ipc::WorkflowDefinition, Refusal>> + Send;

    /// `list_left_out_workflows` — the Kit and carried definitions one repository runs
    /// without, and why. Absent `manifest_id` is the repository Fleet started in.
    fn list_left_out_workflows(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<Vec<ipc::LeftOutWorkflow>, Refusal>> + Send;

    /// `list_triggers` — what one repository runs at each moment of a Job, each
    /// with its level and file, what it replaced, and the files left out with
    /// why. Absent `manifest_id` is the repository Fleet started in.
    fn list_triggers(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ipc::TriggerList, Refusal>> + Send;

    /// `get_trigger` — one Trigger as its file holds it, for an editor. `level`
    /// absent is the copy that runs; present reads one a more specific level
    /// replaced.
    fn get_trigger(
        &self,
        when: ipc::TriggerMoment,
        step: Option<ipc::StepId>,
        name: String,
        level: Option<ipc::TriggerLevel>,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ipc::TriggerDefinition, Refusal>> + Send;

    /// `list_manifests` — the Manifests Fleet holds, and the repository each
    /// was read from. The counterpart to [`Queries::list_workflows`], for the
    /// other id a proposal names.
    fn list_manifests(&self) -> impl Future<Output = Result<Vec<ManifestSummary>, Refusal>> + Send;

    /// `list_models` — what a Job may be spawned as, and what it gets when it
    /// names nothing.
    fn list_models(&self) -> impl Future<Output = Result<ModelChoices, Refusal>> + Send;

    /// `list_worktrees` — every worktree Fleet is holding disk for, and the
    /// test each one did not pass.
    ///
    /// **The read half of the rule [`Commands::reclaim_worktree`] acts on.**
    /// Fleet gives back what passes all five tests without asking; this is what
    /// the rest is chosen from, and the reasons are the whole point of it — an
    /// unmerged branch, uncommitted files and a Job still moving are answered
    /// differently by the person reading them.
    ///
    /// **Not scoped to a Job.** The question is which of these to give back,
    /// which is asked of the set rather than of any one row.
    ///
    /// **A piloted Job's worktree is not in the answer** — `#367`. It is
    /// dropped where the rule lives rather than hidden by a client, because an
    /// act that is drawn is an act somebody eventually clicks.
    fn list_worktrees(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<WorktreesHeld, Refusal>> + Send;







    /// `get_checkout_run_sheet` — what a person can run in the main checkout:
    /// its Setup, its Checks and its Commands, off the Manifest Fleet holds.
    ///
    /// **It needs no Job and never refuses.** This is the file Fleet is
    /// already running on, so the sheet answers against an empty store — which
    /// is the whole of what Journey 9's *Running one* asks for. `repository`
    /// names one by root instead, which may have no root Manifest.
    fn get_checkout_run_sheet(
        &self,
        manifest_id: Option<ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<CheckoutRunSheet, Refusal>> + Send;


    /// `list_checkout_runs` — the main checkout's earlier runs, newest first,
    /// with the records that would not read answered beside them.
    fn list_checkout_runs(
        &self,
        manifest_id: Option<ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<CheckoutRunList, Refusal>> + Send;

    /// `get_checkout_run_output` — one checkout run's log, as a window that
    /// says it is one. [`Queries::get_run_output`]'s shape and its allowlist:
    /// `run_id` is resolved against the checkout's own runs before a file is
    /// opened. [`Refusal::Unacceptable`] where it names none.
    fn get_checkout_run_output(
        &self,
        run_id: String,
        manifest_id: Option<ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<RunOutput, Refusal>> + Send;

    /// `get_checkout_run_diff` — what one checkout run changed, against the
    /// snapshot it took just before. **A snapshot that is gone is answered
    /// as gone**, never as a patch against `HEAD`, which would show a person's
    /// own uncommitted work as the run's. [`Refusal::Unacceptable`] where
    /// `run_id` names no finished run of the checkout.
    fn get_checkout_run_diff(
        &self,
        run_id: String,
        manifest_id: Option<ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<ipc::CheckoutRunDiff, Refusal>> + Send;

    /// `observe_checkout_run` — one checkout run's output on a socket of its
    /// own, for [`Queries::observe_run`]'s reasons.
    fn observe_checkout_run(
        &self,
        run_id: String,
        manifest_id: Option<ManifestId>,
        repository: Option<String>,
    ) -> impl Future<Output = Result<crate::ObservedCheckoutRun, Refusal>> + Send;

    /// `list_servers` — every server Fleet holds, a Job's and the main
    /// checkout's, and the last instance of each that ended.
    fn list_servers(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<ipc::ServerList, Refusal>> + Send;

    /// `observe_server` — one server's output on a socket of its own, for
    /// [`Queries::observe_run`]'s reasons. [`Refusal::Unacceptable`] where
    /// `server_id` names no instance Fleet holds.
    fn observe_server(
        &self,
        server_id: String,
    ) -> impl Future<Output = Result<crate::ObservedServer, Refusal>> + Send;

    /// `search_files` — paths under the checkout narrowed against typed text,
    /// for Bridge's `@` mention popup.
    ///
    /// `manifest_id` names the checkout, absent being the one Fleet was started
    /// in; this reads it before any Job — and so any worktree — exists.
    ///
    /// It never refuses. A directory it cannot read is skipped rather than
    /// failing the whole search, [`Queries::get_capacity`]'s own reasoning for
    /// answering rather than 500ing over a corner of the machine.
    /// `list_repositories` — every repository this Fleet serves, the one it was
    /// started in first, each with its Manifest where one has loaded.
    fn list_repositories(
        &self,
    ) -> impl Future<Output = Result<ipc::RepositoryList, Refusal>> + Send;

    /// `read_request_repository` — which served repository the words of a
    /// request are about, before any Job exists. It never refuses.
    fn read_request_repository(
        &self,
        words: String,
    ) -> impl Future<Output = Result<ipc::RequestRepository, Refusal>> + Send;

    /// `get_fleet_timeline` — the entries Fleet kept for a window of time,
    /// oldest first. It never refuses.
    fn get_fleet_timeline(
        &self,
        ask: ipc::TimelineAsk,
    ) -> impl Future<Output = Result<ipc::FleetTimeline, Refusal>> + Send;

    /// `get_slot_history` — who held one slot and what was checked out in it, newest
    /// first. Refuses a repository Fleet does not serve.
    fn get_slot_history(
        &self,
        ask: ipc::BayHistoryAsk,
    ) -> impl Future<Output = Result<ipc::BayHistory, Refusal>> + Send;

    /// `get_merge_lines` — each served repository's merge line: the hub
    /// Fleet read off the forge, with the queue lists empty. It never refuses.
    fn get_merge_lines(&self) -> impl Future<Output = Result<ipc::MergeLines, Refusal>> + Send;

    /// `checkout_diff` — one uncommitted file of a served checkout that Fleet
    /// holds back, as a unified diff against `HEAD`. **Only a path the hold
    /// lists is read**: [`Refusal::IllegalMove`] for any other and where nothing
    /// is held; [`Refusal::Unacceptable`] on a repository this Fleet does not
    /// serve; [`Refusal::Fault`] where git would not run.
    fn checkout_diff(
        &self,
        ask: ipc::CheckoutDiffAsk,
    ) -> impl Future<Output = Result<ipc::CheckoutDiff, Refusal>> + Send;

    /// What Fleet read off the forge for each served repository, by root: main's
    /// CI and the open pull requests, which `get_merge_lines` carries as `hub`.
    /// **Empty by default**, for a daemon that reads no forge.
    fn merge_hubs(&self) -> impl Future<Output = Vec<(String, ipc::MergeLineHub)>> + Send {
        async { Vec::new() }
    }

    /// `observe_land_check` — one Check's log on a served repository's merge
    /// line, by the root, the branch and the Check.
    ///
    /// **It answers before the socket opens**, for [`Queries::observe_job`]'s
    /// reason, and the three names are all it takes: the file is found from
    /// the branch's outcome, never from a path the caller sent.
    ///
    /// [`Refusal::Unacceptable`] where the root is not served, the branch has
    /// no outcome, or the Check is not one its turn has started.
    fn observe_land_check(
        &self,
        root: String,
        branch: String,
        check: String,
    ) -> impl Future<Output = Result<crate::LandOutput, Refusal>> + Send;

    fn search_files(
        &self,
        query: String,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<FilesFound, Refusal>> + Send;

    /// `list_import_folders` — the folders a person has already worked in with their agent CLI,
    /// newest session first, read from the sessions it kept. **Read only and bounded**; a home with
    /// no sessions is an empty list, and the only `Refusal` is a fault.
    fn list_import_folders(&self) -> impl Future<Output = Result<ipc::ImportFolders, Refusal>> + Send;

    /// `get_import_runs` — how one of those folders was run, from the commands its sessions ran.
    /// [`Refusal::Unacceptable`] where `path` is not a folder the sessions name.
    fn get_import_runs(&self, path: String) -> impl Future<Output = Result<ipc::ImportRuns, Refusal>> + Send;

    /// `get_run_memory` — how a repository runs, as Fleet remembers it from what its Jobs did, each
    /// fact with where it is kept. [`Refusal::NotFound`] where `repository` is not a root Fleet serves.
    fn get_run_memory(&self, repository: String) -> impl Future<Output = Result<ipc::RunMemory, Refusal>> + Send;

    /// `get_run_memory_place` — where the repository's remembered runs are written; `this_mac`
    /// until a person chooses another.
    fn get_run_memory_place(
        &self,
        repository: String,
    ) -> impl Future<Output = Result<ipc::RunMemoryPlace, Refusal>> + Send;

}
