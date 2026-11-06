//! `change_slot_pool`'s `release` for a bay a hosted session holds. The holder a
//! person sends is the session's own id, as `SlotHeld::HostedSession` shows it,
//! and the park is the one a close makes (`Vcs::park_hosted_slot`): what is
//! uncommitted goes to the branch the checkout is on, and everything still
//! running in the slot is ended. That includes the session's own agent, so a
//! session whose turn is running is refused, never closed.
//! `docs/concepts/fleet.md`, *Worktree slots*.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, SlotParkRefused, SlotStanding, Vcs, WorkProduct,
};
use api::Refusal;
use ipc::{ChangeSlotPool, ManifestId, SessionRow, SessionTurn, SlotAct, SlotPoolChanged, WireError};
use store::{Holder, KeptHosting};

use crate::daemon::Fleet;
use crate::leasing::{pool_of, SLOT_SESSION_WORKING, SLOT_UNCOMMITTED};
use crate::repositories::Served;
use crate::saving::released_answer;

impl Fleet
{
    /// [`change_slot_pool`](Fleet::change_slot_pool), with a `release` whose
    /// holder is a session Fleet hosts taken here, where the ledger can be asked.
    /// Any other holder, a terminal's, goes the way it always did.
    pub async fn slot_pool_changed(
        &self,
        asked: ChangeSlotPool,
        manifest_id: Option<&ManifestId>,
    ) -> Result<SlotPoolChanged, Refusal> {
        if let (SlotAct::Release, Some(slot), Some(holder)) =
            (asked.act, asked.slot, asked.holder.as_deref())
        {
            let hosting = self.store().lock().await.hosting(holder).ok().flatten();
            if let Some(hosting) = hosting {
                let served = self.served_named(manifest_id)?;
                let released = self.hosted_released(&served, slot, hosting, false).await;
                if released.is_ok() {
                    self.hosts().slot_freed();
                }
                return released;
            }
        }
        Fleet::change_slot_pool(self, asked, manifest_id)
    }

    /// Park the slot a hosted session holds and give its ledger rows back.
    /// **The holder is checked first, then the turn, then the park**: a slot
    /// re-leased since is refused as `fleet.slot_holder_changed`, and a session
    /// mid-turn, or with a subagent or background task still working past it, as
    /// `fleet.slot_session_working`, with nothing ended and the rows
    /// standing. An idle session's process is let go the way a tune lets it go,
    /// and its next message resumes it.
    /// **`refuse_uncommitted` is the person's Free of a slot a waiting Job's call lists**: a
    /// checkout holding files no commit has is refused as `fleet.slot_uncommitted`, where a Clear
    /// commits them to the Job's branch. Asked after the turn, so a Job that is running is
    /// refused as working first, and before anything is ended.
    pub async fn hosted_released(
        &self,
        served: &Served,
        slot: u32,
        mut hosting: KeptHosting,
        refuse_uncommitted: bool,
    ) -> Result<SlotPoolChanged, Refusal> {
        let id = hosting.session_id.clone();
        let pool = pool_of(served);
        let standing = {
            let (vcs, pool, id) = (Arc::clone(self.vcs()), pool.clone(), id.clone());
            tokio::task::spawn_blocking(move || vcs.slot_standing(&pool, slot, &id))
                .await
                .expect("git panicked reading a slot")
        };
        match standing {
            SlotStanding::Held => {}
            SlotStanding::HeldBy(now) => {
                return Err(self.slot_park_refusal(SlotParkRefused::HolderChanged(now)))
            }
            SlotStanding::Free | SlotStanding::Gone => {
                return Err(self.slot_park_refusal(SlotParkRefused::HolderChanged(String::from("nobody"))))
            }
        }
        let title = self
            .store()
            .lock()
            .await
            .session(&id)
            .ok()
            .flatten()
            .and_then(|kept| kept.title);
        // Subagents and background tasks past the turn live in its process, which a release ends.
        let working = matches!(self.hosts().of(&id).state().turn, SessionTurn::Working { .. }) || self.work_beyond_the_turn(&id).await;
        if refuse_uncommitted && !working {
            let dirty = {
                let (vcs, pool) = (Arc::clone(self.vcs()), pool.clone());
                tokio::task::spawn_blocking(move || vcs.slot_dirty(&pool, slot))
                    .await
                    .expect("git panicked reading a slot")
            };
            match dirty {
                Ok(false) => {}
                Ok(true) => {
                    let name = title.as_deref().unwrap_or("The job");
                    return Err(Refusal::IllegalMove(WireError::raised(
                        SLOT_UNCOMMITTED,
                        format!("{name} has uncommitted work"),
                        self.run_id(),
                    )));
                }
                Err(why) => return Err(self.slot_park_refusal(SlotParkRefused::Vcs(why))),
            }
        }
        {
            let runtime = self.hosts().of(&id);
            let mut state = runtime.state();
            // The one reading of whether a turn runs: what `hosted.turn` publishes.
            if working || matches!(state.turn, SessionTurn::Working { .. }) {
                let name = title.as_deref().unwrap_or("The job");
                return Err(Refusal::IllegalMove(WireError::raised(
                    SLOT_SESSION_WORKING,
                    format!("{name} is working"),
                    self.run_id(),
                )));
            }
            // A line written whose turn has not started would be lost with the process.
            if state.queued == 0 {
                Self::let_go(&mut state);
            }
        }
        self.stopped_servers_in(&adapter_traits::slot_path(served.root(), slot)).await;
        let parked = {
            let (vcs, id) = (Arc::clone(self.vcs()), id.clone());
            tokio::task::spawn_blocking(move || vcs.park_hosted_slot(&pool, slot, &id))
                .await
                .expect("git panicked parking a slot")
        }
        .map_err(|why| self.slot_park_refusal(why))?;
        // Only the slot the ledger records is its row; another the session's agent
        // leased has none to give back.
        if hosting.lease_slot == Some(slot) {
            hosting.lease_slot = None;
            hosting.lease_branch = None;
            let now = self.now().as_str().to_string();
            let mut store = self.store().lock().await;
            let _ = store.keep_hosting(&hosting);
            for kind in ["slot", "branch"] {
                let _ = store.give_back(&Holder::session(&id), Some(kind), &now);
            }
        }
        self.session_slot(&id, title, Some(hosting.manifest_id.clone()), slot, false);
        self.row_put(
            &id,
            SessionRow::Release { id: self.row_id(&id), at: self.instant(), slot, branch: parked.branch.clone() },
        )
        .await;
        let _ = self.published_hosted(&id).await;
        Ok(released_answer(served, slot, parked))
    }
}
