//! A plan's groups, run one at a time with the step's gate at each one's end,
//! driven through the fake harness and the store: spike 022, slice 2. The
//! hermetic claim is `crates/acceptance/tests/drone_per_task.rs`.

use std::sync::Arc;

use api::{Commands, Queries, Refusal};
use core_model::{
    Approach, EvidenceType, GroupId, JobId, JobStatus, NewTask, PlanChange, StepId, TaskId,
    TaskState,
};
use testkit::FakeWorkProduct;
use verification::{Claimed, NotClaimed, ShownBy};

use crate::daemon::Fleet;
use crate::evidence::Call;

use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal_for, fittings, manifest, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// Plan, then an `implement` that works a Drone per task and may go round
/// twice on its own, then a handoff.
fn groups_with_two_retries() -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture-groups.yml"),
        "version: 1\nworkflow_id: fixture-groups\nname: fixture\nstructure: linear\n\
         steps:\n  - id: plan\n    label: \"Plan the change\"\n    \
         evidence: {submitted: {type: plan}}\n    mechanical_checks:\n      \
         - { type: plan_recorded, min_tasks: 1 }\n    delivers: false\n    \
         advance_gate: auto\n  - id: implement\n    label: \"Implement\"\n    \
         follows_plan: true\n    drone_per_task: true\n    retry_limit: 2\n    \
         evidence: {submitted: {type: diff}}\n    \
         mechanical_checks:\n      - { type: diff_nonempty }\n    delivers: false\n    \
         advance_gate: auto\n  - id: handoff\n    label: \"Hand off\"\n    \
         delivers: true\n    advance_gate: auto\n",
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest())
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

/// T1 and T2 in the first group, T3 in the second.
fn two_groups() -> PlanChange {
    let task = |title: &str, group: u32| {
        NewTask::new(title, "", &[], "")
            .expect("a title")
            .in_group(group)
    };
    PlanChange::Recorded {
        approach: Approach::new("Bound the reader and cover it, then say so").expect("one"),
        tasks: vec![
            task("Stop the reader at the end", 1),
            task("Cover the last row", 1),
            task("Note the bound", 2),
        ],
    }
}

fn implement() -> StepId {
    StepId::new("implement")
}

fn task(id: &str) -> TaskId {
    TaskId::read(id).expect("a task id")
}

fn group(id: &str) -> GroupId {
    GroupId::read(id).expect("a group id")
}

fn hand_in(claimed: &'static str) -> Call<'static> {
    Call {
        evidence_type: EvidenceType::Diff,
        claimed: Claimed(claimed),
        shown_by: ShownBy("the diff"),
        not_claimed: NotClaimed(""),
        review: None,
    }
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

async fn states(fleet: &Fixture, job: &JobId) -> Vec<TaskState> {
    fleet
        .plan_of(job)
        .await
        .expect("reads")
        .expect("a plan")
        .tasks()
        .iter()
        .map(|task| task.state())
        .collect()
}

/// A Job at `implement`, its plan recorded in two groups, T1's Drone working.
async fn at_implement(home: &TempDir, work: FakeWorkProduct) -> (Arc<Fixture>, JobId) {
    let mut fittings = fittings(home, work);
    fittings.starting().workflows = one(groups_with_two_retries());
    let fleet = Arc::new(Fleet::assembled(fittings));
    let job = fleet
        .propose(a_proposal_for("bound the reader", "fixture-groups"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(home, &job);
    dispatched(&fleet, &job_id).await.expect("it dispatches");
    fleet
        .change_plan(&job_id, &two_groups())
        .await
        .expect("the plan step records");
    submitted_by_the_one(
        &fleet,
        Call {
            evidence_type: EvidenceType::Plan,
            claimed: Claimed("Planned as two groups."),
            shown_by: ShownBy("the plan recorded with record_plan"),
            not_claimed: NotClaimed(""),
            review: None,
        },
    )
    .await
    .expect("the plan step asked for a plan");
    fleet.turn().await.expect("the plan step's gate runs");
    (fleet, job_id)
}

/// What a turn's gate ruled, by its variant's name.
fn ruled(turned: &crate::turning::Turned) -> String {
    format!("{:?}", turned.ruled())
}

/// G1's two tasks handed in, each by its own Drone, and its gate run once.
async fn g1_handed_in(fleet: &Fixture) -> String {
    submitted_by_the_one(fleet, hand_in("T1 is done."))
        .await
        .expect("T1's hand-in");
    fleet.turn().await.expect("T2's Drone");
    submitted_by_the_one(fleet, hand_in("T2 is done."))
        .await
        .expect("T2's hand-in");
    ruled(&fleet.turn().await.expect("G1's gate"))
}

/// Nothing ever changes, so `diff_nonempty` is red on every run: G1 goes round
/// twice with no press, then its tasks fail, the Record says which group and
/// run, and only then does Restart this task answer.
#[tokio::test]
async fn a_red_group_goes_round_twice_then_its_tasks_fail_and_restart_answers() {
    use TaskState::{Failed, HandedIn, Open, Working};
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home, FakeWorkProduct::untouched()).await;
    assert_eq!(states(&fleet, &job).await, [Working, Open, Open]);

    let first = g1_handed_in(&fleet).await;
    assert!(first.starts_with("Some(HandedBack"), "{first}");
    for round in 2..=3 {
        assert_eq!(
            fleet.load(&job).await.expect("reads").status(),
            JobStatus::Running
        );
        assert_eq!(
            states(&fleet, &job).await,
            [HandedIn, HandedIn, Open],
            "both stay handed in, and T3 waits, before run {round}"
        );
        let refused = Commands::restart_task(
            Arc::clone(&fleet),
            ipc::JobId::from(&job),
            "T1".to_string(),
            ipc::RestartTask::default(),
        )
        .await
        .expect_err("Restart this task waits for the retries to run out");
        assert_eq!(code(&refused), "fleet.task_not_failed");
        submitted_by_the_one(&fleet, hand_in("G1 again."))
            .await
            .expect("the round's hand-in");
        let said = ruled(&fleet.turn().await.expect("G1's gate"));
        let expected = if round < 3 {
            "Some(HandedBack"
        } else {
            "Some(Failed"
        };
        assert!(said.starts_with(expected), "run {round}: {said}");
    }

    let stopped = fleet.load(&job).await.expect("reads");
    assert_eq!(stopped.status(), JobStatus::AwaitingRepair);
    let plan = fleet.plan_of(&job).await.expect("reads").expect("a plan");
    assert_eq!(states(&fleet, &job).await, [Failed, Failed, Open]);
    let why = plan
        .task(task("T2"))
        .and_then(|t| t.failed_reason())
        .expect("a reason");
    assert!(why.contains("G1") && why.contains("run 3"), "{why}");

    // ------------------------------------------- the group and run on the Record
    let history = Queries::get_job_events(&*fleet, ipc::JobId::from(&job))
        .await
        .expect("the history");
    let marked: Vec<(String, Option<String>, Option<u32>)> = history
        .moves
        .iter()
        .filter_map(|row| match &row.moved {
            ipc::Movement::Step(step) if step.step_id.as_str() == "implement" => Some((
                step.to.as_wire().to_string(),
                row.group.clone(),
                row.group_attempt,
            )),
            _ => None,
        })
        .filter(|(_, group, _)| group.is_some())
        .collect();
    let g1 = || Some("G1".to_string());
    assert_eq!(
        marked,
        [
            ("retrying".to_string(), g1(), Some(1)),
            ("retrying".to_string(), g1(), Some(2)),
            ("stopped".to_string(), g1(), Some(3)),
        ],
        "each red run's step move names G1 and its run"
    );
    let runs = fleet
        .store()
        .lock()
        .await
        .step_checks_every_attempt(&job)
        .expect("reads");
    let at_groups: Vec<_> = runs
        .iter()
        .filter(|run| run.step_id == implement())
        .map(|run| (run.attempt.number(), run.group))
        .collect();
    assert_eq!(
        at_groups,
        [
            (1, Some((group("G1"), 1))),
            (2, Some((group("G1"), 2))),
            (3, Some((group("G1"), 3))),
        ],
        "each run's Checks are filed under its group and run"
    );

    // ----------------------------------------------- only now, Restart answers
    let restarted = Commands::restart_task(
        Arc::clone(&fleet),
        ipc::JobId::from(&job),
        "T2".to_string(),
        ipc::RestartTask {
            note: Some("Cover the row past the end too.".to_string()),
        },
    )
    .await
    .expect("a failed task restarts");
    assert_eq!(
        restarted.status,
        ipc::JobStatus::from(JobStatus::Queued),
        "back in the queue, as `restart_step` puts a Job"
    );
    assert_eq!(states(&fleet, &job).await, [Failed, Open, Open]);
    fleet.turn().await.expect("the Job is readmitted");
    assert_eq!(
        states(&fleet, &job).await,
        [Failed, Working, Open],
        "T2 alone is worked again"
    );
}

/// A green group commits once and keeps the step, and the next group's first
/// task gets its Drone; the step moves only after the last group.
#[tokio::test]
async fn a_green_group_commits_once_and_the_next_group_starts_on_the_same_step() {
    use TaskState::{Done, Working};
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home, FakeWorkProduct::changed(&["src/read.rs"])).await;
    let said = g1_handed_in(&fleet).await;
    assert!(
        said.starts_with("Some(Advanced"),
        "G1's gate is green: {said}"
    );
    assert_eq!(states(&fleet, &job).await, [Done, Done, Working]);
    let at = fleet.load(&job).await.expect("reads");
    assert_eq!(
        at.current_step_id(),
        Some(&implement()),
        "a group follows, so the step stays"
    );
    let commits = fleet.vcs().committed();
    assert_eq!(commits.len(), 1, "G1 committed once");
    assert!(commits[0].message.contains("G1"), "{}", commits[0].message);

    submitted_by_the_one(&fleet, hand_in("T3 is done."))
        .await
        .expect("T3's hand-in");
    fleet.turn().await.expect("G2's gate");
    assert_eq!(states(&fleet, &job).await, [Done, Done, Done]);
    let at = fleet.load(&job).await.expect("reads");
    assert_ne!(
        at.current_step_id(),
        Some(&implement()),
        "the last group moves it"
    );

    let detail = Queries::get_job(&*fleet, ipc::JobId::from(&job))
        .await
        .expect("the detail");
    let groups = detail.work_plan.expect("the plan").groups;
    let served: Vec<(String, &str, Option<bool>)> = groups
        .iter()
        .map(|g| {
            (
                g.id.clone(),
                g.state.as_wire(),
                g.attempts.last().map(|run| run.commit.is_some()),
            )
        })
        .collect();
    assert_eq!(
        served,
        [
            ("G1".to_string(), "passed", Some(true)),
            ("G2".to_string(), "passed", Some(false)),
        ],
        "G1 left a commit; G2's work is the step's to land"
    );
}

/// A move places by `after`, and is refused on a task still in its run.
#[tokio::test]
async fn a_move_places_by_after_and_waits_for_a_task_in_its_run() {
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home, FakeWorkProduct::changed(&["src/read.rs"])).await;
    let refused = Commands::move_plan(
        Arc::clone(&fleet),
        ipc::JobId::from(&job),
        ipc::MovePlan {
            group: "G2".to_string(),
            task: Some("T1".to_string()),
            after: None,
        },
    )
    .await
    .expect_err("T1 is working");
    assert_eq!(code(&refused), "fleet.task_in_flight");

    let moved = Commands::move_plan(
        Arc::clone(&fleet),
        ipc::JobId::from(&job),
        ipc::MovePlan {
            group: "G2".to_string(),
            task: Some("T2".to_string()),
            after: Some("T3".to_string()),
        },
    )
    .await
    .expect("an open task moves");
    let order: Vec<(&str, Option<&str>)> = moved
        .tasks
        .iter()
        .map(|t| (t.id.as_str(), t.group.as_deref()))
        .collect();
    assert_eq!(
        order,
        [("T1", Some("G1")), ("T3", Some("G2")), ("T2", Some("G2"))]
    );
    let refused = Commands::move_plan(
        Arc::clone(&fleet),
        ipc::JobId::from(&job),
        ipc::MovePlan {
            group: "G9".to_string(),
            task: None,
            after: None,
        },
    )
    .await
    .expect_err("no group G9");
    assert_eq!(code(&refused), "fleet.no_such_group");
}
