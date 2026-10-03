//! A task's Drone that handed in is let come to rest before it is ended, so
//! its terminating line — the only place the harness names turns and cost —
//! reaches the record. `crate::tasking::next_task_drone`.
//!
//! Job 3 on 3 Oct ended three task Drones within a quarter-second of their
//! hand-ins, each before its terminating line, and `list_job_drones` read
//! every one as `turns: 0` with no cost while each had made 6 to 12 calls.

use std::sync::Arc;
use std::time::Duration;

use core_model::{EvidenceType, JobId, JobStatus};
use testkit::{FakeHarness, FakeWorkProduct};
use verification::{Claimed, NotClaimed, ShownBy};

use crate::converging::StepNorms;
use crate::evidence::Call;
use crate::tests::allowance::ended;
use crate::tests::daemon::{fitted_with, fittings};
use crate::tests::drone_per_task::{at_implement_over, on_implement, Fixture};
use crate::tests::planted::Held;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

const COST: u64 = 61_204;
const TURNS: u32 = 9;

/// Norms that never trip, and a grace after a hand-in no case sits through.
const PATIENT: StepNorms = StepNorms::of(
    u32::MAX,
    Duration::from_secs(86_400),
    Duration::from_secs(3_600),
);

async fn handed_in(fleet: &Fixture) {
    submitted_by_the_one(
        fleet,
        Call {
            evidence_type: EvidenceType::Diff,
            claimed: Claimed("The reader stops at the end."),
            shown_by: ShownBy("read::stops passes"),
            not_claimed: NotClaimed(""),
            review: None,
        },
    )
    .await
    .expect("a task's hand-in is recorded");
}

async fn row_of(fleet: &Fixture, job: &JobId, drone: &core_model::DroneId) -> ipc::JobDrone {
    fleet
        .job_drones(job.into())
        .await
        .expect("the Job's Drones")
        .drones
        .into_iter()
        .find(|row| row.drone_id == ipc::DroneId::from(drone))
        .expect("the Drone is listed")
}

/// A Drone that says nothing until `flag` exists, then comes to rest once,
/// naming its price, and reads on.
fn resting_on(flag: &std::path::Path) -> FakeHarness {
    let script = format!(
        "while [ ! -e '{flag}' ]; do sleep 0.02; done; rm -f '{flag}'; echo RESTED; \
         exec cat >/dev/null",
        flag = flag.display()
    );
    FakeHarness::running("/bin/sh", &["-c", &script]).reading("RESTED", vec![ended(TURNS, COST)])
}

/// The Drone finishes its last message after its hand-in, and Fleet waits for
/// it: the row then carries the harness's own turns and cost, and coming to
/// rest between tasks is not read as a Drone that stalled.
#[tokio::test]
async fn a_tasks_drone_is_ended_once_it_comes_to_rest_and_its_spend_is_kept() {
    let home = TempDir::new();
    let flag = home.path().join("rest");
    let mut fitted = fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/read.rs"]),
        resting_on(&flag),
    );
    fitted.norms = PATIENT;
    let (fleet, job) = at_implement_over(&home, fitted).await;
    let first = on_implement(&fleet, &job).await.expect("T1's Drone");

    handed_in(&fleet).await;
    fleet.turn().await.expect("a turn");
    assert_eq!(
        on_implement(&fleet, &job).await.as_ref(),
        Some(&first),
        "a Drone still finishing its last message is not ended under it"
    );

    std::fs::write(&flag, b"").expect("the flag is written");
    tokio::time::timeout(Duration::from_secs(5), async {
        while row_of(&fleet, &job, &first).await.turns.is_none() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the terminating line reached the transcript");
    fleet.turn().await.expect("a turn");

    let next = on_implement(&fleet, &job).await.expect("T2's Drone");
    assert_ne!(next, first, "a Drone at rest after its hand-in is replaced");
    assert_eq!(
        fleet.load(&job).await.expect("reads").status(),
        JobStatus::Running,
        "a Drone at rest between tasks has not stalled"
    );
    let row = row_of(&fleet, &job, &first).await;
    assert_eq!(row.state, ipc::DroneState::Done);
    assert_eq!(
        (row.turns, row.cost_micros),
        (Some(u64::from(TURNS)), Some(COST)),
        "the stopped Drone carries the row its terminating line made"
    );
}

/// A Drone that never comes to rest is ended once the grace runs out, and a
/// row with no terminating line reads no turns rather than nought.
#[tokio::test]
async fn a_tasks_drone_that_never_rests_is_ended_after_the_grace_and_reads_no_turns() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&["src/read.rs"]));
    fitted.norms = PATIENT;
    fitted.clock = clock.clone();
    let (fleet, job) = at_implement_over(&home, fitted).await;
    let first = on_implement(&fleet, &job).await.expect("T1's Drone");

    handed_in(&fleet).await;
    fleet.turn().await.expect("a turn");
    assert_eq!(on_implement(&fleet, &job).await.as_ref(), Some(&first));

    clock.on(PATIENT.report_grace().as_secs());
    fleet.turn().await.expect("a turn");
    assert_ne!(
        on_implement(&fleet, &job).await.as_ref(),
        Some(&first),
        "the grace bounds the wait"
    );
    let row = row_of(&fleet, &job, &first).await;
    assert_eq!(row.state, ipc::DroneState::Done);
    assert_eq!(
        (row.turns, row.cost_micros),
        (None, None),
        "no terminating line arrived, so nothing names a figure"
    );
}
