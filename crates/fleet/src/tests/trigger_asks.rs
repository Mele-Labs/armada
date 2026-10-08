//! A Trigger on a destructive Command asks the owner before it runs: the Board
//! row rings, `list_alerts` names it, and with `block` on the Job holds until
//! he answers. His Run is the Command's one firing; his Skip records it
//! skipped. Commands are real programs; the Drone and the pull request are
//! fakes.

use std::sync::Arc;

use config::Manifest;
use core_model::{Actor, JobId, JobStatus, TriggerSkipped, TriggerState};
use ipc::{HoldAct, JobAlertKind, TriggerFiringState as Wire};
use testkit::{Delivering, FakeHarness, FakeVcs, FakeWorkProduct};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::tests::admitted::started;
use crate::tests::daemon::{a_proposal, note_evidence, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::tests::triggering::{a_fleet, machine, to_the_delivering_step, Files};

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn wiping(moment: &str, on_failure: &str) -> config::TriggerWritten {
    machine(
        "wipe.yml",
        &format!("name: wipe\nwhen: {moment}\ncommand: wipe\n{on_failure}"),
    )
}

const BLOCKING: &str = "on_failure:\n  block: true\n";

/// `wipe` is destructive and passes once `flag` exists.
fn a_fleet_asking(home: &TempDir, files: &Arc<Files>, flag: &str) -> Arc<Fixture> {
    let text = format!(
        "version: 1\nid: 01FIXTUREMANIFEST\ncommands:\n  wipe:\n    run: \"test -f {flag}\"\n    destructive: true\n"
    );
    let manifest = Manifest::parse(std::path::Path::new("armada.yml"), &text).expect("a Manifest");
    Arc::new(a_fleet(home, files, manifest, Delivering::default()))
}

fn flag_in(home: &TempDir) -> String {
    format!("{}/flag", home.path().display())
}

fn act() -> HoldAct {
    HoldAct {
        trigger: Some("wipe".to_string()),
        addition: None,
    }
}

async fn the_firing(fleet: &Fixture, job: &JobId) -> core_model::TriggerFiring {
    let held = fleet.store().lock().await.trigger_firings(job).unwrap();
    assert_eq!(held.len(), 1, "one firing: {held:?}");
    held.into_iter().next().unwrap()
}

async fn the_alert(fleet: &Fixture, id: &JobId) -> Option<ipc::JobAlert> {
    fleet
        .published(&fleet.load(id).await.unwrap())
        .await
        .unwrap()
        .alert
}

/// The delivering step's Drone has stopped, so the Command can run in its tree.
async fn at_the_gate(fleet: &Fixture, id: &JobId) {
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    assert_eq!(
        fleet.load(id).await.unwrap().status(),
        JobStatus::AwaitingReview
    );
}

/// Where nothing blocks, the Job goes on to land and the Command is run after.
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
async fn a_destructive_trigger_rings_the_row_and_names_itself_on_the_alerts() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![wiping("pr_opened", "")]);
    let fleet = a_fleet_asking(&home, &files, &flag_in(&home));
    let id = to_the_delivering_step(&fleet, &home).await;

    assert_eq!(
        the_firing(&fleet, &id).await.state,
        TriggerState::AwaitingOwner
    );
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
async fn run_executes_the_command_once_and_the_trigger_passes_and_the_bell_goes() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    std::fs::write(&flag, "").unwrap();
    let files = Arc::new(Files::default());
    files.say(vec![wiping("pr_opened", "")]);
    let fleet = a_fleet_asking(&home, &files, &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    landed(&fleet, &id).await;

    let ran = Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act())
        .await
        .expect("ran");
    assert_eq!(ran.state, Wire::Passed);
    let after = the_firing(&fleet, &id).await;
    assert_eq!(
        (after.state, after.exit_code),
        (TriggerState::Passed, Some(0))
    );
    assert!(fleet.alerts(None).await.unwrap().waiting.is_empty());
}

#[tokio::test]
async fn a_run_that_fails_ends_as_a_firing_does_failed_without_block() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![wiping("pr_opened", "")]);
    let fleet = a_fleet_asking(&home, &files, &flag_in(&home));
    let id = to_the_delivering_step(&fleet, &home).await;
    landed(&fleet, &id).await;

    let ran = Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act())
        .await
        .expect("ran");
    assert_eq!(ran.state, Wire::Failed);
    assert_eq!(the_firing(&fleet, &id).await.exit_code, Some(1));
}

#[tokio::test]
async fn with_block_the_gate_is_held_until_he_answers_and_a_failed_run_keeps_it_held() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    let files = Arc::new(Files::default());
    files.say(vec![wiping("pr_opened", BLOCKING)]);
    let fleet = a_fleet_asking(&home, &files, &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
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
    assert_eq!(
        kind,
        Some(JobAlertKind::Held),
        "it failed: the hold is the more pressing"
    );

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
    files.say(vec![wiping("pr_opened", BLOCKING)]);
    let fleet = a_fleet_asking(&home, &files, &flag_in(&home));
    let id = to_the_delivering_step(&fleet, &home).await;
    at_the_gate(&fleet, &id).await;

    let skipped = fleet
        .hold_skip(ipc::JobId::from(&id), act())
        .await
        .expect("skipped");
    assert_eq!((skipped.state, skipped.released), (Wire::Skipped, true));
    let after = the_firing(&fleet, &id).await;
    assert_eq!(after.skipped, Some(TriggerSkipped::ByOwner));
    assert!(the_alert(&fleet, &id).await.is_none());
    fleet.approved(&id, Actor::Human).await.expect("open");
}

#[tokio::test]
async fn a_step_starts_ask_with_block_stops_the_job_before_its_drone_until_he_runs_it() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    std::fs::write(&flag, "").unwrap();
    let files = Arc::new(Files::default());
    files.say(vec![wiping("step_starts\nstep: implement", BLOCKING)]);
    let fleet = a_fleet_asking(&home, &files, &flag);
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(&home, &job);
    fleet.approve(job.id()).await.unwrap();
    let id = job.id().clone();

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
        fleet
            .store()
            .lock()
            .await
            .trigger_firings(&id)
            .unwrap()
            .len(),
        1,
        "not fired a second time on the way in"
    );
}

#[tokio::test]
async fn a_second_run_while_it_executes_is_refused_and_a_restart_leaves_it_asking() {
    let home = TempDir::new();
    let flag = flag_in(&home);
    std::fs::write(&flag, "").unwrap();
    let files = Arc::new(Files::default());
    files.say(vec![wiping("pr_opened", BLOCKING)]);
    let fleet = a_fleet_asking(&home, &files, &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    at_the_gate(&fleet, &id).await;

    // What is written is all a restarted Fleet has: asking, and holding.
    let (firing_id, asking) = fleet
        .store()
        .lock()
        .await
        .asking_firings(&id)
        .unwrap()
        .remove(0);
    assert_eq!(asking.state, TriggerState::AwaitingOwner);
    assert_eq!(fleet.holds_on(&id).await.unwrap().len(), 1);

    fleet.owner_runs().lock().unwrap().insert(firing_id);
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
        the_firing(&fleet, &id).await.state,
        TriggerState::AwaitingOwner
    );
    Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act())
        .await
        .expect("ran");
    assert!(fleet.owner_runs().lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_run_that_fails_with_repair_on_queues_a_repair_a_restart_takes_up_again() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    files.say(vec![wiping("pr_opened", "on_failure:\n  repair: true\n")]);
    let fleet = a_fleet_asking(&home, &files, &flag_in(&home));
    let id = to_the_delivering_step(&fleet, &home).await;
    landed(&fleet, &id).await;

    let ran = Arc::clone(&fleet)
        .hold_rerun(ipc::JobId::from(&id), act())
        .await
        .expect("ran");
    assert_eq!(ran.state, Wire::Repairing);
    fleet.trigger_repairs().lock().unwrap().pop();
    fleet.repairs_recovered().await;
    assert!(
        fleet.repair_next().await,
        "the repair was queued again from the store"
    );
}

#[tokio::test]
async fn nothing_that_is_not_asking_answers_to_run_or_skip() {
    let home = TempDir::new();
    let files = Arc::new(Files::default());
    let fleet = a_fleet_asking(&home, &files, &flag_in(&home));
    let id = to_the_delivering_step(&fleet, &home).await;
    let none = fleet
        .hold_skip(ipc::JobId::from(&id), act())
        .await
        .expect_err("nothing asks");
    assert_eq!(
        (none.status(), none.error().code.as_str()),
        (409, "fleet.no_hold")
    );
}
