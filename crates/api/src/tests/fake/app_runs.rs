//! App runs, as the fake answers them: nothing is run or kept. One run, `dev`, is the one it holds,
//! and its row says which checkout and repository the request named.

use std::sync::Arc;

use ipc::{
    AppRun, AppRunCheckout, AppRunDraft, AppRunList, AppRunOpensIn, AppRunProcess, AppRunState,
    ManifestId, NamedAppRun, PinAppRunPort, RunPlace, SaveAppRun, SavedAppRun, UnpinAppRunPort,
};

use super::FakeDaemon;
use crate::{AppRuns, Refusal};

/// The one run the fake holds; any other name is not a run.
pub const THE_RUN: &str = "dev";

fn the_run(manifest_id: Option<ManifestId>, job_id: Option<String>, state: AppRunState) -> AppRun {
    AppRun {
        name: THE_RUN.to_string(),
        label: "Dev".to_string(),
        opens_in: AppRunOpensIn::App,
        manifest_id: manifest_id.unwrap_or_else(|| ManifestId::carried("01MF")),
        checkout: match job_id {
            Some(job_id) => AppRunCheckout::Job {
                path: "/repos/armada/.armada/slots/slot-1".to_string(),
                job_id,
                branch: None,
                bay: 1,
            },
            None => AppRunCheckout::Main { path: "/repos/armada".to_string() },
        },
        processes: vec![AppRunProcess {
            name: "web".to_string(),
            state,
            command: "serve ${port}".to_string(),
            dir: None,
            port_env: None,
            port: None,
            pinned_port: None,
            links: Vec::new(),
            server_id: None,
            log_path: None,
            ended: None,
        }],
        last_used: None,
        links: Vec::new(),
        setup: None,
        in_kit: true,
        setup_commands: Vec::new(),
    }
}

fn not_a_run(name: &str) -> Refusal {
    Refusal::Unacceptable(ipc::WireError::raised(
        "fleet.not_an_app_run",
        format!("`{name}` is not a run this project declares"),
        super::run_id(),
    ))
}

fn named(asked: &NamedAppRun) -> Result<(), Refusal> {
    match asked.name == THE_RUN {
        true => Ok(()),
        false => Err(not_a_run(&asked.name)),
    }
}

impl AppRuns for FakeDaemon {
    async fn list_app_runs(
        &self,
        manifest_id: Option<ManifestId>,
        job_id: Option<String>,
    ) -> Result<AppRunList, Refusal> {
        Ok(AppRunList { runs: vec![the_run(manifest_id, job_id, AppRunState::NotStarted)] })
    }

    async fn get_app_run_draft(&self, _manifest_id: Option<ManifestId>) -> Result<AppRunDraft, Refusal> {
        Ok(AppRunDraft { root: "/repos/armada".to_string(), place: RunPlace::ThisMac, processes: Vec::new(), links: Vec::new(), setup: Vec::new() })
    }

    async fn start_app_run(
        self: Arc<Self>,
        manifest_id: Option<ManifestId>,
        asked: NamedAppRun,
    ) -> Result<AppRun, Refusal> {
        named(&asked)?;
        Ok(the_run(manifest_id, asked.job_id, AppRunState::Starting))
    }

    async fn stop_app_run(
        &self,
        manifest_id: Option<ManifestId>,
        asked: NamedAppRun,
    ) -> Result<AppRun, Refusal> {
        named(&asked)?;
        Ok(the_run(manifest_id, asked.job_id, AppRunState::Exited))
    }

    async fn restart_app_run(
        self: Arc<Self>,
        manifest_id: Option<ManifestId>,
        asked: NamedAppRun,
    ) -> Result<AppRun, Refusal> {
        named(&asked)?;
        Ok(the_run(manifest_id, asked.job_id, AppRunState::Starting))
    }

    async fn save_app_run(
        self: Arc<Self>,
        manifest_id: Option<ManifestId>,
        asked: SaveAppRun,
    ) -> Result<SavedAppRun, Refusal> {
        Ok(SavedAppRun {
            place: RunPlace::ThisMac,
            file: "armada.yml".to_string(),
            run: AppRun { name: asked.name, ..the_run(manifest_id, asked.job_id, AppRunState::Starting) },
        })
    }

    async fn pin_app_run_port(&self, manifest_id: Option<ManifestId>, asked: PinAppRunPort) -> Result<AppRun, Refusal> {
        Ok(the_run(manifest_id, asked.job_id, AppRunState::NotStarted))
    }

    async fn unpin_app_run_port(&self, manifest_id: Option<ManifestId>, asked: UnpinAppRunPort) -> Result<AppRun, Refusal> {
        Ok(the_run(manifest_id, asked.job_id, AppRunState::NotStarted))
    }
}
