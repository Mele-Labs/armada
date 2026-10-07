//! Which way a pull request opens when a person's approval says nothing: the
//! workflow's delivering step, the repository, this machine, then ready.
//! `crate::approving::pr_mode_default` is the order, and this is what asks the
//! repository and the store for the tiers beneath it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobId};

use crate::adrift::Adrift;
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
    /// Approve with no body: the press, as it ever was, **except where a
    /// default says the pull request opens as a draft**. Then the landing is
    /// kept first, so the Job opens as the repository or this machine said and
    /// not as ready. A default of ready writes nothing: that is every Job
    /// before the default existed, and it reads as a Job with no landing.
    pub async fn approve_as_proposed(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        let drafts = match self.served_by(&job) {
            Ok(served) => {
                let (repository, machine) = self.pr_modes_beneath(&served).await;
                crate::approving::pr_mode_default(job.workflow(), repository, machine)
                    == core_model::PrMode::Draft
            }
            Err(_) => false,
        };
        match drafts {
            true => {
                self.approve_as_left(job_id, &ipc::ApproveDispatch::default())
                    .await
            }
            false => self.approve(job_id).await,
        }
    }

    /// The repository's pull request mode and this machine's, the two tiers
    /// beneath a workflow's step. **The machine's is `None` until a person
    /// turns drafts on**, which reads the same as ready at the bottom of the
    /// order.
    pub(crate) async fn pr_modes_beneath(
        &self,
        served: &crate::repositories::Served,
    ) -> (Option<core_model::PrMode>, Option<core_model::PrMode>) {
        let drafts = self
            .store()
            .lock()
            .await
            .preferences()
            .is_ok_and(|saved| saved.draft_pull_requests);
        (
            served.manifest().pr_mode(),
            drafts.then_some(core_model::PrMode::Draft),
        )
    }

    /// What a Job not yet approved will open as, so Bridge's choice starts on
    /// it. **Absent on an approved Job**, whose `landing` says.
    pub(crate) async fn pr_mode_served(&self, job: &Job, detail: &mut ipc::JobDetail) {
        if detail.landing.is_some() {
            return;
        }
        if let Ok(served) = self.served_by(job) {
            let (repository, machine) = self.pr_modes_beneath(&served).await;
            detail.pr_mode_default =
                Some(crate::approving::pr_mode_default(job.workflow(), repository, machine).into());
        }
    }
}
