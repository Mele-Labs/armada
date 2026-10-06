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

/// A Job parked at a gate, as the victim rule reads it. Instants are epoch
/// milliseconds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parked {
    pub job: JobId,
    /// When it reached the status it is parked at.
    pub in_status_since: i64,
    /// Its latest event, or the moment a person last resumed it or a park of
    /// it was refused, whichever is later.
    pub last_moved: i64,
}

/// The Jobs Fleet may take, in the order it tries them: those not moved within
/// `grace_millis` of `now`, the longest at their status first, ties by Job id.
pub fn release_order(parked: &[Parked], now: i64, grace_millis: i64) -> Vec<JobId> {
    let mut eligible: Vec<&Parked> = parked
        .iter()
        .filter(|one| now - one.last_moved >= grace_millis)
        .collect();
    eligible.sort_by(|a, b| (a.in_status_since, &a.job).cmp(&(b.in_status_since, &b.job)));
    eligible.into_iter().map(|one| one.job.clone()).collect()
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
        // A waiter the Drone bound would hold back gains nothing from a slot.
        // The bound is asked first because it reads nothing; the machine is
        // asked only once there is a waiter, so an idle turn takes no reading.
        if !self.slots().lock().await.room() {
            return Ok(Vec::new());
        }
        let waiters = self.waiting_for_a_slot().await?;
        if waiters.is_empty() {
            return Ok(Vec::new());
        }
        let room = {
            let mut slots = self.slots().lock().await;
            self.room_for(&mut slots).await.granted()
        };
        if !room {
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
            let mut parked = Vec::new();
            for job in &loaded.jobs {
                if taken.contains(job.id()) || !self.could_be_taken(job, &served).await {
                    continue;
                }
                if let Some(found) = self.parked_since(job).await? {
                    parked.push(found);
                }
            }
            let Some(now) = self.now().epoch_millis() else {
                continue;
            };
            for victim in release_order(&parked, now, grace) {
                let Some(victim) = loaded.jobs.iter().find(|job| job.id() == &victim) else {
                    continue;
                };
                match self
                    .paused_by(victim.id(), PausedBy::Fleet, Some(&waiter))
                    .await
                {
                    Ok(_) => {
                        released.push(victim.id().clone());
                        break;
                    }
                    Err(why) => {
                        self.noted_not_released(victim, &waiter, &why);
                        self.spare(victim.id());
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

    /// The Job with its two instants, or nothing where its events would not
    /// read.
    async fn parked_since(&self, job: &Job) -> Result<Option<Parked>, Adrift> {
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
            .and_then(|spared| spared.get(job.id()).and_then(millis));
        let in_status_since = events
            .iter()
            .rev()
            .find(|event| matches!(event.moved(), Moved::Job { to, .. } if *to == job.status()))
            .and_then(|event| millis(event.at()))
            .unwrap_or(last);
        Ok(Some(Parked {
            job: job.id().clone(),
            in_status_since,
            last_moved: last.max(spared.unwrap_or(i64::MIN)),
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
