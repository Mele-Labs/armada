//! A step added to one Job that fails with `repair` on goes through the repair
//! a saved Trigger does: a repair Drone on a branch of its own, the Command run
//! again there, the fix held for the owner, and the branch given back once the
//! fix is placed or the repair fails. Commands are real programs and the Drone
//! is a shell; the pull request and the push are `FakeVcs`.

use std::sync::Arc;

use core_model::{FixChoice, JobId, JobStatus, TriggerState};
use ipc::{TriggerFiringState as Wire, TriggerFixChoice};
use testkit::{Delivered, Delivering, FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::placing_a_fix::FixNotChosen;
use crate::tests::admitted::started;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fitted_over, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::tests::trigger_repair::harness_that_repairs_by;
use crate::tests::triggering::{manifest, Files};

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn a_fleet(home: &TempDir, drone: &str, flag: &str) -> Arc<Fixture> {
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        harness_that_repairs_by(drone),
        FakeVcs::new().delivering(Delivering::default()),
    );
    fittings.starting().manifest = manifest(Some(&format!("test -f {flag}")));
    fittings.locating = Arc::new(Arc::new(Files::default()));
    Arc::new(Fleet::assembled(fittings))
}

/// `deploy_qa` added after `implement`, which fails until the repair makes
/// `flag`. The Job is then on its second step with a Drone working.
async fn with_a_failed_addition(fleet: &Fixture, home: &TempDir, block: bool) -> JobId {
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(home, &job);
    let added = ipc::AddStep {
        runs: ipc::AddedRuns::Script {
            command: "deploy_qa".into(),
        },
        when: ipc::TriggerMoment::StepPasses,
        step: ipc::StepId::carried("implement"),
        block,
        repair: true,
    };
    fleet
        .approve_as_left(
            job.id(),
            &ipc::ApproveDispatch {
                additions: Some(vec![added]),
                ..ipc::ApproveDispatch::default()
            },
        )
        .await
        .unwrap();
    started(fleet, job.id()).await.unwrap();
    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    // A step that blocks stops the Job before the next Drone, which is an Err.
    let _ = fleet.turn().await;
    job.id().clone()
}

async fn addition(fleet: &Fixture, job: &JobId) -> core_model::AddedStep {
    let held = fleet.store().lock().await.job_additions(job).unwrap();
    assert_eq!(held.len(), 1, "one addition: {held:?}");
    held.into_iter().next().unwrap()
}

fn state_of(added: &core_model::AddedStep) -> TriggerState {
    added.fired.as_ref().expect("fired").state
}

async fn the_job_finishes(fleet: &Fixture) {
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
}

#[tokio::test]
async fn a_failed_added_step_is_repaired_on_its_own_branch_and_the_fix_waits_for_a_choice() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let fleet = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = with_a_failed_addition(&fleet, &home, false).await;

    // The failure queued a repair and moved nothing.
    assert_eq!(
        state_of(&addition(&fleet, &id).await),
        TriggerState::Repairing
    );
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Running);
    let detail = fleet.job_detail(ipc::JobId::from(&id)).await.unwrap();
    assert_eq!(detail.additions[0].state, Wire::Repairing);

    assert!(fleet.repair_next().await);
    assert!(!fleet.repair_next().await, "nothing else was waiting");

    let held = addition(&fleet, &id).await;
    assert_eq!(state_of(&held), TriggerState::FixReady);
    assert_eq!(held.repair.tries, 1);
    assert_eq!(held.repair.choice, None, "Fleet does not choose");
    let job = fleet.load(&id).await.unwrap();
    let branch = held.repair.branch.clone().expect("a repair branch");
    assert_ne!(Some(branch.as_str()), job.branch().map(|b| b.as_str()));
    assert_eq!(fleet.vcs().parked_slots().len(), 1, "the fix holds no bay");
    assert!(
        fleet.vcs().deleted_branches().is_empty(),
        "the fix is not placed yet"
    );
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Running);

    let wire = fleet.job_detail(ipc::JobId::from(&id)).await.unwrap();
    let shown = wire.additions[0].repair_record.clone().expect("a repair");
    assert_eq!(
        (shown.attempt, shown.branch.as_deref()),
        (1, Some(branch.as_str()))
    );
    assert_eq!(wire.additions[0].state, Wire::FixReady);

    let alerts = fleet.alerts(None).await.unwrap();
    assert!(
        alerts
            .waiting
            .iter()
            .any(|one| one.why.as_deref().is_some_and(|why| why.contains("fix"))),
        "a fix waiting on his choice is an alert: {alerts:?}"
    );
}

#[tokio::test]
async fn choosing_this_branch_places_the_fix_and_the_repair_branch_is_gone() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let fleet = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = with_a_failed_addition(&fleet, &home, false).await;
    assert!(fleet.repair_next().await);
    let job_branch = fleet
        .load(&id)
        .await
        .unwrap()
        .branch()
        .unwrap()
        .as_str()
        .to_string();
    let repair_branch = addition(&fleet, &id).await.repair.branch.unwrap();

    // A Drone is working on the Job's branch, so the choice is kept.
    let chosen = fleet
        .choose_addition_fix(&id, "a1", FixChoice::ThisBranch)
        .await
        .expect("accepted");
    assert_eq!(chosen.state, TriggerState::FixReady);
    assert_eq!(
        addition(&fleet, &id).await.repair.choice,
        Some(FixChoice::ThisBranch)
    );
    assert!(fleet.vcs().deleted_branches().is_empty(), "not merged yet");

    the_job_finishes(&fleet).await;
    fleet.chosen_fixes_retried().await;

    let placed = addition(&fleet, &id).await;
    assert_eq!(state_of(&placed), TriggerState::Passed);
    let delivered = fleet.vcs().delivered();
    assert!(delivered.contains(&Delivered::MergedBranch {
        branch: repair_branch.clone(),
        into: job_branch.clone(),
    }));
    assert!(delivered.contains(&Delivered::Pushed { branch: job_branch }));
    assert_eq!(
        fleet.vcs().deleted_branches(),
        [repair_branch],
        "the branch the fix came from is given back"
    );
    assert!(matches!(
        fleet.choose_addition_fix(&id, "a1", FixChoice::NewPr).await,
        Err(FixNotChosen::NothingWaiting { .. })
    ));
}

#[tokio::test]
async fn a_new_pr_keeps_the_branch_because_it_is_the_pull_requests_head() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let fleet = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = with_a_failed_addition(&fleet, &home, false).await;
    assert!(fleet.repair_next().await);
    let repair_branch = addition(&fleet, &id).await.repair.branch.unwrap();

    let chosen = fleet
        .choose_addition_fix(&id, "a1", FixChoice::NewPr)
        .await
        .expect("chosen");

    assert_eq!(chosen.state, TriggerState::Passed);
    assert!(chosen.pull_request.is_some());
    assert_eq!(state_of(&addition(&fleet, &id).await), TriggerState::Passed);
    assert!(fleet.vcs().delivered().contains(&Delivered::Pushed {
        branch: repair_branch
    }));
    assert!(fleet.vcs().deleted_branches().is_empty());
    fleet.repairs_recovered().await;
    assert!(
        fleet.vcs().deleted_branches().is_empty(),
        "nor does the sweep at start take it"
    );
}

#[tokio::test]
async fn two_repairs_that_do_not_fix_it_fail_the_step_alert_the_job_and_give_the_branch_back() {
    let home = TempDir::new();
    let flag = format!("{}/never", home.path().display());
    let fleet = a_fleet(&home, "true", &flag);
    let id = with_a_failed_addition(&fleet, &home, false).await;

    assert!(fleet.repair_next().await);

    let failed = addition(&fleet, &id).await;
    assert_eq!(state_of(&failed), TriggerState::Failed);
    assert_eq!(failed.repair.tries, core_model::REPAIR_TRIES);
    assert_eq!(
        fleet.vcs().deleted_branches(),
        [failed.repair.branch.clone().unwrap()]
    );
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Running);
    let alerts = fleet.alerts(None).await.expect("alerts");
    let [one] = alerts.waiting.as_slice() else {
        panic!("one alert: {alerts:?}");
    };
    assert!(one
        .why
        .as_deref()
        .is_some_and(|why| why.contains("deploy_qa")));
    let row = fleet
        .job_detail(ipc::JobId::from(&id))
        .await
        .unwrap()
        .additions
        .remove(0);
    assert_eq!(row.state, Wire::Failed);

    // A Fleet that stopped between the end and the delete finishes it at start.
    fleet.repairs_recovered().await;
    assert_eq!(fleet.vcs().deleted_branches().len(), 2);
}

#[tokio::test]
async fn a_step_that_blocks_holds_the_job_through_the_repair_and_after_two_failed_tries() {
    let home = TempDir::new();
    let flag = format!("{}/never", home.path().display());
    let fleet = a_fleet(&home, "true", &flag);
    let id = with_a_failed_addition(&fleet, &home, true).await;

    assert_eq!(
        state_of(&addition(&fleet, &id).await),
        TriggerState::Repairing
    );
    assert!(
        fleet.is_held(&id).await,
        "the hold waits through the repair"
    );

    assert!(fleet.repair_next().await);

    let held = addition(&fleet, &id).await;
    assert_eq!(
        state_of(&held),
        TriggerState::Held,
        "a repair that did not fix it still holds"
    );
    assert!(fleet.is_held(&id).await);
    assert_eq!(
        fleet.vcs().deleted_branches(),
        [held.repair.branch.unwrap()]
    );
}

#[tokio::test]
async fn the_wire_names_the_trigger_or_the_added_step_and_never_both() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let fleet = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = with_a_failed_addition(&fleet, &home, false).await;
    assert!(fleet.repair_next().await);
    let job = ipc::JobId::from(&id);
    let ask = |trigger: Option<&str>, addition: Option<&str>| ipc::ChooseTriggerFix {
        trigger: trigger.map(str::to_string),
        addition: addition.map(str::to_string),
        choice: TriggerFixChoice::NewPr,
    };

    for both in [ask(Some("x"), Some("a1")), ask(None, None)] {
        let refused = Arc::clone(&fleet)
            .fix_chosen(job.clone(), both)
            .await
            .expect_err("one of the two");
        assert_eq!(refused.error().code.as_str(), "fleet.no_fix_named");
    }
    let refused = Arc::clone(&fleet)
        .fix_chosen(job.clone(), ask(None, Some("a9")))
        .await
        .expect_err("no such step");
    assert_eq!(refused.error().code.as_str(), "fleet.no_fix_waiting");

    let placed = Arc::clone(&fleet)
        .fix_chosen(job.clone(), ask(None, Some("a1")))
        .await
        .expect("placed");
    assert_eq!(placed.state, Wire::Passed);
    let row = fleet.job_detail(job).await.unwrap().additions.remove(0);
    assert_eq!(
        row.repair_record.and_then(|repair| repair.choice),
        Some(TriggerFixChoice::NewPr)
    );
}

#[tokio::test]
async fn a_repair_a_restart_forgot_is_queued_again_from_the_store_for_an_added_step() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let fleet = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = with_a_failed_addition(&fleet, &home, false).await;
    // The process dies: what was queued in memory is gone, and the row says `repairing`.
    fleet.trigger_repairs().lock().unwrap().pop();
    assert!(!fleet.repair_next().await);
    assert_eq!(
        state_of(&addition(&fleet, &id).await),
        TriggerState::Repairing
    );

    fleet.repairs_recovered().await;
    assert!(fleet.repair_next().await);

    let held = addition(&fleet, &id).await;
    assert_eq!(
        (state_of(&held), held.repair.tries),
        (TriggerState::FixReady, 1)
    );
}
