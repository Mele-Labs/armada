//! The mark a run puts on its part lasts as long as the run and no longer.
//! Job 5's Drone asked for its checks at 12:00, heard nothing for 25 minutes,
//! and was then refused a second ask because the first was "still running"
//! with no process behind it.

use std::sync::Arc;
use std::time::Duration;

use crate::gate::CheckBudget;
use crate::tests::dry_run::{
    a_fleet_budgeted, a_quiet_drone, checks_in, one_step, started, the_one_drone, transcript,
    Held, A_CHECK_RUN_HAS_LONG_ENOUGH,
};
use crate::tests::tmp::TempDir;

#[tokio::test]
async fn a_run_whose_task_dies_clears_the_mark_and_tells_the_drone() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = Arc::new(a_fleet_budgeted(
        &home,
        one_step("/bin/true"),
        Arc::clone(&clock),
        3,
        &["src/parse.rs"],
        a_quiet_drone(),
        CheckBudget::of(Duration::from_secs(30)),
    ));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");

    let underway = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    clock.doom_next_reading();
    let said = transcript(&fleet, &home, &job, &drone)
        .await
        .until(A_CHECK_RUN_HAS_LONG_ENOUGH, |said| {
            checks_in(said)
                .iter()
                .any(|turn| turn.contains("stopped before they finished"))
        })
        .await
        .expect("the Drone was never told the run was lost");
    let turns = checks_in(&said);
    assert!(
        turns.iter().any(|turn| turn.contains("Ask again")),
        "the turn gives the Drone nothing to do: {turns:?}"
    );
    assert!(matches!(underway.finished().await, Some(Err(_))));
    assert!(
        !fleet
            .the_only_slot()
            .await
            .lock()
            .await
            .as_ref()
            .is_some_and(|at_work| at_work.is_checking()),
        "the mark is still on"
    );
    fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("a later request is accepted");
}

#[tokio::test]
async fn a_run_that_goes_on_tells_the_drone_where_it_stands_without_being_asked() {
    use crate::places::{Asking, ChecksAtOnce};
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_budgeted(
        &home,
        one_step("/bin/true"),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
        a_quiet_drone(),
        CheckBudget::of(Duration::from_millis(200))
            .waiting(Duration::from_secs(30))
            .reporting(Duration::from_millis(100)),
    ));
    fleet.rechecked(ChecksAtOnce::of(1));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");
    let held = fleet.room(Asking::Gate).place().await;

    fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    let said = transcript(&fleet, &home, &job, &drone)
        .await
        .until(A_CHECK_RUN_HAS_LONG_ENOUGH, |said| {
            checks_in(said)
                .iter()
                .any(|turn| turn.contains("The run is still going"))
        })
        .await
        .expect("the Drone was never told where the run stands");
    assert!(checks_in(&said)
        .iter()
        .any(|turn| turn.contains("`suite`")));
    drop(held);
}
