//! A Skill Trigger, a Skill added step and a Drone added step run on a side
//! Drone, on a branch cut from the Job's, and never gate the Job unless `block`
//! is on. The Drone is a shell; the pull request and the push are `FakeVcs`.

use std::sync::Arc;

use adapter_traits::DroneEvent;
use core_model::{FixChoice, JobId, JobStatus, TriggerState};
use ipc::{AddedRuns, TriggerFiringState as Wire};
use testkit::{Delivering, FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::admitted::started;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fitted_over, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::tests::triggering::{machine, manifest, to_the_delivering_step, Files};

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const SKILL_TRIGGER: &str = "name: tidy\nwhen: pr_opened\nskill: simplify\n";
const DRONE_TRIGGER: &str =
    "name: notes\nwhen: pr_opened\nbrief: Add a changelog line.\non_failure:\n  repair: true\n";
const BLOCKING_SKILL_TRIGGER: &str =
    "name: tidy\nwhen: pr_opened\nskill: simplify\non_failure:\n  block: true\n  repair: true\n";

/// A side Drone is told `RUN THE ...`; it runs `drone` and any other Drone,
/// which is a step's, waits. `ends` makes it print the line the fake reads as
/// a turn ending, so a Drone that does not print it is one that died.
fn harness(drone: &str, ends: bool) -> FakeHarness {
    let end = if ends { "; echo turn-ended" } else { "" };
    FakeHarness::running(
        "/bin/sh",
        &[
            "-c",
            &format!(
                "IFS= read -r line; case \"$line\" in *\"RUN THE \"*) {drone}{end};; *) sleep 30;; esac"
            ),
        ],
    )
    .reading(
        "turn-ended",
        vec![DroneEvent::Ended {
            turns: 1,
            cost_micros: 2_500,
            refusals: 0,
        }],
    )
}

fn a_fleet(
    home: &TempDir,
    drone: &str,
    ends: bool,
    changed: &[&str],
    triggers: Vec<config::TriggerWritten>,
) -> Fixture {
    let files = Arc::new(Files::default());
    files.say(triggers);
    let mut fittings = fitted_over(
        home,
        FakeWorkProduct::changed(changed),
        harness(drone, ends),
        FakeVcs::new().delivering(Delivering::default()),
    );
    fittings.starting().manifest = manifest(None);
    fittings.locating = Arc::new(Arc::clone(&files));
    Fleet::assembled(fittings)
}

async fn only_firing(fleet: &Fixture, job: &JobId) -> core_model::TriggerFiring {
    let held = fleet.store().lock().await.trigger_firings(job).unwrap();
    assert_eq!(held.len(), 1, "one firing: {held:?}");
    held.into_iter().next().unwrap()
}

#[tokio::test]
async fn a_skill_trigger_runs_on_a_side_drone_and_what_it_commits_waits_for_a_choice() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        "touch tidied",
        true,
        &["src/log.rs"],
        vec![machine("tidy.yml", SKILL_TRIGGER)],
    );
    let id = to_the_delivering_step(&fleet, &home).await;

    // Queued and not waited for: the Job is where it was.
    let waiting = only_firing(&fleet, &id).await;
    assert_eq!(waiting.state, TriggerState::Running);
    assert_eq!(waiting.repair.tries, 1, "a Drone is on it");
    let job = fleet.load(&id).await.unwrap();
    assert_eq!(job.status(), JobStatus::Running);
    assert_eq!(job.current_step_id().map(|s| s.as_str()), Some("summarise"));

    assert!(fleet.repair_next().await);
    assert!(!fleet.repair_next().await, "nothing else was waiting");

    let held = only_firing(&fleet, &id).await;
    assert_eq!(held.state, TriggerState::FixReady);
    assert_eq!(held.repair.choice, None, "Fleet does not choose");
    assert_eq!(held.repair.files, ["src/log.rs"]);
    let branch = held.repair.branch.clone().expect("a branch");
    assert_ne!(Some(branch.as_str()), job.branch().map(|b| b.as_str()));
    assert!(
        fleet
            .vcs()
            .cut_from()
            .contains(&job.branch().unwrap().as_str().to_string()),
        "cut from the Job's branch"
    );
    assert_eq!(fleet.vcs().parked_slots().len(), 1, "the fix holds no bay");

    // The Drone was told to run the skill, and its spend is the Job's.
    let told = fleet.harness().rendered();
    assert!(told.len() >= 2, "a step's Drone and the side one: {told:?}");
    let spend = fleet.store().lock().await.spend_for(&id).unwrap();
    assert_eq!(spend.cost_micros, 2_500);

    let wire = fleet.job_detail(ipc::JobId::from(&id)).await.unwrap();
    let shown = wire.triggers.iter().find(|one| one.name == "tidy").unwrap();
    assert_eq!(shown.state, Wire::FixReady);
    assert!(shown.drone, "a skill's firing says a Drone runs it");
    assert_eq!(
        shown.repair.as_ref().and_then(|r| r.branch.clone()),
        Some(branch)
    );
}

#[tokio::test]
async fn a_saved_drone_trigger_runs_its_prompt_and_what_it_commits_waits_for_a_choice() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        "touch noted",
        true,
        &["CHANGELOG.md"],
        vec![machine("notes.yml", DRONE_TRIGGER)],
    );
    let id = to_the_delivering_step(&fleet, &home).await;

    let waiting = only_firing(&fleet, &id).await;
    assert_eq!(waiting.state, TriggerState::Running);
    assert_eq!(waiting.repair.tries, 1, "a Drone is on it");
    assert!(!waiting.on_failure.repair, "repair is ignored for a Drone");

    assert!(fleet.repair_next().await);
    let held = only_firing(&fleet, &id).await;
    assert_eq!(held.state, TriggerState::FixReady);
    assert_eq!(held.repair.files, ["CHANGELOG.md"]);

    // The Trigger's own prompt rode in the Drone's first turn.
    let shown = fleet
        .harness()
        .configured()
        .iter()
        .any(|config| config.prompt().as_str().contains("Add a changelog line."));
    assert!(shown);
    let wire = fleet.job_detail(ipc::JobId::from(&id)).await.unwrap();
    let shown = wire
        .triggers
        .iter()
        .find(|one| one.name == "notes")
        .unwrap();
    assert_eq!(shown.state, Wire::FixReady);
    assert!(shown.drone);
}

#[tokio::test]
async fn a_skill_that_changes_nothing_passes_and_gives_its_branch_back() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        "true",
        true,
        &["src/log.rs"],
        vec![machine("tidy.yml", SKILL_TRIGGER)],
    );
    let id = to_the_delivering_step(&fleet, &home).await;
    // The Job's own work is in; the Drone adds nothing to it.
    fleet.work().forgot(&["src/log.rs"]);
    assert!(fleet.repair_next().await);

    let done = only_firing(&fleet, &id).await;
    assert_eq!(done.state, TriggerState::Passed);
    assert!(done.ended_at.is_some());
    assert_eq!(fleet.vcs().deleted_branches().len(), 1, "nothing to place");
}

#[tokio::test]
async fn a_drone_that_stops_before_it_finishes_fails_the_trigger() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        "exit 1",
        false,
        &["src/log.rs"],
        vec![machine("tidy.yml", SKILL_TRIGGER)],
    );
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(fleet.repair_next().await);

    let failed = only_firing(&fleet, &id).await;
    assert_eq!(failed.state, TriggerState::Failed);
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Running);
}

#[tokio::test]
async fn a_blocking_skill_holds_the_job_while_it_runs_and_lets_go_when_it_changes_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        "true",
        true,
        &["src/log.rs"],
        vec![machine("tidy.yml", BLOCKING_SKILL_TRIGGER)],
    );
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(
        fleet.is_held(&id).await,
        "it holds the review while it runs"
    );
    fleet.work().forgot(&["src/log.rs"]);

    assert!(fleet.repair_next().await);
    assert_eq!(only_firing(&fleet, &id).await.state, TriggerState::Passed);
    assert!(!fleet.is_held(&id).await);
}

#[tokio::test]
async fn a_blocking_skill_that_fails_is_held_and_rerun_goes_again_while_skip_lets_it_go() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        "exit 1",
        false,
        &["src/log.rs"],
        vec![machine("tidy.yml", BLOCKING_SKILL_TRIGGER)],
    );
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(fleet.repair_next().await);
    assert_eq!(only_firing(&fleet, &id).await.state, TriggerState::Held);
    assert!(fleet.is_held(&id).await);

    let act = ipc::HoldAct {
        trigger: Some("tidy".into()),
        addition: None,
    };
    let arc = Arc::new(fleet);
    let rerun = Arc::clone(&arc)
        .hold_rerun(ipc::JobId::from(&id), act.clone())
        .await
        .expect("accepted");
    assert_eq!(rerun.state, Wire::Running);
    assert!(arc.repair_next().await, "the Drone goes again");
    assert_eq!(only_firing(&arc, &id).await.state, TriggerState::Held);

    let skipped = arc
        .hold_skip(ipc::JobId::from(&id), act)
        .await
        .expect("accepted");
    assert!(skipped.released);
    assert!(!arc.is_held(&id).await);
}

#[tokio::test]
async fn choosing_this_branch_places_a_skills_fix_with_no_command_to_run_again() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        "touch tidied",
        true,
        &["src/log.rs"],
        vec![machine("tidy.yml", SKILL_TRIGGER)],
    );
    let id = to_the_delivering_step(&fleet, &home).await;
    assert!(fleet.repair_next().await);
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let chosen = fleet
        .choose_trigger_fix(&id, "tidy", FixChoice::ThisBranch)
        .await
        .expect("placed");
    assert_eq!(chosen.state, TriggerState::Passed);
    assert_eq!(only_firing(&fleet, &id).await.state, TriggerState::Passed);
}

/// `with` added after `implement`, on a Job whose first step's Drone is working.
async fn with_an_added_step(
    fleet: &Fixture,
    home: &TempDir,
    runs: AddedRuns,
    block: bool,
) -> JobId {
    let job = fleet.propose(a_proposal("fix the reader")).await.unwrap();
    worktree_directory(home, &job);
    let added = ipc::AddStep {
        runs,
        when: ipc::TriggerMoment::StepPasses,
        step: ipc::StepId::carried("implement"),
        block,
        // Ignored: a Drone already fixes its own failures.
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
    let _ = fleet.turn().await;
    job.id().clone()
}

async fn the_addition(fleet: &Fixture, job: &JobId) -> core_model::AddedStep {
    let held = fleet.store().lock().await.job_additions(job).unwrap();
    assert_eq!(held.len(), 1, "one addition: {held:?}");
    held.into_iter().next().unwrap()
}

#[tokio::test]
async fn a_drone_step_runs_its_brief_and_what_it_commits_waits_for_a_choice() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, "touch noted", true, &["CHANGELOG.md"], vec![]);
    let id = with_an_added_step(
        &fleet,
        &home,
        AddedRuns::Drone {
            brief: "Add a changelog line.".into(),
        },
        false,
    )
    .await;

    let queued = the_addition(&fleet, &id).await;
    assert_eq!(queued.fired.as_ref().unwrap().state, TriggerState::Running);
    assert!(
        !queued.on_failure.repair,
        "repair is ignored for a Drone step"
    );
    assert_eq!(fleet.load(&id).await.unwrap().status(), JobStatus::Running);

    assert!(fleet.repair_next().await);
    let done = the_addition(&fleet, &id).await;
    assert_eq!(done.fired.as_ref().unwrap().state, TriggerState::FixReady);
    assert_eq!(done.repair.files, ["CHANGELOG.md"]);
    let wire = fleet.job_detail(ipc::JobId::from(&id)).await.unwrap();
    assert_eq!(wire.additions[0].state, Wire::FixReady);
    assert!(!wire.additions[0].repair, "the wire reads repair off");

    // The step's own text rode in the Drone's first turn.
    let shown = fleet
        .harness()
        .configured()
        .iter()
        .any(|config| config.prompt().as_str().contains("Add a changelog line."));
    assert!(shown);
}

#[tokio::test]
async fn a_skill_step_that_changes_nothing_passes_and_one_whose_drone_dies_fails() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, "true", true, &["src/log.rs"], vec![]);
    let id = with_an_added_step(
        &fleet,
        &home,
        AddedRuns::Skill {
            skill: "simplify".into(),
        },
        false,
    )
    .await;
    fleet.work().forgot(&["src/log.rs"]);
    assert!(fleet.repair_next().await);
    assert_eq!(
        the_addition(&fleet, &id).await.fired.unwrap().state,
        TriggerState::Passed
    );

    let home = TempDir::new();
    let fleet = a_fleet(&home, "exit 1", false, &["x"], vec![]);
    let id = with_an_added_step(
        &fleet,
        &home,
        AddedRuns::Skill {
            skill: "simplify".into(),
        },
        true,
    )
    .await;
    assert!(fleet.repair_next().await);
    assert_eq!(
        the_addition(&fleet, &id).await.fired.unwrap().state,
        TriggerState::Held,
        "it blocks, so a failure holds the Job"
    );
}

#[tokio::test]
async fn a_side_run_a_restart_cut_short_is_worked_again() {
    let home = TempDir::new();
    let fleet = a_fleet(
        &home,
        "touch tidied",
        true,
        &["src/log.rs"],
        vec![machine("tidy.yml", SKILL_TRIGGER)],
    );
    let id = to_the_delivering_step(&fleet, &home).await;
    // The process died with the run queued in memory and nothing worked.
    fleet.trigger_repairs().lock().unwrap().pop();
    assert!(!fleet.repair_next().await);

    fleet.repairs_recovered().await;
    assert!(fleet.repair_next().await);
    assert_eq!(only_firing(&fleet, &id).await.state, TriggerState::FixReady);
}
