//! A Drone's process that ends on its own, with no terminating row: the Job's
//! log says how, and a task's Drone is replaced once before the Job escalates.
//! `crate::vanished`.
//!
//! Job 14 on 12 Oct lost the kept Drone of its ninth task 33 seconds after it
//! was spawned: transcript closed at 51 rows, no `ended`, state `failed`, the
//! Job `interrupted`, and nothing in the log to say whether the process was
//! killed or crashed.

use std::path::Path;
use std::time::Duration;

use core_model::{EscalationTrigger, JobId, JobStatus, TaskState, TransitionReason};
use testkit::{FakeHarness, FakeWorkProduct};

use crate::tests::admitted::dispatched;
use crate::daemon::Fleet;
use crate::tests::daemon::{a_proposal, fitted_with, worktree_directory};
use crate::tests::drone_per_task::{at_implement_over, on_implement, Fixture};
use crate::tests::tmp::TempDir;
use crate::tests::waiting::until;
use crate::transcript::log_of;

fn logged(home: &TempDir, handle: &str) -> String {
    std::fs::read_to_string(log_of(&home.path().to_string_lossy(), handle)).unwrap_or_default()
}

/// A Drone that stays up until it is told to die from outside, which the test
/// does by creating `die-N` in `dir`. Whichever Drone moves the file first
/// takes the signal and the others carry on, so each file ends exactly one.
/// It leaves when the test that started it is gone.
fn dying_on_request(dir: &Path) -> FakeHarness {
    let script = format!(
        "mkdir -p '{d}/taken'; \
         while kill -0 $PPID 2>/dev/null; do \
           for f in '{d}'/die-*; do \
             [ -e \"$f\" ] || continue; \
             mv \"$f\" '{d}/taken/' 2>/dev/null && kill -9 $$; \
           done; \
           sleep 0.02; \
         done",
        d = dir.display()
    );
    FakeHarness::running("/bin/sh", &["-c", &script])
}

fn let_one_die(dir: &Path, n: u32) {
    std::fs::write(dir.join(format!("die-{n}")), b"").expect("the signal is written");
}

/// The Drone now on `implement`, once it is not `previous`.
async fn replaced(fleet: &Fixture, job: &JobId, previous: &core_model::DroneId) -> core_model::DroneId {
    until("the Drone was never replaced", 300, Duration::from_millis(20), || async {
        fleet.turn().await.expect("a turn");
        on_implement(fleet, job).await.filter(|now| now != previous)
    })
    .await
}

/// The words that matter, whatever the fixture's other lines are.
fn says_why(log: &str, ended: &str, healed: bool) {
    assert!(
        log.contains("ended on its own before it reported"),
        "the vanish is written down: {log}"
    );
    assert!(log.contains(ended), "and says how: {log}");
    assert!(
        log.contains(&format!("\"healed\":{healed}")),
        "and whether Fleet replaced it: {log}"
    );
}

/// The Job's log names the signal, where it used to say `interrupted` alone.
#[tokio::test]
async fn a_drone_killed_from_outside_is_named_in_the_jobs_log() {
    let home = TempDir::new();
    let fleet = Fleet::assembled(fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        FakeHarness::running("/bin/sh", &["-c", "sleep 0.3; kill -9 $$"]),
    ));
    let job = fleet.propose(a_proposal("be killed")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();

    until("the Job never escalated", 300, Duration::from_millis(20), || async {
        fleet.turn().await.expect("a turn");
        (fleet.load(job.id()).await.unwrap().status() == JobStatus::Escalated).then_some(())
    })
    .await;

    assert_eq!(
        fleet.last_reason(job.id()).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Interrupted))
    );
    says_why(&logged(&home, &job.handle()), "killed by signal 9", false);
}

/// A task's Drone that vanishes is replaced on the same task, and the Job goes
/// on; the log says it was.
#[tokio::test]
async fn a_task_drone_that_vanishes_is_replaced_on_its_task() {
    let home = TempDir::new();
    let fitted = fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/read.rs"]),
        dying_on_request(home.path()),
    );
    let (fleet, job) = at_implement_over(&home, fitted).await;
    let first = on_implement(&fleet, &job).await.expect("T1's Drone");

    let_one_die(home.path(), 1);
    let second = replaced(&fleet, &job, &first).await;

    assert_ne!(first, second);
    assert_eq!(
        fleet.load(&job).await.unwrap().status(),
        JobStatus::Running,
        "the Job did not wait on anybody"
    );
    let states: Vec<TaskState> = fleet
        .plan_of(&job)
        .await
        .unwrap()
        .unwrap()
        .tasks()
        .iter()
        .map(|task| task.state())
        .collect();
    assert_eq!(
        states,
        [TaskState::Working, TaskState::Open, TaskState::Open],
        "the same task, worked again from the top"
    );
    let handle = fleet.load(&job).await.unwrap().handle();
    says_why(&logged(&home, &handle), "killed by signal 9", true);
}

/// The second vanish on the same task is not retried: the Job escalates as it
/// always did, and the log names the cause.
#[tokio::test]
async fn a_task_drone_that_vanishes_twice_escalates_the_job() {
    let home = TempDir::new();
    let fitted = fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/read.rs"]),
        dying_on_request(home.path()),
    );
    let (fleet, job) = at_implement_over(&home, fitted).await;
    let first = on_implement(&fleet, &job).await.expect("T1's Drone");

    let_one_die(home.path(), 1);
    replaced(&fleet, &job, &first).await;
    let_one_die(home.path(), 2);
    until("the Job never escalated", 300, Duration::from_millis(20), || async {
        fleet.turn().await.expect("a turn");
        (fleet.load(&job).await.unwrap().status() == JobStatus::Escalated).then_some(())
    })
    .await;

    assert_eq!(
        fleet.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Interrupted))
    );
    let handle = fleet.load(&job).await.unwrap().handle();
    let log = logged(&home, &handle);
    says_why(&log, "killed by signal 9", false);
    assert_eq!(
        log.matches("ended on its own before it reported").count(),
        2,
        "once replaced, once not: {log}"
    );
}

/// The Job that escalated on the second vanish is one a person can restart:
/// the step stopped under `run_ended`, the worktree is on disk, and the task
/// the gone Drone held is worked again by the step's next Drone.
#[tokio::test]
async fn a_job_interrupted_by_a_task_drone_vanishing_twice_restarts_its_step() {
    let home = TempDir::new();
    let fitted = fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/read.rs"]),
        dying_on_request(home.path()),
    );
    let (fleet, job) = at_implement_over(&home, fitted).await;
    let first = on_implement(&fleet, &job).await.expect("T1's Drone");

    let_one_die(home.path(), 1);
    let second = replaced(&fleet, &job, &first).await;
    let_one_die(home.path(), 2);
    until("the Job never escalated", 300, Duration::from_millis(20), || async {
        fleet.turn().await.expect("a turn");
        (fleet.load(&job).await.unwrap().status() == JobStatus::Escalated).then_some(())
    })
    .await;

    fleet
        .restart_step(&job, None)
        .await
        .expect("the step stopped and the worktree is there, so the restart lands");
    let third = replaced(&fleet, &job, &second).await;

    assert_ne!(second, third);
    let states: Vec<TaskState> = fleet
        .plan_of(&job)
        .await
        .unwrap()
        .unwrap()
        .tasks()
        .iter()
        .map(|task| task.state())
        .collect();
    assert_eq!(
        states,
        [TaskState::Working, TaskState::Open, TaskState::Open],
        "the task the gone Drone held is worked again"
    );
}
