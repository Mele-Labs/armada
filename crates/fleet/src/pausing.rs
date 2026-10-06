//! Pausing a Job and resuming it: kill its process, give its worktree slot back
//! to the pool, and later put its branch in whichever slot is free.
//! `docs/concepts/job.md`, *Pausing*.
//!
//! **Not a status.** A running Job goes `queued` and reads `paused`; a Job
//! waiting at a gate keeps the status it reads at and gains the marker. Either
//! way the record, the steps and the review rows are untouched: what a pause
//! takes away is a process and a slot, and what it keeps is the work, parked
//! on the Job's branch as a WIP commit.
//!
//! **A running Job's Drone is ended before the park, a gate Job's is not.** A
//! live Drone is still writing, so parking under it would commit half a write
//! and free a slot it is working in. A gate holds no Drone, so it parks first
//! and a refusal changes nothing. Where the Drone had to go first and the pool
//! then refuses, the Job is left `escalated` on `would_not_start`, as
//! `kill_drone` leaves a Job whose Drone it took away.

use std::sync::Arc;

use adapter_traits::{
    AgentHarness, Delivery, SlotLeased, SlotParkRefused, SlotStanding, Vcs, WorkProduct,
};
use api::Refusal;
use core_model::{
    Actor, Component, Envelope, EscalationTrigger, FieldValue, Job, JobId, JobStatus, Level,
    PausedBy, StepState, Target,
};
use store::ExtraEnded;

use crate::adrift::Adrift;
use crate::budget::budgeted_for;
use crate::daemon::Fleet;
use crate::leasing::pool_of;

/// What putting a parked Job's branch back in a slot came to.
pub(crate) enum Reseat {
    /// The Job holds a slot again and carries no marker.
    Seated(Job),
    /// Every slot is held. Nothing changed.
    Full,
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
    /// Pause a Job: end its Drones, park its work on its branch and give its
    /// slot back. **Refused, nothing the Job's record holds changes** — except
    /// for a running Job whose Drone had to go before the pool could refuse,
    /// which is left `escalated`.
    pub async fn pause_job(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        let refused = |why: fn(JobId) -> Adrift| why(job_id.clone());
        match job.status() {
            JobStatus::Running
            | JobStatus::Queued
            | JobStatus::AwaitingReview
            | JobStatus::AwaitingRepair
            | JobStatus::Escalated => {}
            status => {
                return Err(Adrift::NotPausable {
                    job: job_id.clone(),
                    status,
                })
            }
        }
        // Paused again while it waits for a slot: nothing to park, and the
        // wait is called off.
        if let Some(pause) = job.pause() {
            if !pause.resuming {
                return Err(refused(|job| Adrift::AlreadyPaused { job }));
            }
            let again = job.paused(PausedBy::Person, self.now());
            self.store()
                .lock()
                .await
                .record_pause(&again)
                .map_err(Adrift::Writing)?;
            self.noted_pause(
                &again,
                None,
                "the Job was paused again before it was resumed",
                None,
            );
            self.published_paused(&again).await;
            return Ok(again);
        }
        self.not_while_checks_run_again(&job)?;
        let served = self.served_by(&job)?;
        let pool = pool_of(&served);
        let Some(slot) = job.worktree_slot() else {
            return Err(refused(|job| Adrift::NothingToPark { job }));
        };
        // Asked before a Drone is ended: a slot another holds, or one gone, is
        // known without touching anything.
        let cannot = |refused| Adrift::CannotPark {
            job: job_id.clone(),
            refused,
        };
        match self.vcs().slot_standing(&pool, slot, job_id.as_str()) {
            SlotStanding::Held => {}
            SlotStanding::HeldBy(who) => return Err(cannot(SlotParkRefused::HeldByAnother(who))),
            SlotStanding::Free | SlotStanding::Gone => {
                return Err(cannot(SlotParkRefused::NotLeased))
            }
        }
        let standing = job.status();
        let mut job = job;
        if matches!(standing, JobStatus::Running | JobStatus::Escalated) {
            job = self.drones_ended_for_a_pause(&job).await?;
        }
        let parked = match self.vcs().park_slot(&pool, slot, job_id.as_str()) {
            Ok(parked) => parked,
            Err(why) => {
                if standing == JobStatus::Running {
                    self.move_job(
                        &job,
                        Target::Escalated(EscalationTrigger::WouldNotStart),
                        Actor::Fleet,
                    )
                    .await?;
                }
                return Err(cannot(why));
            }
        };
        let paused = job.paused(PausedBy::Person, self.now());
        self.store()
            .lock()
            .await
            .record_pause(&paused)
            .map_err(Adrift::Writing)?;
        self.noted_pause(
            &paused,
            Some(slot),
            "the Job was paused and its slot given back",
            parked.commit.as_deref(),
        );
        // **The marker first, then the move**: admission reads the marker, and a
        // `queued` Job without one is a Job it would start again at once.
        if standing == JobStatus::Running {
            self.move_job(&paused, Target::Queued, Actor::Human).await?;
        }
        let job = self.load(job_id).await?;
        self.published_paused(&job).await;
        Ok(job)
    }

    /// Resume a paused Job. **A gate Job takes a slot now if there is one**,
    /// and waits for the first to free if not; a queued one is let into the
    /// line, where admission leases it. Neither starts a Drone itself.
    pub async fn resume_job(&self, job_id: &JobId) -> Result<Job, Adrift> {
        let job = self.load(job_id).await?;
        let Some(pause) = job.pause() else {
            return Err(Adrift::NotPaused {
                job: job_id.clone(),
            });
        };
        if job.status().is_terminal() {
            return Err(Adrift::NotPausable {
                job: job_id.clone(),
                status: job.status(),
            });
        }
        if pause.resuming {
            return Ok(job);
        }
        if job.status() != JobStatus::Queued {
            if let Reseat::Seated(seated) = self.reseat(&job).await? {
                self.published_resumed(&seated).await;
                return Ok(seated);
            }
        }
        let waiting = job.resuming();
        self.store()
            .lock()
            .await
            .record_pause(&waiting)
            .map_err(Adrift::Writing)?;
        self.noted_pause(
            &waiting,
            None,
            "the Job was resumed and waits for a slot",
            None,
        );
        self.published_resumed(&waiting).await;
        Ok(waiting)
    }

    /// `Commands::park_job`.
    pub(crate) async fn pause_answered(
        self: Arc<Self>,
        job_id: ipc::JobId,
    ) -> Result<ipc::JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::pause_job(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// `Commands::resume_job`.
    pub(crate) async fn resume_answered(
        self: Arc<Self>,
        job_id: ipc::JobId,
    ) -> Result<ipc::JobSummary, Refusal> {
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move { Fleet::resume_job(&fleet, &job_id.to_domain()).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    /// Lease the Job's own branch at its tip into whichever slot is free,
    /// record it, and lift the marker. **One write for the slot and the
    /// marker**, so a crash cannot leave a Job holding a slot and still paused.
    ///
    /// **Publishes nothing**: admission calls this holding the roster, and a
    /// summary asks the roster. Its callers outside admission publish.
    pub(crate) async fn reseat(&self, job: &Job) -> Result<Reseat, Adrift> {
        let served = self.served_by(job)?;
        let not_seated = |why: String| Adrift::NotReseated {
            job: job.id().clone(),
            why,
        };
        let branch = job
            .branch()
            .ok_or_else(|| not_seated(String::from("it has no branch")))?;
        let leased = self
            .vcs()
            .lease_existing_slot(&pool_of(&served), branch.as_str(), job.id().as_str())
            .map_err(|cause| not_seated(cause.to_string()))?;
        let SlotLeased::Took { slot, .. } = leased else {
            return Ok(Reseat::Full);
        };
        let seated = job.in_slot(slot).unpaused();
        self.store()
            .lock()
            .await
            .record_pause(&seated)
            .map_err(Adrift::Writing)?;
        self.noted_pause(
            &seated,
            Some(slot),
            "the Job was resumed and its work is in a slot again",
            None,
        );
        Ok(Reseat::Seated(seated))
    }

    /// Give a slot to each Job a resume found the pool full for, as one frees.
    /// **After admission**, which fills the queue first.
    pub(crate) async fn seat_resuming(&self) -> Result<Vec<JobId>, Adrift> {
        let (loaded, _) = self.every_job().await?;
        let mut seated = Vec::new();
        for job in loaded.jobs {
            let waiting = job.is_parked()
                && job.pause().is_some_and(|pause| pause.resuming)
                && job.status() != JobStatus::Queued
                && !job.status().is_terminal();
            if !waiting {
                continue;
            }
            match self.reseat(&job).await? {
                Reseat::Seated(job) => {
                    self.published_resumed(&job).await;
                    seated.push(job.id().clone());
                }
                // Every slot is held, and the rest wait behind this one.
                Reseat::Full => break,
            }
        }
        Ok(seated)
    }

    /// End every Drone on the Job, and stop the step it was working: the
    /// kill ladder's first rungs, `kill_drone`'s order.
    async fn drones_ended_for_a_pause(&self, job: &Job) -> Result<Job, Adrift> {
        let job_id = job.id();
        self.end_the_crew(job_id, ExtraEnded::Killed).await;
        let mut ended = false;
        if let Some(slot) = self.slot_of(job_id).await {
            let mut working = slot.lock().await;
            if working.as_ref().is_some_and(|at_work| at_work.is(job_id)) {
                self.end_the_drone(&mut working).await;
                ended = true;
            }
        }
        self.every_exit_recorded(job_id).await?;
        let job = self.load(job_id).await?;
        let running = job
            .current_step()
            .is_some_and(|row| row.state() == StepState::Running);
        match (running, ended) {
            (false, _) => Ok(job),
            (true, true) => self.stopped_by_hand(&job).await,
            (true, false) => self.stopped_abandoned(&job).await,
        }
    }

    /// Tell every client the Job was paused. **A gate Job moves no status**, so
    /// `job.state_changed` never fires for it and this is the only word.
    async fn published_paused(&self, job: &Job) {
        // A summary that will not build is a refusal for the caller's own
        // read to raise; the pause itself is already written.
        if let Ok(job) = self.summarised(job).await {
            self.publish(ipc::Event::JobPaused(ipc::JobPaused {
                job,
                actor: Actor::Human.into(),
                at: (&self.now()).into(),
            }));
        }
    }

    /// The same for a resume, whether it seated the Job or began its wait.
    async fn published_resumed(&self, job: &Job) {
        if let Ok(job) = self.summarised(job).await {
            self.publish(ipc::Event::JobResumed(ipc::JobResumed {
                job,
                actor: Actor::Human.into(),
                at: (&self.now()).into(),
            }));
        }
    }

    fn noted_pause(&self, job: &Job, slot: Option<u32>, said: &str, commit: Option<&str>) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone());
        if let Some(slot) = slot {
            envelope = envelope.with_field("slot", FieldValue::Str(format!("slot-{slot}")));
        }
        if let Some(commit) = commit {
            envelope = envelope.with_field("wip_commit", FieldValue::Str(commit.to_string()));
        }
        self.noted_in_the_log(job.id(), &envelope);
    }
}
