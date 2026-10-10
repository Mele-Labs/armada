//! A session's slot follows the pool. The agent can lease and release slots
//! itself (`armada worktree lease`), so the slot the ledger records for a
//! session can stop being true without Fleet being told. **Fleet reads the pool
//! at every gate call**, the one place every tool call of a hosted session
//! already passes, and makes the ledger agree with it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use adapters::leasing::Holder;
use ipc::SessionRow;
use store::{AttachmentState, Holder as Keeper, KeptAttachment, KeptHosting};

use crate::daemon::Fleet;
use crate::peer::started_within;
use crate::repositories::Served;

/// What the pool says of a slot a session may own.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Standing {
    /// Held for this session: by Fleet on its behalf, or by its process tree.
    Ours,
    /// Free, or held by somebody else.
    Not,
}

/// Whether `holder` is this session's: the Job-style lease Fleet took for it
/// by id, or a live process that is the agent or one it started.
pub(super) fn standing(
    holder: Option<&Holder>,
    session: &str,
    in_tree: impl Fn(u32) -> bool,
) -> Standing {
    match holder {
        Some(Holder::Job(id)) if id == session => Standing::Ours,
        Some(held) if held.alive() && held.pid().is_some_and(in_tree) => Standing::Ours,
        _ => Standing::Not,
    }
}

/// The pid of the `armada session-keep` serving `socket`, in a `ps -axo
/// pid=,command=` listing.
pub(crate) fn keeper_in(listing: &str, socket: &str) -> Option<u32> {
    listing.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        let pid = words.next()?.parse().ok()?;
        let words: Vec<&str> = words.collect();
        (words.contains(&"session-keep") && words.contains(&socket)).then_some(pid)
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
    /// The pids this session's agent and everything it started are among.
    fn tree_roots(&self, id: &str) -> Vec<u32> {
        let process = self.hosts().of(id).state().process.clone();
        match process.and_then(|process| process.pid()) {
            Some(agent) => vec![agent],
            // No pid yet (READY unread) or no process attached: the keeper
            // that serves this session's socket started the agent, so its
            // tree holds the agent's.
            None => self.keeper_of(id).into_iter().collect(),
        }
    }

    fn keeper_of(&self, id: &str) -> Option<u32> {
        let socket = std::path::Path::new(&self.host().keepers_dir).join(format!("{id}.sock"));
        let listing = std::process::Command::new("ps")
            .args(["-axo", "pid=,command="])
            .output()
            .ok()?;
        keeper_in(
            &String::from_utf8_lossy(&listing.stdout),
            &socket.to_string_lossy(),
        )
    }

    pub(super) fn held_by_the_tree(&self, id: &str, path: &str) -> Standing {
        let roots = self.tree_roots(id);
        let holder = adapters::leasing::holder_of(std::path::Path::new(path));
        standing(holder.as_ref(), id, |pid| {
            started_within(&roots, pid, self.peers().as_ref())
        })
    }

    /// Make the ledger's slot for `id` agree with the pool. A slot the pool no
    /// longer shows as the session's is given back; with none recorded, a slot
    /// held by the session's own tree is attached with its branch. Returns the
    /// hosting as it now stands.
    pub(super) async fn follow_the_pool(
        &self,
        id: &str,
        served: &Served,
        mut hosting: KeptHosting,
    ) -> KeptHosting {
        let root = served.root().trim_end_matches('/').to_string();
        let path = |slot: u32| adapter_traits::slot_path(&root, slot);
        let manifest = hosting.manifest_id.clone();
        let now = self.now().as_str().to_string();
        // A piloted Session holds the Job's own worktree, leased by the Job: the pilot
        // hands it over and takes it back, so the pool is not this one's to read.
        if self.piloted_job_of(id).await.is_some() {
            return hosting;
        }
        if let Some(slot) = hosting.lease_slot {
            if self.held_by_the_tree(id, &path(slot)) == Standing::Ours {
                return hosting;
            }
            // A turn about to move into this very slot is Fleet's own doing.
            if self.hosts().of(id).state().moving.is_some() {
                return hosting;
            }
            hosting.lease_slot = None;
            hosting.lease_branch = None;
            let mut store = self.store().lock().await;
            let _ = store.keep_hosting(&hosting);
            let _ = store.give_back(&Keeper::session(id), Some("slot"), &now);
            let _ = store.give_back(&Keeper::session(id), Some("branch"), &now);
            drop(store);
            let _ = self.published_hosted(id).await;
        }
        if hosting.lease_slot.is_some() || self.tree_roots(id).is_empty() {
            return hosting;
        }
        let count = crate::leasing::pool_of(served).slots();
        let Some((slot, branch)) = (1..=count).find_map(|slot| {
            let at = path(slot);
            (self.held_by_the_tree(id, &at) == Standing::Ours)
                .then(|| adapters::leasing::branch_of(std::path::Path::new(&at)))
                .flatten()
                .map(|branch| (slot, branch))
        }) else {
            return hosting;
        };
        hosting.lease_slot = Some(slot);
        hosting.lease_branch = Some(branch.clone());
        {
            let mut store = self.store().lock().await;
            let _ = store.keep_hosting(&hosting);
            for (kind, target) in [("slot", slot.to_string()), ("branch", branch.clone())] {
                let _ = store.attach(
                    &KeptAttachment {
                        holder: Keeper::session(id),
                        kind: kind.into(),
                        manifest_id: manifest.clone(),
                        target,
                        state: AttachmentState::Standing,
                        detail: Default::default(),
                        since: now.clone(),
                        changed_at: now.clone(),
                    },
                    true,
                );
            }
        }
        self.row_put(
            id,
            SessionRow::Lease {
                id: self.row_id(id),
                at: self.instant(),
                slot,
                branch,
            },
        )
        .await;
        let _ = self.published_hosted(id).await;
        hosting
    }
}
