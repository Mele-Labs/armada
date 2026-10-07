//! The three ways back from a pilot. `docs/concepts/pilot.md`, *Evidence*. #368.
//!
//! **Each is a named act by a person and is recorded as one.** Submitting runs
//! the gates exactly as for a Drone, attesting is stored as attested and never
//! as verified, and superseding is its own terminal state. None of them clears
//! a gate and reads as though it did.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use config::EvidenceType;
use core_model::{
    Actor, Component, DependencyDirection, Envelope, FieldValue, Job, JobId, JobStatus, Level,
    PilotReason, StepId, StepState, StepTarget, Target,
};
use store::{KeptPilot, PilotExit};
use verification::{Claimed, NotClaimed, ShownBy, Submission};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::piloting::Unpilotable;
use crate::rechecking::Leaving;
use crate::regating::came_to;

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
    /// The person is done and says so: the step's gates run on the worktree as
    /// they would for a Drone, and the Job leaves `piloted` for `running` with
    /// no Drone on it.
    ///
    /// **A failing run leaves the Job piloted**, the failure recorded as any
    /// run's is: there is no Drone for it to be handed back to, and the person
    /// is still in the worktree. A Job piloted as Restart Step is handed to a
    /// fresh Drone at the stopped step instead of being gated.
    ///
    /// **Takes Fleet by `Arc` and runs on a task of its own**, for
    /// [`rerun_checks`](Fleet::rerun_checks)' reason.
    pub async fn submit_for_verification(self: Arc<Self>, job_id: &JobId) -> Result<Job, Adrift> {
        let Some(held) = self.rechecking().take(job_id) else {
            return Err(refused(job_id, Unpilotable::Busy));
        };
        let job = self.load(job_id).await?;
        let pilot = self.standing_pilot(&job).await?;
        let worktree = self.surviving_worktree(&job)?;
        let reason = pilot
            .as_ref()
            .map_or(PilotReason::TakeOver, |pilot| pilot.reason);
        let this = Arc::clone(&self);
        let on = job_id.clone();
        let running = tokio::spawn(api::carrying(api::via(), async move {
            let _held = held;
            match reason {
                PilotReason::RestartStep => this.handed_to_a_fresh_drone(&on).await,
                _ => this.verified_by_the_gates(&on, &worktree).await,
            }
        }));
        match running.await {
            Ok(came_to) => came_to,
            Err(_) => Err(Adrift::RecheckAbandoned {
                job: job_id.clone(),
            }),
        }
    }

    /// The person says the work is complete. **Recorded as attested**, with the
    /// words they gave, and never as verified.
    ///
    /// Refused while a step has not advanced: attesting is a verdict on the
    /// work and not a way past the steps, which is what `piloted ->
    /// completed_success`'s guard says.
    pub async fn attest_complete(&self, job_id: &JobId, note: Option<&str>) -> Result<Job, Adrift> {
        let Some(_held) = self.rechecking().take(job_id) else {
            return Err(refused(job_id, Unpilotable::Busy));
        };
        let job = self.load(job_id).await?;
        let pilot = self.standing_pilot(&job).await?;
        if let Some(open) = job
            .steps()
            .iter()
            .find(|row| row.state() != StepState::Advanced)
        {
            return Err(refused(
                job_id,
                Unpilotable::StepsNotAdvanced {
                    step: open.step_id().clone(),
                },
            ));
        }
        let moved = self
            .left_by(
                &job,
                pilot,
                PilotExit::Attested,
                note,
                Target::CompletedSuccess,
            )
            .await?;
        self.noted_exit(&moved, PilotExit::Attested, note);
        Ok(moved)
    }

    /// The work landed outside the Job. **Its own terminal state**, neither
    /// failed nor killed, and a Job waiting on it is released with a warning in
    /// its own log: the upstream never landed as planned.
    pub async fn close_as_superseded(
        &self,
        job_id: &JobId,
        note: Option<&str>,
    ) -> Result<Job, Adrift> {
        let Some(_held) = self.rechecking().take(job_id) else {
            return Err(refused(job_id, Unpilotable::Busy));
        };
        let job = self.load(job_id).await?;
        let pilot = self.standing_pilot(&job).await?;
        let moved = self
            .left_by(&job, pilot, PilotExit::Superseded, note, Target::Superseded)
            .await?;
        self.noted_exit(&moved, PilotExit::Superseded, note);
        self.warned_dependants_of(&moved).await;
        Ok(moved)
    }

    /// The Job is piloted, and its pilot where there is one. **A Job moved to
    /// `piloted` by something that wrote no pilot has none**, and every exit
    /// still works on it.
    async fn standing_pilot(&self, job: &Job) -> Result<Option<KeptPilot>, Adrift> {
        if job.status() != JobStatus::Piloted {
            return Err(refused(
                job.id(),
                Unpilotable::NotPiloted {
                    status: job.status(),
                },
            ));
        }
        Ok(self
            .store()
            .lock()
            .await
            .pilot_of(job.id())
            .map_err(Adrift::Reading)?
            .filter(KeptPilot::standing))
    }

    /// The gates, on the worktree as it stands.
    async fn verified_by_the_gates(
        &self,
        job_id: &JobId,
        worktree: &Worktree,
    ) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        let pilot = self.standing_pilot(&job).await?;
        let (step, entering) = step_to_verify(&job)?;
        let submission = self.submission_of_a_person(&job, &step, worktree).await?;
        // **The baseline the step began from**, so `diff_nonempty` asks whether
        // the work moved and not whether the stopped run's recorded outcome did.
        let began = self
            .store()
            .lock()
            .await
            .step_baseline(job_id, &step)
            .ok()
            .flatten();
        let ruling = self
            .ruled_with_no_drone(&job, &step, worktree, &submission, began.as_ref())
            .await?;
        self.settled_by_the_rerun(job_id, &step, &ruling).await?;
        self.noted_submitted(job_id, &step, &ruling);
        self.noted_undecided(job_id, &step, &ruling);
        if matches!(ruling, Ruling::Failed { .. } | Ruling::HandedBack { .. }) {
            return self.load(job_id).await;
        }
        self.stamped_ended(job_id, PilotExit::Submitted, None).await;
        match self
            .carried_on(&ruling, job_id, &step, Leaving::Pilot(entering), worktree)
            .await
        {
            Ok(carried) => {
                if let Some(pilot) = &pilot {
                    self.handed_back(&carried, pilot).await;
                }
                Ok(carried)
            }
            Err(why) => {
                self.pilot_reopened_for(job_id).await;
                Err(why)
            }
        }
    }

    /// Restart Step: the worktree goes to a fresh Drone at the step that
    /// stopped. **The step is left `stopped`**, as `restart_step` leaves it,
    /// because entering `running` is what counts a run and a run belongs to a
    /// Drone.
    async fn handed_to_a_fresh_drone(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        let pilot = self.standing_pilot(&job).await?;
        self.stamped_ended(job_id, PilotExit::Submitted, None).await;
        let moved = async {
            let running = self.move_job(&job, Target::Running, Actor::Human).await?;
            self.move_job(&running, Target::Queued, Actor::Human).await
        }
        .await;
        match moved {
            Ok(queued) => {
                if let Some(pilot) = &pilot {
                    self.handed_back(&queued, pilot).await;
                }
                Ok(queued)
            }
            Err(why) => {
                self.pilot_reopened_for(job_id).await;
                Err(why)
            }
        }
    }

    /// What the step is submitted on. **The Drone's own evidence where it wrote
    /// any**, and otherwise a claim Fleet words for the person, since a person
    /// at a terminal has no evidence tool.
    ///
    /// A step whose evidence is writing a Drone does (a plan, a review, a note)
    /// has nothing to be submitted on when none was written, and says so.
    async fn submission_of_a_person(
        &self,
        job: &Job,
        step: &StepId,
        worktree: &Worktree,
    ) -> Result<Submission, Adrift> {
        match self.submitted_already(job, step).await {
            Err(Adrift::NothingToRuleOn { .. }) => {}
            answered => return answered,
        }
        let evidence_type = match self.declared_step(job, step)?.evidence_type() {
            None => EvidenceType::Diff,
            Some(
                kind
                @ (EvidenceType::Diff | EvidenceType::FailingTest | EvidenceType::TestSuiteRun),
            ) => kind,
            Some(_) => {
                return Err(refused(
                    job.id(),
                    Unpilotable::NothingToSubmit { step: step.clone() },
                ))
            }
        };
        Submission::submitted(
            evidence_type,
            Claimed("A person took this Job over, did the work themselves and submitted it."),
            ShownBy(&format!("the worktree on {}", worktree.branch())),
            NotClaimed(""),
        )
        .map_err(|_| Adrift::NothingToRuleOn {
            job: job.id().clone(),
            step: step.clone(),
        })
    }

    /// Leave `piloted` for `to`, ending the pilot as `exit`. **The pilot is
    /// stamped ended and the Session gives the worktree back before the Job
    /// moves**, so the summary the move publishes already says how it ended and
    /// a terminal move's own teardown finds the slot as the Job's. A move that
    /// fails puts all of it back.
    async fn left_by(
        &self,
        job: &Job,
        pilot: Option<KeptPilot>,
        exit: PilotExit,
        note: Option<&str>,
        to: Target,
    ) -> Result<Job, Adrift> {
        self.stamped_ended(job.id(), exit, note).await;
        if let Some(pilot) = &pilot {
            self.handed_back(job, pilot).await;
        }
        match self.move_job(job, to, Actor::Human).await {
            Ok(moved) => Ok(moved),
            Err(why) => {
                self.pilot_reopened_for(job.id()).await;
                if let Some(session) = pilot.and_then(|pilot| pilot.session) {
                    self.handed_to(job, &session).await;
                }
                Err(why)
            }
        }
    }

    async fn stamped_ended(&self, job_id: &JobId, exit: PilotExit, note: Option<&str>) {
        let ended = self
            .store()
            .lock()
            .await
            .pilot_ended(job_id, exit, &self.now(), note);
        if let Err(why) = ended {
            self.said_about_the_ledger(
                job_id,
                Level::Warn,
                "the pilot ended and its record could not be written",
                Some(&why.to_string()),
            );
        }
    }

    async fn pilot_reopened_for(&self, job_id: &JobId) {
        if let Err(why) = self.store().lock().await.pilot_reopened(job_id) {
            self.said_about_the_ledger(
                job_id,
                Level::Warn,
                "an exit failed and the pilot could not be put back",
                Some(&why.to_string()),
            );
        }
    }

    /// Each Job still waiting on this one is told in its own log.
    async fn warned_dependants_of(&self, upstream: &Job) {
        let Ok((loaded, _)) = self.every_job().await else {
            return;
        };
        for waiting in loaded.jobs.iter().filter(|other| {
            !other.status().is_terminal()
                && other.dependencies().iter().any(|edge| {
                    edge.direction == DependencyDirection::DependsOn && edge.peer == *upstream.id()
                })
        }) {
            let envelope = Envelope::new(
                self.now(),
                Level::Warn,
                Component::Fleet,
                self.run().clone(),
                "a Job this one depends on was closed as superseded: its work landed outside \
                 Armada, so this dependency is released and not met",
            )
            .in_job(waiting.id().as_ulid().clone())
            .with_field(
                "upstream",
                FieldValue::Str(upstream.id().as_str().to_string()),
            );
            self.noted_in_the_log(waiting.id(), &envelope);
        }
    }

    fn noted_exit(&self, job: &Job, exit: PilotExit, note: Option<&str>) {
        let said = match exit {
            PilotExit::Attested => {
                "a person attested this Job complete: no gate ran, and it is not verified"
            }
            PilotExit::Superseded => "a person closed this Job as superseded by their own work",
            PilotExit::Submitted => "a person submitted this Job for verification",
        };
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone());
        if let Some(note) = note {
            envelope = envelope.with_field("note", FieldValue::Str(note.to_string()));
        }
        self.noted_in_the_log(job.id(), &envelope);
    }

    fn noted_submitted(&self, job: &JobId, step: &StepId, ruling: &Ruling) {
        let envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a person submitted the step for verification and its gates ran on the work as it stands",
        )
        .in_job(job.as_ulid().clone())
        .at_step(step.as_str())
        .with_field("came_to", FieldValue::Str(came_to(ruling).to_string()));
        self.noted_in_the_log(job, &envelope);
    }
}

fn refused(job: &JobId, why: Unpilotable) -> Adrift {
    Adrift::CannotPilot {
        job: job.clone(),
        why,
    }
}

/// The step the gates run on, and the move that opens the run.
///
/// **A stopped step is entered as a re-run is**, which spends no retry. A step
/// at a human gate is gone round again, and a step already running is not moved.
fn step_to_verify(job: &Job) -> Result<(StepId, Option<StepTarget>), Adrift> {
    if let Some((step, trigger)) = job.stopped_on() {
        return Ok((step.clone(), Some(StepTarget::Rechecking(trigger))));
    }
    let row = job
        .current_step()
        .ok_or_else(|| refused(job.id(), Unpilotable::NothingToVerify))?;
    let entering = match row.state() {
        StepState::AwaitingHuman => Some(StepTarget::Revisited),
        StepState::Running => None,
        StepState::Retrying => Some(StepTarget::Running),
        _ => return Err(refused(job.id(), Unpilotable::NothingToVerify)),
    };
    Ok((row.step_id().clone(), entering))
}
