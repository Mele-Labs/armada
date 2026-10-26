//! [`AppRuns`] for Fleet: the run domain in [`super`] put on the wire, and `app_run.changed` after
//! each server fact. **Only the conversion lives here**: what a run is, and what starting it does,
//! is the module above's.
//!
//! **A run's event follows its server's and is never sent in the same breath.** The three `server.*`
//! facts are published under the row's own lock, and a run is read from every sibling row in the
//! book, so reading it there would wait on a lock held by the caller. A tap hands the fact to a task
//! that reads once the lock is let go (`keep_the_runs`), so the last event processed for a run is
//! also the freshest reading of it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{AppRuns, Refusal};
use ipc::{
    AppRun, AppRunCheckout, AppRunDraft, AppRunDraftProcess, AppRunDraftSetup, AppRunLink, AppRunList, AppRunOpensIn,
    AppRunProcess, AppRunSetup, AppRunSetupState, AppRunState, Event, ManifestId, NamedAppRun, PinAppRunPort, SaveAppRun,
    SavedAppRun, ServerState, UnpinAppRunPort,
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::draft::{safe_name, DraftPort, DraftProcess, DraftSetup, DraftSource, RunDraft};
use super::links::{self, notice, Link};
use super::{ProcessState, RunProcess, RunTarget, RunView, SetupState, SetupView};
use crate::daemon::Fleet;

/// What stands in a command for the port Fleet leases the checkout. **The one spelling a person
/// sees**: a draft's own `${port.NAME}` is the Manifest's, and the name is the process's.
const PORT: &str = "${port}";

/// Start telling Bridge how each run stands. Production starts it beside the timeline's, and a test
/// starts it itself.
pub fn keep_the_runs<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let (tell, mut heard) = mpsc::unbounded_channel::<ServerState>();
    fleet.events().tap(move |event| {
        if let Event::ServerStarting(state) | Event::ServerServing(state) | Event::ServerExited(state) = event {
            let _ = tell.send(state.clone());
        }
    });
    // Output is read where Fleet already reads it for `observe_server`: this only keeps the lines
    // that name a refused port, once for each port of each instance.
    let (refuse, mut refused) = mpsc::unbounded_channel::<(ServerState, u16)>();
    let told: std::sync::Mutex<std::collections::HashSet<(String, u16)>> = Default::default();
    fleet.servers().lines_tapped(Arc::new(move |state, lines| {
        for port in lines.iter().filter_map(|(_, line)| notice::refused_port(line)) {
            let first = {
                let mut told = told.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                if told.len() >= MOST_REFUSALS_KEPT {
                    told.clear();
                }
                told.insert((state.id.clone(), port))
            };
            if first {
                let _ = refuse.send((state.clone(), port));
            }
        }
    }));
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(state) = heard.recv() => {
                    for view in fleet.runs_moved_by(&state) {
                        if let Some(run) = fleet.app_run_wired(view, None).await {
                            fleet.publish(Event::AppRunChanged(run));
                        }
                    }
                }
                Some((state, port)) = refused.recv() => fleet.links_noticed(&state, port).await,
                else => break,
            }
        }
    })
}

/// The most (instance, port) pairs the tap remembers having told, so a Fleet that runs for weeks
/// holds a bounded set.
const MOST_REFUSALS_KEPT: usize = 512;

/// A target from the two things a request names: a Job's worktree, else the repository's main.
fn target_of(manifest_id: Option<ManifestId>, job_id: Option<String>) -> RunTarget {
    match job_id.map(|id| id.trim().to_string()).filter(|id| !id.is_empty()) {
        Some(job) => RunTarget::Job(job),
        None => RunTarget::Main(manifest_id),
    }
}

fn state_of(process: &RunProcess) -> AppRunState {
    match process.state {
        ProcessState::NotStarted => AppRunState::NotStarted,
        ProcessState::Starting => AppRunState::Starting,
        ProcessState::Ready => AppRunState::Ready,
        ProcessState::Exited => AppRunState::Exited,
        ProcessState::Failed => AppRunState::Failed,
    }
}

/// `records_root` is where each instance's `log` is read from.
fn process_wired(process: &RunProcess, records_root: Option<&str>) -> AppRunProcess {
    let ready = process.state == ProcessState::Ready;
    let command = match &process.port_name {
        Some(port) => process.serve.replace(&format!("${{port.{port}}}"), PORT),
        None => process.serve.clone(),
    };
    AppRunProcess {
        name: process.name.clone(),
        state: state_of(process),
        command,
        dir: process.dir.clone(),
        port_env: process.port_env.clone(),
        port: process.port(),
        pinned_port: process.pinned,
        links: process.server.as_ref().filter(|_| ready).map(|one| one.links.clone()).unwrap_or_default(),
        server_id: process.server.as_ref().map(|one| one.id.clone()),
        log_path: process
            .server
            .as_ref()
            .zip(records_root)
            .map(|(one, root)| std::path::Path::new(root).join(&one.log).display().to_string()),
        ended: process.server.as_ref().and_then(|one| one.ended.clone()),
        exit_code: process.server.as_ref().and_then(|one| one.exit_code),
    }
}

fn setup_wired(setup: &SetupView) -> AppRunSetup {
    AppRunSetup {
        id: setup.id.clone(),
        log_path: setup.log_path.clone(),
        state: match setup.state {
            SetupState::SettingUp => AppRunSetupState::SettingUp,
            SetupState::Ready => AppRunSetupState::Ready,
            SetupState::Failed => AppRunSetupState::Failed,
        },
        commands: setup.commands.clone(),
        running: setup.running.clone(),
        failed: setup.failed.clone(),
        ended: setup.ended.clone(),
        exit_code: setup.exit_code,
    }
}

/// A link as the wire spells it. `port_of` names the port a process listens on, which
/// `ARMADA_PORT_*` is named for.
fn link_wired(link: &Link, port_of: impl Fn(&str) -> Option<String>) -> AppRunLink {
    let port = port_of(&link.to).unwrap_or_else(|| link.to.clone());
    AppRunLink {
        from: link.from.clone(),
        to: link.to.clone(),
        port: link.port,
        found_in: link.found_in.clone(),
        brief: links::brief(link, &port),
    }
}

/// A proposed process as a person reads it: its own port written as `${port}`.
fn draft_process_wired(process: &DraftProcess) -> AppRunDraftProcess {
    let command = match &process.port {
        Some(port) => process.serve.replace(&format!("${{port.{}}}", port.name), PORT),
        None => process.serve.clone(),
    };
    AppRunDraftProcess {
        name: process.name.clone(),
        dir: process.dir.clone(),
        command,
        port_env: process.port.as_ref().and_then(|port| port.env.clone()),
    }
}

/// A confirmed process as the draft Fleet saves: **a command holding `${port}` listens, on a port
/// named for the process**, and one without it listens on none.
fn draft_process_read(process: AppRunDraftProcess) -> DraftProcess {
    let listens = process.command.contains(PORT);
    DraftProcess {
        serve: process.command.replace(PORT, &format!("${{port.{}}}", process.name)),
        port: listens.then(|| DraftPort { name: process.name.clone(), env: process.port_env }),
        name: process.name,
        dir: process.dir,
        from: DraftSource::Confirmed,
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The checkout a run's path is: the repository's main, or the worktree a Job holds. **`job` is
    /// the Job a request named**, so its answer needs no search; an event names only the path, and
    /// the Job holding it is found from Fleet's record of leases. `None` for any other path, which
    /// no run is asked for.
    async fn app_run_checkout(&self, manifest_id: &ManifestId, path: &str, job: Option<&str>) -> Option<AppRunCheckout> {
        let served = self.served_named(Some(manifest_id)).ok()?;
        if path == served.root() {
            return Some(AppRunCheckout::Main { path: path.to_string() });
        }
        let hostings = self.store().lock().await.hostings().ok()?;
        hostings.into_iter().filter(|held| served.owns(&held.manifest_id)).find_map(|held| {
            let bay = held.lease_slot?;
            let here = match job {
                Some(job) => job == held.session_id,
                None => {
                    let slot = adapter_traits::slot_path(served.root(), bay);
                    slot == path || std::fs::canonicalize(&slot).is_ok_and(|real| real.to_string_lossy() == path)
                }
            };
            here.then(|| AppRunCheckout::Job {
                path: path.to_string(),
                job_id: held.session_id,
                branch: held.lease_branch,
                bay,
            })
        })
    }

    /// A run as the wire spells it. `None` where its checkout is none a run is asked for.
    pub(crate) async fn app_run_wired(&self, view: RunView, job: Option<&str>) -> Option<AppRun> {
        let checkout = self.app_run_checkout(&view.manifest_id, &view.checkout, job).await?;
        let records_root = self.served_named(Some(&view.manifest_id)).ok().map(|served| served.records_root().to_string());
        // Instants are fixed-width RFC 3339 in UTC, so the text orders as the time does.
        let last_used = view.processes.iter().filter_map(|one| one.server.as_ref()).map(|one| &one.started_at).max();
        let port_of = |name: &str| view.processes.iter().find(|one| one.name == name).and_then(|one| one.port_name.clone());
        let links = self.links_shown(&view).await.iter().map(|link| link_wired(link, port_of)).collect();
        Some(AppRun {
            name: view.name,
            label: view.label,
            opens_in: if view.browser { AppRunOpensIn::Browser } else { AppRunOpensIn::App },
            manifest_id: view.manifest_id,
            checkout,
            processes: view.processes.iter().map(|one| process_wired(one, records_root.as_deref())).collect(),
            last_used: last_used.cloned(),
            links,
            setup: view.setup.as_ref().map(setup_wired),
            in_kit: view.in_kit,
            setup_commands: view
                .setup_commands
                .iter()
                .map(|(name, run)| AppRunDraftSetup { name: name.clone(), command: run.clone() })
                .collect(),
        })
    }

    /// [`app_run_answered`](Fleet::app_run_answered), and the run told to every Bridge too: **a pin
    /// changes a row no server event would**, since nothing was restarted.
    async fn app_run_told(&self, view: RunView, target: &RunTarget) -> Result<AppRun, Refusal> {
        let run = self.app_run_answered(view, target).await?;
        self.publish(Event::AppRunChanged(run.clone()));
        Ok(run)
    }

    /// A run Fleet has just read on a target the request named, which always has a checkout.
    async fn app_run_answered(&self, view: RunView, target: &RunTarget) -> Result<AppRun, Refusal> {
        let job = match target {
            RunTarget::Job(job) => Some(job.as_str()),
            RunTarget::Main(_) => None,
        };
        let path = view.checkout.clone();
        // Read the folders on a request and never on an event: an event is told what this read found.
        self.links_refreshed(&view).await;
        self.app_run_wired(view, job).await.ok_or_else(|| {
            self.run_refusal(super::unrunnable::Unrunnable::NotSaved {
                why: format!("{path} is no checkout of this repository that Fleet holds a run on"),
            })
        })
    }
}

impl<H, V, W> AppRuns for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn list_app_runs(
        &self,
        manifest_id: Option<ManifestId>,
        job_id: Option<String>,
    ) -> Result<AppRunList, Refusal> {
        let target = target_of(manifest_id, job_id);
        let mut runs = Vec::new();
        for view in self.runs_listed(&target).await? {
            runs.push(self.app_run_answered(view, &target).await?);
        }
        Ok(AppRunList { runs })
    }

    async fn get_app_run_draft(&self, manifest_id: Option<ManifestId>) -> Result<AppRunDraft, Refusal> {
        let root = self.served_named(manifest_id.as_ref())?.root().to_string();
        let drafted = self.run_drafted(manifest_id.as_ref()).await?;
        let port_of = |name: &str| drafted.ports.get(name).cloned();
        Ok(AppRunDraft {
            place: self.kit_place_of(&root).await?,
            processes: drafted.draft.iter().flat_map(|draft| &draft.processes).map(draft_process_wired).collect(),
            links: drafted.links.iter().map(|link| link_wired(link, port_of)).collect(),
            setup: drafted
                .draft
                .iter()
                .flat_map(|draft| &draft.setup)
                .map(|one| AppRunDraftSetup { name: one.name.clone(), command: one.run.clone() })
                .collect(),
            root,
        })
    }

    async fn start_app_run(self: Arc<Self>, manifest_id: Option<ManifestId>, asked: NamedAppRun) -> Result<AppRun, Refusal> {
        let target = target_of(manifest_id, asked.job_id);
        let (view, _) = Arc::clone(&self).run_started(&target, &asked.name).await?;
        self.app_run_answered(view, &target).await
    }

    async fn stop_app_run(&self, manifest_id: Option<ManifestId>, asked: NamedAppRun) -> Result<AppRun, Refusal> {
        let target = target_of(manifest_id, asked.job_id);
        let view = self.run_stopped(&target, &asked.name).await?;
        self.app_run_answered(view, &target).await
    }

    async fn restart_app_run(self: Arc<Self>, manifest_id: Option<ManifestId>, asked: NamedAppRun) -> Result<AppRun, Refusal> {
        let target = target_of(manifest_id, asked.job_id);
        let view = Arc::clone(&self).run_restarted(&target, &asked.name).await?;
        self.app_run_answered(view, &target).await
    }

    async fn save_app_run(self: Arc<Self>, manifest_id: Option<ManifestId>, asked: SaveAppRun) -> Result<SavedAppRun, Refusal> {
        let words = asked.name.trim().to_string();
        let draft = RunDraft {
            name: safe_name(&words),
            label: Some(words).filter(|words| !words.is_empty()),
            processes: asked.processes.into_iter().map(draft_process_read).collect(),
            setup: asked
                .setup
                .into_iter()
                .map(|one| DraftSetup { name: one.name.trim().to_string(), run: one.command, creates: None })
                .collect(),
            replaces: asked.replaces.map(|one| one.trim().to_string()),
            replacing: None,
        };
        // A Job's run is saved for the Job's repository, which `?manifest_id=` need not name.
        let target = target_of(manifest_id.clone(), asked.job_id);
        let repository = match &target {
            RunTarget::Job(_) => Some(ManifestId::from(self.run_checkout(&target).await?.served().manifest().id())),
            RunTarget::Main(_) => manifest_id,
        };
        let saved = self.run_saved(repository.as_ref(), draft).await?;
        // A run saved over one Kit's copy held stops what it started, then starts as it now reads.
        if let Some(run) = &saved.replaced_run {
            let checkout = self.run_checkout(&target).await?;
            self.servers().setups().stopped(checkout.path(), run).await;
            self.stopped_servers_named(checkout.path(), &saved.replaced).await;
        }
        // **Saved, then started**: a start that refuses leaves the run written, and says so itself.
        let (view, _) = Arc::clone(&self).run_started(&target, &saved.name).await?;
        Ok(SavedAppRun {
            place: saved.saved_in.place,
            file: saved.saved_in.file,
            run: self.app_run_answered(view, &target).await?,
        })
    }

    async fn pin_app_run_port(&self, manifest_id: Option<ManifestId>, asked: PinAppRunPort) -> Result<AppRun, Refusal> {
        let target = target_of(manifest_id, asked.job_id);
        let view = self.run_port_pinned(&target, &asked.process, asked.name.as_deref(), Some(asked.port)).await?;
        self.app_run_told(view, &target).await
    }

    async fn unpin_app_run_port(&self, manifest_id: Option<ManifestId>, asked: UnpinAppRunPort) -> Result<AppRun, Refusal> {
        let target = target_of(manifest_id, asked.job_id);
        let view = self.run_port_pinned(&target, &asked.process, asked.name.as_deref(), None).await?;
        self.app_run_told(view, &target).await
    }
}
