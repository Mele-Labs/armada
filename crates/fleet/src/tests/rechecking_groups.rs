//! Running a group's Checks again after its retries ran out, #1105. Job 3,
//! 4 Oct 2026: every Check passed on a press, and the Judge then read tasks
//! still marked `failed` for the red run, refused, and the step stopped again.
//!
//! **A re-run's pass settles the tasks the red run failed.** The Judge's brief
//! is built from the plan before the step moves, so it is what is asserted.

use std::sync::Arc;

use core_model::{JobId, JobStatus, PlanChange, TaskState, TaskUpdate};
use testkit::{FakeJudge, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::{fittings, one};
use crate::tests::groups::{
    at_implement_on, g1_handed_in, group, hand_in, implement, planned, states, task,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// A Drone per task, two retries, a Check that passes once `pass` exists beside
/// the worktree, and a Judge that reads the plan through `reference_docs`.
fn the_workflow(home: &TempDir) -> config::ResolvedWorkflow {
    let text = "version: 1\nworkflow_id: fixture-groups\nname: fixture\n\
         steps:\n  - id: plan\n    label: \"Plan the change\"\n    \
         evidence: {submitted: {type: plan}}\n    mechanical_checks:\n      \
         - { type: plan_recorded, min_tasks: 1 }\n    delivers: false\n    \
         advance_gate: auto\n  - id: implement\n    label: \"Implement\"\n    \
         follows_plan: true\n    drone_per_task: true\n    retry_limit: 2\n    \
         evidence: {submitted: {type: diff}}\n    \
         mechanical_checks:\n      - { type: diff_nonempty }\n      \
         - { type: manifest_check, check: suite }\n    delivers: false\n    \
         advance_gate: auto_if_judge_passes\n    judge_checks:\n      -\n        \
         criteria:\n          - criterion_id: c1\n            \
         question: \"Does the evidence account for itself?\"\n            \
         on_refusal: refuse\n    evidence_scope:\n      \
         context_source: drone_declared\n      reference_docs:\n        - plan.evidence\n  \
         - id: handoff\n    label: \"Hand off\"\n    delivers: true\n    advance_gate: auto\n";
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture-groups.yml"),
        text,
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest(home))
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

fn manifest(home: &TempDir) -> config::Manifest {
    let at = home.path().display();
    config::Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!(
            "version: 1\nid: 01FIXTUREMANIFEST\nchecks:\n  suite:\n    run: \"/bin/sh -c \
             'test -f {at}/pass'\"\n"
        ),
    )
    .expect("a manifest that parses")
}

/// A Job whose group G1 ran red to the end of its retries: its tasks `failed`,
/// the Job held for repair. `in_g1` is how many tasks G1 holds, one or two.
async fn held_red(
    home: &TempDir,
    judge: &Arc<FakeJudge>,
    plan: &PlanChange,
    in_g1: usize,
) -> (Arc<Fixture>, JobId) {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = one(the_workflow(home));
    fittings.starting().manifest = manifest(home);
    fittings.judge = judge.clone();
    let (fleet, job) = at_implement_on(Arc::new(Fleet::assembled(fittings)), home, plan).await;
    crate::tests::tools::declared_by_the_one(
        &fleet,
        &ipc::mcp::DeclareScope {
            needs: Vec::new(),
            context_paths: vec!["src".to_string()],
        },
    )
    .await
    .expect("the Drone declares once");
    match in_g1 {
        2 => {
            g1_handed_in(&fleet).await;
        }
        _ => {
            submitted_by_the_one(&fleet, hand_in("T1 is done."))
                .await
                .expect("T1's hand-in");
            fleet.turn().await.expect("G1's gate");
        }
    }
    for _ in 0..2 {
        submitted_by_the_one(&fleet, hand_in("G1 again."))
            .await
            .expect("the round's hand-in");
        fleet.turn().await.expect("G1's gate");
    }
    assert_eq!(
        fleet.load(&job).await.expect("reads").status(),
        JobStatus::AwaitingRepair,
        "the retries are spent"
    );
    (fleet, job)
}

fn two_in_g1() -> PlanChange {
    planned(&[("Stop the reader at the end", 1), ("Cover the last row", 1)])
}

/// **Job 3.** The tasks the red run failed read `done` after a pass, the
/// reason is gone, and the Judge's brief never calls them failed.
#[tokio::test]
async fn a_passing_rerun_settles_the_tasks_the_red_run_failed_and_the_judge_never_reads_failed() {
    use TaskState::{Done, Failed};
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::with_no_objection());
    let (fleet, job) = held_red(&home, &judge, &two_in_g1(), 2).await;
    assert_eq!(states(&fleet, &job).await, [Failed, Failed]);
    assert!(judge.asked().is_empty(), "no Judge on a red run");
    std::fs::write(home.path().join("pass"), "").expect("the cause lifts");

    Arc::clone(&fleet)
        .rerun_checks(&job)
        .await
        .expect("the Checks run again");

    assert_eq!(states(&fleet, &job).await, [Done, Done]);
    let plan = fleet.plan_of(&job).await.expect("reads").expect("a plan");
    assert_eq!(plan.task(task("T1")).and_then(|t| t.failed_reason()), None);
    let asked = judge.asked();
    let brief = asked.first().expect("the Judge was asked after the pass");
    assert!(!brief.contains("[failed]"), "{brief}");
    assert!(!brief.contains("were still red"), "{brief}");
    assert!(brief.contains("[done]"), "{brief}");
    assert!(
        brief.contains("settled by a re-run"),
        "the Drone's `shown` is kept, and the brief says when it was written: {brief}"
    );
}

/// A task failed in a group the pass did not clear stays failed, with its
/// reason.
#[tokio::test]
async fn a_task_failed_in_a_group_the_pass_did_not_clear_stays_failed() {
    use TaskState::{Done, Failed};
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::with_no_objection());
    let plan = planned(&[("Stop the reader at the end", 1), ("Note the bound", 2)]);
    let (fleet, job) = held_red(&home, &judge, &plan, 1).await;
    let why = core_model::FailReason::new("G2's Checks were still red on run 3").expect("one");
    let at = fleet.now();
    fleet
        .store()
        .lock()
        .await
        .change_plan(
            &job,
            &PlanChange::Updated {
                task: task("T2"),
                to: TaskUpdate::Failed(why),
                shown: None,
            },
            store::PlanHand::Step(&implement()),
            &at,
        )
        .expect("T2 is marked failed");
    assert_eq!(states(&fleet, &job).await, [Failed, Failed]);
    std::fs::write(home.path().join("pass"), "").expect("the cause lifts");

    Arc::clone(&fleet)
        .rerun_checks(&job)
        .await
        .expect("the Checks run again");

    assert_eq!(states(&fleet, &job).await, [Done, Failed]);
    let plan = fleet.plan_of(&job).await.expect("reads").expect("a plan");
    assert_eq!(
        plan.task(task("T2")).and_then(|t| t.failed_reason()),
        Some("G2's Checks were still red on run 3")
    );
}

/// A re-run that is still red changes nothing.
#[tokio::test]
async fn a_rerun_that_is_still_red_leaves_the_tasks_failed_with_their_reason() {
    use TaskState::Failed;
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::with_no_objection());
    let (fleet, job) = held_red(&home, &judge, &two_in_g1(), 2).await;
    let before = fleet.plan_of(&job).await.expect("reads").expect("a plan");

    let held = Arc::clone(&fleet)
        .rerun_checks(&job)
        .await
        .expect("the Checks run again and answer");

    assert_eq!(held.status(), JobStatus::AwaitingRepair);
    assert_eq!(states(&fleet, &job).await, [Failed, Failed]);
    let after = fleet.plan_of(&job).await.expect("reads").expect("a plan");
    assert_eq!(after, before, "no change was appended");
    assert!(judge.asked().is_empty());
}

/// **#1792.** G1 ran red to the end of its retries and a group follows. A
/// passing re-run closes G1 and keeps the step, so G2's Drone is admission's to
/// start: the step must not advance past a group that never ran.
#[tokio::test]
async fn a_passing_rerun_of_one_group_starts_the_next_and_does_not_advance_the_step() {
    use TaskState::{Done, Open};
    let home = TempDir::new();
    let judge = Arc::new(FakeJudge::with_no_objection());
    let plan = planned(&[("Stop the reader at the end", 1), ("Note the bound", 2)]);
    let (fleet, job) = held_red(&home, &judge, &plan, 1).await;
    std::fs::write(home.path().join("pass"), "").expect("the cause lifts");

    let held = Arc::clone(&fleet)
        .rerun_checks(&job)
        .await
        .expect("the Checks run again");

    assert_eq!(
        states(&fleet, &job).await,
        [Done, Open],
        "G2 is still to run"
    );
    assert_eq!(
        held.status(),
        JobStatus::Queued,
        "admission starts G2's Drone"
    );
    assert_eq!(
        held.current_step_id(),
        Some(&implement()),
        "the step stays while a group is owed"
    );
    assert_eq!(
        held.step(&implement()).map(|row| row.state()),
        Some(core_model::StepState::Running)
    );
    let runs = fleet.group_runs_of(&job).await.expect("reads");
    assert!(runs.passed(group("G1")), "G1's run is closed as passed");
    assert!(runs.open_attempt(group("G1")).is_none());
    let commits = fleet.vcs().committed();
    assert_eq!(commits.len(), 1, "G1 committed once, as its own gate would");
    assert!(commits[0].message.contains("G1"), "{}", commits[0].message);

    // Admission puts the Drone on G2's first task.
    fleet.admit_next().await.expect("admission");
    assert_eq!(fleet.working_on().await.len(), 1, "a Drone is on G2");
    let plan = fleet.plan_of(&job).await.expect("reads").expect("a plan");
    assert_eq!(
        plan.task(task("T2")).map(|t| t.state()),
        Some(TaskState::Working)
    );
}
