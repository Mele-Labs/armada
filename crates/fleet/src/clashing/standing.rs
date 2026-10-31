//! Where a holder writes now, and whether it still does. **Asked at every check**, so a Job that
//! ended, a Session that closed, a worktree that was reclaimed or a branch that was deleted is
//! dropped from the table whether or not anything told Fleet.

use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{AttachmentState, Holder};

use super::Who;
use crate::clock::elapsed;
use crate::daemon::Fleet;

/// How long a terminal Session may go unheard, and not be listening, before it counts as gone: a
/// crash reports no end.
const TERMINAL_GONE: Duration = Duration::from_secs(6 * 60 * 60);

/// Where one holder writes: a Job's one worktree, or every slot a Session holds.
#[derive(Clone, Debug)]
pub(crate) struct Standing {
    pub(crate) manifest: String,
    pub(crate) worktrees: Vec<PathBuf>,
}

/// How the other side is named to an agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Peer {
    Session {
        id: String,
        /// `s-` and the first eight characters of the id: what `SendMessage` takes.
        address: String,
        title: Option<String>,
        branch: Option<String>,
        /// Paused by the person: a message waits for his Resume, so it cannot answer.
        paused: bool,
    },
}

impl Peer {
    /// What a person would call it.
    pub(crate) fn label(&self) -> String {
        match self {
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
        let terminal = session.origin != "bridge";
        let quiet = elapsed(&core_model::Timestamp::from_rfc3339(&session.last_seen_at), &self.now());
        if terminal && quiet > TERMINAL_GONE && !self.hosts().terminals().listening(id) {
            return None;
        }
        let standing = |kind: &str| {
            held.iter()
                .find(|one| one.kind == kind && one.state == AttachmentState::Standing)
        };
        let row = standing("slot");
        let manifest = row
            .map(|slot| slot.manifest_id.clone())
            .or_else(|| session.manifest_id.clone())?;
        let served = self.repositories().serving(&manifest)?;
        // **The pool says whose slot it is**: a row left over from a slot the Session gave up
        // would otherwise set it against whoever works there now, and against itself. The row's
        // slot counts while its branch is still there and checked out in it.
        let mut worktrees = Vec::new();
        let ledger = row.and_then(|slot| slot.target.parse::<u32>().ok());
        let stand = {
            let store = self.store().lock().await;
            crate::slot_holders::stand_of(&store, &manifest, served.root(), &held, id)
        };
        let on_its_branch = |stand: &crate::slot_holders::Stand| match standing("branch") {
            Some(branch) => {
                adapters::clash::branch_exists(Path::new(served.root()), &branch.target)
                    && stand.on(&branch.target)
            }
            None => true,
        };
        if let Some(stand) = stand.filter(|stand| on_its_branch(stand)) {
            worktrees.push(stand.path);
        }
        // Every other slot it holds, its subagents' among them.
        for slot in crate::slot_holders::slots_of(&served, id) {
            if Some(slot) != ledger {
                worktrees.push(PathBuf::from(adapter_traits::slot_path(served.root(), slot)));
            }
        }
        // And every worktree it was credited a write in from outside it.
        for written in self.writes().worktrees_of(&Who::Session(id.to_string())) {
            let at = PathBuf::from(written);
            if !worktrees.contains(&at) && at.is_dir() {
                worktrees.push(at);
            }
        }
        (!worktrees.is_empty()).then_some(Standing { manifest, worktrees })
    }

    /// How an agent is to reach `who`, and what to call it.
    pub(crate) async fn peer_of(&self, who: &Who) -> Option<Peer> {
        match who {
            Who::Session(id) => {
                let store = self.store().lock().await;
                let session = store.session(id).ok()??;
                let branch = store
                    .attachments_of(&Holder::session(id))
                    .ok()?
                    .into_iter()
                    .find(|one| one.kind == "branch" && one.state == AttachmentState::Standing)
                    .map(|one| one.target);
                let paused = store.hosting(id).ok().flatten().is_some_and(|one| one.paused_at.is_some());
                Some(Peer::Session {
                    id: id.clone(),
                    address: crate::session_host::address_of(id),
                    title: session.title,
                    branch,
                    paused,
                })
            }
        }
    }
}
