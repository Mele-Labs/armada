//! A Job's Drones beside the one it keeps: spike 022, slice 5, answer 2.
//! `docs/concepts/plan.md`, *Tasks that may run at once*, has the whole of it.
//!
//! The kept Drone holds the Job's slot from admission to its end and is never
//! asked for again. A Drone beside it asks the machine's cap (which counts
//! Drones), the Job's own cap, memory and its repository's disk, gives way to
//! a Job waiting to start, and waits a turn wherever it is refused.

use std::future::Future;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, Worktree};
use core_model::{
    Component, DroneId, Envelope, FieldValue, Job, JobId, JobStatus, Level, PlanChange, PlanTask,
    StepId, StepState, TaskState, TaskUpdate, WorkPlan,
};
use store::ExtraEnded;

use crate::adrift::Adrift;
use crate::briefing::Opening;
use crate::crossing::{Crossed, Produced, ThePlan};
use crate::daemon::Fleet;
use crate::process::{holder_of, Holder};
use crate::tasking::next_task_beside;
use crate::work_plan::plan_not_kept;
use crate::working::Working;

/// What a Drone beside a Job's kept one is asked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Asking {
    /// Every Drone Fleet is working, kept and beside.
    pub drones: usize,
    /// `settings.concurrency-cap`.
    pub machine_cap: usize,
    /// This Job's, its kept Drone included.
    pub job_drones: usize,
    /// The Job's own cap from its approval; `None` is the machine's.
    pub job_cap: Option<u32>,
    /// Whether an approved Job is waiting for room to start.
    pub a_job_waits: bool,
}

/// Whether a Drone may start beside a Job's kept one, and if not, why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtraRoom {
    Yes,
    MachineFull,
    JobFull,
    /// Answer 2: a Job's extra Drone gives way to a Job waiting to start.
    YieldsToAWaitingJob,
}

/// The cap half of the question; memory and disk are read off the machine.
pub fn extra_room(asking: &Asking) -> ExtraRoom {
    if asking.drones >= asking.machine_cap {
        ExtraRoom::MachineFull
    } else if asking
        .job_cap
        .is_some_and(|cap| asking.job_drones >= cap as usize)
    {
        ExtraRoom::JobFull
    } else if asking.a_job_waits {
        ExtraRoom::YieldsToAWaitingJob
    } else {
        ExtraRoom::Yes
    }
}

tokio::task_local! {
    /// The Drone beside a kept one that made the tool call being answered.
    static CALLING: DroneId;
}

/// The Drone beside a kept one whose tool call this is, where it is one.
pub(crate) fn calling() -> Option<DroneId> {
    CALLING.try_with(Clone::clone).ok()
}

/// Answer `work` as `drone`'s call, so `Fleet::slot_of` takes its own slot.
/// `None` is the Job's kept Drone, which needs nothing named.
pub(crate) async fn as_caller<F: Future>(drone: Option<DroneId>, work: F) -> F::Output {
    match drone {
        Some(drone) => CALLING.scope(drone, work).await,
        None => work.await,
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
    /// A Drone beside each held Job's kept one, where a task may run at once
    /// and there is room. **After admission**, so a waiting Job is asked first.
    pub(crate) async fn crew_next(&self) -> Result<Vec<DroneId>, Adrift> {
        let held = self.slots().lock().await.working_on();
        let mut started = Vec::new();
        let mut waits = None;
        for job_id in held {
            let Some((job, step, plan, task)) = self.beside_owed(&job_id).await? else {
                continue;
            };
            let a_job_waits = match waits {
                Some(known) => known,
                None => *waits.insert(self.a_job_waits().await?),
            };
            if !self.room_beside(&job, a_job_waits).await {
                continue;
            }
            if let Some(drone) = self.put_one_beside(&job, &step, &plan, &task).await? {
                started.push(drone);
            }
        }
        Ok(started)
    }

    /// The task a Drone beside the kept one would take: the kept Drone on a
    /// task, the group short of its gate, and a task safe beside every one.
    async fn beside_owed(
        &self,
        job_id: &JobId,
    ) -> Result<Option<(Job, StepId, WorkPlan, PlanTask)>, Adrift> {
        let job = self.load(job_id).await?;
        if job.status() != JobStatus::Running || self.evidence_waiting_for(job_id) > 0 {
            return Ok(None);
        }
        let Some(step) = job
            .current_step()
            .filter(|row| row.state() == StepState::Running)
            .map(|row| row.step_id().clone())
        else {
            return Ok(None);
        };
        let live = self.live_tasks(job_id);
        if live.is_empty() {
            return Ok(None);
        }
        let Some((plan, _)) = self.task_to_work(&job, &step).await? else {
            return Ok(None);
        };
        let runs = self.group_runs_of(job_id).await?;
        let Some(task) = next_task_beside(&plan, &runs, &live).cloned() else {
            return Ok(None);
        };
        if self.left_for_the_kept(job_id, task.id()) {
            return Ok(None);
        }
        Ok(Some((job, step, plan, task)))
    }

    /// Whether an approved Job is waiting for room: the one admission would
    /// start next, short of nothing of its own.
    async fn a_job_waits(&self) -> Result<bool, Adrift> {
        let mut skipped = Vec::new();
        while let Some(job) = self.next_queued(&skipped).await? {
            if !self.volume_is_short(&job).await && !self.slot_is_short(&job) {
                return Ok(true);
            }
            skipped.push(job.id().clone());
        }
        Ok(false)
    }

    /// Answer 2's caps, then memory and the repository's own disk.
    async fn room_beside(&self, job: &Job, a_job_waits: bool) -> bool {
        let job_cap = self.store().lock().await.drone_cap(job.id()).ok().flatten();
        let asking = {
            let mut slots = self.slots().lock().await;
            Asking {
                drones: slots.count(),
                machine_cap: slots.cap(),
                job_drones: slots.of_job(job.id()),
                job_cap,
                a_job_waits,
            }
        };
        if extra_room(&asking) != ExtraRoom::Yes {
            return false;
        }
        let short_of_memory = self
            .machine_reading()
            .await
            .is_some_and(|reading| reading.memory().spare() < self.headroom().memory_spare());
        !short_of_memory && !self.volume_is_short(job).await
    }

    /// Start one Drone beside the kept one, on `task`, in the Job's one copy.
    /// **A spawn that does not start stops nothing**: the task waits for the
    /// kept Drone, and the Job's log says why.
    async fn put_one_beside(
        &self,
        job: &Job,
        step: &StepId,
        plan: &WorkPlan,
        task: &PlanTask,
    ) -> Result<Option<DroneId>, Adrift> {
        let worktree = self.surviving_worktree(job)?;
        let drone = DroneId::carried(self.mint().ulid());
        let slot = self.slots().lock().await.joined(job.id(), &drone);
        let mut working = slot.lock().await;
        match self
            .started_beside(job, step, worktree, plan, task, &drone)
            .await
        {
            Ok(at_work) => {
                *working = Some(at_work);
                Ok(Some(drone))
            }
            Err(why) => {
                drop(working);
                self.slots().lock().await.left(job.id(), &drone);
                self.left_for_kept(job.id(), task.id());
                self.noted_beside(job.id(), &drone, "did not start", &why.to_string());
                Ok(None)
            }
        }
    }

    async fn started_beside(
        &self,
        job: &Job,
        step: &StepId,
        worktree: Worktree,
        plan: &WorkPlan,
        task: &PlanTask,
        drone: &DroneId,
    ) -> Result<Working, Adrift> {
        let job_id = job.id().clone();
        let recorded = self
            .store()
            .lock()
            .await
            .step_evidence(&job_id)
            .map_err(Adrift::Reading)?;
        let crossed =
            Crossed::nothing().and_produced(Produced::before(job.workflow(), step, &recorded));
        // No catch-up onto the base: the kept Drone is working in this copy.
        let opening = Opening::fresh()
            .worded(self.prompts())
            .carrying(crossed)
            .holding_off(&self.held_off(&job_id).await)
            .carrying_the_plan(Some(ThePlan::for_task(plan, task)))
            .ruling_out(self.dismissed_for(job, step).await?);
        let brief = opening
            .turn(job, job.workflow(), step, None)
            .map_err(|cause| Adrift::NotConfigurable {
                job: job_id.clone(),
                cause,
            })?;
        let (opened_with, headings, kinds) = (
            brief.as_str().to_string(),
            brief.headings().to_vec(),
            brief.kinds().to_vec(),
        );
        let config = self
            .spawn_config(job, step, &worktree, brief.prompt(), Some(task))
            .await
            .map_err(|cause| Adrift::NotConfigurable {
                job: job_id.clone(),
                cause,
            })?;
        let recording = self
            .recording(job, drone, step)
            .map_err(|cause| Adrift::NoTranscript {
                job: job_id.clone(),
                cause,
            })?;
        let started = crate::drone::start(self.harness().as_ref(), &config)
            .await
            .map_err(|cause| Adrift::NoDrone {
                job: job_id.clone(),
                cause: Box::new(cause),
            })?;
        let pid = started.session.pid();
        let process = match holder_of(pid) {
            Ok(Holder::Held(at)) => Some((pid, at.as_str().to_string())),
            _ => None,
        };
        self.drone_joined(&job_id, drone, pid);
        self.put_on_task(job, step, drone, task.id()).await?;
        {
            let mut store = self.store().lock().await;
            store
                .record_extra_drone(&job_id, drone, process)
                .map_err(Adrift::Writing)?;
            if let Ok(model) = core_model::ModelName::new(config.model().as_str()) {
                store
                    .record_drone_model(&job_id, drone, &model)
                    .map_err(Adrift::Writing)?;
            }
        }
        self.published_beside(job, step, drone, true).await?;
        let mut at_work = Working::holding(
            job_id,
            drone.clone(),
            step.clone(),
            worktree,
            started,
            Arc::clone(self.harness()),
            recording,
            self.liveness()
                .at(self.served_by(job)?.manifest(), job, step),
            self.now(),
        );
        at_work.briefed(&opened_with, headings, kinds);
        at_work.on_task(task.id());
        Ok(at_work)
    }

    /// One Job's Drones beside its kept one, this turn: each that handed in
    /// and came to rest, or left, is ended, and every one is ended once the
    /// Job is not running.
    pub(crate) async fn crew_turn(&self, job_id: &JobId) -> Result<(), Adrift> {
        let crew = self.slots().lock().await.crew_of(job_id);
        if crew.is_empty() {
            return Ok(());
        }
        let running = self.load(job_id).await?.status() == JobStatus::Running;
        for (drone, slot) in crew {
            let mut working = slot.lock().await;
            let Some(at_work) = working.as_ref() else {
                continue;
            };
            let grace = self.norms().report_grace();
            let how = if at_work.has_handed_in() {
                if running && !at_work.settled(&self.now(), grace) {
                    continue;
                }
                ExtraEnded::Done
            } else if !running {
                ExtraEnded::Failed
            } else if at_work.transcript_ended() && at_work.exited().await.unwrap_or(true) {
                ExtraEnded::Failed
            } else {
                continue;
            };
            self.end_one_beside(job_id, &drone, &mut working, how)
                .await?;
        }
        Ok(())
    }

    /// End every Drone beside this Job's kept one: the step stopped, or a
    /// person ended the Job or its kept Drone.
    pub(crate) async fn end_the_crew(&self, job_id: &JobId, how: ExtraEnded) {
        let crew = self.slots().lock().await.crew_of(job_id);
        for (drone, slot) in crew {
            let mut working = slot.lock().await;
            if let Err(why) = self.end_one_beside(job_id, &drone, &mut working, how).await {
                self.noted_adrift(&why);
            }
        }
    }

    /// End one Drone beside the kept one. **A task it did not hand in goes
    /// back to `open` and waits for the kept Drone**, so a Drone that keeps
    /// dying, or one a person stopped, is not started again beside it.
    pub(crate) async fn end_one_beside(
        &self,
        job_id: &JobId,
        drone: &DroneId,
        working: &mut Option<Working>,
        how: ExtraEnded,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.take() else {
            return Ok(());
        };
        let task = at_work.task();
        let step = at_work.standing().1;
        self.drone_parted(job_id, drone);
        self.stood_down_paying(at_work).await?;
        let at = self.now();
        self.store()
            .lock()
            .await
            .record_extra_left(job_id, drone, how, &at)
            .map_err(Adrift::Writing)?;
        if let (Some(task), false) = (task, how == ExtraEnded::Done) {
            self.left_for_kept(job_id, task);
            self.reopened(job_id, &step, task).await?;
        }
        let job = self.load(job_id).await?;
        self.published_beside(&job, &step, drone, false).await
    }

    /// At boot: a Drone beside a kept one an earlier Fleet left running is
    /// ended where `holder_of` finds it at its recorded start, and its task
    /// goes back to `open`. **Never adopted**: its task waits for the kept one.
    pub(crate) async fn crew_left_behind(&self, jobs: &[Job]) {
        for job in jobs.iter().filter(|job| !job.status().is_terminal()) {
            let Ok(bound) = self.store().lock().await.task_drones(job.id()) else {
                continue;
            };
            for left in bound.iter().filter(|b| b.extra && b.left.is_none()) {
                if let Some((pid, started)) = &left.process {
                    let same =
                        matches!(holder_of(*pid), Ok(Holder::Held(at)) if at.as_str() == started);
                    if let (true, Some(group)) = (same, std::num::NonZeroU32::new(*pid)) {
                        crate::group::end_the_group(group);
                    }
                }
                let at = self.now();
                let kept = self.store().lock().await.record_extra_left(
                    job.id(),
                    &left.drone_id,
                    ExtraEnded::Failed,
                    &at,
                );
                if let Err(why) = kept.map_err(Adrift::Writing) {
                    self.noted_adrift(&why);
                }
                if let Err(why) = self.reopened(job.id(), &left.step_id, left.task).await {
                    self.noted_adrift(&why);
                }
            }
        }
    }

    /// A task back to `open`, where its Drone did not hand it in.
    async fn reopened(
        &self,
        job_id: &JobId,
        step: &StepId,
        task: core_model::TaskId,
    ) -> Result<(), Adrift> {
        let Some(plan) = self.plan_of(job_id).await? else {
            return Ok(());
        };
        if plan.task(task).map(|t| t.state()) != Some(TaskState::Working) {
            return Ok(());
        }
        let at = self.now();
        let change = PlanChange::Updated {
            task,
            to: TaskUpdate::Open,
            shown: None,
        };
        let plan = self
            .store()
            .lock()
            .await
            .change_plan(job_id, &change, store::PlanHand::Step(step), &at)
            .map_err(|why| plan_not_kept(job_id, why))?;
        self.publish(ipc::Event::JobPlanChanged(ipc::JobPlanChanged::task_moved(
            job_id,
            &plan,
            task,
            core_model::Actor::Fleet,
            &at,
        )));
        Ok(())
    }

    /// `drone.spawned` or `drone.exited` for a Drone beside the kept one, which
    /// is on no record of moves (one Drone a step): `store::crew` keeps it.
    async fn published_beside(
        &self,
        job: &Job,
        step: &StepId,
        drone: &DroneId,
        arrived: bool,
    ) -> Result<(), Adrift> {
        let summary = self.published(job).await?;
        let at = (&self.now()).into();
        self.publish(match arrived {
            true => ipc::Event::DroneSpawned(ipc::DroneSpawned {
                job: summary,
                step_id: step.into(),
                drone_id: drone.into(),
                branch: job.branch().map(|branch| branch.as_str().to_string()),
                actor: core_model::Actor::Fleet.into(),
                at,
            }),
            false => ipc::Event::DroneExited(ipc::DroneExited {
                job: summary,
                step_id: step.into(),
                drone_id: drone.into(),
                actor: core_model::Actor::Fleet.into(),
                at,
            }),
        });
        Ok(())
    }

    fn noted_beside(&self, job: &JobId, drone: &DroneId, what: &str, because: &str) {
        let envelope = Envelope::new(
            self.now(),
            Level::Warn,
            Component::Fleet,
            self.run().clone(),
            "a Drone beside the kept one stopped short; its task waits for the kept Drone",
        )
        .in_job(job.as_ulid().clone())
        .with_field("drone_id", FieldValue::Str(drone.as_str().to_string()))
        .with_field("what", FieldValue::Str(what.to_string()))
        .with_field("because", FieldValue::Str(because.to_string()));
        self.noted_in_the_log(job, &envelope);
    }
}
