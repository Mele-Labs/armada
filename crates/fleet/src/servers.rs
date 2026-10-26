//! Servers: a Command with `serve`, held by Fleet — `docs/concepts/fleet.md`,
//! *Servers*, and `docs/concepts/manifest.md`, *Commands that keep running*.
//!
//! | Module | Holds |
//! |---|---|
//! | [`held`] | Which servers are up, by holder and name, and the last that ended |
//! | [`launched`] | Starting one: its row, its log directory and its task |
//! | [`left`] | The record on disk that finds a server a crashed Fleet left running |
//! | [`moved`] | What a held server serves, once the checkout moved past it |
//! | [`running`] | One server: `run`, `serve`, `ready`, the end |
//! | [`unservable`] | Every refusal, and its code on the wire |
//!
//! **One instance per holder per name**, and a second request is answered with
//! the one already up. **Off the turn loop**:
//! each is a task of its own on the `Arc` the listener holds.
//!
//! **Teardown, then release.** A slot's servers are stopped before it is given
//! back, and every server is stopped when Fleet stops, before the main
//! checkout's span goes. A
//! server outliving the memory that held it would hold its port with nothing
//! left to hand it on or stop it.

mod held;
mod launched;
pub(crate) mod left;
mod moved;
mod running;
mod unservable;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ServerEntry, ServerLink, ServerPhase, ServerPort, ServerState, StartedBy, WireError,
};

use crate::checkouts::Checkout;
use crate::daemon::Fleet;
use crate::rehearsing::records;
pub(crate) use held::{Holder, Servers};
pub(crate) use launched::Launch;
pub use unservable::Unservable;

/// A path that is not a checkout of the repository a server was asked for.
const NOT_A_CHECKOUT: &str = "fleet.not_a_checkout";

/// How long a stop, or a slot's teardown, waits for a server to end. The group
/// is ended with `SIGKILL`, so what is left is a reap and a publish.
const STOPPING: Duration = Duration::from_secs(30);

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
    /// Start `name` in `checkout` — its main checkout, or a worktree somebody
    /// cut by hand, on the span that checkout holds while Fleet runs —
    /// **or answer with the instance already up**: `true` beside the state
    /// where this call started it. It reads the Manifest Fleet holds.
    /// `${port.NAME}` is resolved in every field from the span the server runs
    /// under, and the claim's variables ride in its environment.
    pub(crate) async fn hold_server(
        self: Arc<Self>,
        checkout: Checkout,
        name: &str,
        by: StartedBy,
        session: Option<String>,
    ) -> Result<(ServerState, bool), Unservable> {
        let declared = checkout.served().manifest().clone();
        self.hold_server_from(checkout, &declared, name, by, session).await
    }

    /// [`hold_server`](Fleet::hold_server) reading the server from `manifest`, which is not
    /// the one Fleet holds: **a run is read fresh when it starts**, so one saved a moment ago
    /// starts without a restart. Its ports are claimed or widened to fit `manifest`.
    pub(crate) async fn hold_server_from(
        self: Arc<Self>,
        checkout: Checkout,
        manifest: &config::Manifest,
        name: &str,
        by: StartedBy,
        session: Option<String>,
    ) -> Result<(ServerState, bool), Unservable> {
        let served = checkout.served().clone();
        let Some(server) = manifest.server(name).cloned() else {
            return Err(match manifest.command(name) {
                Some(_) => Unservable::IsACommand {
                    name: name.to_string(),
                },
                None => Unservable::NotAServer {
                    name: name.to_string(),
                    servers: manifest.server_names(),
                    last_read: served
                        .repository()
                        .reading()
                        .map(|reading| unservable::LastRead {
                            at: reading.at.as_str().to_string(),
                            at_restart: reading.at_restart,
                        }),
                },
            });
        };
        let ports = self
            .checkout_ports_declared(checkout.path(), served.manifest(), manifest)
            .await
            .map_err(|why| Unservable::NoPorts { why: why.to_string() })?;
        let env = match crate::ports::env_names(manifest) {
            Ok(names) => crate::ports::env_vars(&names, &ports),
            Err(_) => Vec::new(),
        };
        let (holder, worktree, under, at) = (
            Holder::Checkout(checkout.path().to_string()),
            // A server's own folder under the checkout, where it declares one.
            match server.dir() {
                Some(dir) => Path::new(checkout.path()).join(dir),
                None => PathBuf::from(checkout.path()),
            },
            checkout.under(),
            ipc::ServerCheckout {
                path: checkout.path().to_string(),
                branch: checkout.branch().map(str::to_string),
                commit: self.commit_serving_now(&served).await,
                // Nothing has landed since a server that has not started.
                behind: Some(0),
            },
        );
        let resolved = |text: &str| crate::ports::resolve_ports(text, &ports);
        let launch = Launch {
            holder,
            name: name.to_string(),
            manifest_id: ipc::ManifestId::from(served.manifest().id()),
            checkout: at,
            under,
            ports: named_ports(&server, &ports),
            links: server
                .links()
                .iter()
                .map(|link| ServerLink {
                    url: resolved(link.url()),
                    name: link.name().map(str::to_string),
                })
                .collect(),
            by,
            session,
            whose: checkout.whose(),
            run: server.run().map(resolved),
            serve: resolved(server.serve()),
            ready: server.ready().map(resolved),
            worktree,
            env,
        };
        self.launched(served.records_root(), launch).await
    }

    /// End an instance's group, and answer with it once it has ended.
    pub(crate) async fn stopped_server(&self, id: &str) -> Result<ServerState, Unservable> {
        let Some((stop, mut now)) = self.servers().stopping(id) else {
            return Err(match self.servers().has_ended(id) {
                true => Unservable::NotRunning { id: id.to_string() },
                false => Unservable::NoSuchServer { id: id.to_string() },
            });
        };
        let _ = stop.send(true);
        let ended = tokio::time::timeout(
            STOPPING,
            now.wait_for(|seen| seen.phase == ServerPhase::Exited),
        )
        .await;
        match ended {
            Ok(Ok(seen)) => Ok(seen.clone()),
            _ => Err(Unservable::NotKept {
                why: String::from("the server did not end after it was stopped"),
            }),
        }
    }

    /// Every instance Fleet holds, and the last of each that ended.
    pub(crate) fn server_list(&self) -> ipc::ServerList {
        ipc::ServerList {
            servers: self.servers().every(),
        }
    }

    /// An instance's socket, resolved before it opens: **the subscription
    /// first, then the log as history**. An ended one opens too, with its
    /// whole log — which is how one that fell over is read.
    pub(crate) fn observed_server(&self, id: &str) -> Result<api::ObservedServer, Unservable> {
        let Some((state, live, dir)) = self.servers().watching(id) else {
            return Err(Unservable::NoSuchServer { id: id.to_string() });
        };
        let log = dir.join(records::LOG);
        let (history, skipped, read_to, unreadable) = match records::history(&log, live.is_none()) {
            Some((history, skipped, read_to)) => (history, skipped, read_to, false),
            None => (Vec::new(), 0, 0, true),
        };
        Ok(api::ObservedServer {
            id: state.id,
            name: state.name,
            path: state.log,
            live,
            history,
            skipped,
            read_to,
            unreadable,
        })
    }

    /// The servers `manifest` declares, each with `holder`'s instance.
    pub(crate) fn declared_servers(
        &self,
        holder: &Holder,
        manifest: &config::Manifest,
    ) -> Vec<ServerEntry> {
        manifest
            .server_names()
            .into_iter()
            .filter_map(|name| {
                let server = manifest.server(&name)?;
                Some(ServerEntry {
                    run: server.run().map(str::to_string),
                    serve: server.serve().to_string(),
                    ready: server.ready().map(str::to_string),
                    links: server
                        .links()
                        .iter()
                        .map(|link| ServerLink {
                            url: link.url().to_string(),
                            name: link.name().map(str::to_string),
                        })
                        .collect(),
                    destructive: server.is_destructive(),
                    instance: self.servers().instance(holder, &name),
                    name,
                })
            })
            .collect()
    }

    /// Stop the servers held for the checkout at `path`, and wait for each: a
    /// slot about to be given back has no one left to serve. **By where they
    /// run**, which is the only record there is of a server in a slot: a
    /// server's `StartedBy` says a person or a Drone, never a session.
    pub(crate) async fn stopped_servers_in(&self, path: &str) {
        stopped_in(self.servers().clone(), path).await;
    }

    /// Stop the servers named, in the checkout at `path`, and wait for each. **A run stopped as a
    /// whole**: only its own servers go, never another the checkout holds.
    pub(crate) async fn stopped_servers_named(&self, path: &str, names: &[String]) {
        let held = self.servers().held_named(&Holder::Checkout(path.to_string()), names);
        stopped_and_waited(held).await;
    }

    /// Stop every server Fleet holds. **Called
    /// once, as Fleet stops, before the main checkout's span is released** —
    /// `pub` for `released_checkout_ports`' reason: the composition root
    /// is what calls it. See `armada::serve`.
    pub async fn stopped_every_server(&self) {
        stopped_and_waited(self.servers().held_by(None)).await;
    }

    /// End what a crashed Fleet left running — **first thing at startup,
    /// before the main checkout's span is re-probed**, so the ports those
    /// servers held are free when it is. See [`left`].
    pub(crate) async fn reaped_left_servers(&self, records_root: &str) -> left::Reaped {
        let root = records_root.to_string();
        tokio::task::spawn_blocking(move || left::reaped(&root))
            .await
            .unwrap_or_default()
    }

    /// `Commands::start_server`: a person starting one in a checkout, or a Session's agent through the
    /// door, which `caller` places and the server then names. **The `Arc` is handed on**, so the server is
    /// a task of its own.
    ///
    /// **Not `budgeted`**: a server answers through its own refusal.
    pub(crate) async fn start_server_answered(
        self: Arc<Self>,
        caller: Option<api::Caller>,
        asked: ipc::StartServer,
        manifest_id: Option<ipc::ManifestId>,
    ) -> Result<ServerState, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let session = caller.and_then(|caller| self.session_holding(&caller));
        let checkout = match asked.checkout.as_deref() {
            Some(path) => {
                Checkout::beside(served, path).map_err(|why| self.checkout_refusal(why))?
            }
            None => Checkout::main(served),
        };
        let refusing = Arc::clone(&self);
        let state = Fleet::hold_server(Arc::clone(&self), checkout, &asked.name, ipc::StartedBy::Person, session)
            .await
            .map(|(state, _)| state)
            .map_err(|why| refusing.server_refusal(why))?;
        if let (Some(session), Some((url, title))) = (&state.session_id, page_of(&state)) {
            let step = self.live_page_seen(session.as_str(), &url, &title);
            self.publish_step(session.as_str().to_string(), step);
        }
        Ok(state)
    }

    /// A path that is not a checkout of the repository asked about. **A 422**,
    /// `NotAServer`'s shape: a name that names nothing rather than a conflict
    /// with where things stand.
    pub(crate) fn checkout_refusal(&self, why: crate::checkouts::NotACheckout) -> Refusal {
        Refusal::Unacceptable(WireError::raised(
            NOT_A_CHECKOUT,
            why.to_string(),
            self.run_id(),
        ))
    }

    pub(crate) fn server_refusal(&self, why: Unservable) -> Refusal {
        let (code, refusal) = why.spelled();
        refusal(WireError::raised(code, why.to_string(), self.run_id()))
    }
}

/// [`Fleet::stopped_servers_in`], for a task that holds the book and not Fleet.
pub(crate) async fn stopped_in(servers: Servers, path: &str) {
    let path = std::fs::canonicalize(path)
        .map(|real| real.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string());
    stopped_and_waited(servers.held_by(Some(&Holder::Checkout(path)))).await;
}

/// Signal every one, then wait for every one: the stops overlap rather than
/// queue.
async fn stopped_and_waited(held: Vec<held::Stopping>) {
    for (stop, _) in &held {
        let _ = stop.send(true);
    }
    for (_, mut now) in held {
        let _ = tokio::time::timeout(
            STOPPING,
            now.wait_for(|seen| seen.phase == ServerPhase::Exited),
        )
        .await;
    }
}

/// Where a server's page is and what to call it: its first link, else its first port on `localhost`.
fn page_of(state: &ServerState) -> Option<(String, String)> {
    match state.links.first() {
        Some(link) => Some((link.url.clone(), link.name.clone().unwrap_or_else(|| state.name.clone()))),
        None => state.ports.first().map(|port| (format!("http://localhost:{}", port.port), state.name.clone())),
    }
}

/// The declared ports a server's fields name, in the order they first appear
/// — `serve` first, so the first is where the server is.
fn named_ports(server: &config::Server, ports: &BTreeMap<String, u16>) -> Vec<ServerPort> {
    let fields = std::iter::once(server.serve())
        .chain(server.ready())
        .chain(server.links().iter().map(|link| link.url()))
        .chain(server.run());
    let mut named: Vec<ServerPort> = Vec::new();
    for field in fields {
        let mut rest = field;
        while let Some(at) = rest.find("${port.") {
            let after = &rest[at + "${port.".len()..];
            let Some(end) = after.find('}') else {
                break;
            };
            let name = &after[..end];
            if let Some(port) = ports.get(name) {
                if !named.iter().any(|had| had.name == name) {
                    named.push(ServerPort {
                        name: name.to_string(),
                        port: *port,
                    });
                }
            }
            rest = &after[end + 1..];
        }
    }
    named
}

/// Take away a holder's server directories that ended longer ago than
/// `kept_for` — the ad-hoc run log retention a run's directory has. **By the
/// file's own age**, since the directory is what outlived the instance.
fn swept(dir: &Path, live: &[String], kept_for: Duration) {
    let Ok(listing) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in listing.flatten() {
        if live.contains(&entry.file_name().to_string_lossy().into_owned()) {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|held| held.modified())
            .ok()
            .and_then(|at| at.elapsed().ok())
            .is_some_and(|age| age > kept_for);
        if old {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}
