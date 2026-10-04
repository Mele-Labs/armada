//! Serving a Job's work for the person its step stopped for.
//!
//! A step with `evidence.walked` asks Fleet to start the repository's `walk`
//! server in the Job's worktree when the step holds for a person, so they walk
//! the work rather than read about it. **The workflow asks and the repository
//! answers**, `captured`'s split: the workflow ships with the app, and which
//! server shows a repository's work — or that none does — is the repository's.
//! A repository naming no `walk` stops exactly as it did before.
//!
//! **It gates nothing.** A server that will not start says so in the Job's log
//! and the step still waits for the person, who can start it from the run
//! sheet like any other.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Component, Envelope, FieldValue, Job, Level, ResolvedStep, StepState};
use ipc::{ServerState, StartedBy};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::servers::{Place, Unservable};
use crate::turning::Turned;

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
    /// Start the Job's `walk` server for `step`, or `None` where the step does
    /// not ask or the repository names none. **What the Job froze decides**,
    /// as for every other server of a Job.
    pub(crate) async fn walked_at_stop(
        self: Arc<Self>,
        job: Job,
        step: &ResolvedStep,
    ) -> Option<Result<ServerState, Unservable>> {
        if !step.walked() {
            return None;
        }
        let served = self.served_by(&job).ok()?;
        let manifest = self.effective_manifest_in(&served, &job).await.0;
        let name = manifest.walk()?.to_string();
        Some(
            self.hold_server_for(Place::Job(job), &name, StartedBy::Person, true)
                .await
                .map(|(state, _)| state),
        )
    }

    /// Every Job this turn held for a person, on a step asking to be walked.
    ///
    /// **Fire-and-forget and only reachable from `keep_turning`**, for
    /// `probed`'s reason: a server outlives the turn, so starting one needs the
    /// `Arc<Fleet>` only that loop holds.
    pub(crate) fn walked(self: &Arc<Self>, turned: &Turned) {
        for worked in &turned.each {
            if !matches!(worked.ruled, Some(Ruling::HeldForReview { .. })) {
                continue;
            }
            let fleet = Arc::clone(self);
            let job_id = worked.job.clone();
            tokio::spawn(async move {
                let Ok(job) = fleet.load(&job_id).await else {
                    return;
                };
                let Some(row) = job.current_step() else {
                    return;
                };
                if row.state() != StepState::AwaitingHuman {
                    return;
                }
                let Some(step) = job.workflow().step(row.step_id()).cloned() else {
                    return;
                };
                if let Some(Err(why)) = Arc::clone(&fleet).walked_at_stop(job, &step).await {
                    fleet.noted_not_walked(&job_id, &step, &why);
                }
            });
        }
    }

    fn noted_not_walked(&self, job: &core_model::JobId, step: &ResolvedStep, why: &Unservable) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "the step's walk server did not start",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.id().as_str())
        .with_field("said", FieldValue::Str(why.to_string()));
        self.noted_in_the_log(job, &envelope);
    }
}
