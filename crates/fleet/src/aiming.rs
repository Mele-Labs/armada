//! Where a Job lands, set after its approval for a Job that named nowhere.
//! Since 23.22. **Only once, and only before the work goes out**: a pull request opens against the target, so a target moved after it
//! would be a record the forge disagrees with.

use std::fmt;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{Component, Envelope, FieldValue, Job, JobId, JobStatus, Landing, Level};

use crate::adrift::Adrift;
use crate::budget::budgeted_for;
use crate::daemon::Fleet;

/// Why a landing target was not set, **and nothing was kept**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unaimed {
    Blank,
    /// The Job is still at its approval gate, where the approval sets it.
    AtTheGate,
    Ended(JobStatus),
    /// The Job already lands somewhere a person chose.
    AlreadyAimed(String),
    /// The work went out against where it was going to land.
    WentOut,
}

impl fmt::Display for Unaimed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unaimed::Blank => write!(
                out,
                "a landing target names a branch, and this one is blank"
            ),
            Unaimed::AtTheGate => write!(
                out,
                "the job is still at its approval gate, where the approval sets where it lands"
            ),
            Unaimed::Ended(status) => {
                write!(
                    out,
                    "the job is {}, so where it lands is settled",
                    status.as_wire()
                )
            }
            Unaimed::AlreadyAimed(target) => write!(out, "the job already lands in `{target}`"),
            Unaimed::WentOut => write!(
                out,
                "the work already went out against the base, so where it lands is settled"
            ),
        }
    }
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
    /// `set_landing_target`, from Bridge, answered with the Job.
    pub(crate) async fn set_landing_target_by_person(
        self: Arc<Self>,
        job_id: ipc::JobId,
        body: ipc::SetLandingTarget,
    ) -> Result<ipc::JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { fleet.aimed(&job_id.to_domain(), &body.target).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// Land this approved Job in `target`, where it lands in the base and its
    /// work has not gone out. **Every refusal before the write.**
    pub async fn aimed(&self, job_id: &JobId, target: &str) -> Result<Job, Adrift> {
        let refused = |why| Adrift::TargetNotSet {
            job: job_id.clone(),
            why,
        };
        let named =
            core_model::branch_named(Some(target)).ok_or_else(|| refused(Unaimed::Blank))?;
        let job = self.load(job_id).await?;
        match job.status() {
            JobStatus::AwaitingApproval => return Err(refused(Unaimed::AtTheGate)),
            status if status.is_terminal() => return Err(refused(Unaimed::Ended(status))),
            _ => {}
        }
        let landing = self.landing_of(job_id).await;
        if let Some(already) = &landing.target {
            return Err(refused(Unaimed::AlreadyAimed(already.as_str().to_string())));
        }
        let delivery = self
            .store()
            .lock()
            .await
            .delivery_for(job_id)
            .map_err(Adrift::Reading)?;
        if delivery.commit.is_some() || delivery.pull_request.is_some() {
            return Err(refused(Unaimed::WentOut));
        }
        let served = self.served_by(&job)?;
        let held = self
            .vcs()
            .branches(served.root(), served.manifest().base())
            .map_err(|why| Adrift::BranchesUnread {
                job: Some(job_id.clone()),
                why: why.to_string(),
            })?;
        if !held.iter().any(|branch| branch.name == named.as_str()) {
            return Err(Adrift::NoSuchBranch {
                job: job_id.clone(),
                named: named.as_str().to_string(),
            });
        }
        let aimed = Landing {
            target: Some(named.clone()),
            ..landing
        };
        self.store()
            .lock()
            .await
            .set_landing(job_id, &aimed)
            .map_err(Adrift::Writing)?;
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            AIMED,
        )
        .in_job(job_id.as_ulid().clone())
        .with_field("target", FieldValue::Str(named.as_str().to_string()));
        self.noted_in_the_log(job_id, &envelope);
        Ok(job)
    }
}

/// The Job's log line where a person set where it lands.
const AIMED: &str = "a person set where the approved job lands";
