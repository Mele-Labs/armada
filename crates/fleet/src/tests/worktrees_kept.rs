//! `GET /worktrees` answered from memory: what a request costs, and what makes
//! the snapshot behind it read again.

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use axum::Router;
use ipc::{ChangeSlotPool, SlotAct};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::a_fleet;
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn root(home: &TempDir) -> String {
    home.path().to_string_lossy().to_string()
}

/// Wait for something a background read does. Real time: a paused clock would
/// run the wait out while the read is still on a blocking thread.
async fn until(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..500 {
        if done() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("never happened: {what}");
}

/// A Fleet whose keeper has made its first read, and the router in front of it.
async fn warm(home: &TempDir) -> (Arc<Fixture>, Router) {
    let fleet = Arc::new(a_fleet(home, FakeWorkProduct::changed(&["src/log.rs"])));
    fleet.vcs().hold_slot(&root(home), 1, "an agent's session");
    crate::keep_worktrees_warm(Arc::clone(&fleet));
    until("the first read", || fleet.vcs().pool_reads() >= 1).await;
    // The read has been kept by the time the keeper looks again.
    until("it was kept", || fleet.worktrees_memory().is_kept()).await;
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        ipc::RunId::carried("01RUN"),
        events,
    ));
    (fleet, app)
}

async fn held(app: &Router) -> ipc::WorktreesHeld {
    let (status, body) = call(app, "GET", "/worktrees", "").await;
    assert_eq!(status, StatusCode::OK);
    ipc::decode("held", &body).expect("the answer decodes")
}

fn held_slots(list: &ipc::WorktreesHeld) -> Vec<u32> {
    let mut slots: Vec<u32> = list
        .slots
        .iter()
        .filter(|one| !matches!(one.held, ipc::SlotHolding::Free | ipc::SlotHolding::Unmade))
        .map(|one| one.slot)
        .collect();
    slots.sort_unstable();
    slots
}

#[tokio::test]
async fn a_request_is_answered_from_the_snapshot_without_reading_the_pool() {
    let home = TempDir::new();
    let (fleet, app) = warm(&home).await;
    let reads = fleet.vcs().pool_reads();

    let first = held(&app).await;
    let second = held(&app).await;

    assert_eq!(held_slots(&first), vec![1], "the slot the snapshot saw held");
    assert_eq!(first, second);
    assert_eq!(fleet.vcs().pool_reads(), reads, "no pool was read to answer");
}

#[tokio::test]
async fn what_is_served_is_what_the_live_read_makes() {
    let home = TempDir::new();
    let (fleet, app) = warm(&home).await;

    let served = held(&app).await;
    let live = fleet.worktrees_live().await.expect("a live read");

    assert_eq!(served, live);
}

#[tokio::test]
async fn a_slot_taken_elsewhere_is_in_the_snapshot_soon_but_not_in_the_answer_that_noticed() {
    let home = TempDir::new();
    let (fleet, app) = warm(&home).await;
    held(&app).await;
    let reads = fleet.vcs().pool_reads();

    fleet.vcs().hold_slot(&root(&home), 2, "nvim (pid 44698)");
    let noticed = held(&app).await;
    assert_eq!(held_slots(&noticed), vec![1], "the request did not wait for a read");

    until("a read behind the request", || fleet.vcs().pool_reads() > reads).await;
    until("the slot in the snapshot", || {
        fleet
            .worktrees_memory()
            .serves(|list| held_slots(list) == vec![1, 2])
    })
    .await;
    assert_eq!(held_slots(&held(&app).await), vec![1, 2]);
}

#[tokio::test]
async fn an_act_on_the_pool_is_in_the_answer_that_follows_it() {
    let home = TempDir::new();
    let (fleet, app) = warm(&home).await;
    let before = held(&app).await.slots.len();

    api::Commands::change_slot_pool(
        &*fleet,
        ChangeSlotPool {
            act: SlotAct::Add,
            slot: None,
            holder: None,
        },
        None,
    )
    .await
    .expect("a slot added");

    assert_eq!(held(&app).await.slots.len(), before + 1);
}

#[tokio::test]
async fn an_old_snapshot_is_read_again_behind_the_request_that_finds_it() {
    let home = TempDir::new();
    let (fleet, app) = warm(&home).await;
    held(&app).await;
    let reads = fleet.vcs().pool_reads();

    fleet
        .worktrees_memory()
        .aged(crate::worktrees_kept::STALE_AFTER + Duration::from_secs(1));
    held(&app).await;
    assert_eq!(fleet.vcs().pool_reads(), reads, "answered before it read");

    until("a read behind the request", || fleet.vcs().pool_reads() > reads).await;
}

#[tokio::test]
async fn nobody_asking_for_two_minutes_reads_nothing_and_the_next_request_is_live() {
    let home = TempDir::new();
    let (fleet, app) = warm(&home).await;
    held(&app).await;
    fleet
        .worktrees_memory()
        .aged(crate::worktrees_kept::WATCHED_FOR + Duration::from_secs(1));
    let reads = fleet.vcs().pool_reads();

    tokio::time::sleep(Duration::from_millis(700)).await;
    assert_eq!(fleet.vcs().pool_reads(), reads, "the keeper read for nobody");

    fleet.vcs().hold_slot(&root(&home), 3, "an agent's session");
    assert_eq!(held_slots(&held(&app).await), vec![1, 3], "live, so already true");
    assert!(fleet.vcs().pool_reads() > reads);
}

#[tokio::test]
async fn without_a_keeper_every_request_reads() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"])));
    let events = fleet.events();
    let app = api::router(api::Served::sharing(
        Arc::clone(&fleet),
        ipc::RunId::carried("01RUN"),
        events,
    ));

    held(&app).await;
    held(&app).await;

    assert_eq!(fleet.vcs().pool_reads(), 2);
}

#[tokio::test]
async fn a_change_reads_the_slots_that_changed_and_not_the_whole_pool() {
    let home = TempDir::new();
    let (fleet, app) = warm(&home).await;
    held(&app).await;
    let (reads, wholes) = (fleet.vcs().pool_reads(), fleet.vcs().whole_pool_reads());
    assert_eq!(wholes, 1, "the first read has nothing kept to lean on");

    fleet.worktrees_memory().changed();
    until("a read for the change", || fleet.vcs().pool_reads() > reads).await;

    assert_eq!(fleet.vcs().whole_pool_reads(), wholes, "only the whole read asks for every slot");
}

#[tokio::test]
async fn the_read_every_stale_after_asks_for_every_slot() {
    let home = TempDir::new();
    let (fleet, app) = warm(&home).await;
    held(&app).await;
    let wholes = fleet.vcs().whole_pool_reads();

    fleet
        .worktrees_memory()
        .aged(crate::worktrees_kept::STALE_AFTER + Duration::from_secs(1));
    held(&app).await;

    until("a whole read", || fleet.vcs().whole_pool_reads() > wholes).await;
}

#[tokio::test]
async fn reads_for_changes_do_not_put_the_whole_read_back() {
    let home = TempDir::new();
    let (fleet, app) = warm(&home).await;
    held(&app).await;
    let stale = crate::worktrees_kept::STALE_AFTER;
    let (reads, wholes) = (fleet.vcs().pool_reads(), fleet.vcs().whole_pool_reads());

    // The whole read is two seconds from due; a change is read in that time.
    fleet.worktrees_memory().aged(stale - Duration::from_secs(2));
    fleet.worktrees_memory().changed();
    until("the read for the change", || fleet.vcs().pool_reads() > reads).await;
    assert_eq!(fleet.vcs().whole_pool_reads(), wholes);

    // Three seconds on, the whole read is due though the last read is not old.
    fleet.worktrees_memory().aged(Duration::from_secs(3));
    until("the whole read", || fleet.vcs().whole_pool_reads() > wholes).await;
}
