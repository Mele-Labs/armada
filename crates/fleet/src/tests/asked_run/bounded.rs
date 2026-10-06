//! A Drone's run is bounded as a whole, and told when it has to wait.
//! The wait for a place used to be bounded by nothing: Job 3's Drone heard
//! nothing for 43 minutes.

use std::sync::Arc;
use std::time::Duration;

use crate::gate::CheckBudget;
use crate::places::{Asking, ChecksAtOnce};
use crate::tests::asked_run::{
    a_fleet_budgeted, a_quiet_drone, checks_in, one_step, started, the_one_drone, transcript,
    Fixture, Held, A_CHECK_RUN_HAS_LONG_ENOUGH,
};
use crate::tests::tmp::TempDir;

/// A Fleet with one place, whose run may wait `place_wait` for it.
fn a_full_machine(home: &TempDir, place_wait: Duration) -> Fixture {
    let fleet = a_fleet_budgeted(
        home,
        one_step("/bin/true"),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
        a_quiet_drone(),
        CheckBudget::of(Duration::from_millis(200)).waiting(place_wait),
    );
    fleet.rechecked(ChecksAtOnce::of(1));
    fleet
}

async fn marked(fleet: &Fixture) -> bool {
    fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .is_some_and(|at_work| at_work.is_checking())
}

fn count(turns: &[String], phrase: &str) -> usize {
    turns.iter().filter(|turn| turn.contains(phrase)).count()
}

#[tokio::test]
async fn a_run_that_never_gets_a_place_is_stopped_and_the_drone_is_told() {
    let home = TempDir::new();
    let fleet = Arc::new(a_full_machine(&home, Duration::from_millis(300)));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");
    let held = fleet.room(Asking::Gate).place().await;

    let underway = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    let said = transcript(&fleet, &home, &job, &drone)
        .await
        .until(A_CHECK_RUN_HAS_LONG_ENOUGH, |said| {
            count(&checks_in(said), "did not finish") > 0
        })
        .await
        .expect("the Drone was never told the run was stopped");
    let turns = checks_in(&said);

    assert_eq!(
        count(&turns, "still waiting for a Check slot"),
        1,
        "the turn does not say it was waiting: {turns:?}"
    );
    assert_eq!(count(&turns, "Waiting for a Check slot, 1 of 1 in use"), 1);
    assert!(underway.finished().await.is_some());
    assert!(!marked(&fleet).await, "the mark is still on");

    drop(held);
    fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("a later request is accepted");
}

#[tokio::test]
async fn a_run_that_waits_is_told_once_and_then_runs() {
    let home = TempDir::new();
    let fleet = Arc::new(a_full_machine(&home, Duration::from_secs(30)));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");
    let held = fleet.room(Asking::Gate).place().await;

    let underway = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    transcript(&fleet, &home, &job, &drone)
        .await
        .until(A_CHECK_RUN_HAS_LONG_ENOUGH, |said| {
            count(&checks_in(said), "Waiting for a Check slot") > 0
        })
        .await
        .expect("the Drone was not told it is queued");
    drop(held);
    let finished = underway.finished().await;
    let said = transcript(&fleet, &home, &job, &drone)
        .await
        .settled(A_CHECK_RUN_HAS_LONG_ENOUGH)
        .await;
    let turns = checks_in(&said);

    assert!(matches!(finished, Some(Ok(_))), "{finished:?}");
    assert_eq!(count(&turns, "Waiting for a Check slot"), 1, "{turns:?}");
    assert_eq!(count(&turns, "The run is over"), 1, "{turns:?}");
}
