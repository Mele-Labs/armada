//! Every Drone a Job has had, read under the Job — `list_job_drones`.
//!
//! **One Job through three Drones**, so each state is read off a real history
//! rather than a planted one: killed by a person, restarted and finished, and
//! the next step's still running. Every Drone earns its figure through a real
//! child, `crate::tests::paying`'s reason.

use std::time::Duration;

use api::Queries;
use ipc::{DroneState, JobDrone};
use testkit::FakeWorkProduct;

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::admitted::started;
use crate::tests::allowance::{approved, ended, Fixture, SHIPPED};
use crate::tests::daemon::diff_evidence;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

const COST: u64 = 146_473;
const TURNS: u32 = 7;

/// A Drone that names its price on its first line and then sits reading.
fn a_drone_priced() -> testkit::FakeHarness {
    testkit::FakeHarness::running(
        "/bin/sh",
        &["-c", "echo PRICED; while IFS= read -r line; do :; done"],
    )
    .reading("PRICED", vec![ended(TURNS, COST)])
}

async fn listed(fleet: &Fixture, job: &core_model::JobId) -> Vec<JobDrone> {
    fleet
        .list_job_drones(ipc::JobId::from(job))
        .await
        .expect("the Job's Drones read")
        .drones
}

/// Read until the newest Drone carries the turns its transcript has. The child
/// has to be scheduled and its line written before there is anything to read.
async fn until_priced(fleet: &Fixture, job: &core_model::JobId, count: usize) -> Vec<JobDrone> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let drones = listed(fleet, job).await;
            if drones.len() == count && drones[count - 1].turns.is_some() {
                return drones;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the running Drone's terminating line reached its transcript")
}

#[tokio::test]
async fn a_jobs_drones_are_listed_killed_done_and_running_with_what_each_spent() {
    let home = TempDir::new();
    let mut fittings = crate::tests::daemon::fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_drone_priced(),
    );
    fittings.allowance = SHIPPED;
    let fleet = Fleet::assembled(fittings);
    let job = approved(&fleet, &home, "a Job worked by three Drones").await;

    let first = until_priced(&fleet, &job, 1).await;
    assert_eq!(first[0].state, DroneState::Running);
    assert_eq!(first[0].ended_at, None, "it has not stopped");
    assert_eq!(
        (first[0].turns, first[0].cost_micros),
        (Some(u64::from(TURNS)), Some(COST)),
        "a running Drone carries what it has spent so far"
    );

    fleet.kill_drone(&job).await.expect("a person ends it");
    fleet
        .restart_step(&job, None)
        .await
        .expect("and restarts the step");
    started(&fleet, &job)
        .await
        .expect("the restart is admitted");
    until_priced(&fleet, &job, 2).await;
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the second Drone reports its diff");
    let turned = fleet.turn().await.expect("the gate rules");
    assert!(
        matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
        "the fixture did not advance: {:?}",
        turned.ruled()
    );

    let drones = until_priced(&fleet, &job, 3).await;
    let states: Vec<DroneState> = drones.iter().map(|drone| drone.state).collect();
    assert_eq!(
        states,
        [DroneState::Killed, DroneState::Done, DroneState::Running],
        "a person ending a Drone is not the Drone failing: {drones:?}"
    );
    let (killed, done, running) = (&drones[0], &drones[1], &drones[2]);
    assert_eq!(killed.step_id.as_str(), "implement");
    assert_eq!(done.step_id.as_str(), "implement");
    assert_eq!(running.step_id.as_str(), "summarise");
    assert_ne!(
        killed.drone_id, done.drone_id,
        "a restart is a second Drone"
    );
    for stopped in [killed, done] {
        assert!(
            stopped.ended_at.is_some(),
            "a stopped Drone says when: {stopped:?}"
        );
        assert!(stopped.ended_at.as_ref().map(|at| at.as_str()) >= Some(stopped.since.as_str()));
        assert_eq!(
            stopped.cost_micros,
            Some(COST),
            "and carries the row the Job's spend is summed from"
        );
        assert_eq!(stopped.turns, Some(u64::from(TURNS)));
    }
    assert_eq!(running.ended_at, None);
    assert_eq!(running.turns, Some(u64::from(TURNS)));
}

/// A Job that names nothing is refused, rather than answered with no Drones.
#[tokio::test]
async fn a_job_that_names_nothing_has_no_drones_to_list() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(crate::tests::daemon::fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_drone_priced(),
    ));
    let refused = fleet
        .list_job_drones(ipc::JobId::carried("01NOSUCHJOB"))
        .await;
    assert!(
        matches!(refused, Err(api::Refusal::NoSuchJob(_))),
        "{refused:?}"
    );
}

/// Wait for the Job's first Drone to be on the record.
async fn until_spawned(fleet: &Fixture, job: &core_model::JobId) -> Vec<JobDrone> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let drones = listed(fleet, job).await;
            if !drones.is_empty() {
                return drones;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the Drone was spawned")
}

/// Job 3 on 3 Oct: a Drone that submitted and ended its run read `running`,
/// and nothing else, for the seven minutes its step's Checks ran. Fleet is
/// holding it for the gate's answer, so it is still on the step; the row says
/// since when it has been at rest, which is what tells it from one working.
#[tokio::test]
async fn a_drone_fleet_holds_after_its_run_ended_says_since_when_it_rested() {
    let home = TempDir::new();
    let mut fittings = crate::tests::daemon::fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_drone_priced(),
    );
    fittings.allowance = SHIPPED;
    let fleet = Fleet::assembled(fittings);
    let job = approved(&fleet, &home, "a Job whose Drone has rested").await;

    let drones = until_priced(&fleet, &job, 1).await;
    assert_eq!(drones[0].state, DroneState::Running, "Fleet still holds it");
    let rested = drones[0]
        .at_rest_since
        .clone()
        .expect("a Drone whose run ended says when");
    assert!(rested.as_str() >= drones[0].since.as_str());

    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("its evidence goes to the gate");
    let held = listed(&fleet, &job).await;
    assert_eq!(held[0].state, DroneState::Running);
    assert_eq!(
        held[0].at_rest_since.as_ref(),
        Some(&rested),
        "at the gate it is resting, not working"
    );
}

/// A Drone part-way through its run has not rested, and a stopped Drone that
/// never wrote a terminating line names no turns: absent, never nought.
#[tokio::test]
async fn a_drone_mid_run_has_not_rested_and_one_stopped_before_its_end_names_no_turns() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(crate::tests::daemon::fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        testkit::FakeHarness::that_listens(),
    ));
    let job = approved(&fleet, &home, "a Job whose Drone is mid-run").await;

    let drones = until_spawned(&fleet, &job).await;
    assert_eq!(drones[0].state, DroneState::Running);
    assert_eq!(drones[0].at_rest_since, None, "it is working");
    assert_eq!(drones[0].turns, None);

    fleet.kill_drone(&job).await.expect("a person ends it");
    let stopped = listed(&fleet, &job).await;
    assert_eq!(stopped[0].state, DroneState::Killed);
    assert_eq!(stopped[0].at_rest_since, None, "it has left, not rested");
    assert_eq!(
        (stopped[0].turns, stopped[0].cost_micros),
        (None, None),
        "no terminating line, so no figure: {stopped:?}"
    );
}

/// The fold over a transcript: turns summed and the last cost, and rest is
/// the last terminating line with no run started after it. Rows Fleet writes
/// after it, a Check's or a produced file's, do not wake it.
#[test]
fn a_transcript_rests_at_its_last_end_until_a_run_starts_again() {
    use crate::drones_had::so_far;
    use ipc::{Saw, TranscriptRow, Voice};
    let row = |ts: &str, by: Voice, saw: Saw| TranscriptRow {
        ts: ipc::Instant::carried(ts),
        step: None,
        by,
        drone_id: None,
        saw,
    };
    let started = |ts: &str| {
        row(
            ts,
            Voice::Drone,
            Saw::Started {
                session: String::from("s"),
                model: String::from("m"),
                mcp_servers: 1,
            },
        )
    };
    let end = |ts: &str, turns: u32, cost_micros: u64| {
        row(
            ts,
            Voice::Drone,
            Saw::Ended {
                turns,
                cost_micros,
                refusals: 0,
            },
        )
    };
    let produced = |ts: &str| row(ts, Voice::Fleet, Saw::Produced { files: Vec::new() });

    let mid_run = so_far([started("03:45:05")]);
    assert_eq!((mid_run.spent, mid_run.at_rest_since), (None, None));

    let resumed = so_far([
        started("03:45:05"),
        end("03:45:24", 11, 186_790),
        started("03:45:31"),
    ]);
    assert_eq!(resumed.spent, Some((11, 186_790)));
    assert_eq!(resumed.at_rest_since, None, "a run started after the end");

    let held = so_far([
        started("03:45:05"),
        end("03:45:24", 11, 186_790),
        started("03:45:31"),
        end("03:45:35", 2, 211_931),
        produced("03:45:40"),
    ]);
    assert_eq!(held.spent, Some((13, 211_931)));
    assert_eq!(held.at_rest_since, Some(ipc::Instant::carried("03:45:35")));
}
