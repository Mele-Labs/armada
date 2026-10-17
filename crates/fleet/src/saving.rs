//! What a pool release does with a session's uncommitted files: commit them to
//! its branch, then give the slot back. `docs/concepts/fleet.md`, *Worktree
//! slots*, and the decision record `2026-10-05-a-branch-commit-is-enough-to-free-a-slot`.
//!
//! Nothing is pushed. A sweep never comes here: it gives back only what is
//! provably safe, and a tree with uncommitted files is not.

use adapter_traits::{AgentHarness, Delivery, SlotParkRefused, SlotParked, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ChangeSlotPool, ManifestId, SlotPoolChanged, SlotReleased, WireError};

use crate::daemon::Fleet;
use crate::leasing::{
    pool_of, NO_SUCH_SLOT, SLOT_BUSY, SLOT_HOLDER_CHANGED, SLOT_HOLDER_UNNAMED, SLOT_NOT_PARKABLE,
};
use crate::repositories::Served;

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
    /// `change_slot_pool`'s `release`: commit what an agent session holds
    /// uncommitted in a slot to its branch and give the slot back, the same
    /// park a Clear makes. **Only for the holder the person was
    /// shown**, so a slot re-leased since is refused. The session's process is
    /// never touched.
    pub(crate) fn session_released(
        &self,
        served: &Served,
        asked: &ChangeSlotPool,
    ) -> Result<SlotPoolChanged, Refusal> {
        let raised = |code: &str, said: String| WireError::raised(code, said, self.run_id());
        let (Some(slot), Some(holder)) = (asked.slot, asked.holder.as_deref()) else {
            return Err(Refusal::Unacceptable(raised(
                if asked.slot.is_none() {
                    NO_SUCH_SLOT
                } else {
                    SLOT_HOLDER_UNNAMED
                },
                String::from("a release names the slot and the holder it was shown"),
            )));
        };
        let parked = self
            .vcs()
            .release_session_slot(&pool_of(served), slot, holder)
            .map_err(|why| self.slot_park_refusal(why))?;
        Ok(released_answer(served, slot, parked))
    }

    /// A refused park of a session's slot, as the 409 it is.
    pub(crate) fn slot_park_refusal(&self, why: SlotParkRefused) -> Refusal {
        let raised = |code: &str, said: String| WireError::raised(code, said, self.run_id());
        let said = why.said();
        match why {
            SlotParkRefused::HolderChanged(_)
            | SlotParkRefused::HeldByAnother(_)
            | SlotParkRefused::NotLeased => Refusal::IllegalMove(raised(SLOT_HOLDER_CHANGED, said)),
            SlotParkRefused::Busy => Refusal::IllegalMove(raised(SLOT_BUSY, said)),
            _ => Refusal::IllegalMove(raised(SLOT_NOT_PARKABLE, said)),
        }
    }
}

/// What a release answers once the pool has parked the slot.
pub(crate) fn released_answer(served: &Served, slot: u32, parked: SlotParked) -> SlotPoolChanged {
    let saved = parked.commit.map(|commit| ipc::ReclaimedSaved {
        commit,
        files: parked.files,
    });
    SlotPoolChanged {
        manifest_id: ManifestId::from(served.manifest().id()),
        slot,
        released: Some(SlotReleased {
            branch: parked.branch,
            saved,
        }),
    }
}
