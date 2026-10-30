//! `finish_up`: a hosted session gives back the slots it holds, the branches already in the base
//! and the servers running in those slots, in one call on Fleet's door. A write in the checkout
//! cannot do this: once the slot is released, the next write takes a new lease for the session.
//! `docs/concepts/session.md`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SlotHeld, Vcs, WorkProduct};
use api::Refusal;
use ipc::{FinishUp, FinishedUp, SessionRow};
use store::Holder;

use crate::daemon::Fleet;
use crate::leasing::pool_of;

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
    /// Release every slot the pool holds for the session, as `armada worktree release` does, and
    /// stop the servers running in each one it released. Then delete the branches those slots were
    /// on, the one the session was leased and its pull requests', where the base already holds
    /// them. **Anything
    /// refused stays and is named in `kept`**; nothing is committed for the agent.
    pub(crate) async fn finished_up(
        &self,
        caller: Option<api::Caller>,
        finish: FinishUp,
    ) -> Result<FinishedUp, Refusal> {
        let id = self.waiting_session(caller, finish.session_id.as_ref())?;
        let (session, mut hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        self.lifecycles_hold(&mut *self.store().lock().await, &id, "finish_up")?;
        let served = self.served_named(Some(&ipc::ManifestId::carried(&hosting.manifest_id)))?;
        let pool = pool_of(&served);
        let held: Vec<(u32, Option<String>)> = {
            let (vcs, pool, id) = (Arc::clone(self.vcs()), pool.clone(), id.clone());
            tokio::task::spawn_blocking(move || {
                vcs.slot_pool(&pool)
                    .into_iter()
                    .filter(|reading| matches!(&reading.held, SlotHeld::Hosted(holder) if *holder == id))
                    .map(|reading| (reading.slot, reading.branch))
                    .collect()
            })
            .await
            .unwrap_or_default()
        };
        let mut done = FinishedUp::default();
        let mut branches: Vec<String> = hosting.lease_branch.clone().into_iter().collect();
        branches.extend(self.pull_branches_of(&id, &hosting.manifest_id).await);
        for (slot, branch) in held {
            let released = {
                let (vcs, pool, id) = (Arc::clone(self.vcs()), pool.clone(), id.clone());
                tokio::task::spawn_blocking(move || vcs.release_slot(&pool, slot, &id))
                    .await
                    .expect("git panicked releasing a slot")
            };
            match released {
                Ok(()) => {
                    done.released.push(slot);
                    self.stopped_servers_in(&adapter_traits::slot_path(served.root(), slot)).await;
                    self.session_slot(&id, session.title.clone(), Some(hosting.manifest_id.clone()), slot, false);
                    if let Some(branch) = &branch {
                        self.row_put(
                            &id,
                            SessionRow::Release { id: self.row_id(&id), at: self.instant(), slot, branch: branch.clone() },
                        )
                        .await;
                    }
                }
                Err(why) => done.kept.push(format!("slot {slot}: {}", why.0)),
            }
            branches.extend(branch);
        }
        branches.sort();
        branches.dedup();
        for branch in branches {
            let deleted = {
                let (vcs, pool, branch) = (Arc::clone(self.vcs()), pool.clone(), branch.clone());
                tokio::task::spawn_blocking(move || vcs.delete_merged_branch(&pool, &branch))
                    .await
                    .expect("git panicked deleting a branch")
            };
            match deleted {
                Ok(true) => done.deleted_branches.push(branch),
                Ok(false) => {}
                Err(why) => done.kept.push(format!("branch {branch}: {}", why.0)),
            }
        }
        let slot_released = hosting.lease_slot.is_some_and(|slot| done.released.contains(&slot));
        // A branch that stayed is named again by the next call, once the base holds it.
        let branch_gone = hosting.lease_branch.as_ref().is_some_and(|branch| {
            let stayed = format!("branch {branch}:");
            !done.kept.iter().any(|one| one.starts_with(&stayed))
        });
        if slot_released || branch_gone {
            if slot_released {
                hosting.lease_slot = None;
            }
            if branch_gone {
                hosting.lease_branch = None;
            }
            let now = self.now().as_str().to_string();
            let mut store = self.store().lock().await;
            let _ = store.keep_hosting(&hosting);
            if slot_released {
                for kind in ["slot", "branch"] {
                    let _ = store.give_back(&Holder::session(&id), Some(kind), &now);
                }
            }
        }
        if !done.released.is_empty() {
            self.hosts().slot_freed();
            let _ = self.published_hosted(&id).await;
        }
        Ok(done)
    }
}
