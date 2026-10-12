//! How many times the Judges sent each step's current pass back to its Drone, on the step a
//! person reads. #2105, `docs/concepts/judge.md`, *When the Judges agree*.
//!
//! **Filled after `ipc::JobDetail::of`**, as the approval's facts are: the count is a read of the
//! Job's log, `store::Store::step_sent_back_by_judges`, which is what `AGREED_RETRIES` is asked
//! against, so the step and the gate cannot disagree about it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Job;

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
    /// Each step's count, absent where none went back. **A store that will not say leaves it
    /// absent**, which reads as nothing sent back rather than as a failed read of the Job.
    pub(crate) async fn sent_back_served(&self, job: &Job, detail: &mut ipc::JobDetail) {
        let store = self.store().lock().await;
        for step in &mut detail.steps {
            let times = store
                .step_sent_back_by_judges(job.id(), &step.step_id.to_domain())
                .unwrap_or(0);
            step.sent_back = ipc::JudgesSentBack::of(times);
        }
    }
}
