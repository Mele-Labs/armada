//! Edit this task: a person changes one task's title, brief, files, done-when
//! or model (#1657). Spike 022, slice 3, with the body its lock agrees —
//! `EditTask` as Bridge sends it, only the changed fields.
//!
//! **Before the plan's gate and after a failure alike**: the plan decides
//! which tasks take an edit (`core_model::PlanChange::Edited`), an open or a
//! failed one, so the act asks no step where the Job is. **Recorded as a
//! person's change after whatever the plan was approved as**, and the next
//! Drone put on the task reads the edit in its brief.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{PlanChange, TaskEdit, TaskId};

use crate::adrift::Adrift;
use crate::budget::budgeted_for;
use crate::daemon::Fleet;

/// What an `edit_task` body changes, or `None` where it changes nothing,
/// blanks the title, or names a blank model. **Whether the model is one this
/// Fleet offers is not this function's to say**: `list_models` is Fleet's.
pub fn the_edit(body: &ipc::EditTask) -> Option<TaskEdit> {
    let scope: Option<Vec<&str>> = body
        .scope
        .as_ref()
        .map(|paths| paths.iter().map(String::as_str).collect());
    let model = match &body.model {
        Some(named) => Some(core_model::ModelName::new(named).ok()?),
        None => None,
    };
    TaskEdit::new(
        body.title.as_deref(),
        body.note.as_deref(),
        scope.as_deref(),
        body.expects.as_deref(),
        model,
    )
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
    /// Edit this task, from Bridge. A model `list_models` does not offer is
    /// refused before anything else is read, as `set_model` refuses one.
    pub(crate) async fn edit_task_by_person(
        self: Arc<Self>,
        job_id: ipc::JobId,
        task: String,
        body: ipc::EditTask,
    ) -> Result<ipc::WorkPlan, Refusal> {
        let job = job_id.to_domain();
        let task = TaskId::read(task.trim()).ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        if let Some(named) = &body.model {
            self.offered(named)
                .map_err(|why| self.refusal(why.about(&job)))?;
        }
        let edit = the_edit(&body).ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let change = PlanChange::Edited { task, edit };
        let plan = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            let job = job.clone();
            async move { fleet.changed_by_person(&job, &change).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        let runs = self
            .group_runs_of(&job)
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(ipc::WorkPlan::of(&plan, &runs))
    }

    /// Set the Job's tier map, from Bridge, and answer with the Job.
    pub(crate) async fn set_tiers_by_person(
        self: Arc<Self>,
        job_id: ipc::JobId,
        body: ipc::SetTiers,
    ) -> Result<ipc::JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                let id = job_id.to_domain();
                let job = fleet.load(&id).await?;
                fleet
                    .set_tiers(&id, &body.tiers)
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
