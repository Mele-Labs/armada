//! Taking a Job over: the mark, the pull, and the worktree changing hands.
//! `docs/concepts/pilot.md`, *The mark* and *The piloted session*.
//!
//! **A piloted session is a Session**, so a person names the one they pilot
//! from and the Job's slot and branch go to the ledger as its, handed over.
//! Starting it is the session host's act: it takes [`Fleet::handoff_bundle`].
//!
//! **The pull has one body and two callers.** A take over marks the Job and
//! pulls for itself, since no Drone has an `escape_hatch` tool yet; the tool's
//! handler will be [`Fleet::hatch_pulled`], which refuses an unmarked pull.
//! The build sweep is handed [`Fleet::piloted_checkouts`] and `armada clean`
//! skips a piloted Job and names it.

mod bundle;
mod exits;
mod served;

use std::collections::BTreeMap;
use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level,
    PilotReason, StepId, StepState, Target,
};
use store::{AttachmentState, Holder, KeptAttachment, KeptPilot, Narrative};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Why a Job cannot be taken over, or a pilot ended. **Each is checked before
/// anything moves.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unpilotable {
    /// Another take over, or a run of this Job's Checks, is out.
    Busy,
    /// The status has no edge to `piloted`: queued, proposed, or over.
    NotPilotable { status: JobStatus },
    AlreadyPiloted,
    /// An exit was asked of a Job that is not piloted.
    NotPiloted { status: JobStatus },
    /// Restart Step names a step to hand a fresh Drone, and none stopped.
    NoStepToRestart,
    /// No step is stopped, held at a gate or being worked, so there are no
    /// gates to run.
    NothingToVerify,
    /// The step's evidence is written by its Drone, and none was written.
    NothingToSubmit { step: StepId },
    /// Attesting is a verdict on finished work, not a way past a step.
    StepsNotAdvanced { step: StepId },
}

impl Unpilotable {
    /// The refusal code. **All 409**: the Job is as it was.
    pub fn code(&self) -> &'static str {
        match self {
            Unpilotable::Busy => "fleet.pilot_busy",
            Unpilotable::NotPilotable { .. } => "fleet.not_pilotable",
            Unpilotable::AlreadyPiloted => "fleet.already_piloted",
            Unpilotable::NotPiloted { .. } => "fleet.not_piloted",
            Unpilotable::NoStepToRestart => "fleet.no_step_to_restart",
            Unpilotable::NothingToVerify => "fleet.nothing_to_verify",
            Unpilotable::NothingToSubmit { .. } => "fleet.nothing_to_submit",
            Unpilotable::StepsNotAdvanced { .. } => "fleet.steps_not_advanced",
        }
    }

    pub fn said(&self) -> String {
        match self {
            Unpilotable::Busy => String::from(
                "this Job is being taken over, or its Checks are running. Wait for that to finish",
            ),
            Unpilotable::NotPilotable { status } => format!(
                "this Job is {}, and only a Job that is running, held at a gate, held for repair, \
                 escalated or waiting on an attestation can be taken over",
                status.as_wire()
            ),
            Unpilotable::AlreadyPiloted => String::from("a person has already taken this Job over"),
            Unpilotable::NotPiloted { status } => format!(
                "this Job is {}, and only a piloted Job has a pilot to end",
                status.as_wire()
            ),
            Unpilotable::NoStepToRestart => String::from(
                "no step of this Job stopped, so there is no step to hand a fresh Drone. Take it \
                 over instead",
            ),
            Unpilotable::NothingToVerify => String::from(
                "no step of this Job is stopped, at a gate or being worked, so there are no gates \
                 to run. Attest it complete or close it as superseded",
            ),
            Unpilotable::NothingToSubmit { step } => format!(
                "`{}` asks for written evidence that its Drone writes, and none was written. \
                 Restart the step to have it written, or attest the Job complete",
                step.as_str()
            ),
            Unpilotable::StepsNotAdvanced { step } => format!(
                "`{}` has not advanced. Attesting is a verdict on the work and not a way past a \
                 step: submit it for verification, or close the Job as superseded",
                step.as_str()
            ),
        }
    }
}

/// What a Drone is told when it pulls the hatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hatch {
    /// A person asked for it, and the Job is theirs.
    Pulled,
    /// **Nothing more than this is said**: a refusal that explains itself
    /// teaches a Drone to look for the mark.
    Unavailable,
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
    /// A person takes the Job over: Fleet marks it, ends its Drone where one
    /// is live, and the worktree is theirs. **Refused with nothing changed**
    /// where the Job cannot be piloted.
    ///
    /// `session` is the Session they pilot from, named so the Board can say who
    /// is in the worktree. It is not started here.
    pub async fn take_over(
        &self,
        job_id: &JobId,
        reason: PilotReason,
        session: Option<&str>,
    ) -> Result<Job, Adrift> {
        let refused = |why| Adrift::CannotPilot {
            job: job_id.clone(),
            why,
        };
        // The re-run's own hold, so a take over and a run of these Checks, or
        // two take overs, cannot both be reading the Job at once.
        let Some(_busy) = self.rechecking().take(job_id) else {
            return Err(refused(Unpilotable::Busy));
        };
        let job = self.load(job_id).await?;
        match job.status() {
            JobStatus::Running
            | JobStatus::Escalated
            | JobStatus::AwaitingReview
            | JobStatus::AwaitingRepair
            | JobStatus::AwaitingAttestation => {}
            JobStatus::Piloted => return Err(refused(Unpilotable::AlreadyPiloted)),
            status => return Err(refused(Unpilotable::NotPilotable { status })),
        }
        if reason == PilotReason::RestartStep && !has_a_step_to_restart(&job) {
            return Err(refused(Unpilotable::NoStepToRestart));
        }
        // The person is going to work in it, so it has to be there.
        self.surviving_worktree(&job)?;
        self.store()
            .lock()
            .await
            .mark_for_pilot(job_id, reason, session, self.run().as_str(), &self.now())
            .map_err(Adrift::Writing)?;
        match self.pulled(&job, reason, session, None).await {
            Ok(piloted) => Ok(piloted),
            Err(why) => {
                // A mark that outlives a failed pull is the one thing a later
                // unbidden pull could pass through, so a failure to take it
                // back is said.
                if let Err(also) = self.store().lock().await.drop_pilot(job_id) {
                    self.said_about_the_ledger(
                        job_id,
                        Level::Warn,
                        "a failed take over left its mark, and it could not be taken back",
                        Some(&also.to_string()),
                    );
                }
                Err(why)
            }
        }
    }

    /// A Drone pulled the hatch, with what it said it was stuck on.
    ///
    /// **Allowed only on a Job Fleet marked in this run.** An unmarked pull
    /// escalates the Job as `hatch_unbidden`, where it is still running, and
    /// answers [`Hatch::Unavailable`] and nothing else.
    pub async fn hatch_pulled(
        &self,
        job_id: &JobId,
        narrative: Narrative,
    ) -> Result<Hatch, Adrift> {
        let job = self.load(job_id).await?;
        let marked = self
            .store()
            .lock()
            .await
            .pilot_of(job_id)
            .map_err(Adrift::Reading)?
            .filter(|pilot| {
                pilot.piloted_at.is_none()
                    && pilot.exit.is_none()
                    && pilot.marked_run == self.run().as_str()
            });
        let Some(mark) = marked else {
            if job.status() == JobStatus::Running {
                self.move_job(
                    &job,
                    Target::Escalated(EscalationTrigger::HatchUnbidden),
                    Actor::Drone,
                )
                .await?;
            }
            return Ok(Hatch::Unavailable);
        };
        self.pulled(&job, mark.reason, mark.session.as_deref(), Some(narrative))
            .await?;
        Ok(Hatch::Pulled)
    }

    /// The one body of a pull: the Drone ends, the pilot is stamped, the Job
    /// moves and the worktree is handed over.
    ///
    /// **The pilot is stamped before the Job moves**, so the summary the move
    /// publishes already names who is in the worktree.
    async fn pulled(
        &self,
        job: &Job,
        reason: PilotReason,
        session: Option<&str>,
        narrative: Option<Narrative>,
    ) -> Result<Job, Adrift> {
        let job_id = job.id();
        // `kill_drone`'s order: the process, then the step, then the Job.
        let job = self.drones_ended_for_a_pause(job).await?;
        self.store()
            .lock()
            .await
            .pilot_began(job_id, &self.now(), narrative.as_ref())
            .map_err(Adrift::Writing)?;
        let job = self
            .move_job(&job, Target::Piloted(reason), Actor::Human)
            .await?;
        if let Some(session) = session {
            self.handed_to(&job, session).await;
        }
        self.noted_piloted(&job, reason, session);
        Ok(job)
    }

    /// Every piloted Job's checkout, for the sweeps that walk checkouts without
    /// asking a Job.
    pub async fn piloted_checkouts(&self) -> Vec<PathBuf> {
        let Ok((loaded, _)) = self.every_job().await else {
            return Vec::new();
        };
        loaded
            .jobs
            .iter()
            .filter(|job| job.status() == JobStatus::Piloted)
            .filter_map(|job| self.worktree_of(job).ok().flatten())
            .map(|worktree| PathBuf::from(worktree.path()))
            .collect()
    }

    /// The Job's pilot as its row carries it, once the pilot began.
    pub(crate) fn piloted_on_row(&self, store: &store::Store, job: &Job) -> Option<ipc::Piloted> {
        let kept = store.pilot_of(job.id()).ok().flatten()?;
        let since = kept.piloted_at.as_ref()?;
        Some(ipc::Piloted {
            reason: kept.reason.as_wire().to_string(),
            session_id: kept.session.clone(),
            since: ipc::Instant::from(since),
            exit: kept.exit.map(|exit| exit.as_wire().to_string()),
            ended_at: kept.ended_at.as_ref().map(ipc::Instant::from),
            note: kept.note.clone(),
        })
    }

    /// The Session takes the Job's slot and branch, handed over. **The Job's
    /// own rows are given back in the same breath**, so `who_owns` names one
    /// holder.
    async fn handed_to(&self, job: &Job, session: &str) {
        let now = self.now().as_str().to_string();
        let handed = BTreeMap::from([("handed".to_string(), format!("job {}", job.id().as_str()))]);
        for (kind, target) in held_by(job) {
            let row = KeptAttachment {
                holder: Holder::session(session),
                kind: kind.into(),
                manifest_id: job.owner_manifest_id().as_str().to_string(),
                target: target.clone(),
                state: AttachmentState::Standing,
                detail: handed.clone(),
                since: now.clone(),
                changed_at: now.clone(),
            };
            let wrote = {
                let mut store = self.store().lock().await;
                store.attach(&row, true).and_then(|_| {
                    store.settle(
                        &Holder::job(job.id().as_str()),
                        kind,
                        job.owner_manifest_id().as_str(),
                        &target,
                        AttachmentState::GivenBack,
                        &now,
                    )
                })
            };
            if let Err(why) = wrote {
                self.said_about_the_ledger(
                    job.id(),
                    Level::Warn,
                    "the worktree was handed to a Session and the ledger could not be told",
                    Some(&why.to_string()),
                );
            }
        }
    }

    /// The Session gives the slot and branch back and the Job holds them again.
    pub(crate) async fn handed_back(&self, job: &Job, pilot: &KeptPilot) {
        let Some(session) = pilot.session.as_deref() else {
            return;
        };
        let now = self.now().as_str().to_string();
        for (kind, target) in held_by(job) {
            let settled = self.store().lock().await.settle(
                &Holder::session(session),
                kind,
                job.owner_manifest_id().as_str(),
                &target,
                AttachmentState::GivenBack,
                &now,
            );
            if let Err(why) = settled {
                self.said_about_the_ledger(
                    job.id(),
                    Level::Warn,
                    "the Session gave the worktree back and the ledger could not be told",
                    Some(&why.to_string()),
                );
            }
        }
        if let Some(slot) = job.worktree_slot() {
            self.job_holds_slot(job, slot).await;
        }
        self.job_holds_branch(job).await;
    }

    fn noted_piloted(&self, job: &Job, reason: PilotReason, session: Option<&str>) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a person took this Job over: its Drone ended and the worktree is theirs",
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("reason", FieldValue::Str(reason.as_wire().to_string()));
        if let Some(session) = session {
            envelope = envelope.with_field("session", FieldValue::Str(session.to_string()));
        }
        self.noted_in_the_log(job.id(), &envelope);
    }
}

/// Whether a step stopped, or is running and will be stopped by the pull.
fn has_a_step_to_restart(job: &Job) -> bool {
    job.stopped_on().is_some()
        || job
            .current_step()
            .is_some_and(|row| row.state() == StepState::Running)
}

/// The ledger kinds a Job holds a checkout by, with their targets.
fn held_by(job: &Job) -> Vec<(&'static str, String)> {
    let mut held = Vec::new();
    if let Some(slot) = job.worktree_slot() {
        held.push(("slot", slot.to_string()));
    }
    if let Some(branch) = job.branch() {
        held.push(("branch", branch.as_str().to_string()));
    }
    held
}
