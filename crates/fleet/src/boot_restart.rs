//! A Job this boot interrupted has its step restarted, through the act a
//! person's restart takes, signed as Fleet's.
//!
//! What refuses stays where it was, `escalated` for `interrupted`, and the
//! Job's log says why. `docs/concepts/drone.md`, *A Drone that outlives its
//! Fleet is adopted*, has the rule.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Actor, Component, Envelope, JobId, JobStatus, Level, StepState};
use store::{Moved, RecordedEvent};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::resume::Ending;

/// Automatic restarts in a row, with nothing in between that a Drone or a
/// person did, after which Fleet stops restarting. A Fleet that crashes in a
/// loop restarts a step twice and then leaves it for a person.
const IN_A_ROW: usize = 2;

/// What the log says about one event, as far as the count cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    /// `escalated -> queued`, signed Fleet: an automatic restart.
    Restarted,
    /// A Drone or a person got something done: a step advanced, the Job
    /// reached review, or anybody but Fleet moved the Job.
    Progress,
    Other,
}

pub(crate) fn mark(event: &RecordedEvent) -> Mark {
    match event.moved() {
        Moved::Job { to, .. } if event.actor() == Actor::Fleet => {
            if *to == JobStatus::Queued && event.under() == JobStatus::Escalated {
                Mark::Restarted
            } else if *to == JobStatus::AwaitingReview {
                Mark::Progress
            } else {
                Mark::Other
            }
        }
        Moved::Job { .. } => Mark::Progress,
        Moved::Step { to, .. } if *to == StepState::Advanced => Mark::Progress,
        Moved::Step { .. } | Moved::Drone { .. } => Mark::Other,
    }
}

/// How many automatic restarts the newest events hold before anything got
/// done. Oldest first, as the store answers.
pub(crate) fn restarted_since_progress(marks: &[Mark]) -> usize {
    marks
        .iter()
        .rev()
        .take_while(|mark| **mark != Mark::Progress)
        .filter(|mark| **mark == Mark::Restarted)
        .count()
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
    /// Restart the step of a Job this boot has just interrupted. **`true` if
    /// the Job is back in the queue.** Admission still decides when a Drone
    /// starts, and the Job's own caps still hold: a Job already past either
    /// ceiling stays `escalated`.
    pub(crate) async fn restarted_after_boot(&self, job_id: &JobId) -> Result<bool, Adrift> {
        let job = self.load(job_id).await?;
        let step = job
            .stopped_on()
            .map(|(step, _)| step.as_str().to_string())
            .unwrap_or_default();
        let refused = match self.reasons_not_to_restart(job_id).await {
            Ok(None) => match self
                .restart_step_by(job_id, None, Ending::Unheard, Actor::Fleet)
                .await
            {
                Ok(_) => {
                    self.noted_at_boot(
                        job_id,
                        &step,
                        Level::Info,
                        "Fleet restarted this step after its own restart; the Drone on it was gone",
                    );
                    return Ok(true);
                }
                Err(why) => why.to_string(),
            },
            Ok(Some(reason)) => reason,
            Err(why) => why.to_string(),
        };
        self.noted_at_boot(
            job_id,
            &step,
            Level::Warn,
            &format!("Fleet did not restart this step after its own restart: {refused}"),
        );
        Ok(false)
    }

    async fn reasons_not_to_restart(&self, job_id: &JobId) -> Result<Option<String>, Adrift> {
        let job = self.load(job_id).await?;
        let spent = self.spend_of(job_id).await?;
        if let Some(over) = self.allowance_for(&job).exceeded_by(&spent) {
            return Ok(Some(format!("the {over} cap is spent")));
        }
        let marks: Vec<Mark> = self
            .store()
            .lock()
            .await
            .events_for(job_id)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?
            .iter()
            .map(mark)
            .collect();
        let restarted = restarted_since_progress(&marks);
        Ok((restarted >= IN_A_ROW).then(|| {
            format!("it was restarted this way {restarted} times with nothing done in between")
        }))
    }

    fn noted_at_boot(&self, job: &JobId, step: &str, level: Level, line: &str) {
        let envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            line,
        )
        .in_job(job.as_ulid().clone())
        .at_step(step);
        self.noted_in_the_log(job, &envelope);
    }
}
