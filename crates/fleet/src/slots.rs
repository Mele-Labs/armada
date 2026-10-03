//! How many Drones Fleet works at once, and where each one is held: a Job's
//! kept Drone in the Job's slot, and each Drone beside it in a slot of its own
//! (spike 022, slice 5).
//!
//! **The bound is configured, not computed.** [`Concurrency`] is handed in by
//! the composition root as the shipped number, replaced by a saved one at
//! assembly and again whenever a person saves — `crate::limits` — and
//! `settings.toml`'s `concurrency-cap` row is the specification it satisfies.
//! Machine headroom is `crate::headroom`'s, a separate question.
//!
//! **It bounds Drones, never approvals.** `docs/concepts/fleet.md` holds that
//! rule; nothing here is reached until a person has approved, because
//! `admit_next` picks from `queued` and nothing but an approval puts a Job
//! there.
//!
//! # Two locks, and the order between them is the whole of the concurrency
//!
//! The roster — this type, behind one mutex — and each Job's own slot, behind
//! its own. **The roster is always taken first.** It is held briefly to find a
//! slot, and across an admission, so one dispatch runs at a time; a slot is held
//! for one thing about one Job at a time, and never across a Check or a Judge
//! call — `crate::settling` and `crate::turning` say what that cost.
//!
//! One lock over the whole set would have been the working slot again under
//! another name: a Job running `cargo nextest` would have held every other
//! Drone out of `submit_evidence` for the length of it. What waits on a Job's
//! own slot is that Job's own work, and only for as long as one step of it.

use std::collections::BTreeMap;
use std::sync::Arc;

use core_model::{DroneId, JobId};
use tokio::sync::Mutex;

use crate::working::Working;

/// How many Drones Fleet may be working at one time. **Drones, not Jobs**,
/// since slice 5: a Job's kept Drone counts from its admission to its end,
/// and each Drone it runs beside that one counts while it runs.
///
/// **No `Default`**, for the reason [`Liveness`](crate::Liveness) has none: the
/// number is a decision somebody made and wrote down, and a type that supplies
/// one lets a caller not make it.
///
/// Zero is not expressible. A Fleet that may work no Jobs is a Fleet that
/// dispatches nothing and reports nothing about why, so [`Concurrency::of`]
/// raises a zero to one rather than admitting a value with no working state
/// under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Concurrency(usize);

impl Concurrency {
    pub const fn of(jobs: usize) -> Concurrency {
        Concurrency(if jobs == 0 { 1 } else { jobs })
    }

    pub const fn jobs(&self) -> usize {
        self.0
    }
}

/// One Job's working slot: the same `Option<Working>` the single slot was,
/// with its own lock.
///
/// **`Some` is still the whole of what "this Job is being worked" means.** What
/// changed is that there is one of these per Job rather than one for Fleet.
pub(crate) type Slot = Arc<Mutex<Option<Working>>>;

/// Which Jobs are being worked, and how many may be.
///
/// **It holds no `Working` itself.** Every value in the map is a lock somebody
/// else may be inside, which is what lets one Job's gate run a Check without
/// another Job's Drone waiting on it.
pub(crate) struct Slots {
    cap: Concurrency,
    held: BTreeMap<JobId, Slot>,
    /// Each Job's Drones beside its kept one, each in a slot of its own so one
    /// Drone's tool call never waits behind another's.
    crew: BTreeMap<JobId, BTreeMap<DroneId, Slot>>,
    /// Each held Job's one transcript channel, made at its admission and
    /// dropped when the Job leaves the roster.
    channels: BTreeMap<JobId, api::Channel>,
    turns: api::Turns,
}

impl Slots {
    pub(crate) fn bounded_by(cap: Concurrency, turns: api::Turns) -> Slots {
        Slots {
            cap,
            held: BTreeMap::new(),
            crew: BTreeMap::new(),
            channels: BTreeMap::new(),
            turns,
        }
    }

    /// Forget every slot whose Drone has gone.
    ///
    /// A slot is emptied by the code holding it — `end_the_drone`, `reap`,
    /// `stood_down` — and that code cannot remove the entry, because removing
    /// it means taking the roster while holding a slot and the order forbids
    /// it. So the entry is swept here instead, before anything counts.
    ///
    /// **`try_lock`, never `lock`.** A slot somebody is inside is a Job being
    /// worked whatever it holds, and a sweep that waited for it would be the
    /// roster blocking on a Check.
    fn sweep(&mut self) {
        let kept = |slot: &Slot| match slot.try_lock() {
            Ok(held) => held.is_some(),
            Err(_) => true,
        };
        self.held.retain(|_, slot| kept(slot));
        for crew in self.crew.values_mut() {
            crew.retain(|_, slot| kept(slot));
        }
        self.crew.retain(|_, crew| !crew.is_empty());
        let held = &self.held;
        self.channels.retain(|job, _| held.contains_key(job));
    }

    /// How many Drones are being worked: one per held Job, its kept Drone,
    /// and each Drone beside one.
    pub(crate) fn count(&mut self) -> usize {
        self.sweep();
        self.held.len() + self.crew.values().map(BTreeMap::len).sum::<usize>()
    }

    /// How many Drones this Job is working, its kept one included.
    pub(crate) fn of_job(&mut self, job: &JobId) -> usize {
        self.sweep();
        usize::from(self.held.contains_key(job)) + self.crew.get(job).map_or(0, BTreeMap::len)
    }

    /// How many Drones may be worked at once. `settings.concurrency-cap`.
    ///
    /// **Read beside [`Slots::count`] and never on its own.** "2 of 2" is one
    /// fact and a bound with no occupancy beside it says nothing about whether
    /// anything can start.
    pub(crate) fn cap(&self) -> usize {
        self.cap.jobs()
    }

    /// Replace the bound. **Nothing already held is closed**: a bound lowered
    /// below [`Slots::count`] only stops [`Slots::room`] answering yes until
    /// enough Jobs finish.
    pub(crate) fn rebound(&mut self, cap: Concurrency) {
        self.cap = cap;
    }

    /// Whether another Drone may start. **The predicate `admit_next` opens
    /// with, and the one `queued_reason` answers `waiting_on_resources` from.**
    /// One answer, not two — a Board saying a Job is blocked while Fleet is
    /// starting it is what a second predicate here would produce.
    pub(crate) fn room(&mut self) -> bool {
        self.count() < self.cap.jobs()
    }

    /// Every Job being worked, oldest id first.
    pub(crate) fn working_on(&mut self) -> Vec<JobId> {
        self.sweep();
        self.held.keys().cloned().collect()
    }

    /// Every Job's slot, for a turn to walk.
    ///
    /// The handles are cloned out and the roster lock is dropped before any of
    /// them is taken, which is what keeps the order — roster, then slot — with
    /// no path holding both.
    pub(crate) fn each(&mut self) -> Vec<(JobId, Slot)> {
        self.sweep();
        self.held
            .iter()
            .map(|(job, slot)| (job.clone(), Arc::clone(slot)))
            .collect()
    }

    /// This Job's slot, where it has one. `None` is a Job with no Drone.
    pub(crate) fn slot_of(&self, job: &JobId) -> Option<Slot> {
        self.held.get(job).map(Arc::clone)
    }

    /// This Job's slot, made if it has none.
    ///
    /// **Admission's, and nothing else calls it.** A slot that exists holding
    /// nothing counts against the bound while the dispatch that will fill it
    /// runs, which is right: the Job has been taken out of the queue.
    pub(crate) fn opened_for(&mut self, job: &JobId) -> Slot {
        if !self.channels.contains_key(job) {
            let channel = self.turns.opening(&ipc::JobId::from(job));
            self.channels.insert(job.clone(), channel);
        }
        Arc::clone(
            self.held
                .entry(job.clone())
                .or_insert_with(|| Arc::new(Mutex::new(None))),
        )
    }

    /// A slot for one more Drone beside this Job's kept one.
    pub(crate) fn joined(&mut self, job: &JobId, drone: &DroneId) -> Slot {
        Arc::clone(
            self.crew
                .entry(job.clone())
                .or_default()
                .entry(drone.clone())
                .or_insert_with(|| Arc::new(Mutex::new(None))),
        )
    }

    /// The slot of a Drone beside this Job's kept one, where it is one.
    pub(crate) fn crew_slot(&self, job: &JobId, drone: &DroneId) -> Option<Slot> {
        self.crew.get(job)?.get(drone).map(Arc::clone)
    }

    /// Every Drone beside this Job's kept one, cloned out for a turn to walk.
    pub(crate) fn crew_of(&mut self, job: &JobId) -> Vec<(DroneId, Slot)> {
        self.sweep();
        self.crew.get(job).map_or_else(Vec::new, |crew| {
            crew.iter()
                .map(|(drone, slot)| (drone.clone(), Arc::clone(slot)))
                .collect()
        })
    }

    /// Forget a slot beside a Job's kept one, whatever it holds: a spawn that
    /// did not start.
    pub(crate) fn left(&mut self, job: &JobId, drone: &DroneId) {
        if let Some(crew) = self.crew.get_mut(job) {
            crew.remove(drone);
        }
    }

    /// Forget this Job's slot outright, whatever it holds.
    ///
    /// For the one caller that knows the dispatch failed and must not leave the
    /// bound spent on a Job with no Drone.
    pub(crate) fn closed(&mut self, job: &JobId) {
        self.held.remove(job);
        self.channels.remove(job);
    }
}
