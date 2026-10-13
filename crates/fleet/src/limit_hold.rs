//! A Drone the plan's limit stopped waits for the limit to reset, and is not failed for it.
//! `docs/concepts/job.md`, *Held for the plan's limit*.
//!
//! The Job is parked as Fleet's own pause with an `until`, so its branch is kept, its slot is
//! given back and admission leaves it alone until the reset and a minute more. Then the pause
//! machinery that already reseats a parked Job puts a fresh Drone on the step. **The step stays
//! `running`**, as a Drone's question leaves it (`crate::questioning`): stopping it and entering
//! it again would spend one of the step's retries on nothing the work did. Holding is not gating:
//! the API had already refused the run.

use adapter_traits::{AgentHarness, Delivery, SlotStanding, Vcs, WorkProduct};
use core_model::{JobId, JobStatus, PausedBy, StepId, Timestamp};

use crate::adrift::Adrift;
use crate::clock::rfc3339_utc;
use crate::daemon::Fleet;
use crate::leasing::pool_of;
use crate::working::{StoodDown, Working};

/// How long past the reset a held Job waits, so the window is surely open when its Drone starts.
const MARGIN_MILLIS: i64 = 60_000;

const SAID: &str =
    "the plan's limit stopped the Drone; the Job waits for it to reset and its slot is given back";

/// The instant a limit resetting at `reset` (unix seconds) opens. The marker holds this, so Bridge
/// draws the reset itself; admission waits [`MARGIN_MILLIS`] past it.
pub(crate) fn reset_at(reset: i64) -> Timestamp {
    Timestamp::from_rfc3339(rfc3339_utc(reset * 1000))
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
    /// Whether the Job holds a slot the pool will take back. Asked before anything moves, so a
    /// `false` leaves the Job for the ordinary escalation.
    async fn holdable(&self, job_id: &JobId) -> Option<u32> {
        let job = self.load(job_id).await.ok()?;
        let slot = job.worktree_slot()?;
        let served = self.served_by(&job).ok()?;
        let standing = self
            .vcs()
            .slot_standing(&pool_of(&served), slot, job_id.as_str());
        matches!(standing, SlotStanding::Held).then_some(slot)
    }

    async fn parked_for_limit(&self, job_id: &JobId, slot: u32, reset: i64) -> Result<(), Adrift> {
        let job = self.load(job_id).await?;
        let pool = pool_of(&self.served_by(&job)?);
        self.parked_and_marked(
            job,
            JobStatus::Running,
            PausedBy::Fleet,
            Some(reset_at(reset)),
            slot,
            &pool,
            SAID,
        )
        .await?;
        let dropped = self.empty_the_inbox(job_id).await;
        self.dropped_with_the_job(job_id, dropped);
        Ok(())
    }

    /// Hold a Job whose Drone has already exited. **`false` and nothing changed** where the pool
    /// would not take the slot back.
    pub(crate) async fn held_for_limit(
        &self,
        job_id: &JobId,
        step: &StepId,
        reset: i64,
    ) -> Result<bool, Adrift> {
        let Some(slot) = self.holdable(job_id).await else {
            return Ok(false);
        };
        self.drone_left(job_id, step).await?;
        self.parked_for_limit(job_id, slot, reset).await?;
        Ok(true)
    }

    /// Hold a Job whose Drone has ended its run and is still standing: end it, then park.
    /// `None` and nothing changed where the pool would not take the slot back.
    pub(crate) async fn drone_held_for_limit(
        &self,
        reset: i64,
        working: &mut Option<Working>,
    ) -> Result<Option<StoodDown>, Adrift> {
        let Some(job_id) = working.as_ref().map(|at_work| at_work.standing().0) else {
            return Ok(None);
        };
        let Some(slot) = self.holdable(&job_id).await else {
            return Ok(None);
        };
        let Some(at_work) = working.take() else {
            return Ok(None);
        };
        let stood_down = self.stood_down_paying(at_work).await?;
        self.every_exit_recorded(&job_id).await?;
        self.parked_for_limit(&job_id, slot, reset).await?;
        Ok(Some(stood_down))
    }

    /// Whether admission must leave this Job alone: held for a limit that has not reset yet, and
    /// not lifted by a person's resume.
    pub(crate) fn held_by_a_limit(&self, job: &core_model::Job) -> bool {
        let Some(pause) = job.pause().filter(|pause| !pause.resuming) else {
            return false;
        };
        let (Some(until), Some(now)) = (
            pause.until.as_ref().and_then(Timestamp::epoch_millis),
            self.now().epoch_millis(),
        ) else {
            return false;
        };
        now < until + MARGIN_MILLIS
    }
}
