//! Overruling a verdict: a person reads a refusal, disagrees, and the stopped
//! step advances with the work that was already done left standing.
//!
//! **The fifth act on an escalated Job**, and the only one that keeps the work.
//! `docs/concepts/job.md` has the table and the argument — including which
//! triggers this lifts, which it will not, and why an unappealable verdict is
//! worse than no verdict at all. `crates/ipc/operations/` keys it
//! `override_verdict`.
//!
//! Which triggers this lifts is [`StepLevelTrigger::overrulable`], which sits
//! beside the vocabulary rather than here: the classification in
//! `core_model::Stuck` has to answer the same question, and two exhaustive
//! matches over one set is how a button and the sentence beside it come to
//! disagree.
//!
//! One thing here is not in either document because it is about this code.
//! Whether a Drone is there decides nothing at all: the act
//! applies either way and the Job carries on the same way either way, because
//! the overridden part's Drone is ended and a fresh one takes the next part.
//! That a live session exists is still what separates `crate::resume`'s two
//! acts from each other; it stopped separating anything here.
use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level,
    StepId, StepLevelTrigger, StepTarget, Target,
};
use verification::OutcomeTurn;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Why a person says the verdict is wrong. **Never empty**, except where
/// [`Fleet::override_verdict`] takes none.
///
/// A type of its own rather than `resume::Redirection`, which is structurally
/// the same string: that one is delivered to a Drone as a turn and this one
/// never leaves the record. The Drone did nothing wrong and is told only that
/// the step was accepted.
///
/// It is required on a refusal because a person disagreeing with a
/// Judge is the strongest signal there is that a criterion is mis-stated, and a
/// count of overrides with no reasons beside it gives the rate and never the
/// cause. An override that says nothing is also how the act this module keeps
/// visible becomes the one somebody reaches for to quiet a gate. **A gaming
/// flag, or a gate that could not decide, is overruled with or without one**:
/// see [`Fleet::override_verdict`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overruling(String);

impl Overruling {
    /// `None` where there is nothing in it a person could later read.
    pub fn saying(reason: &str) -> Option<Overruling> {
        let said = reason.trim();
        (!said.is_empty()).then(|| Overruling(said.to_string()))
    }

    pub fn text(&self) -> &str {
        &self.0
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
    /// Overrule the Judge and let the Job go on from where it stopped.
    ///
    /// The actor is **human** on both moves. Fleet overrules nothing of its own
    /// accord — disagreeing with a verdict is exactly the part a person was
    /// escalated to.
    ///
    /// **The step moves beneath `escalated`, and it is the only move that
    /// may** — `core_model::overruled_while_frozen` is that exception and says
    /// why. It cannot be deferred: it *is* the act. That separates it from
    /// `restart_step`, which defers its step move entirely, because entering
    /// `running` opens a run and a run belongs to a Drone.
    ///
    /// **A workflow with a step left goes back in the queue**, at
    /// `escalated -> queued`, and `crate::readmitting` spawns when
    /// `concurrency-cap` has room. Until #50's follow-on this act opened a slot
    /// of its own and consulted no bound. **An override of the last step
    /// finishes the Job here instead**, because no Drone is owed and there is
    /// nothing for the bound to bound.
    ///
    /// **Neither is refused because the cap is spent.** The act lands, and a
    /// Job that waits says `waiting_on_resources`.
    pub async fn override_verdict(
        &self,
        job_id: &JobId,
        overruling: Option<&Overruling>,
    ) -> Result<Job, Adrift> {
        // Opened rather than looked up: the Job whose Drone has gone has no
        // slot in the roster, and `completed` may still have work to land
        // through one. It holds nothing on the path that re-queues, and
        // `Slots::sweep` forgets it the moment this call lets it go.
        let slot = self.slot_for(job_id).await;
        let mut working = slot.lock().await;
        let job = self.load(job_id).await?;
        self.not_while_checks_run_again(&job)?;
        let (step, overruled) = self.overridable(&job).await?;
        // **Blank is refused on a refusal and taken on a gaming flag or an
        // undecided gate.** A flag's record already names the pattern, the
        // question and the brief a person disagreed with; an undecided gate
        // has no Judge opinion to learn from. A refusal overruled in silence
        // leaves nothing saying why the Judge was wrong.
        if overruling.is_none()
            && !matches!(
                overruled.trigger(),
                EscalationTrigger::EvidenceSuspect | EscalationTrigger::GateUndecided
            )
        {
            return Err(Adrift::Unreasoned {
                job: job_id.clone(),
            });
        }
        // Read before anything moves, and discarded. A worktree that is gone
        // is a Job whose earlier steps' work is not on disk, and what is being
        // asked for there is a redispatch — the same refusal `restart_step`
        // makes, made before the Job has been half-moved rather than after.
        // `crate::readmitting` reaches the same directory again at the spawn.
        self.surviving_worktree(&job)?;
        let passed = self.declared_step(&job, &step)?.clone();
        let next = job.workflow().after(&step).cloned();
        // **Before the move below, for `crate::dispatch::act_on`'s reason.** The
        // Drone this act stands down is still in the slot when the step advance
        // is published, and a redirect since the refusal is spend nothing has
        // written down yet. `Fleet::paid_so_far`.
        self.paid_so_far(&working).await?;

        // **First, and beneath `escalated`.** See the note above: this move is
        // the act, and the Job's own move is what follows from it rather than
        // the other way round.
        let job = self
            .move_step_by(&job, &step, StepTarget::Overridden(overruled), Actor::Human)
            .await?;
        // After the move and before anything else can fail, so the reason a
        // person gave is on the record whatever happens next.
        self.noted_override(job_id, &step, overruled, overruling);

        if next.is_none() {
            // Nothing is owed a Drone, so nothing waits on the bound. The Job
            // takes the edge it has always taken from here, and `completed`
            // lands the work through the slot above and ends the Drone.
            let told = OutcomeTurn::approved(&passed, None, &self.prompts().wording());
            let job = self.move_job(&job, Target::Running, Actor::Human).await?;
            let done = self
                .completed(&job, &told, job_id, &mut working, Actor::Human)
                .await?;
            // **The freed place is the next turn's to fill** — `#428`. It is
            // one turn later than it was, and the Job that takes it is not this
            // one: admitting here meant a client that stopped waiting for an
            // override killed a dispatch on a Job nobody was watching.
            return Ok(done);
        }
        // **The overridden part's Drone ends here, before the Job waits.** The
        // part is settled and there is no more work in it, so a process left
        // standing would spend a place against `concurrency-cap` on a Job that
        // is only queued — which is the overrun this change exists to close,
        // arriving from the other side. `crate::boundary` used to do it, on the
        // way to a spawn this act no longer makes; `stood_down` is that same
        // ordering, called directly.
        self.stood_down(job_id, &mut working).await?;
        // The actor is **human**, for `crate::reviewing`'s reason: a person
        // took the Job out of `escalated`, and Fleet only decides which turn it
        // gets a process back. What the next Drone is told — that a person
        // settled the part before — is `crate::readmitting`'s to assemble, off
        // the `advanced` step this call leaves behind.
        // **It answers `queued` and no longer `running`**, which is `#428`:
        // exactly as an approval no longer dispatches for itself. The next
        // Drone is `crate::readmitting`'s, on the first turn with room.
        self.move_job(&job, Target::Queued, Actor::Human).await
    }

    /// The step a person may overrule, and the verdict they are overruling.
    ///
    /// Three things have to hold, and each refusal names a different act as the
    /// one that applies. The Job is `escalated` or `awaiting_repair`; a step of
    /// it stopped; and the trigger that stopped it is one
    /// [`StepLevelTrigger::overrulable`] admits, which includes a machine having
    /// been unable to rule.
    ///
    /// **A failed Check is overruled the same way a Judge's refusal is.** A Job
    /// held at `awaiting_repair` carries a step stopped on `gate_failure`, the
    /// trigger a Judge refusal writes too, and the owner ruled on 2026-10-06
    /// that any failed Check, `build` and `test` included, may be moved on by a
    /// person who knows the work is done. Nothing reads the Check runs: what
    /// failed stays on the record as `failed(gate_failure)` beside `advanced`,
    /// and the Check runs themselves are not rewritten.
    async fn overridable(&self, job: &Job) -> Result<(StepId, StepLevelTrigger), Adrift> {
        if !matches!(
            job.status(),
            JobStatus::Escalated | JobStatus::AwaitingRepair
        ) {
            return Err(Adrift::NotResumable {
                job: job.id().clone(),
                status: job.status(),
            });
        }
        // `Job::stopped_on` is the one reading, shared with `crate::regating`
        // and with the classification: the state and the `failed(<trigger>)`
        // verdict together, because a stopped row with no verdict is a row
        // nothing could ever say why about.
        let (step, overruled) = job.stopped_on().ok_or_else(|| Adrift::NoStepStopped {
            job: job.id().clone(),
        })?;
        let step = step.clone();
        if !overruled.overrulable() {
            return Err(Adrift::NotTheJudges {
                job: job.id().clone(),
                step,
                trigger: overruled.trigger(),
            });
        }
        Ok((step, overruled))
    }

    /// Write the override into the Job's own log.
    ///
    /// **Fields, never an interpolated message**, for the reason
    /// `crate::settling` gives about a decline: `overruled` is what a query
    /// groups on and it is a trigger spelling from the registry, so overrides
    /// stay countable against the refusals they answer. The person's words are
    /// carried whole beside it, because the count says the rate and only the
    /// sentence says the cause.
    ///
    /// A log line that will not write does not stop the act. The step has
    /// already moved and the move is the record; this is the half no column
    /// holds.
    fn noted_override(
        &self,
        job: &JobId,
        step: &StepId,
        overruled: StepLevelTrigger,
        overruling: Option<&Overruling>,
    ) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "a person overruled the gate and the step advanced",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field(
            "overruled",
            FieldValue::Str(overruled.as_wire().to_string()),
        );
        // Absent rather than blank where a flag was overruled without a word, so
        // a count of `said` is a count of reasons.
        let envelope = match overruling {
            Some(said) => envelope.with_field("said", FieldValue::Str(said.text().to_string())),
            None => envelope,
        };
        self.noted_in_the_log(job, &envelope);
    }
}
