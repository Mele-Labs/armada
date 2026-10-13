//! The settings a person changes on one Job. Each is one line in `commanding`'s `Commands` impl,
//! which cannot be split across files and reached the 1200-line rule.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{JobId, JobSummary};

use crate::budget::budgeted_for;
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
    /// How this Job meets a blocked command, changed. Loaded first, so an id
    /// naming nothing is a 404 rather than a setting written against nothing.
    pub(super) async fn when_blocked_set(
        self: Arc<Self>,
        job_id: JobId,
        setting: ipc::SetWhenBlocked,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let job = fleet.load(&id).await?;
                let when = crate::permitting::domain_setting(setting.when_blocked);
                Fleet::set_when_blocked(&fleet, &id, when)
                    .await
                    .map_err(|why| why.about(&id))?;
                Ok(job)
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// How this Job meets a Judge criterion that refuses, changed. Loaded
    /// first, for `set_when_blocked`'s reason.
    pub(super) async fn when_refused_set(
        self: Arc<Self>,
        job_id: JobId,
        setting: ipc::SetWhenRefused,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let job = fleet.load(&id).await?;
                let when = crate::asking::domain_when_refused(setting.when_refused);
                Fleet::set_when_refused(&fleet, &id, when).await?;
                Ok(job)
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The model this Job's later steps spawn on, chosen or cleared. Loaded
    /// first for `set_when_blocked`'s reason; the refusals are
    /// `crate::permitting::NotPermitted`'s.
    pub(super) async fn model_set(
        self: Arc<Self>,
        job_id: JobId,
        choice: ipc::SetModel,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let job = fleet.load(&id).await?;
                Fleet::set_model(&fleet, &id, choice.model.as_deref())
                    .await
                    .map_err(|why| why.about(&id))?;
                Ok(job)
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// The model this Job's review step spawns on, chosen or cleared, on `set_model`'s terms.
    pub(super) async fn review_model_set(
        self: Arc<Self>,
        job_id: JobId,
        choice: ipc::SetModel,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let job = fleet.load(&id).await?;
                Fleet::set_review_model(&fleet, &id, choice.model.as_deref())
                    .await
                    .map_err(|why| why.about(&id))?;
                Ok(job)
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// A command a person allowed for this Job, taken back.
    pub(super) async fn allowed_command_removed(
        self: Arc<Self>,
        job_id: JobId,
        removing: ipc::RemoveAllowedCommand,
    ) -> Result<JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let job = fleet.load(&id).await?;
                Fleet::remove_allowed_command(&fleet, &id, &removing.run)
                    .await
                    .map_err(|why| why.about(&id))?;
                Ok(job)
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }
}
