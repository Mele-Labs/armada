//! A step whose plan is worked one task at a time, a Drone each, driven through
//! the fake harness: spike 022, slice 1b. The hermetic claim is
//! `crates/acceptance/tests/drone_per_task.rs`; this is the spawning.

use adapter_traits::Grant;
use core_model::{
    Approach, EvidenceType, JobId, JobStep, NewTask, PlanChange, StepId, TaskId, TaskState,
};
use testkit::FakeWorkProduct;
use verification::{Claimed, NotClaimed, ShownBy};

use crate::daemon::Fleet;
use crate::evidence::Call;
use crate::gate::Ruling;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal_for, fittings, manifest, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;
use crate::work_plan::{permitted, plan_grants, NotPlanned};

/// Plan, then an `implement` that works a Drone per task, then a handoff.
pub(crate) fn a_drone_per_task() -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture-per-task.yml"),
        "version: 1\nworkflow_id: fixture-per-task\nname: fixture\n\
         steps:\n  - id: plan\n    label: \"Plan the change\"\n    \
         evidence: {submitted: {type: plan}}\n    mechanical_checks:\n      \
         - { type: plan_recorded, min_tasks: 1 }\n    delivers: false\n    \
         advance_gate: auto\n  - id: implement\n    label: \"Implement\"\n    \
         follows_plan: true\n    drone_per_task: true\n    \
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

fn three_tasks() -> PlanChange {
    let task = |title: &str| NewTask::new(title, "", &[], "").expect("a title");
    PlanChange::Recorded {
        approach: Approach::new("Bound the reader, cover it, say so").expect("an approach"),
        tasks: vec![
            task("Stop the reader at the end"),
            task("Cover the last row"),
            task("Note the bound"),
        ],
    }
}

fn implement() -> StepId {
    StepId::new("implement")
}

fn task(id: &str) -> TaskId {
    TaskId::read(id).expect("a task id")
}

pub(crate) type Fixture = Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

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

pub(crate) async fn on_implement(fleet: &Fixture, job: &JobId) -> Option<core_model::DroneId> {
    fleet
        .load(job)
        .await
        .expect("reads")
        .step(&implement())
        .and_then(JobStep::assigned_drone)
        .cloned()
}

/// A Job at `implement`, its plan recorded with three tasks.
async fn at_implement(home: &TempDir) -> (Fixture, JobId) {
    at_implement_over(
        home,
        fittings(home, FakeWorkProduct::changed(&["src/read.rs"])),
    )
    .await
}

/// [`at_implement`], over fittings a case planted: a Drone that comes to rest,
/// or a grace it has to come to rest in. `crate::tests::coming_to_rest`.
pub(crate) async fn at_implement_over(
    home: &TempDir,
    mut fittings: crate::daemon::Fittings<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>,
) -> (Fixture, JobId) {
    fittings.starting().workflows = one(a_drone_per_task());
    let fleet = Fleet::assembled(fittings);
    let job = fleet
        .propose(a_proposal_for("bound the reader", "fixture-per-task"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    worktree_directory(home, &job);
    dispatched(&fleet, &job_id).await.expect("it dispatches");
    fleet
        .change_plan(&job_id, &three_tasks())
        .await
        .expect("the plan step records");
    submitted_by_the_one(
        &fleet,
        Call {
            evidence_type: EvidenceType::Plan,
            claimed: Claimed("Planned as three tasks."),
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

/// Each task gets a Drone of its own, in plan order, and Fleet writes every
/// state: working at the spawn, handed in at the hand-in, done at green.
#[tokio::test]
async fn each_task_is_worked_by_a_drone_of_its_own_and_fleet_marks_it() {
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home).await;
    use TaskState::{Done, HandedIn, Open, Working};
    assert_eq!(states(&fleet, &job).await, [Working, Open, Open]);

    let mut drones = vec![on_implement(&fleet, &job).await.expect("T1's Drone")];
    for (n, (claimed, shown)) in [
        ("The reader stops at the end.", "read::stops passes"),
        ("The last row is covered.", "the new case in read.rs"),
        ("The bound is noted.", "the doc comment on read_to"),
    ]
    .into_iter()
    .enumerate()
    {
        let recorded = submitted_by_the_one(
            &fleet,
            Call {
                evidence_type: EvidenceType::Diff,
                claimed: Claimed(claimed),
                shown_by: ShownBy(shown),
                not_claimed: NotClaimed(""),
                review: None,
            },
        )
        .await
        .expect("a task's hand-in is recorded");
        assert_eq!(recorded.word(), "recorded");
        let plan = fleet.plan_of(&job).await.expect("reads").expect("a plan");
        let handed = &plan.tasks()[n];
        assert_eq!(handed.state(), HandedIn);
        assert_eq!(handed.shown().map(|shown| shown.as_str()), Some(shown));
        let last = n == 2;
        assert_eq!(
            fleet.evidence_waiting_for(&job),
            usize::from(last),
            "only the last hand-in puts the step's submission in the inbox"
        );
        let turned = fleet.turn().await.expect("a turn");
        if !last {
            assert!(turned.ruled().is_none(), "no gate runs between tasks");
            let next = on_implement(&fleet, &job).await.expect("the next Drone");
            assert!(!drones.contains(&next), "a Drone of its own");
            drones.push(next);
            assert_eq!(states(&fleet, &job).await[n + 1], Working);
        } else {
            assert!(
                matches!(turned.ruled(), Some(Ruling::Advanced { .. })),
                "the step's gate runs once, on every task: {:?}",
                turned.ruled()
            );
        }
    }
    assert_eq!(states(&fleet, &job).await, [Done, Done, Done]);

    let evidence = fleet
        .store()
        .lock()
        .await
        .step_evidence(&job)
        .expect("reads");
    let (_, submitted) = evidence
        .iter()
        .find(|(step, _)| *step == implement())
        .expect("implement's submission was kept");
    for said in [
        "T1: The reader stops at the end.",
        "T2: The last row is covered.",
        "T3: The bound is noted.",
    ] {
        assert!(submitted.claimed.contains(said), "{}", submitted.claimed);
    }

    let listed = fleet
        .job_drones((&job).into())
        .await
        .expect("the Job's Drones");
    let on_tasks: Vec<_> = listed
        .drones
        .iter()
        .filter(|drone| drone.step_id.as_str() == "implement")
        .map(|drone| (drone.task.as_deref(), drone.state))
        .collect();
    assert_eq!(
        on_tasks,
        [
            (Some("T1"), ipc::DroneState::Done),
            (Some("T2"), ipc::DroneState::Done),
            (Some("T3"), ipc::DroneState::Done),
        ],
        "each Drone names its task, and one that handed in reads done"
    );
}

/// Between a hand-in and the next spawn the Drone is owed a successor, which is
/// what `reap` asks before reading its exit as one that left with nothing.
/// The last hand-in owes none: the gate decides.
#[tokio::test]
async fn a_drone_that_handed_in_is_between_tasks_until_the_last_one() {
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home).await;
    async fn between(fleet: &Fixture, job: &JobId) -> bool {
        let slot = fleet.slot_of(job).await.expect("a slot");
        let held = slot.lock().await;
        let at_work = held.as_ref().expect("a Drone");
        fleet.between_tasks(at_work).await.expect("reads")
    }
    let hand_in = || {
        submitted_by_the_one(
            &fleet,
            Call {
                evidence_type: EvidenceType::Diff,
                claimed: Claimed("Done."),
                shown_by: ShownBy("the diff"),
                not_claimed: NotClaimed(""),
                review: None,
            },
        )
    };
    assert!(!between(&fleet, &job).await, "T1's Drone has not handed in");
    hand_in().await.expect("recorded");
    assert!(
        between(&fleet, &job).await,
        "T1 is in and T2 is owed a Drone"
    );
    fleet.turn().await.expect("a turn");
    hand_in().await.expect("recorded");
    fleet.turn().await.expect("a turn");
    hand_in().await.expect("recorded");
    assert!(
        !between(&fleet, &job).await,
        "the last hand-in owes no Drone; the gate decides"
    );
}

/// A task's Drone is offered no plan tool, and a call is refused by name.
#[tokio::test]
async fn a_tasks_drone_is_given_no_plan_tool_and_a_call_says_why() {
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home).await;
    let loaded = fleet.load(&job).await.expect("reads");
    let step = loaded.workflow().step(&implement()).expect("implement");
    assert!(step.drone_per_task() && step.follows_plan());
    assert_eq!(plan_grants(step), Vec::<Grant>::new());
    let update = PlanChange::Updated {
        task: task("T3"),
        to: core_model::TaskUpdate::Done,
        shown: None,
    };
    assert!(matches!(
        permitted(step, &update),
        Err(NotPlanned::FleetMarksTasks { .. })
    ));
    assert!(matches!(
        fleet.change_plan(&job, &update).await,
        Err(NotPlanned::FleetMarksTasks { .. })
    ));
    assert_eq!(states(&fleet, &job).await[2], TaskState::Open);
}
