//! What a person is handed when they take a Job over. `docs/concepts/pilot.md`,
//! *The handoff bundle*.
//!
//! **Read off the record, and the Drone supplies one field.** The bundle is
//! the Job whole, every move it made, the evidence each step submitted, the plan
//! each run declared beside what the worktree holds now, and the narrative a
//! Drone gave where it gave one. Nothing in it is a thing a Drone could have
//! shaped except that narrative, which is why it is carried apart.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Queries as _, Refusal};
use core_model::Job;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::piloting::Unpilotable;

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
    /// The bundle for a Job that has been taken over, current or last.
    ///
    /// **The seam the session host reads**: it starts a Session on
    /// `worktree.path` with this as its first context. Refused where the Job
    /// was never piloted, since there is no pilot for the bundle to be for.
    pub async fn handoff_bundle(&self, job_id: ipc::JobId) -> Result<ipc::HandoffBundle, Refusal> {
        let id = job_id.to_domain();
        let job = self.load(&id).await.map_err(|why| self.refusal(why))?;
        let pilot = self
            .store()
            .lock()
            .await
            .pilot_of(&id)
            .map_err(|why| self.refusal(Adrift::Reading(why)))?
            .filter(|pilot| pilot.piloted_at.is_some());
        let Some(pilot) = pilot else {
            return Err(self.refusal(Adrift::CannotPilot {
                job: id,
                why: Unpilotable::NotPiloted {
                    status: job.status(),
                },
            }));
        };
        let detail = self.job_detail(job_id.clone()).await?;
        let history = self.get_job_events(job_id.clone()).await?;
        let evidence = self.get_evidence(job_id.clone()).await?;
        let (plans, plan_declared, changed) = self.plan_against_worktree(&job).await?;
        let stopped_on = job.stopped_on().map(|(step, trigger)| ipc::StoppedOn {
            step_id: step.into(),
            trigger: Some(trigger.as_wire().to_string()),
        });
        let worktree = self
            .worktree_of(&job)
            .map_err(|why| self.refusal(why))?
            .map(|worktree| ipc::HandoffWorktree {
                path: worktree.path().to_string(),
                branch: worktree.branch().to_string(),
            });
        Ok(ipc::HandoffBundle {
            job: detail,
            history,
            evidence,
            reason: pilot.reason.as_wire().to_string(),
            session_id: pilot.session,
            worktree,
            stopped_on,
            plans,
            plan_declared,
            changed,
            narrative: pilot.narrative.map(|said| ipc::DroneNarrative {
                trying_to: said.trying_to,
                blocked_by: said.blocked_by,
                tried: said.tried,
            }),
        })
    }

    /// Every declared plan, whether the step in hand declared one, and the
    /// worktree's files marked against its newest.
    async fn plan_against_worktree(
        &self,
        job: &Job,
    ) -> Result<(Vec<ipc::DeclaredPlan>, bool, Vec<ipc::ChangedFile>), Refusal> {
        let plans = self
            .store()
            .lock()
            .await
            .step_plans(job.id())
            .map_err(|why| self.refusal(Adrift::Reading(why)))?;
        let in_hand = job
            .stopped_on()
            .map(|(step, _)| step)
            .or_else(|| job.current_step_id());
        let declared = in_hand.and_then(|step| {
            plans
                .iter()
                .filter(|plan| plan.step_id == *step)
                .max_by_key(|plan| plan.attempt.number())
        });
        let changed = match self.worktree_of(job).map_err(|why| self.refusal(why))? {
            Some(worktree) => {
                let files = self
                    .work()
                    .changed_files(&worktree)
                    .map_err(|cause| self.unreadable(job.id(), cause))?;
                crate::footprint::seen(&files, declared.map(|plan| &plan.paths))
            }
            None => Vec::new(),
        };
        Ok((
            plans.iter().map(crate::footprint::declared).collect(),
            declared.is_some(),
            changed,
        ))
    }
}
