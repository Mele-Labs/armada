//! Running an app in development, with no Job: a repository's `runs:` started on its main
//! checkout, or on a Job's worktree. `docs/concepts/manifest.md`, *Commands that keep running*.
//!
//! | Module | Holds |
//! |---|---|
//! | [`draft`] | A run proposed from what the repository runs and what its files say |
//! | [`save`] | A confirmed draft written into `armada.yml` or Kit's copy |
//! | [`links`] | A process expecting another on a port it does not get: found, kept, briefed |
//! | [`pins`] | A process pinned to a number, or given back to leasing |
//! | [`serving`] | The wire: [`api::AppRuns`] over these, and `app_run.changed` after each server fact |
//! | [`unrunnable`] | Every refusal, and its code on the wire |
//!
//! **A run is a set of servers and no supervisor of its own.** Each process is started through
//! `hold_server`, so its port, its log, its `server.*` events and its stop are the ones every
//! server has. What a run adds is only the grouping: [`RunView`] is read from the instances the
//! book holds, and [`Fleet::runs_moved_by`] says which runs a server event changed, so the wire
//! can publish a run's new state after each of them.
//!
//! **A run is read from the file when it starts, and Fleet's boot Manifest is not.** A run saved a
//! moment ago has to start with no restart, so [`Fleet::run_manifest`] reads the tree's own
//! `armada.yml` with Kit's copy over it each time a run is listed, started or stopped. **Only runs,
//! their servers and the ports they name are taken from that read.** The rest of what Fleet booted
//! with, Checks and workflows among it, stays as it was resolved: a Check that changed under a
//! running Fleet is not one a gate should start enforcing mid-flight (#1564).
//!
//! **One instance per checkout and run** follows from one per checkout and server: a second
//! start finds each server up and answers with them. A server two runs list is one process, so
//! stopping either run stops it.

pub(crate) mod draft;
pub(crate) mod links;
mod pins;
pub(crate) mod save;
mod serving;
mod unrunnable;

pub use serving::keep_the_runs;

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::{LoadError, Manifest};
use ipc::{ServerPhase, ServerPort, ServerState, StartedBy, WireError};

use crate::checkouts::Checkout;
use crate::repositories::Served;
use crate::daemon::Fleet;
use crate::servers::Holder;
use unrunnable::Unrunnable;

/// Which checkout a run is started on. **A target and never the caller's own Session**, so Bridge's
/// ▶ on a repository and ▶ inside a Job say where they mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RunTarget {
    /// The repository's main checkout, named by its Manifest where there are several repositories.
    Main(Option<ipc::ManifestId>),
    /// The worktree a Job holds, by the Job's id.
    Job(String),
}

/// Where one process of a run stands. A server that exits on its own has failed, whatever its
/// code, so `Exited` is only one somebody stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProcessState {
    /// Nothing of this process has been started on this checkout.
    NotStarted,
    Starting,
    Ready,
    /// Stopped by a person or by Fleet.
    Exited,
    /// Ended on its own, or never started: its `ended` says how.
    Failed,
}

impl ProcessState {
    pub(crate) fn of(server: &ServerState) -> ProcessState {
        match (server.phase, server.stopped) {
            (ServerPhase::Starting, _) => ProcessState::Starting,
            (ServerPhase::Serving, _) => ProcessState::Ready,
            (ServerPhase::Exited, true) => ProcessState::Exited,
            (ServerPhase::Exited, false) => ProcessState::Failed,
        }
    }
}

/// One server of a run: its state, its port and the instance row behind them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunProcess {
    /// The server's name in the Manifest.
    pub(crate) name: String,
    pub(crate) state: ProcessState,
    /// The ports its fields name, `serve` first. Empty before it has started once.
    pub(crate) ports: Vec<ServerPort>,
    /// The live instance, or the last that ended. What `observe_server` and `stop_server` take the
    /// id of.
    pub(crate) server: Option<ServerState>,
    /// The folder under the checkout it starts in. `None` is the root.
    pub(crate) dir: Option<String>,
    /// What starts it, as the Manifest wrote it.
    pub(crate) serve: String,
    /// The port its `serve` names, `${port.NAME}`'s NAME: what `ARMADA_PORT_*` is named for.
    pub(crate) port_name: Option<String>,
    /// The number it is pinned to, where it is.
    pub(crate) pinned: Option<u16>,
}

/// The name in the first `${port.NAME}` of `serve`.
pub(crate) fn port_name_in(serve: &str) -> Option<String> {
    let rest = serve.split_once("${port.")?.1;
    Some(rest.split_once('}')?.0.to_string()).filter(|name| !name.is_empty())
}

impl RunProcess {
    /// Where it answers: the first port its fields name.
    pub(crate) fn port(&self) -> Option<u16> {
        self.ports.first().map(|port| port.port)
    }
}

/// How a run stands as a whole. **Only tests read it**: the wire sends each process's state and a
/// client derives the whole's from them.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunPhase {
    /// Nothing of it is up, and nothing of it fell over.
    Stopped,
    /// Something is starting and nothing has failed.
    Starting,
    /// Every process is ready.
    Running,
    /// Some processes are ready and others are stopped or not started.
    Partial,
    /// A process ended on its own. The rest may still be up.
    Failed,
}

/// One run on one checkout, as it stands now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunView {
    /// The run's name in the Manifest.
    pub(crate) name: String,
    /// What a person is shown: its `label`, else its name.
    pub(crate) label: String,
    /// The repository it runs in, by the id its servers carry.
    pub(crate) manifest_id: ipc::ManifestId,
    /// The checkout it runs on, by path.
    pub(crate) checkout: String,
    /// Whether it is opened at an address: some server of it listens on a leased port and offers a link.
    pub(crate) browser: bool,
    pub(crate) processes: Vec<RunProcess>,
}

#[cfg(test)]
impl RunView {
    pub(crate) fn phase(&self) -> RunPhase {
        let any = |wanted: ProcessState| self.processes.iter().any(|one| one.state == wanted);
        let all_ready = !self.processes.is_empty()
            && self.processes.iter().all(|one| one.state == ProcessState::Ready);
        if any(ProcessState::Failed) {
            RunPhase::Failed
        } else if all_ready {
            RunPhase::Running
        } else if any(ProcessState::Starting) {
            RunPhase::Starting
        } else if any(ProcessState::Ready) {
            RunPhase::Partial
        } else {
            RunPhase::Stopped
        }
    }
}

/// A run's view from the instances held on `path`.
fn viewed(
    manifest: &Manifest,
    servers: &crate::servers::Servers,
    id: &ipc::ManifestId,
    path: &str,
    name: &str,
) -> Option<RunView> {
    let run = manifest.app_run(name)?;
    let holder = Holder::Checkout(path.to_string());
    let browser = run.servers().iter().filter_map(|server| manifest.server(server)).any(|server| {
        !server.links().is_empty() && server.serve().contains("${port")
    });
    let processes = run
        .servers()
        .iter()
        .map(|server| {
            let held = servers.instance(&holder, server);
            let declared = manifest.server(server);
            let serve = declared.map(|one| one.serve().to_string()).unwrap_or_default();
            let port_name = port_name_in(&serve);
            RunProcess {
                name: server.clone(),
                state: held.as_ref().map_or(ProcessState::NotStarted, ProcessState::of),
                ports: held.as_ref().map(|one| one.ports.clone()).unwrap_or_default(),
                server: held,
                dir: declared.and_then(|one| one.dir()).map(str::to_string),
                pinned: port_name.as_deref().and_then(|name| manifest.port(name)).and_then(|port| port.fixed()),
                port_name,
                serve,
            }
        })
        .collect();
    Some(RunView {
        name: name.to_string(),
        label: run.label().unwrap_or(name).to_string(),
        manifest_id: id.clone(),
        checkout: path.to_string(),
        browser,
        processes,
    })
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
    /// The checkout `target` names. **A Job's is read from Fleet's own record of its lease**, never
    /// from a path a caller sent.
    pub(crate) async fn run_checkout(&self, target: &RunTarget) -> Result<Checkout, Refusal> {
        match target {
            RunTarget::Main(manifest_id) => {
                Ok(Checkout::main(self.served_named(manifest_id.as_ref())?))
            }
            RunTarget::Job(id) => {
                let (_, hosting) = self
                    .session_and_hosting(id)
                    .await
                    .map_err(|_| self.run_refusal(Unrunnable::NoSuchJob { id: id.clone() }))?;
                let served = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id)))?;
                if hosting.lease_slot.is_none() {
                    return Err(self.run_refusal(Unrunnable::JobHasNoWorktree { id: id.clone() }));
                }
                let path = Self::directory_of(&served, &hosting);
                Checkout::of_a_job(served, &path).map_err(|why| self.checkout_refusal(why))
            }
        }
    }

    /// Every run the Manifest declares, in the file's order, as each stands on `target`.
    pub(crate) async fn runs_listed(&self, target: &RunTarget) -> Result<Vec<RunView>, Refusal> {
        let checkout = self.run_checkout(target).await?;
        let manifest = self.run_manifest(checkout.served(), checkout.path()).await?;
        let id = ipc::ManifestId::from(checkout.served().manifest().id());
        Ok(manifest
            .app_run_names()
            .iter()
            .filter_map(|name| viewed(&manifest, self.servers(), &id, checkout.path(), name))
            .collect())
    }

    /// One run as it stands on `target`. The wire lists every run, so only tests read one.
    #[cfg(test)]
    pub(crate) async fn run_read(&self, target: &RunTarget, name: &str) -> Result<RunView, Refusal> {
        let checkout = self.run_checkout(target).await?;
        let manifest = self.run_manifest(checkout.served(), checkout.path()).await?;
        self.run_viewed(&manifest, &checkout, name)
    }

    /// Start `name` on `target` — **or answer with the run already up**, `true` beside the view
    /// where this call started at least one of its processes. A process that will not start stops
    /// the ones this call started, so a run is up whole or not at all.
    pub(crate) async fn run_started(
        self: Arc<Self>,
        target: &RunTarget,
        name: &str,
    ) -> Result<(RunView, bool), Refusal> {
        let checkout = self.run_checkout(target).await?;
        self.run_started_on(checkout, name).await
    }

    /// [`run_started`](Fleet::run_started) on a checkout already resolved.
    pub(crate) async fn run_started_on(
        self: Arc<Self>,
        checkout: Checkout,
        name: &str,
    ) -> Result<(RunView, bool), Refusal> {
        let manifest = self.run_manifest(checkout.served(), checkout.path()).await?;
        let run = manifest
            .app_run(name)
            .cloned()
            .ok_or_else(|| self.not_a_run(&manifest, name))?;
        // One run's starts at a time: each reads what ports the others hold before it picks.
        let turn = self.servers().starting();
        let _turn = turn.lock().await;
        let mut started_here: Vec<String> = Vec::new();
        for server in run.servers() {
            let held = Fleet::hold_server_from(
                Arc::clone(&self),
                checkout.clone(),
                &manifest,
                server,
                StartedBy::Person,
                None,
            )
            .await;
            match held {
                Ok((_, true)) => started_here.push(server.clone()),
                Ok((_, false)) => {}
                Err(why) => {
                    self.stopped_servers_named(checkout.path(), &started_here).await;
                    return Err(self.server_refusal(why));
                }
            }
        }
        Ok((self.run_viewed(&manifest, &checkout, name)?, !started_here.is_empty()))
    }

    /// Stop every process of `name` on `target`, and wait for each. **Only this run's servers**: no
    /// other the checkout holds. A run with nothing up answers as it stands.
    pub(crate) async fn run_stopped(&self, target: &RunTarget, name: &str) -> Result<RunView, Refusal> {
        let checkout = self.run_checkout(target).await?;
        self.run_stopped_on(&checkout, name).await
    }

    /// [`run_stopped`](Fleet::run_stopped) on a checkout already resolved.
    pub(crate) async fn run_stopped_on(&self, checkout: &Checkout, name: &str) -> Result<RunView, Refusal> {
        let manifest = self.run_manifest(checkout.served(), checkout.path()).await?;
        let run = manifest
            .app_run(name)
            .cloned()
            .ok_or_else(|| self.not_a_run(&manifest, name))?;
        self.stopped_servers_named(checkout.path(), run.servers()).await;
        self.run_viewed(&manifest, checkout, name)
    }

    /// Stop `name`, then start it again: the run as a whole, never one process of it.
    pub(crate) async fn run_restarted(
        self: Arc<Self>,
        target: &RunTarget,
        name: &str,
    ) -> Result<RunView, Refusal> {
        self.run_stopped(target, name).await?;
        self.run_started(target, name).await.map(|(view, _)| view)
    }

    /// The runs a server's event changed, each as it now stands. **What the wire publishes after
    /// every `server.starting`, `server.serving` and `server.exited`**: the event carries the
    /// instance, and this says whose run it is part of. **Read from what the checkout was last
    /// run from**, not the file again: an event is not a reason to reach Kit's copy where it is kept.
    pub(crate) fn runs_moved_by(&self, state: &ServerState) -> Vec<RunView> {
        let Ok(served) = self.served_named(state.manifest_id.as_ref()) else {
            return Vec::new();
        };
        let seen = self.servers().declared_at(&state.checkout.path);
        let manifest = seen.as_deref().unwrap_or_else(|| served.manifest());
        let id = ipc::ManifestId::from(served.manifest().id());
        manifest
            .app_run_names()
            .iter()
            .filter(|name| {
                manifest
                    .app_run(name)
                    .is_some_and(|run| run.servers().contains(&state.name))
            })
            .filter_map(|name| viewed(manifest, self.servers(), &id, &state.checkout.path, name))
            .collect()
    }

    /// The Manifest runs are read from on the checkout at `path`: **the tree's own `armada.yml`**
    /// where it has one, so a Job that adds a server can run it, else the repository's file, each
    /// with Kit's copy laid over it. A tree with neither falls back to the Manifest Fleet holds.
    /// One that will not read is refused, since what it declares is not known.
    pub(crate) async fn run_manifest(&self, served: &Served, path: &str) -> Result<Manifest, Refusal> {
        let held = served.manifest();
        let own = held.path().file_name().map(|file| Path::new(path).join(file));
        let file = match own {
            Some(own) if path != served.root() && own.is_file() => own,
            _ => held.path().to_path_buf(),
        };
        let copy = self.kit_copy_of(served.root()).await?;
        let read = tokio::task::spawn_blocking(move || Manifest::load_over_copy(&file, copy))
            .await
            .map_err(|why| self.run_refusal(Unrunnable::ManifestUnreadable { why: why.to_string() }))?;
        let manifest = match read {
            Ok(manifest) => manifest,
            Err(LoadError::Unreadable { cause, .. }) if cause.kind() == std::io::ErrorKind::NotFound => held.clone(),
            Err(why) => return Err(self.run_refusal(Unrunnable::ManifestUnreadable { why: why.to_string() })),
        };
        self.servers().declared_seen(path, manifest.clone());
        Ok(manifest)
    }

    fn run_viewed(&self, manifest: &Manifest, checkout: &Checkout, name: &str) -> Result<RunView, Refusal> {
        let id = ipc::ManifestId::from(checkout.served().manifest().id());
        viewed(manifest, self.servers(), &id, checkout.path(), name).ok_or_else(|| self.not_a_run(manifest, name))
    }

    fn not_a_run(&self, manifest: &Manifest, name: &str) -> Refusal {
        self.run_refusal(Unrunnable::NotARun {
            name: name.to_string(),
            runs: manifest.app_run_names().to_vec(),
        })
    }

    fn run_refusal(&self, why: Unrunnable) -> Refusal {
        let (code, refusal) = why.spelled();
        refusal(WireError::raised(code, why.to_string(), self.run_id()))
    }
}
