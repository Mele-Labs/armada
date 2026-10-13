//! The hold on a Job while a Commander's repair Drone is out, and what a Fleet
//! that restarts does about one. `#2170`; `docs/concepts/commander.md`, *The
//! repair*.
//!
//! The hold is Fleet's memory ([`Underway`]) with a row in the store beside it
//! (`store::RepairOut`). The Drone goes with the process that read it, so a
//! repair found at boot is treated as one that handed nothing in, and not
//! resumed: a headless turn keeps no session to resume, and the path for a
//! repair that ended badly already exists. The Job's log says so, the Commander
//! is woken, and the loop bound counts it. What the boot adds is the hold: the
//! Job stays held to a second repair and to a person's restart until the wake is
//! made, which is the first turn after boot. A Job must never be left held with
//! nothing running.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobId, StepId};
use store::RepairOut;

use crate::daemon::Fleet;

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
    /// At boot, for one repository's Jobs: take the hold again for each repair
    /// left out over a Job that is still held, and note it for the first turn.
    /// A row over a Job that has moved on is given back.
    pub(crate) async fn repairs_left_out(&self, jobs: &[Job]) {
        let Ok(out) = self.store().lock().await.repairs_out() else {
            return;
        };
        for repair in out {
            let Some(job) = jobs.iter().find(|job| job.id() == &repair.job_id) else {
                continue;
            };
            match self.held_for_a_person(job) {
                Ok(()) => {
                    self.repairing().restored(job.id());
                    self.stirred_by_a_lost_repair(repair);
                }
                Err(_) => self.repair_dealt_with(job.id(), &repair.drone_id).await,
            }
        }
    }

    /// The wake for a repair the last Fleet lost: the hold given back first, as
    /// a repair that ends gives it back before the wake whose turn may send
    /// another.
    pub(crate) async fn repair_lost(self: &Arc<Self>, repair: RepairOut) {
        self.repairing().released(&repair.job_id);
        self.nothing_handed_in(
            &repair.job_id,
            &StepId::new(repair.step.as_str()),
            &repair.broke,
            String::from("Fleet restarted while the repair Drone was out"),
        )
        .await;
        self.repair_dealt_with(&repair.job_id, &repair.drone_id).await;
    }
}

/// The Jobs a repair Drone is on, so a second is refused and a person's
/// restart does not put a step Drone beside it. In memory, with
/// `store::RepairOut` beside it: a Fleet that restarts ends the Drone with the
/// process that read it, and reads the hold back from the store
/// (`Underway::restored`) until it has dealt with the repair.
#[derive(Clone, Default)]
pub(crate) struct Underway(Arc<Mutex<BTreeSet<JobId>>>);

impl Underway {
    pub(crate) fn taking(&self, job: &JobId) -> Option<Held> {
        let mut held = self.0.lock().expect("not poisoned");
        held.insert(job.clone()).then(|| Held {
            all: self.clone(),
            job: job.clone(),
        })
    }

    pub(crate) fn holds(&self, job: &JobId) -> bool {
        self.0.lock().expect("not poisoned").contains(job)
    }

    /// The hold a previous Fleet had, taken again at boot for a repair whose
    /// Drone is gone. Nothing gives it back on its own: the wake that deals
    /// with the repair does, with [`released`](Underway::released).
    pub(crate) fn restored(&self, job: &JobId) {
        self.0.lock().expect("not poisoned").insert(job.clone());
    }

    pub(crate) fn released(&self, job: &JobId) {
        self.0.lock().expect("not poisoned").remove(job);
    }
}

/// A Job's hold, given back when the repair task ends however it ends.
pub(crate) struct Held {
    all: Underway,
    job: JobId,
}

impl Drop for Held {
    fn drop(&mut self) {
        self.all.0.lock().expect("not poisoned").remove(&self.job);
    }
}
