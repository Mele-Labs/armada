//! A Trigger with `repair` on that fails gets a repair Drone on a branch of its
//! own, the Command run again there, and the fix held for the owner. The
//! Job's status and step never move. Commands are real programs and the Drone
//! is a shell; the pull request and the push are `FakeVcs`.

use std::sync::Arc;

use adapter_traits::{BranchMerged, Review, Vcs};
use core_model::{FixChoice, JobId, JobStatus, TriggerState};
use testkit::{Delivered, Delivering, FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::placing_a_fix::FixNotChosen;
use crate::tests::daemon::{fitted_over, note_evidence};
use crate::tests::tools::submitted_by_the_one;
use crate::tests::tmp::TempDir;
use crate::tests::triggering::{machine, manifest, to_the_delivering_step, Files};

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const REPAIRING_DEPLOY: &str =
    "name: deploy\nwhen: pr_opened\ncommand: deploy_qa\non_failure:\n  repair: true\n";

/// A Drone told to repair runs `drone`; any other is a step's and waits.
pub(crate) fn harness_that_repairs_by(drone: &str) -> FakeHarness {
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
    a_fleet_delivering(home, drone, flag, Delivering::default())
}

fn a_fleet_delivering(
    home: &TempDir,
    drone: &str,
    flag: &str,
    delivering: Delivering,
) -> (Fixture, Arc<Files>) {
    let files = Arc::new(Files::default());
    files.say(vec![machine("deploy.yml", REPAIRING_DEPLOY)]);
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        harness_that_repairs_by(drone),
        FakeVcs::new().delivering(delivering),
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
    assert!(
        fleet.vcs().deleted_branches().is_empty(),
        "the branch is the pull request's head"
    );
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

/// The Job's last step settles, so no Drone is working on its branch.
async fn the_job_finishes(fleet: &Fixture) {
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
}

#[tokio::test]
async fn choosing_this_branch_waits_for_the_step_then_merges_pushes_and_runs_it_again() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let (fleet, _files) = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(fleet.repair_next().await);
    let job_branch = fleet.load(&id).await.unwrap().branch().unwrap().as_str().to_string();
    let repair_branch = firing(&fleet, &id).await.repair.branch.unwrap();

    // A Drone is working on the Job's branch, so the choice is kept and not refused.
    let chosen = fleet
        .choose_trigger_fix(&id, "deploy", FixChoice::ThisBranch)
        .await
        .expect("accepted");
    assert_eq!(chosen.state, TriggerState::FixReady);
    assert_eq!(firing(&fleet, &id).await.repair.choice, Some(FixChoice::ThisBranch));
    assert!(!fleet
        .vcs()
        .delivered()
        .iter()
        .any(|one| matches!(one, Delivered::MergedBranch { .. })));
    assert!(
        fleet.alerts(None).await.unwrap().waiting.is_empty(),
        "he has chosen, so it no longer waits on him"
    );
    fleet.chosen_fixes_retried().await;
    assert_eq!(firing(&fleet, &id).await.state, TriggerState::FixReady, "still working");

    the_job_finishes(&fleet).await;
    fleet.chosen_fixes_retried().await;

    let held = firing(&fleet, &id).await;
    assert_eq!(held.state, TriggerState::Passed);
    let delivered = fleet.vcs().delivered();
    assert!(delivered.contains(&Delivered::MergedBranch {
        branch: repair_branch,
        into: job_branch.clone(),
    }));
    assert!(delivered.contains(&Delivered::Pushed { branch: job_branch }));
    assert_eq!(
        fleet.vcs().deleted_branches(),
        [firing(&fleet, &id).await.repair.branch.unwrap()],
        "the fix is on the Job's branch, so the repair branch is given back"
    );
    let opened = delivered
        .iter()
        .filter(|one| matches!(one, Delivered::OpenedForReview { .. }))
        .count();
    assert_eq!(opened, 1, "no pull request of its own");
}

#[tokio::test]
async fn a_fix_that_no_longer_merges_goes_back_to_the_owner_with_the_choice_cleared() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let delivering = Delivering {
        branch_merge: BranchMerged::PutBack {
            files: vec![String::from("src/log.rs")],
        },
        ..Delivering::default()
    };
    let (fleet, _files) = a_fleet_delivering(&home, &format!("touch {flag}"), &flag, delivering);
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(fleet.repair_next().await);
    fleet
        .choose_trigger_fix(&id, "deploy", FixChoice::ThisBranch)
        .await
        .expect("accepted");
    the_job_finishes(&fleet).await;

    fleet.chosen_fixes_retried().await;

    let held = firing(&fleet, &id).await;
    assert_eq!((held.state, held.repair.choice), (TriggerState::FixReady, None));
    let alerts = fleet.alerts(None).await.unwrap();
    assert!(alerts.waiting.iter().any(|one| one.why.as_deref().is_some_and(|why| why.contains("choice"))));
}

#[tokio::test]
async fn a_fix_waiting_on_a_choice_is_an_alert_until_he_chooses() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let (fleet, _files) = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(fleet.alerts(None).await.unwrap().waiting.is_empty(), "repairing is not waiting on him");
    assert!(fleet.repair_next().await);

    let alerts = fleet.alerts(None).await.unwrap();
    let [one] = alerts.waiting.as_slice() else {
        panic!("one alert: {alerts:?}");
    };
    assert_eq!(one.status, "running");
    assert!(one.why.as_deref().is_some_and(|why| why.contains("deploy") && why.contains("choice")));
    assert!(one.since.is_some());

    fleet.choose_trigger_fix(&id, "deploy", FixChoice::NewPr).await.unwrap();
    assert!(fleet.alerts(None).await.unwrap().waiting.is_empty());
}

#[tokio::test]
async fn a_failed_alert_has_the_time_of_the_failure_and_clears_when_the_trigger_passes_later() {
    let home = TempDir::new();
    let flag = format!("{}/never", home.path().display());
    let (fleet, _files) = a_fleet(&home, "true", &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(fleet.repair_next().await);
    let failed = firing(&fleet, &id).await;
    let alerts = fleet.alerts(None).await.unwrap();
    assert_eq!(alerts.waiting.len(), 1);
    assert!(failed.repair.settled_at.is_some());
    assert_eq!(
        alerts.waiting[0].since,
        failed.repair.settled_at.as_ref().map(Into::into)
    );

    // The step runs again and the Trigger passes this time.
    let frozen = fleet.store().lock().await.frozen_triggers(&id).unwrap();
    let later = core_model::TriggerFiring::running(&frozen[0], fleet.now());
    {
        let mut store = fleet.store().lock().await;
        let row = store.open_firing(&id, &later).unwrap();
        store.settle_firing(row, &later.ended(Some(0), fleet.now())).unwrap();
    }
    assert!(fleet.alerts(None).await.unwrap().waiting.is_empty());
}

#[tokio::test]
async fn a_job_whose_disk_was_given_back_has_no_repair_alert() {
    let home = TempDir::new();
    let flag = format!("{}/never", home.path().display());
    let (fleet, _files) = a_fleet(&home, "true", &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(fleet.repair_next().await);
    assert_eq!(fleet.alerts(None).await.unwrap().waiting.len(), 1);

    fleet.store().lock().await.retain_job(&id, &fleet.now()).unwrap();

    assert!(fleet.alerts(None).await.unwrap().waiting.is_empty());
}

#[tokio::test]
async fn a_repair_a_restart_forgot_is_queued_again_from_the_store_and_runs() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let (fleet, _files) = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    // The process dies: what was queued in memory is gone, and the row says `repairing`.
    fleet.trigger_repairs().lock().unwrap().pop();
    assert!(!fleet.repair_next().await);
    assert_eq!(firing(&fleet, &id).await.state, TriggerState::Repairing);

    fleet.repairs_recovered().await;
    assert!(fleet.repair_next().await);

    let held = firing(&fleet, &id).await;
    assert_eq!(held.state, TriggerState::FixReady);
    assert_eq!(held.repair.tries, 1);
}

#[tokio::test]
async fn an_attempt_a_restart_cut_short_is_redone_and_not_counted_twice() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let (fleet, _files) = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    fleet.trigger_repairs().lock().unwrap().pop();
    // Died between the Drone and the rerun of its first try.
    let row = fleet.store().lock().await.firings_with_ids(&id).unwrap()[0].0;
    let cut = core_model::RepairRecord { tries: 1, ..Default::default() };
    fleet.store().lock().await.settle_repair(row, TriggerState::Rerunning, &cut, None).unwrap();

    fleet.repairs_recovered().await;
    assert!(fleet.repair_next().await);

    let held = firing(&fleet, &id).await;
    assert_eq!((held.state, held.repair.tries), (TriggerState::FixReady, 1));
}

/// Take every slot but `keeping`, as other Jobs would.
fn fill_the_pool(fleet: &Fixture, job: &core_model::Job) -> Vec<(u32, String)> {
    let served = fleet.served_by(job).unwrap();
    let pool = crate::leasing::pool_of(&served);
    let mut taken = Vec::new();
    for n in 0.. {
        let holder = format!("other-{n}");
        let spec = adapter_traits::WorktreeSpec::for_job(served.root(), &format!("other-{n}")).unwrap();
        match fleet.vcs().lease_slot(&pool, &spec, &holder).unwrap() {
            adapter_traits::SlotLeased::Took { slot, .. } => taken.push((slot, holder)),
            adapter_traits::SlotLeased::Full => break,
        }
    }
    taken
}

#[tokio::test]
async fn a_repair_with_no_free_slot_waits_for_one_and_is_not_failed() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let (fleet, _files) = a_fleet(&home, &format!("touch {flag}"), &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    let job = fleet.load(&id).await.unwrap();
    let taken = fill_the_pool(&fleet, &job);

    assert!(!fleet.repair_next().await, "nothing could be worked");
    assert_eq!(firing(&fleet, &id).await.state, TriggerState::Repairing);
    assert!(!fleet.repair_next().await, "still waiting, still queued");

    let (slot, holder) = &taken[0];
    let pool = crate::leasing::pool_of(&fleet.served_by(&job).unwrap());
    fleet.vcs().release_slot(&pool, *slot, holder).unwrap();
    assert!(fleet.repair_next().await);
    assert_eq!(firing(&fleet, &id).await.state, TriggerState::FixReady);
}

#[tokio::test]
async fn a_slot_held_by_a_repair_nothing_is_working_is_given_back_at_start_and_named_for_its_job() {
    let home = TempDir::new();
    let flag = format!("{}/fixed", home.path().display());
    let (fleet, _files) = a_fleet(&home, "true", &flag);
    let id = to_the_delivering_step(&fleet, &home).await;
    fleet.trigger_repairs().lock().unwrap().pop();
    let job = fleet.load(&id).await.unwrap();
    let served = fleet.served_by(&job).unwrap();
    let pool = crate::leasing::pool_of(&served);
    let holder = crate::repairing::holder_of(&id, &crate::trigger_repair::Subject::Firing(99));
    let spec = adapter_traits::WorktreeSpec::for_job(served.root(), "repair-left").unwrap();
    let adapter_traits::SlotLeased::Took { slot, .. } = fleet.vcs().lease_slot(&pool, &spec, &holder).unwrap() else {
        panic!("a slot")
    };

    let shown = fleet.pool_slots().await.unwrap();
    let repair = shown.iter().find(|one| one.reading.slot == slot).unwrap();
    assert_eq!(repair.job_title.as_deref(), Some(job.title().as_str()), "shown under its Job");

    fleet.repairs_recovered().await;

    assert!(fleet.vcs().parked_slots().contains(&(slot, holder)));
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
        fleet.vcs().deleted_branches(),
        [held.repair.branch.clone().unwrap()],
        "a repair that failed leaves no branch behind"
    );
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
