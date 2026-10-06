//! Auto-release: when work is waiting for a worktree slot and the pool is
//! full, pause a parked Job so the work can start. `docs/concepts/fleet.md`,
//! *A paused Job gives its slot back*.
//!
//! **One victim per waiter per turn, and the victim is never resumed by
//! Fleet.** A Job is a victim only at a gate with no Drone, so what a pause
//! takes from it is a slot and nothing a person is waiting on. Fleet cannot
//! see what Bridge shows, so a Job whose last event is under the grace window
//! is left alone, and so is one a person just resumed or one the park just
//! refused: both sit in `spared` and read as moved at that instant, which is
//! also what stops a refusal being retried every tick.

use std::collections::{BTreeMap, BTreeSet};

use adapter_traits::{AgentHarness, Delivery, SlotStanding, Vcs, WorkProduct};
use core_model::{
    Component, Envelope, FieldValue, Job, JobId, JobStatus, Level, PausedBy, Timestamp,
};
use store::Moved;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::leasing::pool_of;
use crate::repositories::Served;

/// Jobs Fleet must not pick as victims until the grace window after the
/// instant here has passed.
pub(crate) type Spared = std::sync::Mutex<BTreeMap<JobId, Timestamp>>;

const MINUTE_MILLIS: i64 = 60_000;

/// A parked Job as a victim, and when it reached the status it is parked at.
struct Candidate {
    job: Job,
    in_status_since: i64,
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
    /// **After admission**, which has already started whatever the pool had
    /// room for: what is still waiting for a slot now is waiting for a pool
    /// that is full. Answers the Jobs paused.
    pub(crate) async fn release_for_waiters(&self) -> Result<Vec<JobId>, Adrift> {
        // A waiter that the Drone bound or the machine would hold back anyway
        // gains nothing from a slot.
        let room = {
            let mut slots = self.slots().lock().await;
            self.room_for(&mut slots).await.granted()
        };
        if !room {
            return Ok(Vec::new());
        }
        let waiters = self.waiting_for_a_slot().await?;
        if waiters.is_empty() {
            return Ok(Vec::new());
        }
        let (loaded, _) = self.every_job().await?;
        let mut released: Vec<JobId> = Vec::new();
        for waiter in waiters {
            let Ok(served) = self.served_by(&waiter) else {
                continue;
            };
            let allowed = served.manifest().auto_release();
            if !allowed.on {
                continue;
            }
            let grace = i64::from(allowed.grace_minutes.get()) * MINUTE_MILLIS;
            let taken: BTreeSet<&JobId> = released.iter().collect();
            let mut candidates = Vec::new();
            for job in &loaded.jobs {
                if taken.contains(job.id()) || !self.could_be_taken(job, &served).await {
                    continue;
                }
                if let Some(found) = self.aged_candidate(job, grace).await? {
                    candidates.push(found);
                }
            }
            candidates.sort_by(|a, b| {
                (a.in_status_since, a.job.id()).cmp(&(b.in_status_since, b.job.id()))
            });
            for victim in candidates {
                match self
                    .paused_by(victim.job.id(), PausedBy::Fleet, Some(&waiter))
                    .await
                {
                    Ok(_) => {
                        released.push(victim.job.id().clone());
                        break;
                    }
                    Err(why) => {
                        self.noted_not_released(&victim.job, &waiter, &why);
                        self.spare(victim.job.id());
                    }
                }
            }
        }
        Ok(released)
    }

    /// Leave this Job alone for a grace window from now. A person's resume
    /// calls it, so a Job they just took back is not taken again.
    pub(crate) fn spare(&self, job: &JobId) {
        if let Ok(mut spared) = self.spared().lock() {
            spared.insert(job.clone(), self.now());
        }
    }

    /// The Jobs waiting for a slot of their own, oldest approval first: never
    /// started or re-queued, clear to run in every way but the pool being
    /// full. **A resume is not one**, so a person's Resume never forces
    /// another Job out.
    async fn waiting_for_a_slot(&self) -> Result<Vec<Job>, Adrift> {
        let mut skipped = Vec::new();
        let mut waiters = Vec::new();
        while let Some(job) = self.next_queued(&skipped).await? {
            skipped.push(job.id().clone());
            if job.pause().is_none()
                && !self.volume_is_short(&job).await
                && self.slot_is_short(&job)
            {
                waiters.push(job);
            }
        }
        Ok(waiters)
    }

    /// The part of the victim rule that costs no read.
    async fn could_be_taken(&self, job: &Job, served: &Served) -> bool {
        let at_a_gate = matches!(
            job.status(),
            JobStatus::AwaitingReview | JobStatus::AwaitingRepair | JobStatus::Escalated
        );
        let (Some(slot), true) = (job.worktree_slot(), at_a_gate) else {
            return false;
        };
        if job.pause().is_some() || job.redirect_waiting().is_some() {
            return false;
        }
        let Ok(its) = self.served_by(job) else {
            return false;
        };
        if its.root() != served.root() {
            return false;
        }
        // An escalated Job may still hold an idle Drone; it is not parked.
        if self.slots().lock().await.slot_of(job.id()).is_some() {
            return false;
        }
        matches!(
            self.vcs()
                .slot_standing(&pool_of(&its), slot, job.id().as_str()),
            SlotStanding::Held
        )
    }

    /// The Job with its two instants, or nothing where it moved inside the
    /// window or its events would not read.
    async fn aged_candidate(&self, job: &Job, grace: i64) -> Result<Option<Candidate>, Adrift> {
        let events = self
            .store()
            .lock()
            .await
            .events_for(job.id())
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        let millis = |at: &Timestamp| at.epoch_millis();
        let Some(last) = events.iter().filter_map(|event| millis(event.at())).max() else {
            return Ok(None);
        };
        let spared = self
            .spared()
            .lock()
            .ok()
            .and_then(|spared| spared.get(job.id()).and_then(|at| millis(at)));
        let Some(now) = millis(&self.now()) else {
            return Ok(None);
        };
        let moved = last.max(spared.unwrap_or(i64::MIN));
        if now - moved < grace {
            return Ok(None);
        }
        let in_status_since = events
            .iter()
            .rev()
            .find(|event| matches!(event.moved(), Moved::Job { to, .. } if *to == job.status()))
            .and_then(|event| millis(event.at()))
            .unwrap_or(last);
        Ok(Some(Candidate {
            job: job.clone(),
            in_status_since,
        }))
    }

    fn noted_not_released(&self, victim: &Job, waiter: &Job, why: &Adrift) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "Fleet would not pause the Job to free its slot, and leaves it alone for now",
        )
        .in_job(victim.id().as_ulid().clone())
        .with_field(
            "needed_by",
            FieldValue::Str(waiter.id().as_str().to_string()),
        )
        .with_field("cause", FieldValue::Str(why.to_string()));
        self.noted_in_the_log(victim.id(), &envelope);
    }
}
