//! A Trigger with `repair` on that fails gets a repair Drone on a branch of its
//! own, the Command run again there, and the fix held for the owner. The
//! Job's status and step never move. Commands are real programs and the Drone
//! is a shell; the pull request and the push are `FakeVcs`.

use std::sync::Arc;

use adapter_traits::Review;
use core_model::{FixChoice, JobId, JobStatus, TriggerState};
use testkit::{Delivered, Delivering, FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::placing_a_fix::FixNotChosen;
use crate::tests::daemon::fitted_over;
use crate::tests::tmp::TempDir;
use crate::tests::triggering::{machine, manifest, to_the_delivering_step, Files};

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const REPAIRING_DEPLOY: &str =
    "name: deploy\nwhen: pr_opened\ncommand: deploy_qa\non_failure:\n  repair: true\n";

/// A Drone told to repair runs `drone`; any other is a step's and waits.
fn harness_that_repairs_by(drone: &str) -> FakeHarness {
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            &format!(
                "IFS= read -r line; case \"$line\" in *\"REPAIR THE TRIGGER\"*) {drone};; *) sleep 30;; esac"
            ),
        ],
    )
}

/// `deploy_qa` passes once `flag` exists, so it fails until a repair makes it.
fn a_fleet(home: &TempDir, drone: &str, flag: &str) -> (Fixture, Arc<Files>) {
    let files = Arc::new(Files::default());
    files.say(vec![machine("deploy.yml", REPAIRING_DEPLOY)]);
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        harness_that_repairs_by(drone),
        FakeVcs::new().delivering(Delivering::default()),
    );
    fittings.starting().manifest = manifest(Some(&format!("test -f {flag}")));
    fittings.locating = Arc::new(Arc::clone(&files));
    (Fleet::assembled(fittings), files)
}

async fn firing(fleet: &Fixture, job: &JobId) -> core_model::TriggerFiring {
    let held = fleet.store().lock().await.trigger_firings(job).unwrap();
    assert_eq!(held.len(), 1, "one firing: {held:?}");
    held.into_iter().next().unwrap()
}

async fn the_job_has_not_moved(fleet: &Fixture, job: &JobId) {
    let job = fleet.load(job).await.unwrap();
    assert_eq!(job.status(), JobStatus::Running);
    assert_eq!(job.current_step_id().map(|s| s.as_str()), Some("summarise"));
}

#[tokio::test]
async fn a_failed_trigger_is_repaired_on_its_own_branch_and_the_fix_waits_for_a_choice() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let (fleet, _files) = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = to_the_delivering_step(&fleet, &home).await;

    // The failure queued a repair and moved nothing.
    let waiting = firing(&fleet, &id).await;
    assert_eq!(waiting.state, TriggerState::Repairing);
    assert_eq!(waiting.exit_code, Some(1));
    the_job_has_not_moved(&fleet, &id).await;

    assert!(fleet.repair_next().await);
    assert!(!fleet.repair_next().await, "nothing else was waiting");

    let held = firing(&fleet, &id).await;
    assert_eq!(held.state, TriggerState::FixReady);
    assert_eq!(held.repair.tries, 1);
    assert_eq!(held.repair.choice, None, "Fleet does not choose");
    let job = fleet.load(&id).await.unwrap();
    let repair_branch = held.repair.branch.clone().expect("a repair branch");
    assert_ne!(Some(repair_branch.as_str()), job.branch().map(|b| b.as_str()));
    assert!(
        fleet.vcs().cut_from().contains(&job.branch().unwrap().as_str().to_string()),
        "cut from the Job's branch, not the base"
    );
    assert_eq!(
        fleet.harness().configured().last().map(|config| config
            .toolbelt()
            .granted()
            .contains(&adapter_traits::Grant::ChangeTheWorktree)),
        Some(true),
        "unlike the worktree's repair, this one writes"
    );
    assert_eq!(fleet.vcs().released_slots().len(), 0);
    assert_eq!(fleet.vcs().parked_slots().len(), 1, "the fix holds no bay");
    the_job_has_not_moved(&fleet, &id).await;
}

#[tokio::test]
async fn choosing_a_new_pr_opens_one_from_the_repair_branch_and_passes_the_trigger() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let (fleet, _files) = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(fleet.repair_next().await);
    let repair_branch = firing(&fleet, &id).await.repair.branch.unwrap();

    let chosen = fleet
        .choose_trigger_fix(&id, "deploy", FixChoice::NewPr)
        .await
        .expect("chosen");

    assert_eq!(chosen.state, TriggerState::Passed);
    let url = chosen.pull_request.expect("a pull request");
    let held = firing(&fleet, &id).await;
    assert_eq!(held.state, TriggerState::Passed);
    assert_eq!(held.repair.choice, Some(FixChoice::NewPr));
    assert_eq!(held.repair.pull_request.as_deref(), Some(url.as_str()));
    let delivered = fleet.vcs().delivered();
    assert!(delivered.contains(&Delivered::Pushed { branch: repair_branch }));
    let opened: Vec<&Review> = delivered
        .iter()
        .filter_map(|one| match one {
            Delivered::OpenedForReview { review, .. } => Some(review),
            _ => None,
        })
        .collect();
    assert_eq!(opened.len(), 2, "the Job's own, then the repair's: {opened:?}");
    assert!(opened[1].title().starts_with("Repair `deploy`"));
    the_job_has_not_moved(&fleet, &id).await;
    // Chosen once.
    assert!(matches!(
        fleet.choose_trigger_fix(&id, "deploy", FixChoice::NewPr).await,
        Err(FixNotChosen::NothingWaiting { .. })
    ));
}

#[tokio::test]
async fn choosing_this_branch_merges_the_fix_onto_the_jobs_branch_pushes_and_runs_it_again() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let (fleet, _files) = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(fleet.repair_next().await);
    let job_branch = fleet.load(&id).await.unwrap().branch().unwrap().as_str().to_string();
    let repair_branch = firing(&fleet, &id).await.repair.branch.unwrap();

    let chosen = fleet
        .choose_trigger_fix(&id, "deploy", FixChoice::ThisBranch)
        .await
        .expect("chosen");

    assert_eq!(chosen.state, TriggerState::Passed);
    assert_eq!(chosen.pull_request, None);
    let delivered = fleet.vcs().delivered();
    assert!(delivered.contains(&Delivered::MergedBranch {
        branch: repair_branch,
        into: job_branch.clone(),
    }));
    assert!(delivered.contains(&Delivered::Pushed { branch: job_branch }));
    let opened = delivered
        .iter()
        .filter(|one| matches!(one, Delivered::OpenedForReview { .. }))
        .count();
    assert_eq!(opened, 1, "no pull request of its own");
    assert_eq!(firing(&fleet, &id).await.state, TriggerState::Passed);
    the_job_has_not_moved(&fleet, &id).await;
}

#[tokio::test]
async fn two_repairs_that_do_not_fix_it_fail_the_trigger_and_alert_the_job() {
    let home = TempDir::new();
    let flag = format!("{}/never", home.path().display());
    let (fleet, _files) = a_fleet(&home, "true", &flag);
    let id = to_the_delivering_step(&fleet, &home).await;

    assert!(fleet.repair_next().await);

    let held = firing(&fleet, &id).await;
    assert_eq!(held.state, TriggerState::Failed);
    assert_eq!(held.repair.tries, core_model::REPAIR_TRIES);
    assert!(held.ended_at.is_some());
    assert_eq!(
        fleet.harness().configured().len(),
        2 + core_model::REPAIR_TRIES as usize,
        "a Drone for each of the two steps, and two repair Drones"
    );
    the_job_has_not_moved(&fleet, &id).await;
    let alerts = fleet.alerts(None).await.expect("alerts");
    let [one] = alerts.waiting.as_slice() else {
        panic!("one alert: {alerts:?}");
    };
    assert_eq!(one.status, "running");
    assert!(one.why.as_deref().is_some_and(|why| why.contains("deploy")));
    assert!(matches!(
        fleet.choose_trigger_fix(&id, "deploy", FixChoice::NewPr).await,
        Err(FixNotChosen::NothingWaiting { .. })
    ));
}

#[tokio::test]
async fn a_trigger_without_repair_on_is_never_queued() {
    let home = TempDir::new();
    let flag = format!("{}/never", home.path().display());
    let (fleet, files) = a_fleet(&home, "true", &flag);
    files.say(vec![machine(
        "deploy.yml",
        "name: deploy\nwhen: pr_opened\ncommand: deploy_qa\n",
    )]);
    let id = to_the_delivering_step(&fleet, &home).await;

    assert_eq!(firing(&fleet, &id).await.state, TriggerState::Failed);
    assert!(!fleet.repair_next().await);
    assert!(fleet.alerts(None).await.unwrap().waiting.is_empty());
}
