//! Every Drone a Job has had, read off its history — `list_job_drones`.
//!
//! **The record, not the roster.** `crate::rostered` loses a Drone the moment it
//! exits; the history keeps `drone_spawned` and `drone_exited` for every one and
//! the spend rows outlive them, so this reads both and holds no slot. Nothing
//! records a Drone's state, so it is read off the moves around it — the rule is
//! `list_job_drones`'s note in `crates/ipc/operations/`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{
    DroneId, DronePresence, EscalationTrigger, JobStatus, StepId, StepState, Timestamp,
};
use ipc::{DroneState, JobDrone, JobDrones, Saw, TranscriptRow};
use store::{Moved, RecordedEvent};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::transcript::transcript_of;

/// One Drone off the history, before anything is known about what it spent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Had {
    pub drone: DroneId,
    pub step: StepId,
    pub state: DroneState,
    pub spawned_at: Timestamp,
    pub left_at: Option<Timestamp>,
}

impl Had {
    /// A Drone put on a task. **One that handed it in and left is done**, not
    /// failed: the next task's Drone ends its tenure on the step, and leaving
    /// is what a hand-in asks of it. `crate::tasking`.
    pub(crate) fn on_task(self, bound: &store::TaskDrone) -> Had {
        let state = match (self.state, &bound.handed_in) {
            (DroneState::Failed, Some(_)) => DroneState::Done,
            (state, _) => state,
        };
        Had { state, ..self }
    }
}

/// Every Drone the history names, in the order they were spawned. `events` is
/// the Job's whole history in `seq` order, as `Store::events_for` answers.
pub(crate) fn drones_had(events: &[RecordedEvent]) -> Vec<Had> {
    let spawns = events
        .iter()
        .enumerate()
        .filter_map(|(at, event)| match event.moved() {
            Moved::Drone {
                step_id,
                drone_id,
                presence: DronePresence::Spawned,
            } => Some((at, step_id, drone_id)),
            _ => None,
        });
    spawns
        .map(|(spawned, step, drone)| {
            let exited = events
                .iter()
                .enumerate()
                .skip(spawned + 1)
                .find(|(_, event)| {
                    matches!(
                        event.moved(),
                        Moved::Drone { drone_id, presence: DronePresence::Exited, .. }
                            if drone_id == drone
                    )
                });
            Had {
                drone: drone.clone(),
                step: step.clone(),
                state: match exited {
                    None => DroneState::Running,
                    Some((exit, _)) => ended_as(events, spawned, exit, step),
                },
                spawned_at: events[spawned].at().clone(),
                left_at: exited.map(|(_, event)| event.at().clone()),
            }
        })
        .collect()
}

/// A Drone beside a kept one, off its task binding and how it left.
fn beside(bound: &store::TaskDrone) -> Had {
    let state = match &bound.left {
        None => DroneState::Running,
        Some((_, store::ExtraEnded::Done)) => DroneState::Done,
        Some((_, store::ExtraEnded::Killed)) => DroneState::Killed,
        Some((_, store::ExtraEnded::Failed)) => DroneState::Failed,
    };
    Had {
        drone: bound.drone_id.clone(),
        step: bound.step_id.clone(),
        state,
        spawned_at: bound.spawned_at.clone(),
        left_at: bound.left.as_ref().map(|(at, _)| at.clone()),
    }
}

/// How a Drone that has left ended.
///
/// **`done` is asked first**: a Job killed after its last Drone reached a
/// person's gate killed no Drone. `killed` is the next move after the exit
/// being the one `kill_drone` or `kill_job` writes. A kill while evidence is
/// waiting stops no step, and the exit row's actor is always Fleet, so that
/// Drone reads as whatever the gate makes of its step.
fn ended_as(events: &[RecordedEvent], spawned: usize, exit: usize, step: &StepId) -> DroneState {
    // Its tenure ends at the next Drone put on the same step: a restart or a
    // send-back, and that Drone's to answer for.
    let finished = events
        .iter()
        .skip(spawned + 1)
        .take_while(|event| {
            !matches!(
                event.moved(),
                Moved::Drone { step_id, presence: DronePresence::Spawned, .. } if step_id == step
            )
        })
        .any(|event| {
            matches!(
                event.moved(),
                Moved::Step { step_id, to: StepState::Advanced | StepState::AwaitingHuman, .. }
                    if step_id == step
            )
        });
    if finished {
        return DroneState::Done;
    }
    let next = events
        .iter()
        .skip(exit + 1)
        .find(|event| !matches!(event.moved(), Moved::Drone { .. }));
    let killed = next.is_some_and(|event| match event.moved() {
        Moved::Step {
            step_id,
            why: Some(why),
            ..
        } => step_id == step && why.trigger() == EscalationTrigger::DroneKilled,
        Moved::Job { to, .. } => *to == JobStatus::Killed,
        _ => false,
    });
    match killed {
        true => DroneState::Killed,
        false => DroneState::Failed,
    }
}

/// What one Drone's transcript says it has done so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SoFar {
    /// Turns summed and the last cost, `crate::allowance::spent`'s fold.
    /// **`None` where it has no terminating line**, never nought.
    pub spent: Option<(u64, u64)>,
    /// When its last run ended, where no run has started since: a Drone Fleet
    /// is holding at rest, for the gate or for a person.
    pub at_rest_since: Option<ipc::Instant>,
}

/// [`SoFar`], folded over a transcript's rows in the order they were written.
///
/// **A run is woken by a start or by Fleet speaking into it**, never by a
/// row Fleet writes beside it: a Check's outcome or a produced file comes
/// after the end and is not the Drone working.
pub(crate) fn so_far(rows: impl IntoIterator<Item = TranscriptRow>) -> SoFar {
    rows.into_iter().fold(SoFar::default(), |mut so_far, row| {
        match row.saw {
            Saw::Ended {
                turns, cost_micros, ..
            } => {
                let (summed, _) = so_far.spent.unwrap_or((0, 0));
                so_far.spent = Some((summed + u64::from(turns), cost_micros));
                so_far.at_rest_since = Some(row.ts);
            }
            Saw::Started { .. } | Saw::Instructed { .. } => {
                so_far.at_rest_since = None;
            }
            _ => {}
        }
        so_far
    })
}

/// [`so_far`] off one Drone's own transcript file, rather than off its slot,
/// because reading the slot waits behind a Check. A row that will not decode
/// is skipped; a file that will not open has said nothing.
async fn read_so_far(records_root: &str, handle: &str, drone: &DroneId) -> SoFar {
    let Ok(file) = fs::File::open(transcript_of(records_root, handle, drone)).await else {
        return SoFar::default();
    };
    let mut lines = BufReader::new(file).lines();
    let mut rows = Vec::new();
    while let Ok(Some(line)) = lines.next_line().await {
        if let Ok(row) = ipc::decode::<TranscriptRow>("a transcript row", line.as_bytes()) {
            rows.push(row);
        }
    }
    so_far(rows)
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
    /// `list_job_drones`. A stopped Drone carries its spend row, the figure
    /// the Job's spend is summed from; a running one has no row yet, or one
    /// that trails it, so it carries its transcript's, and says off the same
    /// read whether it is resting.
    pub(crate) async fn job_drones(&self, job_id: ipc::JobId) -> Result<JobDrones, Refusal> {
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let (events, spends, on_tasks, models) = {
            let store = self.store().lock().await;
            let events = store.events_for(job.id()).map_err(|cause| {
                self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(cause)))
            })?;
            let spends = store
                .drone_spends_for(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let on_tasks = store
                .task_drones(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            let models = store
                .drone_models(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            (events, spends, on_tasks, models)
        };
        let mut drones = Vec::new();
        let mut had_all = drones_had(&events);
        // A Drone beside the kept one is on no record of moves: `store::crew`.
        had_all.extend(on_tasks.iter().filter(|bound| bound.extra).map(beside));
        had_all.sort_by(|a, b| a.spawned_at.as_str().cmp(b.spawned_at.as_str()));
        for had in had_all {
            let on_task = on_tasks.iter().find(|bound| bound.drone_id == had.drone);
            let had = match on_task {
                Some(bound) => had.on_task(bound),
                None => had,
            };
            let (turns, cost_micros, at_rest_since) = match had.state {
                DroneState::Running => {
                    let served = self.served_by(&job).map_err(|why| self.refusal(why))?;
                    let read = read_so_far(served.records_root(), &job.handle(), &had.drone).await;
                    let (turns, cost) = read.spent.unzip();
                    (turns, cost, read.at_rest_since)
                }
                // A row's cost is set by the first terminating line and its
                // turns are summed from nought, so a row with no cost saw no
                // terminating line and its nought is no count at all.
                _ => match spends.iter().find(|(drone, _)| *drone == had.drone) {
                    Some((_, spend)) => (
                        spend.cost_micros.map(|_| spend.turns),
                        spend.cost_micros,
                        None,
                    ),
                    None => (None, None, None),
                },
            };
            drones.push(JobDrone {
                drone_id: (&had.drone).into(),
                step_id: (&had.step).into(),
                task: on_task.map(|bound| bound.task.to_string()),
                model: models
                    .iter()
                    .find(|(drone, _)| *drone == had.drone)
                    .map(|(_, model)| model.clone()),
                state: had.state,
                since: (&had.spawned_at).into(),
                ended_at: had.left_at.as_ref().map(Into::into),
                at_rest_since,
                turns,
                cost_micros,
            });
        }
        Ok(JobDrones { job_id, drones })
    }
}
