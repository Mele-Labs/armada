//! A Script added to one Job on a destructive Command asks the owner before it
//! runs, as a saved Trigger does: the Board row rings, `list_alerts` names it,
//! and with `block` on the Job holds until he answers. His Run is the Command's
//! one firing; his Skip records it skipped. Commands are real programs; the
//! Drone and the pull request are fakes.

use std::sync::Arc;

use core_model::{Actor, JobId, JobStatus, NotRun, TriggerState};
use ipc::{AddStep, AddedRuns, HoldAct, JobAlertKind, TriggerFiringState as Wire, TriggerMoment};

use crate::adrift::Adrift;
use crate::tests::admitted::started;
use crate::tests::daemon::{a_proposal, diff_evidence, note_evidence, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::tests::trigger_asks::{a_fleet_asking, flag_in, Fixture};
use crate::tests::triggering::Files;
use crate::trigger_repair::Subject;

fn wipe(when: TriggerMoment, step: &str, block: bool, repair: bool) -> AddStep {
    AddStep {
        runs: AddedRuns::Script {
            command: "wipe".to_string(),
        },
        when,
        step: ipc::StepId::carried(step),
        block,
        repair,
    }
}

fn act() -> HoldAct {
    HoldAct {
        trigger: None,
        addition: Some("a1".to_string()),
    }
}

async fn approved_with(fleet: &Fixture, home: &TempDir, added: AddStep) -> JobId {
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(home, &job);
    let approval = ipc::ApproveDispatch {
        additions: Some(vec![added]),
        ..ipc::ApproveDispatch::default()
    };
    fleet.approve_as_left(job.id(), &approval).await.unwrap();
    job.id().clone()
}

/// The Job is on the delivering step, where the pull request opened.
async fn at_the_delivering_step(fleet: &Fixture, home: &TempDir, added: AddStep) -> JobId {
    let id = approved_with(fleet, home, added).await;
    started(fleet, &id).await.unwrap();
    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    id
}

async fn the_addition(fleet: &Fixture, job: &JobId) -> core_model::AddedStep {
    let held = fleet.store().lock().await.job_additions(job).unwrap();
    assert_eq!(held.len(), 1, "one addition: {held:?}");
    held.into_iter().next().unwrap()
}

fn state_of(added: &core_model::AddedStep) -> TriggerState {
    added.fired.as_ref().expect("fired").state
}

async fn the_alert(fleet: &Fixture, id: &JobId) -> Option<ipc::JobAlert> {
    fleet
        .published(&fleet.load(id).await.unwrap())
        .await
        .unwrap()
        .alert
}

async fn at_the_gate(fleet: &Fixture, id: &JobId) {
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    assert_eq!(
        fleet.load(id).await.unwrap().status(),
        JobStatus::AwaitingReview
    );
}

async fn landed(fleet: &Fixture, id: &JobId) {
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    assert_eq!(
        fleet.load(id).await.unwrap().status(),
        JobStatus::CompletedSuccess,
        "asking held nothing"
    );
}

#[tokio::test]
async fn an_added_destructive_script_rings_the_row_and_names_itself_on_the_alerts() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet_asking(&home, &files, &flag_in(&home));
    let added = wipe(TriggerMoment::PrOpened, "summarise", false, false);
    let id = at_the_delivering_step(&fleet, &home, added).await;

    assert_eq!(
        state_of(&the_addition(&fleet, &id).await),
        TriggerState::AwaitingOwner
    );
    let detail = fleet.job_detail(ipc::JobId::from(&id)).await.unwrap();
    assert_eq!(detail.additions[0].state, Wire::AwaitingOwner);
    let alert = the_alert(&fleet, &id).await.expect("the bell rings");
    assert_eq!(
        (alert.kind, alert.trigger.as_str()),
        (JobAlertKind::Asks, "wipe")
    );

    let alerts = fleet.alerts(None).await.unwrap();
    let [one] = alerts.waiting.as_slice() else {
        panic!("one alert: {alerts:?}");
    };
    assert!(one.why.as_deref().is_some_and(|why| why.contains("wipe")));
    assert!(
        fleet.holds_on(&id).await.unwrap().is_empty(),
        "no block, no hold"
    );
}

#[tokio::test]
async fn run_executes_the_command_once_and_the_step_passes_and_the_bell_goes() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    std::fs::write(&flag, "").unwrap();
    let files = Arc::new(Files::default());
    let fleet = a_fleet_asking(&home, &files, &flag);
    let added = wipe(TriggerMoment::PrOpened, "summarise", false, false);
    let id = at_the_delivering_step(&fleet, &home, added).await;
    landed(&fleet, &id).await;

    let ran = Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act())
        .await
        .expect("ran");
    assert_eq!(ran.state, Wire::Passed);
    let after = the_addition(&fleet, &id).await;
    assert_eq!(state_of(&after), TriggerState::Passed);
    assert_eq!(after.fired.and_then(|fired| fired.exit_code), Some(0));
    assert!(fleet.alerts(None).await.unwrap().waiting.is_empty());
}

#[tokio::test]
async fn a_run_that_fails_ends_as_a_step_does_failed_without_block_and_repairing_with_repair() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet_asking(&home, &files, &flag_in(&home));
    let added = wipe(TriggerMoment::PrOpened, "summarise", false, false);
    let id = at_the_delivering_step(&fleet, &home, added).await;
    landed(&fleet, &id).await;
    let ran = Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act())
        .await
        .expect("ran");
    assert_eq!(ran.state, Wire::Failed);
    assert_eq!(
        the_addition(&fleet, &id).await.fired.unwrap().exit_code,
        Some(1)
    );

    let home = TempDir::new();
    let fleet = a_fleet_asking(&home, &files, &flag_in(&home));
    let added = wipe(TriggerMoment::PrOpened, "summarise", false, true);
    let id = at_the_delivering_step(&fleet, &home, added).await;
    landed(&fleet, &id).await;
    let ran = Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act())
        .await
        .expect("ran");
    assert_eq!(ran.state, Wire::Repairing);
    let queued = fleet
        .trigger_repairs()
        .lock()
        .unwrap()
        .pop()
        .expect("queued");
    assert_eq!(queued.subject, Subject::Addition("a1".to_string()));
}

#[tokio::test]
async fn with_block_the_gate_is_held_until_he_answers_and_a_failed_run_keeps_it_held() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    let files = Arc::new(Files::default());
    let fleet = a_fleet_asking(&home, &files, &flag);
    let added = wipe(TriggerMoment::PrOpened, "summarise", true, false);
    let id = at_the_delivering_step(&fleet, &home, added).await;
    at_the_gate(&fleet, &id).await;

    let held = fleet.approved(&id, Actor::Human).await.expect_err("asking");
    assert!(matches!(held, Adrift::TriggerHolds { .. }), "{held:?}");

    let wire = ipc::JobId::from(&id);
    let failed = Arc::clone(&fleet)
        .hold_rerun(wire.clone(), act())
        .await
        .expect("ran");
    assert_eq!((failed.state, failed.released), (Wire::Held, false));
    let kind = the_alert(&fleet, &id).await.map(|alert| alert.kind);
    assert_eq!(kind, Some(JobAlertKind::Held));

    std::fs::write(&flag, "").unwrap();
    let passed = Arc::clone(&fleet)
        .hold_rerun(wire, act())
        .await
        .expect("ran");
    assert_eq!((passed.state, passed.released), (Wire::Passed, true));
    assert!(the_alert(&fleet, &id).await.is_none());
    fleet
        .approved(&id, Actor::Human)
        .await
        .expect("the gate is open");
}

#[tokio::test]
async fn skip_records_it_skipped_by_the_owner_and_opens_the_gate() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet_asking(&home, &files, &flag_in(&home));
    let added = wipe(TriggerMoment::PrOpened, "summarise", true, false);
    let id = at_the_delivering_step(&fleet, &home, added).await;
    at_the_gate(&fleet, &id).await;

    let skipped = fleet
        .hold_skip(ipc::JobId::from(&id), act())
        .await
        .expect("skipped");
    assert_eq!((skipped.state, skipped.released), (Wire::Skipped, true));
    let after = the_addition(&fleet, &id).await;
    assert_eq!(
        after.fired.and_then(|fired| fired.not_run),
        Some(NotRun::ByOwner)
    );
    assert!(the_alert(&fleet, &id).await.is_none());
    fleet.approved(&id, Actor::Human).await.expect("open");
}

#[tokio::test]
async fn a_step_starts_ask_with_block_stops_the_job_before_its_drone_until_he_runs_it() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    std::fs::write(&flag, "").unwrap();
    let files = Arc::new(Files::default());
    let fleet = a_fleet_asking(&home, &files, &flag);
    let added = wipe(TriggerMoment::StepStarts, "implement", true, false);
    let id = approved_with(&fleet, &home, added).await;

    let held = started(&fleet, &id).await.expect_err("asking first");
    assert!(matches!(held, Adrift::TriggerHolds { .. }), "{held:?}");
    assert_eq!(
        fleet.load(&id).await.unwrap().status(),
        JobStatus::Escalated
    );
    assert!(fleet.harness().configured().is_empty(), "no Drone yet");
    let blocked = fleet.alerts(None).await.unwrap().blocked;
    assert!(
        blocked
            .iter()
            .any(|one| one.why.as_deref().is_some_and(|why| why.contains("wipe"))),
        "{blocked:?}"
    );

    let ran = Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act())
        .await
        .expect("ran");
    assert_eq!((ran.state, ran.released), (Wire::Passed, true));
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Queued);
    started(&fleet, &id).await.unwrap();
    assert_eq!(
        state_of(&the_addition(&fleet, &id).await),
        TriggerState::Passed,
        "not asked a second time on the way in"
    );
}

#[tokio::test]
async fn a_second_run_while_it_executes_is_refused_and_a_restart_leaves_it_asking() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    std::fs::write(&flag, "").unwrap();
    let files = Arc::new(Files::default());
    let fleet = a_fleet_asking(&home, &files, &flag);
    let added = wipe(TriggerMoment::PrOpened, "summarise", true, false);
    let id = at_the_delivering_step(&fleet, &home, added).await;
    at_the_gate(&fleet, &id).await;

    // What is written is all a restarted Fleet has: asking, and holding.
    assert_eq!(
        state_of(&the_addition(&fleet, &id).await),
        TriggerState::AwaitingOwner
    );
    assert_eq!(fleet.holds_on(&id).await.unwrap().len(), 1);

    fleet
        .owner_runs()
        .lock()
        .unwrap()
        .insert(Subject::Addition("a1".to_string()));
    let again = Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act())
        .await
        .expect_err("already running");
    assert_eq!(
        (again.status(), again.error().code.as_str()),
        (409, "fleet.hold_already_running")
    );
    fleet.owner_runs().lock().unwrap().clear();
    assert_eq!(
        state_of(&the_addition(&fleet, &id).await),
        TriggerState::AwaitingOwner,
        "refusing left it asking"
    );
    Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act())
        .await
        .expect("ran");
    assert!(fleet.owner_runs().lock().unwrap().is_empty());
}

#[tokio::test]
async fn nothing_that_is_not_asking_answers_to_run_or_skip() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet_asking(&home, &files, &flag_in(&home));
    let added = wipe(TriggerMoment::PrOpened, "summarise", false, false);
    let id = approved_with(&fleet, &home, added).await;
    let none = fleet
        .hold_skip(ipc::JobId::from(&id), act())
        .await
        .expect_err("nothing asks yet");
    assert_eq!(
        (none.status(), none.error().code.as_str()),
        (409, "fleet.no_hold")
    );
}
