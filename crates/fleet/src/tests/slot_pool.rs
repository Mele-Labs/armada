//! A person reshaping the pool from Cleanup's bay grid: a slot added, closed,
//! reopened and removed, what admission makes of each, and the refusal Bridge
//! shows on the slot. The pool's own git rules are `adapters`' tests.

use axum::http::StatusCode;
use core_model::{JobStatus, QueuedReason};
use ipc::{ChangeSlotPool, SlotAct};
use testkit::FakeWorkProduct;

use crate::tests::admitted::admit;
use crate::tests::daemon::{a_fleet, a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;

fn root(home: &TempDir) -> String {
    home.path().to_string_lossy().to_string()
}

fn asked(act: SlotAct, slot: Option<u32>) -> ChangeSlotPool {
    ChangeSlotPool {
        act,
        slot,
        holder: None,
    }
}

/// Every slot held but the eighth, which a person closed: a new Job waits,
/// and starts there once it is reopened.
#[tokio::test]
async fn a_closed_slot_is_not_leased_and_reopened_it_is() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    for n in 1..=7 {
        fleet.vcs().hold_slot(&root(&home), n, "an agent's session");
    }
    fleet
        .change_slot_pool(asked(SlotAct::Close, Some(8)), None)
        .expect("closed");

    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    fleet.approve(job.id()).await.unwrap();
    admit(&fleet).await.unwrap();
    let loaded = fleet.load(job.id()).await.unwrap();
    assert_eq!(loaded.status(), JobStatus::Queued);
    assert_eq!(
        fleet.queued_reason(&loaded).await.unwrap().reason,
        Some(QueuedReason::WaitingOnResources)
    );

    fleet
        .change_slot_pool(asked(SlotAct::Open, Some(8)), None)
        .expect("reopened");
    admit(&fleet).await.unwrap();
    let started = fleet.load(job.id()).await.unwrap();
    assert_eq!(started.status(), JobStatus::Running);
    assert_eq!(started.worktree_slot(), Some(8));
}

/// With every slot held, an added one is where the next Job starts.
#[tokio::test]
async fn an_added_slot_is_where_a_waiting_job_starts() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    for n in 1..=8 {
        fleet.vcs().hold_slot(&root(&home), n, "an agent's session");
    }
    let added = fleet
        .change_slot_pool(asked(SlotAct::Add, None), None)
        .expect("added");
    assert_eq!(added.slot, 9);

    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    fleet.approve(job.id()).await.unwrap();
    admit(&fleet).await.unwrap();
    assert_eq!(fleet.load(job.id()).await.unwrap().worktree_slot(), Some(9));
}

/// The wire: `closed` on each slot of `GET /worktrees`, and a refusal coded
/// for Bridge to say on the slot.
#[tokio::test]
async fn the_wire_carries_closed_and_names_why_a_slot_cannot_go() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fleet.vcs().hold_slot(&root(&home), 1, "an agent's session");
    let events = fleet.events();
    let app = api::router(api::Served::by(fleet, ipc::RunId::carried("01RUN"), events));
    let post = |body: &'static str| {
        let app = app.clone();
        async move { crate::tests::http::call(&app, "POST", "/worktrees/slots", body).await }
    };

    let (status, _) = post(r#"{"act":"close","slot":2}"#).await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = crate::tests::http::call(&app, "GET", "/worktrees", "").await;
    let held: ipc::WorktreesHeld = ipc::decode("held", &body).unwrap();
    let closed: Vec<u32> = held
        .slots
        .iter()
        .filter(|one| one.closed)
        .map(|one| one.slot)
        .collect();
    assert_eq!(closed, vec![2]);

    let (status, body) = post(r#"{"act":"remove","slot":1}"#).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let error: ipc::WireError = ipc::decode("a refusal", &body).unwrap();
    assert_eq!(error.code, "fleet.slot_held");

    let (status, body) = post(r#"{"act":"remove","slot":12}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let error: ipc::WireError = ipc::decode("a refusal", &body).unwrap();
    assert_eq!(error.code, "fleet.no_such_slot");

    let (status, body) = post(r#"{"act":"remove","slot":3}"#).await;
    assert_eq!(status, StatusCode::OK);
    let changed: ipc::SlotPoolChanged = ipc::decode("changed", &body).unwrap();
    assert_eq!(changed.slot, 3);
    let (_, body) = crate::tests::http::call(&app, "GET", "/worktrees", "").await;
    let held: ipc::WorktreesHeld = ipc::decode("held", &body).unwrap();
    assert!(held.slots.iter().all(|one| one.slot != 3));
}

fn released_for(slot: u32, holder: &str) -> ChangeSlotPool {
    ChangeSlotPool {
        act: SlotAct::Release,
        slot: Some(slot),
        holder: Some(holder.to_string()),
    }
}

/// A session's slot goes back to the pool for the holder the person was shown,
/// and the answer names the branch and the files committed to it.
#[test]
fn a_slot_a_session_holds_is_released_and_says_what_was_committed() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fleet.vcs().hold_slot(&root(&home), 2, "claude (pid 44698)");

    let released = fleet
        .change_slot_pool(released_for(2, "claude (pid 44698)"), None)
        .expect("released");

    let released = released
        .released
        .expect("it says what it did to the branch");
    assert_eq!(
        released.saved.expect("the files were committed").files,
        vec!["wip.txt"]
    );
    assert_eq!(fleet.vcs().slot_holders(&root(&home))[1], None);
}

/// A slot re-leased since the person looked is refused, and keeps its holder.
#[test]
fn a_slot_another_session_took_since_is_refused_and_left_alone() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fleet.vcs().hold_slot(&root(&home), 2, "claude (pid 51)");

    let refused = fleet
        .change_slot_pool(released_for(2, "claude (pid 44698)"), None)
        .expect_err("a different holder");

    let api::Refusal::IllegalMove(error) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(error.code, "fleet.slot_holder_changed");
    assert_eq!(
        fleet.vcs().slot_holders(&root(&home))[1],
        Some(String::from("claude (pid 51)"))
    );
}

#[test]
fn a_release_naming_no_holder_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let mut bare = released_for(2, "x");
    bare.holder = None;

    let refused = fleet.change_slot_pool(bare, None).expect_err("no holder");

    assert!(matches!(refused, api::Refusal::Unacceptable(_)));
}
