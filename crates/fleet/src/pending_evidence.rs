//! Evidence a restart found still waiting for the gate, ruled on after boot
//! has let commands through, and before anything else decides whether its
//! Drone survived. #796.
//!
//! Ruled through `crate::rechecking::ruled_with_no_drone`, not through
//! `crate::settling::settle`: the Drone that submitted did not survive the
//! restart, so there is no slot for `settle` to ever find.
//!
//! **Not inside `reconcile`**: the gate can take as long as a Job's Checks, and
//! commands wait on reconciliation. `reconcile` only notes which Jobs have a
//! row and keeps them out of the adoption pass, which would read their Drone as
//! gone and lose the evidence. [`Fleet::rule_on_pending_evidence`] rules in a
//! task; each Job is held as a re-run of its Checks is, so no act takes its
//! worktree meanwhile, and the adoption pass runs on it once it is ruled.
//!
//! **`Ruling::Advanced` is queued rather than handed to `act_on`**, which
//! starts the next Drone outright — right for the turn loop, where the Job
//! already held a slot, wrong on a boot that has counted nothing against
//! `concurrency-cap` yet. `crate::rechecking::carried_on` makes the same
//! override.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use core_model::{
    Actor, Component, Envelope, FieldValue, Job, JobId, JobStatus, Level, StepId, StepTarget,
    Target,
};
use tokio::task::JoinHandle;
use verification::{Claimed, NotClaimed, ShownBy, Submission};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::drone_moves::steps_holding_a_drone;
use crate::gate::Ruling;
use crate::reconciled::Reconciled;
use crate::regating::came_to;
use crate::rechecking::Held;

/// One pending row, with its Job held against any act that would take the
/// worktree until it is ruled on.
pub(crate) struct Awaiting {
    row: store::PendingEvidence,
    _held: Held,
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
    /// The `running` Jobs among `jobs` that have a pending row. **Runs no
    /// Check**: called from `reconcile`. A row whose Job already left
    /// `running` has nothing for the gate to say and is cleared here.
    pub(crate) async fn with_pending_evidence(
        &self,
        jobs: &[Job],
    ) -> Result<Vec<JobId>, Adrift> {
        let pending = self
            .store()
            .lock()
            .await
            .pending_evidence()
            .map_err(Adrift::BootRead)?;
        let mut awaiting = Vec::new();
        for row in pending {
            // Not this repository's Job — served, and reconciled, whenever
            // the repository that owns it is added. `crate::mending`'s note.
            let Some(job) = jobs.iter().find(|job| job.id() == &row.job_id) else {
                continue;
            };
            if job.status() == JobStatus::Running {
                awaiting.push(row.job_id);
                continue;
            }
            if let Err(why) = self.store().lock().await.forget_pending_evidence(&row.job_id) {
                self.noted_adrift(&Adrift::Writing(why));
            }
        }
        Ok(awaiting)
    }

    /// Take the rows of `jobs` and hold each Job. **Done before commands are
    /// let through**, so no act slips in between the boot and the ruling.
    pub(crate) async fn held_for_pending_evidence(
        &self,
        jobs: &[JobId],
    ) -> Result<Vec<Awaiting>, Adrift> {
        let pending = self
            .store()
            .lock()
            .await
            .pending_evidence()
            .map_err(Adrift::BootRead)?;
        Ok(pending
            .into_iter()
            .filter(|row| jobs.contains(&row.job_id))
            .filter_map(|row| {
                let _held = self.rechecking().take(&row.job_id)?;
                Some(Awaiting { row, _held })
            })
            .collect())
    }

    /// Rule on the rows of `jobs` in a task of its own and answer once they
    /// are held, so the caller can let commands through. The handle answers
    /// what the ruling and the adoption pass after it came to.
    ///
    /// **A stop in the middle leaves what a crash leaves**: the row is cleared
    /// only after its ruling is applied, and the next boot rules again.
    pub async fn rule_on_pending_evidence(
        self: &Arc<Self>,
        jobs: &[JobId],
    ) -> Result<JoinHandle<Reconciled>, Adrift> {
        let awaiting = self.held_for_pending_evidence(jobs).await?;
        let fleet = Arc::clone(self);
        Ok(tokio::spawn(async move {
            fleet.ruled_on_awaiting(awaiting).await
        }))
    }

    /// Each row ruled on and cleared, then its Job's Drone accounted for, in
    /// the order `crate::mending` states: the adoption pass reads the same rows.
    pub(crate) async fn ruled_on_awaiting(&self, awaiting: Vec<Awaiting>) -> Reconciled {
        let mut done = Reconciled::default();
        for Awaiting { row, _held } in awaiting {
            let job_id = row.job_id.clone();
            match self.ruled_on_one_pending(&row).await {
                Ok(true) => done.recovered_evidence.push(job_id.clone()),
                Ok(false) => {}
                Err(why) => self.noted_adrift(&why),
            }
            if let Err(why) = self.accounted_for_after_ruling(&job_id, &mut done).await {
                self.noted_adrift(&why);
            }
        }
        done
    }

    /// The adoption pass the boot kept this Job out of. A pointer a ruling
    /// leaves looking live is found here.
    async fn accounted_for_after_ruling(
        &self,
        job_id: &JobId,
        done: &mut Reconciled,
    ) -> Result<(), Adrift> {
        let job = self.load(job_id).await?;
        let steps = steps_holding_a_drone(&job);
        if steps.is_empty() && job.status() != JobStatus::Running {
            return Ok(());
        }
        self.accounted_for_drone(job, steps, done).await
    }

    /// One row. `Ok(true)` where the gate ruled on it, `Ok(false)` where the
    /// row named nothing left to rule on.
    async fn ruled_on_one_pending(&self, row: &store::PendingEvidence) -> Result<bool, Adrift> {
        let job_id = row.job_id.clone();
        // Read now and not at boot: the row may have waited behind another.
        let slot = &self.load(&job_id).await?;
        // A Job that left `running` before Fleet stopped, or a row an
        // earlier crash left between the ruling and the clear — either way
        // the gate has nothing left to say about it.
        if slot.status() != JobStatus::Running {
            self.store()
                .lock()
                .await
                .forget_pending_evidence(&job_id)
                .map_err(Adrift::Writing)?;
            return Ok(false);
        }
        let submission = Submission::submitted(
            row.evidence_type,
            Claimed(&row.claimed),
            ShownBy(&row.shown_by),
            NotClaimed(&row.not_claimed),
        )
        // Unreachable in practice — `kept_pending` never writes an empty
        // `claimed` or `shown_by` — kept for `submitted_already`'s reason:
        // a row that will not pass this was written by something that did
        // not share the constructor, which is a corrupt record and not
        // evidence to rule on.
        .map_err(|_| Adrift::NothingToRuleOn {
            job: job_id.clone(),
            step: row.step_id.clone(),
        })?;
        let worktree = self.surviving_worktree(slot)?;
        // The start the step was measured from before Fleet stopped. `None`
        // for a step that began before it was kept, which is read from what
        // its earlier gate recorded, as it was.
        let entered_with = self
            .store()
            .lock()
            .await
            .step_baseline(&job_id, &row.step_id)
            .ok()
            .flatten();
        let ruling = self
            .ruled_with_no_drone(
                slot,
                &row.step_id,
                &worktree,
                &submission,
                entered_with.as_ref(),
            )
            .await?;
        self.noted_recovered(&job_id, &row.step_id, &ruling);
        self.noted_undecided(&job_id, &row.step_id, &ruling);
        self.carried_without_a_drone(&ruling, &job_id, &row.step_id, &worktree)
            .await?;
        self.store()
            .lock()
            .await
            .forget_pending_evidence(&job_id)
            .map_err(Adrift::Writing)?;
        Ok(true)
    }

    /// The ruling, applied with no live Drone to hand `act_on`. **`Advanced`
    /// is queued rather than acted on** — this module's own header. Every
    /// other ruling starts no Drone, and `act_on` already treats an empty
    /// slot as the ordinary shape of a Fleet that restarted —
    /// `crate::boundary::crossed_onto`'s own doc.
    async fn carried_without_a_drone(
        &self,
        ruling: &Ruling,
        job_id: &JobId,
        step: &StepId,
        worktree: &Worktree,
    ) -> Result<Job, Adrift> {
        if let Ruling::Advanced { .. } = ruling {
            let job = self.load(job_id).await?;
            let job = self.move_step(&job, step, StepTarget::Advanced).await?;
            return self.move_job(&job, Target::Queued, Actor::Fleet).await;
        }
        let slot = self.slot_for(job_id).await;
        let mut working = slot.lock().await;
        self.act_on(ruling, job_id, step, &mut working).await?;
        drop(working);
        if matches!(ruling, Ruling::HeldForReview { .. }) {
            let held = self.load(job_id).await?;
            self.compose_review_at_gate(&held, step, worktree).await;
        }
        self.load(job_id).await
    }

    fn noted_recovered(&self, job: &JobId, step: &StepId, ruling: &Ruling) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "evidence this Job's Drone submitted survived a restart; Fleet ruled on it at boot",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("came_to", FieldValue::Str(came_to(ruling).to_string()));
        self.noted_in_the_log(job, &envelope);
    }
}
