//! The diff of a fix a repair or a side run holds: what its branch changes
//! against the Job's own. `get_repair_diff`; `docs/concepts/trigger.md`.
//!
//! **Read from the repository with no worktree**, so it answers after the
//! Drone is done and the branch is parked, and until the fix is placed and the
//! branch is given back.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{JobDiff, RepairOf, WireError, Work};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The Trigger or added step has no repair branch recorded. A 422.
const NO_REPAIR_BRANCH: &str = "fleet.no_repair_branch";

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
    pub(crate) async fn repair_diff(
        &self,
        job_id: ipc::JobId,
        of: RepairOf,
    ) -> Result<JobDiff, Refusal> {
        let domain = job_id.to_domain();
        let job = self.load(&domain).await.map_err(|why| self.refusal(why))?;
        let none = || {
            Refusal::Unacceptable(
                WireError::raised(
                    NO_REPAIR_BRANCH,
                    String::from("no repair branch is recorded for that"),
                    self.run_id(),
                )
                .about_job(job_id.clone()),
            )
        };
        let branch = self.repair_branch_of(&job, &of).await?.ok_or_else(none)?;
        let from = job
            .branch()
            .map(|branch| branch.as_str().to_string())
            .ok_or_else(none)?;
        let served = self.served_by(&job).map_err(|why| self.refusal(why))?;
        let read = self
            .work()
            .branch_work(served.root(), &from, &branch)
            .map_err(|cause| self.unreadable(&domain, cause))?;
        Ok(JobDiff {
            job_id,
            work: Some(Work {
                files: crate::footprint::seen(&read.changed, None),
                measured_from: Some(from),
                measured_whole: true,
                plan_declared: false,
                patch: Some(read.patch.as_str().to_string()).filter(|text| !text.is_empty()),
            }),
        })
    }

    /// The branch the named Trigger's latest firing, or the added step, wrote
    /// its fix on.
    async fn repair_branch_of(
        &self,
        job: &core_model::Job,
        of: &RepairOf,
    ) -> Result<Option<String>, Refusal> {
        let store = self.store().lock().await;
        if let Some(addition) = &of.addition {
            return Ok(store
                .job_additions(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?
                .into_iter()
                .find(|one| &one.id == addition)
                .and_then(|one| one.repair.branch));
        }
        let Some(trigger) = &of.trigger else {
            return Ok(None);
        };
        Ok(store
            .firings_with_ids(job.id())
            .map_err(|why| self.refusal(Adrift::Reading(why)))?
            .into_iter()
            .rev()
            .find(|(_, one)| &one.name == trigger && one.repair.branch.is_some())
            .and_then(|(_, one)| one.repair.branch))
    }
}
