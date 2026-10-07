//! Taking a Job over: the mark, the pull, what keeps the worktree, and the
//! bundle. `docs/concepts/pilot.md`. #366, #367.
//!
//! The three exits are `piloting_exits`.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::Router;
use core_model::{
    Actor, EscalationTrigger, JobId, JobStatus, PilotReason, StepState, Target, TransitionReason,
};
use ipc::{HandoffBundle, JobSummary, RunId, WireError};
use store::{AttachmentState, Holder, Narrative};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::piloting::{Hatch, Unpilotable};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet, a_proposal, spec_held};
use crate::tests::http::call;
use crate::tests::reviewing::{a_fleet_reviewing_the_first_step, at_the_gate};
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

pub(super) fn summary(body: &[u8]) -> JobSummary {
    ipc::decode("a Job summary", body).expect("a JobSummary")
}

#[tokio::test]
async fn taking_over_a_running_job_ends_its_drone_stops_its_step_and_pilots_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = running(&fleet, &home, "fix the reader").await;
    assert!(fleet.working_on().await.contains(&id), "a Drone is on it");

    let piloted = fleet
        .take_over(&id, PilotReason::TakeOver, None)
        .await
        .expect("a running Job is taken over");

    assert_eq!(piloted.status(), JobStatus::Piloted);
    assert!(piloted.assigned_drone().is_none(), "its Drone is gone");
    assert!(!fleet.working_on().await.contains(&id));
    let step = piloted.current_step().expect("a step");
    assert_eq!(step.state(), StepState::Stopped, "the step stopped with it");
    assert_eq!(
        fleet.last_reason(&id).await.unwrap(),
        Some(TransitionReason::Pilot(PilotReason::TakeOver)),
        "the reason the move carried is the outcome chosen"
    );
    let row = fleet.summarised(&piloted).await.expect("a row");
    let pilot = row.piloted.expect("the row says it is piloted");
    assert_eq!(pilot.reason, "take_over");
    assert!(pilot.session_id.is_none() && pilot.exit.is_none());
}

#[tokio::test]
async fn a_take_over_works_from_a_gate_and_from_an_escalation_and_restart_step_wants_a_stopped_step(
) {
    let home = TempDir::new();
    let fleet = a_fleet_reviewing_the_first_step(&home, changed());
    let gated = at_the_gate(&fleet, &home).await;
    assert_eq!(
        fleet.load(&gated).await.unwrap().status(),
        JobStatus::AwaitingReview
    );

    let refused = fleet
        .take_over(&gated, PilotReason::RestartStep, None)
        .await
        .expect_err("no step stopped at a gate");
    assert!(
        matches!(
            refused,
            Adrift::CannotPilot {
                why: Unpilotable::NoStepToRestart,
                ..
            }
        ),
        "{refused:?}"
    );
    assert_eq!(
        fleet.load(&gated).await.unwrap().status(),
        JobStatus::AwaitingReview,
        "nothing moved"
    );
    let piloted = fleet
        .take_over(&gated, PilotReason::TakeOver, None)
        .await
        .expect("a gate Job is taken over");
    assert_eq!(piloted.status(), JobStatus::Piloted);
    assert_eq!(
        piloted.current_step().map(|step| step.state()),
        Some(StepState::AwaitingHuman),
        "a step at a gate is left where it stood"
    );

    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = running(&fleet, &home, "an escalated one").await;
    fleet.kill_drone(&id).await.expect("its Drone ends");
    assert_eq!(
        fleet.load(&id).await.unwrap().status(),
        JobStatus::Escalated
    );
    let restarted = fleet
        .take_over(&id, PilotReason::RestartStep, None)
        .await
        .expect("an escalated Job is taken over");
    assert_eq!(restarted.status(), JobStatus::Piloted);
    assert_eq!(
        fleet.last_reason(&id).await.unwrap(),
        Some(TransitionReason::Pilot(PilotReason::RestartStep))
    );
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

    let proposed = fleet.propose(a_proposal("not yet approved")).await.unwrap();
    refused(act(proposed.id(), "take_over").await, "fleet.not_pilotable");

    let id = running(&fleet, &home, "a working Job").await;
    for exit in [
        "submit_for_verification",
        "attest_complete",
        "close_as_superseded",
    ] {
        refused(act(&id, exit).await, "fleet.not_piloted");
    }
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Running);

    let (status, body) = act(&id, "take_over").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary(&body).status.as_wire(), "piloted");
    refused(act(&id, "take_over").await, "fleet.already_piloted");

    let held = fleet.rechecking().take(&id).expect("another act is out");
    refused(act(&id, "attest_complete").await, "fleet.pilot_busy");
    drop(held);
}

#[tokio::test]
async fn a_pull_of_the_hatch_on_a_job_nobody_marked_escalates_it_and_says_nothing_more() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = running(&fleet, &home, "a Drone reaching for the hatch").await;

    let answer = fleet
        .hatch_pulled(&id, Narrative::default())
        .await
        .expect("an unmarked pull is answered");

    assert_eq!(answer, Hatch::Unavailable);
    let job = fleet.load(&id).await.unwrap();
    assert_eq!(job.status(), JobStatus::Escalated);
    assert_eq!(
        fleet.last_reason(&id).await.unwrap(),
        Some(TransitionReason::Escalation(
            EscalationTrigger::HatchUnbidden
        ))
    );
}

#[tokio::test]
async fn a_pull_on_a_marked_job_pilots_it_and_keeps_what_the_drone_said() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = running(&fleet, &home, "a Drone asked to pull").await;
    fleet
        .store()
        .lock()
        .await
        .mark_for_pilot(
            &id,
            PilotReason::TakeOver,
            Some("a-session"),
            fleet.run().as_str(),
            &fleet.now(),
        )
        .unwrap();

    let answer = fleet
        .hatch_pulled(
            &id,
            Narrative {
                trying_to: "make the reader stop one line later".into(),
                blocked_by: "the writer's tests will not build".into(),
                tried: "two passes over the bound".into(),
            },
        )
        .await
        .expect("a marked pull");

    assert_eq!(answer, Hatch::Pulled);
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Piloted);
    let bundle = fleet
        .handoff_bundle(ipc::JobId::from(&id))
        .await
        .expect("a bundle");
    let said = bundle.narrative.expect("the Drone's narrative");
    assert_eq!(said.blocked_by, "the writer's tests will not build");
    assert_eq!(bundle.session_id.as_deref(), Some("a-session"));
}

#[tokio::test]
async fn a_mark_left_by_an_earlier_run_is_not_honoured() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = running(&fleet, &home, "a stale mark").await;
    fleet
        .store()
        .lock()
        .await
        .mark_for_pilot(
            &id,
            PilotReason::TakeOver,
            None,
            "an-earlier-run",
            &fleet.now(),
        )
        .unwrap();

    let answer = fleet.hatch_pulled(&id, Narrative::default()).await.unwrap();

    assert_eq!(answer, Hatch::Unavailable);
    assert_eq!(
        fleet.load(&id).await.unwrap().status(),
        JobStatus::Escalated,
        "it was a Drone reaching unbidden"
    );
}

#[tokio::test]
async fn the_bundle_is_served_once_a_job_is_piloted_and_not_before() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home, changed()));
    let app = served(&fleet);
    let id = running(&fleet, &home, "hand me this worktree").await;
    let path = format!("/jobs/{}/handoff", id.as_str());

    let (status, body) = call(&app, "GET", &path, "").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(code(&body), "fleet.not_piloted");

    fleet
        .take_over(&id, PilotReason::TakeOver, Some("a-session"))
        .await
        .expect("taken over");
    let (status, body) = call(&app, "GET", &path, "").await;
    assert_eq!(status, StatusCode::OK);
    let bundle: HandoffBundle = ipc::decode("a bundle", &body).expect("a bundle");

    assert_eq!(bundle.reason, "take_over");
    assert_eq!(bundle.session_id.as_deref(), Some("a-session"));
    assert_eq!(bundle.job.job.id.as_str(), id.as_str());
    assert_eq!(bundle.job.steps.len(), 2, "the frozen workflow's steps");
    let stopped = bundle.stopped_on.expect("the step it stopped on");
    assert_eq!(stopped.step_id.as_str(), "implement");
    assert_eq!(stopped.trigger.as_deref(), Some("drone_killed"));
    let worktree = bundle.worktree.expect("where the person works");
    assert!(worktree.path.contains("slot-1"), "{}", worktree.path);
    assert!(
        worktree.branch.starts_with("armada/"),
        "{}",
        worktree.branch
    );
    assert!(bundle.narrative.is_none(), "no Drone said anything");
    assert!(
        !bundle.history.moves.is_empty(),
        "every move the Job made is in it"
    );
}

#[tokio::test]
async fn a_named_session_holds_the_slot_and_branch_as_handed_over_and_the_job_does_not() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = running(&fleet, &home, "piloted from a session").await;
    let before = fleet.load(&id).await.unwrap();
    let slot = before.worktree_slot().expect("a slot").to_string();
    let branch = before.branch().expect("a branch").as_str().to_string();

    fleet
        .take_over(&id, PilotReason::TakeOver, Some("pilot-1"))
        .await
        .expect("taken over");

    let store = fleet.store().lock().await;
    let held = store.attachments_of(&Holder::session("pilot-1")).unwrap();
    let standing = |kind: &str, target: &str| {
        held.iter().find(|row| {
            row.kind == kind && row.target == target && row.state == AttachmentState::Standing
        })
    };
    let handed = format!("job {}", id.as_str());
    for (kind, target) in [("slot", &slot), ("branch", &branch)] {
        let row = standing(kind, target).unwrap_or_else(|| panic!("the Session holds {kind}"));
        assert_eq!(row.detail.get("handed"), Some(&handed));
    }
    let job_rows = store.attachments_of(&Holder::job(id.as_str())).unwrap();
    assert!(
        job_rows
            .iter()
            .filter(|row| row.kind == "slot" || row.kind == "branch")
            .all(|row| row.state == AttachmentState::GivenBack),
        "the Job's own rows are given back: {job_rows:?}"
    );
}

#[tokio::test]
async fn a_piloted_job_takes_no_slot_leaves_the_scheduler_and_cannot_be_paused_or_released() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, changed());
    let id = running(&fleet, &home, "a person is working it").await;
    fleet
        .take_over(&id, PilotReason::TakeOver, None)
        .await
        .expect("taken over");

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
    fleet
        .take_over(&id, PilotReason::TakeOver, None)
        .await
        .expect("taken over");

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
    fleet
        .take_over(&first, PilotReason::TakeOver, None)
        .await
        .expect("the oldest is taken over");
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
