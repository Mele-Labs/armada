//! A session's slot follows the pool. The agent can lease and release slots
//! itself (`armada worktree lease`), so the slot the ledger records for a
//! session can stop being true without Fleet being told. **Fleet reads the pool
//! at every gate call**, the one place every tool call of a hosted session
//! already passes, and makes the ledger agree with it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SlotHeld, SlotParkRefused, Vcs, WorkProduct};
use adapters::leasing::Holder;
use ipc::SessionRow;
use store::{AttachmentState, Holder as Keeper, KeptAttachment, KeptHosting};

use crate::daemon::Fleet;
use crate::peer::{runs_from, started_within};
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
        Some(Holder::Hosted(id)) if id == session => Standing::Ours,
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

/// A Session as a pool slot's holder names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SlotSession {
    pub(crate) id: String,
    pub(crate) title: Option<String>,
}

/// The pid in a holder as the pool says it: `zsh (pid 4120)`.
pub(crate) fn pid_in(said: &str) -> Option<u32> {
    said.rsplit_once("(pid ")?.1.strip_suffix(')')?.parse().ok()
}

/// `ps -axo pid=,command=`, for [`keeper_in`]. Blocking, and empty where `ps`
/// cannot run.
pub(crate) fn process_listing() -> String {
    std::process::Command::new("ps")
        .args(["-axo", "pid=,command="])
        .output()
        .map(|listing| String::from_utf8_lossy(&listing.stdout).into_owned())
        .unwrap_or_default()
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

    /// The hosted Session whose agent is `pid`, or started it, in a `ps`
    /// listing. **A subagent's or a tool's lease is the Session's own work**,
    /// so a bay held by one names the Session it began from. `None` for a
    /// process no hosted Session started, and for one that has gone.
    pub(crate) fn session_holding_pid(&self, pid: u32, listing: &str) -> Option<String> {
        let mut ids: Vec<String> = self.hosts().all().into_iter().map(|(id, _)| id).collect();
        ids.sort();
        ids.into_iter().find(|id| {
            let roots = match self.hosts().of(id).state().process.as_ref().and_then(|one| one.pid()) {
                Some(agent) => vec![agent],
                None => {
                    let socket = std::path::Path::new(&self.host().keepers_dir)
                        .join(format!("{id}.sock"));
                    keeper_in(listing, &socket.to_string_lossy())
                        .into_iter()
                        .collect()
                }
            };
            started_within(&roots, pid, self.peers().as_ref())
        })
    }

    /// The Session the ledger holds under `id`, with its title.
    pub(crate) async fn session_in_ledger(&self, id: &str) -> Option<SlotSession> {
        let kept = self.store().lock().await.session(id).ok()??;
        Some(SlotSession {
            id: kept.id,
            title: kept.title,
        })
    }

    pub(super) fn held_by_the_tree(&self, id: &str, path: &str) -> Standing {
        let roots = self.tree_roots(id);
        let holder = adapters::leasing::holder_of(std::path::Path::new(path));
        standing(holder.as_ref(), id, |pid| {
            started_within(&roots, pid, self.peers().as_ref())
        })
    }

    /// Every directory this session may write in: the one it runs in, and every other slot the
    /// pool holds for it or for something it started. **A subagent's `armada worktree lease` is
    /// the session's**: it holds the slot under the session's id, or under a process the
    /// session's tree holds, so a slot the ledger has never recorded for the session is still
    /// its own.
    pub(super) fn held_directories(
        &self,
        id: &str,
        served: &Served,
        hosting: &KeptHosting,
    ) -> Vec<String> {
        let root = served.root().trim_end_matches('/').to_string();
        let mut held = vec![Self::directory_of(served, hosting)];
        held.push(self.scratch_of(id).to_string_lossy().into_owned());
        for slot in crate::leasing::bays_of(served) {
            let at = adapter_traits::slot_path(&root, slot);
            if hosting.lease_slot != Some(slot) && self.held_by_the_tree(id, &at) == Standing::Ours {
                // A process holder cannot say whose it was once it is gone.
                let _ = adapters::leasing::note_session(std::path::Path::new(&at), id);
                held.push(at);
            }
        }
        held
    }

    /// Whether a process is one the session started, however deep, or runs a file from a
    /// directory it holds: what it may `kill`. The second covers a dev Fleet its subagent started
    /// and which left the tree by `setsid`.
    pub(super) fn started_by<'a>(&'a self, id: &str, held: &'a [String]) -> impl Fn(u32) -> bool + 'a {
        let roots = self.tree_roots(id);
        move |pid| {
            started_within(&roots, pid, self.peers().as_ref())
                || runs_from(pid, held, self.peers().as_ref())
        }
    }

    /// The head branch of every pull request the session holds or held on the ledger. **A branch
    /// the session cut itself is its own** though no slot is on it any more: a session that
    /// switches its slot back after opening a pull request was refused deleting the merged branch
    /// and asked the owner to (#2343, 15 Oct 2026).
    pub(super) async fn pull_branches_of(&self, id: &str, manifest: &str) -> Vec<String> {
        let rows = self.store().lock().await.attachments_of(&Keeper::session(id));
        rows.unwrap_or_default()
            .into_iter()
            .filter(|row| row.kind == crate::pull_requesting::PR && row.manifest_id == manifest)
            .filter_map(|row| row.detail.get("branch").cloned())
            .filter(|branch| !branch.is_empty())
            .collect()
    }

    /// The branches the session may delete by hand: the one it was leased, each one a slot it
    /// holds is on and each of its pull requests', where the base already holds every commit
    /// on them.
    pub(super) async fn own_merged_branches(
        &self,
        id: &str,
        served: &Served,
        hosting: &KeptHosting,
    ) -> Vec<String> {
        let pool = crate::leasing::pool_of(served);
        let (vcs, session) = (Arc::clone(self.vcs()), id.to_string());
        let mut branches: Vec<String> = hosting.lease_branch.clone().into_iter().collect();
        branches.extend(self.pull_branches_of(id, &hosting.manifest_id).await);
        let lease = hosting.lease_slot;
        tokio::task::spawn_blocking(move || {
            for reading in vcs.slot_pool(&pool) {
                if matches!(&reading.held, SlotHeld::Hosted(holder) if *holder == session)
                    || lease == Some(reading.slot)
                {
                    branches.extend(reading.branch);
                }
            }
            branches.sort();
            branches.dedup();
            branches.retain(|branch| vcs.branch_is_merged(&pool, branch));
            branches
        })
        .await
        .unwrap_or_default()
    }

    /// Park every slot an ending session holds: the one the ledger records,
    /// and any other `armada worktree lease` took for it (a subagent's), which
    /// the pool holds under the session's id and would otherwise hold for
    /// good. Committed to the branch each is on, pushed nowhere, with what
    /// still runs in it ended first (`Vcs::park_hosted_slot`).
    ///
    /// **Returns the slots the pool still holds for the session**, with why.
    /// The caller leaves those slots' ledger rows standing: a row given back over a held slot is how one gets stranded.
    pub(super) async fn park_every_slot_of(
        &self,
        id: &str,
        served: &Served,
        own: Option<u32>,
    ) -> Vec<(u32, SlotParkRefused)> {
        let pool = crate::leasing::pool_of(served);
        let root = served.root().to_string();
        let session = id.to_string();
        let slots = {
            let (vcs, pool, session) = (Arc::clone(self.vcs()), pool.clone(), session.clone());
            tokio::task::spawn_blocking(move || {
                let mut slots: Vec<u32> = own.into_iter().collect();
                for reading in vcs.slot_pool(&pool) {
                    if matches!(&reading.held, SlotHeld::Hosted(holder) if *holder == session)
                        && !slots.contains(&reading.slot)
                    {
                        slots.push(reading.slot);
                    }
                }
                slots
            })
            .await
            .unwrap_or_default()
        };
        for slot in &slots {
            self.stopped_servers_in(&adapter_traits::slot_path(&root, *slot)).await;
        }
        let vcs = Arc::clone(self.vcs());
        let refused = tokio::task::spawn_blocking(move || {
            slots
                .into_iter()
                .filter_map(|slot| match vcs.park_hosted_slot(&pool, slot, &session) {
                    Ok(_) => None,
                    Err(why) if why.holds_still() => Some((slot, why)),
                    // Not its slot any more: released by the agent or taken by another.
                    Err(_) => None,
                })
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        refused
    }

    /// Slots the pool holds under the id of a session whose row says it ended:
    /// closed before this fix parked them, or refused then. Each is parked as a
    /// close parks it, and its ledger rows given back once the pool lets go.
    /// Run where the pool is reconciled, so nothing needs a timer.
    pub(crate) async fn freed_from_ended_sessions(&self, served: &Served) {
        let pool = crate::leasing::pool_of(served);
        let vcs = Arc::clone(self.vcs());
        let held: Vec<(u32, String)> = {
            let pool = pool.clone();
            tokio::task::spawn_blocking(move || {
                vcs.slot_pool(&pool)
                    .into_iter()
                    .filter_map(|reading| match reading.held {
                        SlotHeld::Hosted(holder) => Some((reading.slot, holder)),
                        _ => None,
                    })
                    .collect()
            })
            .await
            .unwrap_or_default()
        };
        let mut seen = std::collections::BTreeSet::new();
        for (_, id) in held {
            if !seen.insert(id.clone()) {
                continue;
            }
            let ended = matches!(
                self.store().lock().await.session(&id),
                Ok(Some(kept)) if kept.state == store::SessionState::Ended
            );
            if !ended {
                continue;
            }
            if !self.park_every_slot_of(&id, served, None).await.is_empty() {
                continue;
            }
            let now = self.now().as_str().to_string();
            let mut store = self.store().lock().await;
            for kind in ["slot", "branch"] {
                let _ = store.give_back(&Keeper::session(&id), Some(kind), &now);
            }
        }
    }

    /// Make the ledger's slot for `id` agree with the pool. A slot the pool no
    /// longer shows as the session's is given back; with none recorded, a slot
    /// held by the session's own tree is attached with its branch. Returns the
    /// hosting as it now stands.
    pub(crate) async fn follow_the_pool(
        &self,
        id: &str,
        served: &Served,
        mut hosting: KeptHosting,
    ) -> KeptHosting {
        let root = served.root().trim_end_matches('/').to_string();
        let path = |slot: u32| adapter_traits::slot_path(&root, slot);
        let manifest = hosting.manifest_id.clone();
        let now = self.now().as_str().to_string();
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
            self.session_slot(id, None, Some(hosting.manifest_id.clone()), slot, false);
            let _ = self.published_hosted(id).await;
        }
        if hosting.lease_slot.is_some() || self.tree_roots(id).is_empty() {
            return hosting;
        }
        let Some((slot, branch)) = crate::leasing::bays_of(served).into_iter().find_map(|slot| {
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
        self.session_slot(id, None, Some(hosting.manifest_id.clone()), slot, true);
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
