//! Starting one server: its row, its log directory and its task. **The tail of `hold_server`,
//! shared with a review walk**, which starts a command a person typed in a slot Fleet leased
//! rather than one a Manifest declares.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Event, ServerLink, ServerPhase, ServerPort, ServerState, StartedBy};
use tokio::sync::watch;

use super::held::{Holder, Live};
use super::running::Plan;
use super::{swept, Unservable};
use crate::daemon::Fleet;
use crate::rehearsing::records;

/// Everything a server needs that is decided before it starts. Every `${port.NAME}` in `serve`,
/// `ready`, `run` and the links is already resolved.
pub(crate) struct Launch {
    pub(crate) holder: Holder,
    pub(crate) name: String,
    pub(crate) manifest_id: ipc::ManifestId,
    pub(crate) checkout: ipc::ServerCheckout,
    /// Where under `.armada/servers/` its log directory goes.
    pub(crate) under: String,
    pub(crate) ports: Vec<ServerPort>,
    pub(crate) links: Vec<ServerLink>,
    pub(crate) by: StartedBy,
    /// The Session whose agent asked for it, where one did.
    pub(crate) session: Option<String>,
    /// How a record left behind by a crashed Fleet says whose server this was.
    pub(crate) whose: String,
    pub(crate) run: Option<String>,
    pub(crate) serve: String,
    pub(crate) ready: Option<String>,
    pub(crate) worktree: PathBuf,
    pub(crate) env: Vec<(String, String)>,
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
    /// Start the server `launch` describes, **or answer with the instance its holder already has
    /// up**, `true` beside the state where this call started it.
    pub(crate) async fn launched(
        self: Arc<Self>,
        records_root: &str,
        launch: Launch,
    ) -> Result<(ServerState, bool), Unservable> {
        let Launch {
            holder,
            name,
            manifest_id,
            checkout,
            under,
            ports,
            links,
            by,
            session,
            whose,
            run,
            serve,
            ready,
            worktree,
            env,
        } = launch;
        if let Some(up) = self.servers().running(&holder, &name) {
            return Ok((up.borrow().clone(), false));
        }
        let id = self.mint().ulid().as_str().to_string();
        let servers_dir = Path::new(records_root)
            .join(".armada")
            .join("servers")
            .join(&under);
        swept(
            &servers_dir,
            &self.servers().live_ids(),
            self.run_log_retention(),
        );
        let dir = servers_dir.join(&id);
        std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::File::create(dir.join(records::LOG)).map(|_| ()))
            .map_err(|why| Unservable::NotKept {
                why: why.to_string(),
            })?;
        let state = ServerState {
            id: id.clone(),
            name: name.clone(),
            manifest_id: Some(manifest_id),
            checkout,
            phase: ServerPhase::Starting,
            serve: serve.clone(),
            ports,
            links,
            started_by: by,
            session_id: session.as_deref().map(ipc::SessionId::carried),
            started_at: ipc::Instant::from(&self.now()),
            serving_since: None,
            ended_at: None,
            exit_code: None,
            ended: None,
            stopped: false,
            log: format!(".armada/servers/{under}/{id}/{}", records::LOG),
        };
        let (stop, stopped) = watch::channel(false);
        // The sender is shared rather than owned by the task: where a server
        // stands is written by the task on each phase, and by
        // `crate::noticing` when a merge moves the checkout it serves out from
        // under it.
        let now = Arc::new(watch::channel(state.clone()).0);
        let feed = api::RunFeed::new();
        let live = Live {
            now: Arc::clone(&now),
            stop: Arc::new(stop),
            feed: feed.clone(),
            dir: dir.clone(),
        };
        let held = match self.servers().take(&holder, &name, &id, live) {
            Ok(held) => held,
            // Another request took the name between the look and the take.
            Err(up) => {
                let _ = std::fs::remove_dir_all(&dir);
                return Ok((up.borrow().clone(), false));
            }
        };
        // The row as it stands, under its write lock and changing nothing: a
        // merge may already have moved it, and published under the same lock.
        now.send_if_modified(|now| {
            self.publish(Event::ServerStarting(now.clone()));
            false
        });
        let plan = Plan {
            whose,
            run,
            serve,
            ready,
            worktree,
            env,
            dir,
            feed,
            now,
        };
        let this = Arc::clone(&self);
        tokio::spawn(async move { this.served(plan, held, stopped).await });
        Ok((state, true))
    }
}
