//! A repository's app run from Bridge: listed, drafted, started, stopped, restarted and saved.
//! `docs/concepts/manifest.md`, *Commands that keep running*.
//!
//! **Its own surface**, for [`Requests`](super::Requests)' reason: whole on its own. Each run is a
//! set of servers Fleet already holds, so the output of a process is `observe_server`'s.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    AppRun, AppRunDraft, AppRunList, ManifestId, NamedAppRun, PinAppRunPort, SaveAppRun, SavedAppRun, UnpinAppRunPort,
};

use crate::daemon::Refusal;

pub trait AppRuns: Send + Sync + 'static {
    /// `list_app_runs` — every run the Manifest declares, on the Job's worktree where `job_id`
    /// names one, else on the main checkout of `manifest_id`.
    fn list_app_runs(
        &self,
        manifest_id: Option<ManifestId>,
        job_id: Option<String>,
    ) -> impl Future<Output = Result<AppRunList, Refusal>> + Send;

    /// `get_app_run_draft` — the processes proposed that no saved run already starts. Saves
    /// nothing, and answers where the repository already declares runs.
    fn get_app_run_draft(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<AppRunDraft, Refusal>> + Send;

    /// `start_app_run` — every server of the run, **or the run already up**.
    ///
    /// **By `Arc`, for `start_server`'s reason**: each server is a task of its own and outlives
    /// the request. It answers with the processes `starting`.
    fn start_app_run(
        self: Arc<Self>,
        manifest_id: Option<ManifestId>,
        asked: NamedAppRun,
    ) -> impl Future<Output = Result<AppRun, Refusal>> + Send;

    /// `stop_app_run` — every server of the run, each waited for.
    fn stop_app_run(
        &self,
        manifest_id: Option<ManifestId>,
        asked: NamedAppRun,
    ) -> impl Future<Output = Result<AppRun, Refusal>> + Send;

    /// `restart_app_run` — stop it, then start it again.
    fn restart_app_run(
        self: Arc<Self>,
        manifest_id: Option<ManifestId>,
        asked: NamedAppRun,
    ) -> impl Future<Output = Result<AppRun, Refusal>> + Send;

    /// `save_app_run` — write a confirmed draft into the person's copy in Kit, then start it.
    fn save_app_run(
        self: Arc<Self>,
        manifest_id: Option<ManifestId>,
        asked: SaveAppRun,
    ) -> impl Future<Output = Result<SavedAppRun, Refusal>> + Send;

    /// `pin_app_run_port` — pin a process to the number its siblings expect it on, in the person's
    /// copy in Kit. Answers the run as it stands: **nothing is restarted**.
    fn pin_app_run_port(
        &self,
        manifest_id: Option<ManifestId>,
        asked: PinAppRunPort,
    ) -> impl Future<Output = Result<AppRun, Refusal>> + Send;

    /// `unpin_app_run_port` — give a process back to leasing.
    fn unpin_app_run_port(
        &self,
        manifest_id: Option<ManifestId>,
        asked: UnpinAppRunPort,
    ) -> impl Future<Output = Result<AppRun, Refusal>> + Send;
}
