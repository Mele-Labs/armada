//! Where a holder writes now, and whether it still does. **Asked at every check**, so a Job that
//! ended, a Session that closed, a worktree that was reclaimed or a branch that was deleted is
//! dropped from the table whether or not anything told Fleet.

use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{AttachmentState, Holder};

use super::Who;
use crate::converging::elapsed;
use crate::daemon::Fleet;

/// How long a terminal Session may go unheard, and not be listening, before it counts as gone: a
/// crash reports no end.
const TERMINAL_GONE: Duration = Duration::from_secs(6 * 60 * 60);

/// Where one holder writes.
#[derive(Clone, Debug)]
pub(crate) struct Standing {
    pub(crate) manifest: String,
    pub(crate) worktree: PathBuf,
}

/// How the other side is named to an agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Peer {
    Job {
        id: String,
        handle: String,
        title: String,
    },
    Session {
        id: String,
        /// `s-` and the first eight characters of the id: what `SendMessage` takes.
        address: String,
        title: Option<String>,
        branch: Option<String>,
    },
}

impl Peer {
    /// What a person would call it.
    pub(crate) fn label(&self) -> String {
        match self {
            Peer::Job { handle, .. } => handle.clone(),
            Peer::Session { address, title, .. } => title.clone().unwrap_or_else(|| address.clone()),
        }
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
    /// Where `who` writes, or `None` once it is over, however that came about.
    pub(crate) async fn writing_in(&self, who: &Who) -> Option<Standing> {
        match who {
            Who::Job(id) => {
                let job = self.load(id).await.ok()?;
                if job.status().is_terminal() {
                    return None;
                }
                let served = self.served_by(&job).ok()?;
                let spec = self.tree_spec(&served, &job)?;
                let worktree = PathBuf::from(spec.worktree_path());
                worktree.is_dir().then(|| Standing {
                    manifest: served.manifest().id().as_str().to_string(),
                    worktree,
                })
            }
            Who::Session(id) => self.session_standing(id).await,
        }
    }

    async fn session_standing(&self, id: &str) -> Option<Standing> {
        let (session, held) = {
            let store = self.store().lock().await;
            let session = store.session(id).ok()??;
            let held = store.attachments_of(&Holder::session(id)).ok()?;
            (session, held)
        };
        if session.state == store::SessionState::Ended {
            return None;
        }
        let standing = |kind: &str| {
            held.iter()
                .find(|one| one.kind == kind && one.state == AttachmentState::Standing)
        };
        let slot = standing("slot")?;
        let served = self.repositories().serving(&slot.manifest_id)?;
        let worktree = PathBuf::from(adapter_traits::slot_path(served.root(), slot.target.parse().ok()?));
        if !worktree.is_dir() {
            return None;
        }
        // **The pool says whose slot it is**: a row left over from a slot the Session gave up
        // would otherwise set it against whoever works there now, and against itself.
        let stand = {
            let store = self.store().lock().await;
            crate::slot_holders::stand_of(&store, &slot.manifest_id, served.root(), &held, id)?
        };
        if let Some(branch) = standing("branch") {
            if !adapters::needs::branch_exists(Path::new(served.root()), &branch.target)
                || !stand.on(&branch.target)
            {
                return None;
            }
        }
        let terminal = session.origin != "bridge";
        let quiet = elapsed(&core_model::Timestamp::from_rfc3339(&session.last_seen_at), &self.now());
        if terminal && quiet > TERMINAL_GONE && !self.hosts().terminals().listening(id) {
            return None;
        }
        Some(Standing {
            manifest: slot.manifest_id.clone(),
            worktree,
        })
    }

    /// How an agent is to reach `who`, and what to call it.
    pub(crate) async fn peer_of(&self, who: &Who) -> Option<Peer> {
        match who {
            Who::Job(id) => {
                let job = self.load(id).await.ok()?;
                Some(Peer::Job {
                    id: id.as_str().to_string(),
                    handle: job.handle(),
                    title: job.title().as_str().to_string(),
                })
            }
            Who::Session(id) => {
                let store = self.store().lock().await;
                let session = store.session(id).ok()??;
                let branch = store
                    .attachments_of(&Holder::session(id))
                    .ok()?
                    .into_iter()
                    .find(|one| one.kind == "branch" && one.state == AttachmentState::Standing)
                    .map(|one| one.target);
                Some(Peer::Session {
                    id: id.clone(),
                    address: crate::session_host::address_of(id),
                    title: session.title,
                    branch,
                })
            }
        }
    }
}
