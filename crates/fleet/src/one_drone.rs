//! A person's message or stop, addressed to one Drone of a Job rather than to
//! the Job: #1666, spike 022 slice 5. A Drone that is not live is refused with
//! `fleet.drone_not_live`, never quietly redirected at another.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{DroneId, Job, JobId};
use store::ExtraEnded;

use crate::adrift::Adrift;
use crate::crew::as_caller;
use crate::daemon::Fleet;
use crate::resume::Redirection;

/// Which of a Job's live Drones a request named.
enum Named {
    /// The Job's kept Drone: the Job-wide act applies.
    Kept,
    /// One beside it.
    Beside,
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
    /// Stop one Drone. **The kept one is `kill_drone`**: its step stops, and so
    /// does every Drone beside it. One beside it ends alone, and its task waits
    /// for the kept Drone.
    pub async fn kill_one_drone(&self, job_id: &JobId, drone: &DroneId) -> Result<Job, Adrift> {
        match self.which_live(job_id, drone).await? {
            Named::Kept => self.kill_drone(job_id).await,
            Named::Beside => {
                let slot = self.slots().lock().await.crew_slot(job_id, drone);
                if let Some(slot) = slot {
                    let mut working = slot.lock().await;
                    self.end_one_beside(job_id, drone, &mut working, ExtraEnded::Killed)
                        .await?;
                }
                self.load(job_id).await
            }
        }
    }

    /// A person's words to one Drone. The kept one is `redirect`; one beside
    /// it is told in its own session, and the Job does not move.
    pub async fn redirect_one_drone(
        &self,
        job_id: &JobId,
        drone: &DroneId,
        instruction: &Redirection,
        by: api::Redirector,
    ) -> Result<Job, Adrift> {
        let named = match self.which_live(job_id, drone).await? {
            Named::Kept => None,
            Named::Beside => Some(drone.clone()),
        };
        as_caller(named, self.redirect(job_id, instruction, by)).await
    }

    async fn which_live(&self, job_id: &JobId, drone: &DroneId) -> Result<Named, Adrift> {
        let (kept, beside) = {
            let slots = self.slots().lock().await;
            (slots.slot_of(job_id), slots.crew_slot(job_id, drone))
        };
        if let Some(slot) = kept {
            let holds = slot
                .lock()
                .await
                .as_ref()
                .is_some_and(|at_work| at_work.drone().2 == *drone);
            if holds {
                return Ok(Named::Kept);
            }
        }
        if let Some(slot) = beside {
            if slot.lock().await.is_some() {
                return Ok(Named::Beside);
            }
        }
        Err(Adrift::DroneNotLive {
            job: job_id.clone(),
            drone: drone.clone(),
        })
    }
}
