//! Every Drone a Job has had, read off its history — `list_job_drones`.
//!
//! **The record, not the roster.** `crate::rostered` loses a Drone the moment it
//! exits; the history keeps `drone_spawned` and `drone_exited` for every one and
//! the spend rows outlive them, so this reads both and holds no slot. Nothing
//! records a Drone's state, so it is read off the moves around it — the rule is
//! `list_job_drones`'s note in `crates/ipc/operations.toml`.

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

/// Turns summed and the last cost, off one Drone's own transcript, or `None`
/// where it has no terminating line. `crate::allowance::spent`'s fold, over the
/// file rather than the slot, because reading the slot waits behind a Check.
async fn spent_so_far(records_root: &str, handle: &str, drone: &DroneId) -> Option<(u64, u64)> {
    let file = fs::File::open(transcript_of(records_root, handle, drone))
        .await
        .ok()?;
    let mut lines = BufReader::new(file).lines();
    let mut spent = None;
    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(row) = ipc::decode::<TranscriptRow>("a transcript row", line.as_bytes()) else {
            continue;
        };
        if let Saw::Ended {
            turns, cost_micros, ..
        } = row.saw
        {
            let (summed, _) = spent.unwrap_or((0, 0));
            spent = Some((summed + u64::from(turns), cost_micros));
        }
    }
    spent
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
    /// that trails it, so it carries its transcript's.
    pub(crate) async fn job_drones(&self, job_id: ipc::JobId) -> Result<JobDrones, Refusal> {
        let job = self
            .load(&job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        let (events, spends) = {
            let store = self.store().lock().await;
            let events = store.events_for(job.id()).map_err(|cause| {
                self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(cause)))
            })?;
            let spends = store
                .drone_spends_for(job.id())
                .map_err(|why| self.refusal(Adrift::Reading(why)))?;
            (events, spends)
        };
        let mut drones = Vec::new();
        for had in drones_had(&events) {
            let (turns, cost_micros) = match had.state {
                DroneState::Running => {
                    let served = self.served_by(&job).map_err(|why| self.refusal(why))?;
                    match spent_so_far(served.records_root(), &job.handle(), &had.drone).await {
                        Some((turns, cost)) => (Some(turns), Some(cost)),
                        None => (None, None),
                    }
                }
                _ => match spends.iter().find(|(drone, _)| *drone == had.drone) {
                    Some((_, spend)) => (Some(spend.turns), spend.cost_micros),
                    None => (None, None),
                },
            };
            drones.push(JobDrone {
                drone_id: (&had.drone).into(),
                step_id: (&had.step).into(),
                state: had.state,
                since: (&had.spawned_at).into(),
                ended_at: had.left_at.as_ref().map(Into::into),
                turns,
                cost_micros,
            });
        }
        Ok(JobDrones { job_id, drones })
    }
}
