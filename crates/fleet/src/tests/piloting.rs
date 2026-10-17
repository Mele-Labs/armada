//! A piloted Job: what keeps its worktree, and what the scheduler and the pool leave alone.
//! `docs/concepts/pilot.md`. #366, #367.
//!
//! The three exits are `piloting_exits`.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::Router;
use core_model::{Actor, Job, JobId, JobStatus, PilotReason, Target};
use ipc::{RunId, WireError};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet, a_proposal, spec_held};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;

pub(super) type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

pub(super) fn changed() -> FakeWorkProduct {
    FakeWorkProduct::changed(&["src/log.rs"])
}

/// A Job dispatched onto a slot that exists on disk, its Drone standing in it.
pub(super) async fn running(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("proposed");
    spec_held(home, &job).expect("a slot, made early");
    let job = dispatched(fleet, job.id()).await.expect("dispatched");
    assert_eq!(job.status(), JobStatus::Running);
    job.id().clone()
}

pub(super) fn served(fleet: &Arc<Fixture>) -> Router {
    api::router(api::Served::sharing(
        Arc::clone(fleet),
        RunId::carried("01RUN"),
        fleet.events(),
    ))
}

pub(super) fn code(body: &[u8]) -> String {
    ipc::decode::<WireError>("a wire error", body)
        .expect("an error body")
        .code
}

/// A Job a person has taken over by hand: its Drone ended where one was working, the move made, and
/// the pilot row planted that taking over used to write. **Nothing takes a Job over now**; the
/// exits are what is left of piloting and they are held on a Job already in the state.
pub(super) async fn taken_over(fleet: &Fixture, id: &JobId, reason: PilotReason) -> Job {
    if fleet.load(id).await.unwrap().status() == JobStatus::Running {
        fleet.kill_drone(id).await.expect("its Drone ends");
    }
    let job = fleet.load(id).await.unwrap();
    let piloted = fleet
        .move_job(&job, Target::Piloted(reason.clone()), Actor::Human)
        .await
        .expect("moved to piloted");
    let now = fleet.now();
    fleet
        .store()
        .lock()
        .await
        .plant_a_pilot(id, reason, &now)
        .expect("a pilot planted");
    piloted
}

#[tokio::test]
async fn each_refusal_is_a_409_with_its_own_code_and_moves_nothing() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, changed()));
    let app = served(&fleet);
    let act = |job: &JobId, act: &'static str| {
        let app = app.clone();
        let path = format!("/jobs/{}/{act}", job.as_str());
        async move { call(&app, "POST", &path, "").await }
    };
    let refused = |(status, body): (StatusCode, Vec<u8>), expected: &str| {
        assert_eq!(status, StatusCode::CONFLICT, "{expected}");
        assert_eq!(code(&body), expected);
    };

    let id = running(&fleet, &home, "a working Job").await;
    for exit in [
        "submit_for_verification",
        "attest_complete",
        "close_as_superseded",
    ] {
        refused(act(&id, exit).await, "fleet.not_piloted");
    }
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Running);

    taken_over(&fleet, &id, PilotReason::TakeOver).await;
    let held = fleet.rechecking().take(&id).expect("another act is out");
    refused(act(&id, "attest_complete").await, "fleet.pilot_busy");
    drop(held);
}

#[tokio::test]
async fn a_piloted_job_takes_no_slot_leaves_the_scheduler_and_cannot_be_paused_or_released() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = running(&fleet, &home, "a person is working it").await;
    taken_over(&fleet, &id, PilotReason::TakeOver).await;

    for _ in 0..3 {
        fleet.turn().await.expect("a turn");
    }

    let job = fleet.load(&id).await.unwrap();
    assert_eq!(job.status(), JobStatus::Piloted, "nothing moved it");
    assert!(fleet.working_on().await.is_empty(), "no Drone, no slot");
    assert!(job.assigned_drone().is_none());
    let refusal = fleet
        .pause_job(&id)
        .await
        .expect_err("not a Job Fleet may park");
    assert!(
        matches!(refusal, crate::adrift::Adrift::NotPausable { .. }),
        "auto-release reads the same refusal: {refusal:?}"
    );
    assert_eq!(
        fleet.piloted_checkouts().await.len(),
        1,
        "the build sweep is told whose checkout to leave"
    );
}

#[tokio::test]
async fn a_reclaim_of_a_piloted_job_is_refused_and_says_why() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = running(&fleet, &home, "do not take this").await;
    taken_over(&fleet, &id, PilotReason::TakeOver).await;

    let refused = fleet.reclaim_worktree(&id).await.expect_err("refused");

    let said = refused.to_string();
    assert!(
        said.contains("is piloted") && said.contains("closing it as superseded"),
        "it says whose worktree it is and what to do instead: {said}"
    );
    assert_eq!(
        fleet.load(&id).await.unwrap().status(),
        JobStatus::Piloted,
        "and nothing moved"
    );
    let moved = fleet
        .move_job(
            &fleet.load(&id).await.unwrap(),
            Target::Killed,
            Actor::Human,
        )
        .await
        .expect("a person may still kill it");
    assert_eq!(moved.status(), JobStatus::Killed);
}

#[tokio::test]
async fn auto_release_passes_over_a_piloted_job_that_would_otherwise_be_the_oldest_taken() {
    use crate::tests::planted::Held;
    use crate::tests::releasing::{a_pool_of, a_waiter, three_at_a_gate, GRACE_SECONDS};

    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(&home, 3, "", &clock, "implement");
    let (first, second, _third) = three_at_a_gate(&fleet, &home, &clock).await;
    taken_over(&fleet, &first, PilotReason::TakeOver).await;
    let waiter = a_waiter(&fleet, &home, "the one that waits").await;

    clock.on(GRACE_SECONDS);
    let turned = fleet.turn().await.expect("a turn");

    assert_eq!(
        turned.released,
        vec![second],
        "the oldest is the person's, so the next is taken"
    );
    let piloted = fleet.load(&first).await.unwrap();
    assert_eq!(piloted.status(), JobStatus::Piloted);
    assert!(piloted.pause().is_none() && piloted.worktree_slot().is_some());
    assert_eq!(
        fleet.load(&waiter).await.unwrap().status(),
        JobStatus::Queued
    );
}
