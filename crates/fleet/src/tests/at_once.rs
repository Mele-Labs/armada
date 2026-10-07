//! Tasks marked safe together, run at once by Drones of their own, driven
//! through the fake harness: spike 022, slice 5. The hermetic claim is
//! `crates/acceptance/tests/drone_per_task/at_once.rs`; this is the spawning,
//! the stop and the join.

use adapter_traits::{CallDetail, DroneEvent};
use core_model::{
    Approach, DroneId, EvidenceType, JobId, JobStatus, NewTask, PlanChange, TaskId, TaskState,
};
use testkit::{FakeHarness, FakeWorkProduct, EDITS};
use verification::{Claimed, NotClaimed, ShownBy};

use crate::adrift::Adrift;
use crate::crew::as_caller;
use crate::daemon::Fleet;
use crate::evidence::{Call, Recorded};
use crate::slots::Concurrency;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal_for, fitted_with, one, worktree_directory};
use crate::tests::drone_per_task::a_drone_per_task;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

/// T1 names T2 as safe beside it; T3 runs alone.
fn t1_beside_t2() -> PlanChange {
    let task = |title: &str| NewTask::new(title, "", &[], "").expect("a title");
    PlanChange::Recorded {
        approach: Approach::new("Bound the reader and the session, then cover both")
            .expect("an approach"),
        tasks: vec![
            task("Bound the reader").beside(&[2]),
            task("Bound the session"),
            task("Cover both"),
        ],
    }
}

fn task(id: &str) -> TaskId {
    TaskId::read(id).expect("a task id")
}

fn diff(claimed: &str) -> Call<'_> {
    Call {
        evidence_type: EvidenceType::Diff,
        claimed: Claimed(claimed),
        shown_by: ShownBy("the diff"),
        not_claimed: NotClaimed(""),
        review: None,
    }
}

/// A Drone that edits `src/shared.rs` as it starts, then listens.
fn a_drone_that_edits_one_file() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo EDITS; cat"]).reading(
        "EDITS",
        vec![DroneEvent::Called {
            tool: EDITS.to_string(),
            call: "an-edit".to_string(),
            detail: CallDetail::of("src/shared.rs"),
        }],
    )
}

/// A Job at `implement` on a machine allowing three Drones, its plan recorded
/// with T1 beside T2, and the turn after its plan step's gate taken: the turn
/// that puts T1's Drone on and, room allowing, T2's beside it.
async fn at_implement(home: &TempDir, harness: FakeHarness, cap: Option<u32>) -> (Fixture, JobId) {
    let mut fittings = fitted_with(home, FakeWorkProduct::changed(&["src/read.rs"]), harness);
    fittings.starting().workflows = one(a_drone_per_task());
    fittings.concurrency = Concurrency::of(3);
    let fleet = Fleet::assembled(fittings);
    let job = fleet
        .propose(a_proposal_for("bound the reader", "fixture-per-task"))
        .await
        .expect("a Job at the approval gate");
    let job_id = job.id().clone();
    fleet
        .store()
        .lock()
        .await
        .set_drone_cap(&job_id, cap)
        .expect("the cap is kept");
    worktree_directory(home, &job);
    dispatched(&fleet, &job_id).await.expect("it dispatches");
    fleet
        .change_plan(&job_id, &t1_beside_t2())
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

/// Every Drone beside the kept one, as Drone and pid.
fn beside(fleet: &Fixture, job: &JobId) -> Vec<DroneId> {
    fleet
        .crew_at_work()
        .into_iter()
        .filter(|(of, _, _)| of == job)
        .map(|(_, drone, _)| drone)
        .collect()
}

/// Wait until a Drone beside the kept one has been heard making its edit call,
/// as a real Drone's edits are read long before it hands in.
async fn heard_editing(fleet: &Fixture, job: &JobId, drone: &DroneId) {
    for _ in 0..400 {
        let slot = fleet.slots().lock().await.crew_slot(job, drone);
        if let Some(slot) = slot {
            let held = slot.lock().await;
            let edited = held.as_ref().is_some_and(|at| {
                at.heard()
                    .iter()
                    .any(|event| matches!(event, DroneEvent::Called { tool, .. } if tool == EDITS))
            });
            if edited {
                return;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("the Drone was never heard editing");
}

/// Hand in as one Drone beside the kept one.
async fn handed_in_beside(
    fleet: &Fixture,
    job: &JobId,
    drone: &DroneId,
    claimed: &str,
) -> Result<Recorded, crate::evidence::NotSubmitted> {
    as_caller(
        Some(drone.clone()),
        fleet.submit_evidence(job, diff(claimed)),
    )
    .await
}

/// Two Drones on one step, only the one beside the kept Drone asks: the held
/// command names it, and an allow still releases that Drone's call.
#[tokio::test]
async fn a_held_command_names_the_drone_that_asked_and_not_its_neighbour() {
    use crate::tests::permitting::{asked, until_waiting};
    use core_model::WhenBlocked;
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home, FakeHarness::that_listens(), None).await;
    let extra = beside(&fleet, &job).pop().expect("T2's Drone");
    fleet
        .set_when_blocked(&job, WhenBlocked::AskMe)
        .await
        .expect("the setting is recorded");
    let asking = asked("Bash", "npm publish", "c1");

    let (answer, answered) = tokio::join!(
        as_caller(Some(extra.clone()), fleet.permission(&job, &asking)),
        async {
            let waiting = until_waiting(&fleet, &job).await;
            assert_eq!(
                waiting.drone_id,
                Some(ipc::DroneId::from(&extra)),
                "T2's Drone asked, and the kept Drone did not"
            );
            fleet
                .answer_command(
                    &job,
                    "c1",
                    crate::permitting::Answered::of(ipc::CommandAnswer::AllowForJob, None),
                )
                .await
        }
    );
    answered.expect("the answer reaches the Drone that asked");
    assert_eq!(answer, api::PermissionAnswer::Allow);
}

#[tokio::test]
async fn two_tasks_marked_safe_together_run_at_once_with_drones_of_their_own() {
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home, FakeHarness::that_listens(), None).await;
    use TaskState::{Done, HandedIn, Working};
    // The turn that puts the kept Drone on T1 puts one beside it on T2.
    assert_eq!(
        states(&fleet, &job).await,
        [Working, Working, TaskState::Open]
    );
    let extra = beside(&fleet, &job).pop().expect("T2's Drone");
    let listed = fleet.job_drones((&job).into()).await.expect("listed");
    let running: Vec<(Option<&str>, &str)> = listed
        .drones
        .iter()
        .filter(|d| d.state == ipc::DroneState::Running)
        .map(|d| (d.task.as_deref(), d.drone_id.as_str()))
        .collect();
    assert_eq!(running.len(), 2, "two Drones running: {running:?}");
    assert!(running.contains(&(Some("T2"), extra.as_str())));
    fleet.turn().await.expect("a turn");
    assert_eq!(beside(&fleet, &job).len(), 1, "T3 is not safe beside them");

    // T2 hands in from its own Drone; the kept Drone's T1 is untouched.
    handed_in_beside(&fleet, &job, &extra, "The session is bounded.")
        .await
        .expect("T2's hand-in is its own");
    assert_eq!(
        states(&fleet, &job).await,
        [Working, HandedIn, TaskState::Open]
    );
    fleet.turn().await.expect("a turn");
    assert!(
        beside(&fleet, &job).is_empty(),
        "the Drone that handed in ends"
    );

    // The kept Drone goes on to T3, then the group joins and its gate runs.
    submitted_by_the_one(&fleet, diff("The reader is bounded."))
        .await
        .expect("T1's hand-in");
    fleet.turn().await.expect("the kept Drone moves to T3");
    assert_eq!(states(&fleet, &job).await, [HandedIn, HandedIn, Working]);
    submitted_by_the_one(&fleet, diff("Both are covered."))
        .await
        .expect("T3's hand-in fills the inbox");
    assert_eq!(fleet.evidence_waiting_for(&job), 1);
    fleet.turn().await.expect("the gate runs");
    assert_eq!(states(&fleet, &job).await, [Done, Done, Done]);

    let listed = fleet.job_drones((&job).into()).await.expect("listed");
    let t2 = listed
        .drones
        .iter()
        .find(|d| d.drone_id.as_str() == extra.as_str())
        .expect("T2's Drone is listed");
    assert_eq!(
        (t2.task.as_deref(), t2.state),
        (Some("T2"), ipc::DroneState::Done)
    );
}

#[tokio::test]
async fn one_drone_is_stopped_or_told_and_the_other_goes_on() {
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home, FakeHarness::that_listens(), None).await;
    let extra = beside(&fleet, &job).pop().expect("T2's Drone");

    let said = crate::resume::Redirection::saying("Bound it at the last row").expect("words");
    fleet
        .redirect_one_drone(&job, &extra, &said, api::Redirector::Person)
        .await
        .expect("one Drone beside the kept one is told");

    let after = fleet
        .kill_one_drone(&job, &extra)
        .await
        .expect("one Drone is stopped");
    assert_eq!(after.status(), JobStatus::Running, "the Job goes on");
    assert!(beside(&fleet, &job).is_empty());
    assert_eq!(
        states(&fleet, &job).await,
        [TaskState::Working, TaskState::Open, TaskState::Open],
        "T1's Drone goes on, and T2 waits for it"
    );
    fleet.turn().await.expect("a turn");
    assert!(
        beside(&fleet, &job).is_empty(),
        "a task whose Drone was stopped is not started again beside the kept one"
    );

    for refused in [
        fleet.kill_one_drone(&job, &extra).await.map(|_| ()),
        fleet
            .redirect_one_drone(&job, &extra, &said, api::Redirector::Person)
            .await
            .map(|_| ()),
    ] {
        assert!(
            matches!(refused, Err(Adrift::DroneNotLive { .. })),
            "a Drone that is gone is refused, never another told: {refused:?}"
        );
    }
}

#[tokio::test]
async fn two_that_edited_one_file_leave_the_group_uncommitted_and_run_again_in_turn() {
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home, a_drone_that_edits_one_file(), None).await;
    let extra = beside(&fleet, &job).pop().expect("T2's Drone");
    heard_editing(&fleet, &job, &extra).await;

    handed_in_beside(&fleet, &job, &extra, "The session is bounded.")
        .await
        .expect("T2's hand-in");
    fleet.turn().await.expect("a turn");
    submitted_by_the_one(&fleet, diff("The reader is bounded."))
        .await
        .expect("T1's hand-in");
    fleet.turn().await.expect("the kept Drone moves to T3");
    submitted_by_the_one(&fleet, diff("Both are covered."))
        .await
        .expect("T3's hand-in is the join");

    assert_eq!(
        fleet.evidence_waiting_for(&job),
        0,
        "the group does not reach its gate, so it does not commit"
    );
    use TaskState::{HandedIn, Open};
    assert_eq!(states(&fleet, &job).await, [Open, Open, HandedIn]);
    let runs = fleet.group_runs_of(&job).await.expect("reads");
    let pair: Vec<_> = runs
        .apart()
        .iter()
        .map(|a| (a.tasks, a.paths.clone()))
        .collect();
    assert_eq!(
        pair,
        [((task("T1"), task("T2")), vec!["src/shared.rs".to_string()])]
    );

    // Run again one after the other: the kept Drone takes T1, nothing beside it.
    fleet.turn().await.expect("a turn");
    fleet.turn().await.expect("a turn");
    assert_eq!(
        states(&fleet, &job).await,
        [TaskState::Working, Open, HandedIn]
    );
    assert!(beside(&fleet, &job).is_empty(), "T2 waits for T1");
}

#[tokio::test]
async fn a_job_at_its_own_cap_starts_nothing_beside_its_kept_drone() {
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home, FakeHarness::that_listens(), Some(1)).await;
    fleet.turn().await.expect("a turn");
    assert!(
        beside(&fleet, &job).is_empty(),
        "the Job's cap is one Drone"
    );
    assert_eq!(
        states(&fleet, &job).await,
        [TaskState::Working, TaskState::Open, TaskState::Open]
    );
}

/// A Drone beside the kept one asks for the step's Checks: the result is told
/// to it, on its own slot, and the log says so. The run is spawned, and a
/// task-local does not cross a spawn, so what it ended on was the kept Drone's.
#[tokio::test]
async fn a_run_a_crew_drone_asked_for_is_told_to_that_drone() {
    use ipc::mcp::ChecksAsk;
    let home = TempDir::new();
    let (fleet, job) = at_implement(&home, FakeHarness::that_listens(), None).await;
    let fleet = std::sync::Arc::new(fleet);
    let extra = beside(&fleet, &job).pop().expect("T2's Drone");

    let running = as_caller(
        Some(extra.clone()),
        fleet.run_checks(&job, ChecksAsk::everything(false)),
    )
    .await
    .expect("the run starts");
    let told = tokio::time::timeout(std::time::Duration::from_secs(10), running.finished())
        .await
        .expect("the run ends");
    assert!(told.is_some(), "the result was never told to T2's Drone");

    let handle = fleet.name_of(&job).expect("the Job has a handle");
    let log = std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        &handle,
    ))
    .expect("the Job's own log");
    assert!(
        log.contains("the Drone asked for the step's checks and they were run"),
        "{log}"
    );
}
