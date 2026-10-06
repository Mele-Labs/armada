//! An asked run's record: its own row, written when the run starts and closed
//! when it ends by any route. `docs/concepts/manifest.md`, *A Drone's own run*.
//!
//! **Never a Check row**, so nothing that reads the gate's rows can count a dry
//! result as a pass; `store::asked_runs` is where they live.

use core_model::{DroneId, JobId, StepId, TaskId, Timestamp};
use ipc::mcp::CheckReport;

use super::{Plan, Readings};
use crate::daemon::Fleet;
use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

/// How a run's task came to an end, as the task that ended it knows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ending {
    /// The run came back with an answer, or a fault it could not make one of.
    Ran,
    /// The run's time-box ended it.
    TimedOut,
    /// The task running it panicked or was cancelled.
    Lost,
}

/// A run's row, and the attempt it was asked in.
#[derive(Clone, Copy, Debug)]
pub(super) struct Asked {
    pub(super) row: Option<i64>,
    pub(super) attempt: u32,
}

/// Who asked: a Drone on a task or on a step.
pub(crate) fn requester(
    job: &JobId,
    handle: &str,
    step: &StepId,
    drone: &DroneId,
    task: Option<TaskId>,
) -> ipc::Requester {
    let (job, step, drone) = (
        ipc::JobId::from(job),
        ipc::StepId::from(step),
        ipc::DroneId::from(drone),
    );
    match task {
        Some(task) => ipc::Requester::drone_on_task(&job, &step, &task.to_string(), &drone),
        None => ipc::Requester::drone_on_step(&job, &step, &drone),
    }
    .with_handle(handle)
}

/// One stored row as the wire has it.
pub(crate) fn wired(job: &core_model::Job, run: &store::AskedRun) -> ipc::AskedRun {
    ipc::AskedRun {
        id: run.id,
        requester: requester(job.id(), &job.handle(), &run.step, &run.drone, run.task),
        attempt: run.attempt,
        started_at: ipc::Instant::from(&run.started_at),
        finished_at: run.finished_at.as_ref().map(ipc::Instant::from),
        state: match run.state {
            store::AskedState::Running => ipc::AskedRunState::Running,
            store::AskedState::Passed => ipc::AskedRunState::Passed,
            store::AskedState::Failed => ipc::AskedRunState::Failed,
            store::AskedState::Stopped => ipc::AskedRunState::Stopped,
            store::AskedState::Lost => ipc::AskedRunState::Lost,
        },
        checks: run.checks.clone(),
        narrowed: run.narrowed,
        only_check: run.only_check.clone(),
        logs: run.logs.clone(),
    }
}

/// What a run came to, for its row.
///
/// **A run its time-box or a fault cut short is `stopped`**: it measured
/// nothing to the end, which is not a Check failing. `lost` is only a task that died.
pub(super) fn state_of(
    ran: &Result<(CheckReport, crate::reuse::KeptDryRun), String>,
    ending: Ending,
) -> store::AskedState {
    match (ending, ran) {
        (Ending::Lost, _) => store::AskedState::Lost,
        (Ending::TimedOut, _) | (_, Err(_)) => store::AskedState::Stopped,
        (Ending::Ran, Ok((report, _))) => {
            if report.failed() > 0 {
                store::AskedState::Failed
            } else if report.ran.iter().any(|row| row.stopped.is_some()) {
                store::AskedState::Stopped
            } else {
                store::AskedState::Passed
            }
        }
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
    /// Keep the start of the run. **A row that will not write costs the record
    /// and not the run**: the Drone has asked and is owed its answer.
    pub(super) async fn asked_begins(
        &self,
        plan: &Plan,
        read: &Readings,
        checks: Vec<String>,
        at: &Timestamp,
    ) -> Asked {
        let begun = store::AskedRunBegun {
            drone: plan.drone.clone(),
            task: plan.task,
            step: plan.step.clone(),
            attempt: read.attempt.number(),
            fleet: self.run().clone(),
            at: at.clone(),
            checks,
            narrowed: read.narrow,
            only_check: read.only_check.clone(),
        };
        let row = self
            .store()
            .lock()
            .await
            .begin_asked_run(plan.record.id(), &begun)
            .ok();
        Asked {
            row,
            attempt: begun.attempt,
        }
    }

    /// Close the run's row, whichever way it ended.
    pub(super) async fn asked_ends(
        &self,
        asked: Asked,
        state: store::AskedState,
        logs: &[String],
    ) {
        let Some(row) = asked.row else { return };
        let now = self.now();
        let _ = self
            .store()
            .lock()
            .await
            .end_asked_run(row, state, &now, logs);
    }
}
