//! Who asked for a run, where a Drone did. `docs/concepts/manifest.md`,
//! *Who asked for a run*.
//!
//! **Placed by the connection, never by what the call says**: the agent's door
//! hands on the connection a call arrived on (`api::asking`) and Fleet matches
//! it against the Drones it holds, as it does a Drone's own tool call. A
//! caller no Drone holds is a person, or a Helm session, and reads `outside`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::crew::as_caller;
use crate::daemon::Fleet;
use crate::dry_run::asked::requester;

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
    /// The requester of the run being started: the Drone whose connection the
    /// call is on, with the step and task it is at, else `outside`.
    pub(super) async fn asked_by(&self) -> ipc::Requester {
        let Some(caller) = api::asking() else {
            return ipc::Requester::outside();
        };
        let Ok((job, drone)) = self.placed_drone(&caller) else {
            return ipc::Requester::outside();
        };
        let Some(slot) = as_caller(drone, self.slot_of(&job)).await else {
            return ipc::Requester::outside();
        };
        let (step, drone, task) = {
            let working = slot.lock().await;
            let Some(at_work) = working.as_ref() else {
                return ipc::Requester::outside();
            };
            let (_, step, _) = at_work.standing();
            let (_, _, drone) = at_work.drone();
            (step, drone, at_work.task())
        };
        match self.load(&job).await {
            Ok(record) => requester(&job, &record.handle(), &step, &drone, task),
            Err(_) => ipc::Requester::outside(),
        }
    }
}
