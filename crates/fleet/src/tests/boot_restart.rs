//! A Job a Fleet restart interrupted has its step restarted, and only that
//! Job. **Real processes**, for `crate::tests::adopting`'s reason: a Drone is
//! interrupted only if there was a Drone, and what makes it gone is a process
//! that ended while no Fleet held it.

use core_model::{Actor, EscalationTrigger, JobId, JobStatus, Target, TransitionReason};
use testkit::FakeHarness;

use crate::boot_restart::{mark, restarted_since_progress, Mark};
use crate::tests::adopting::{
    a_drone_that_keeps_working, a_fleet, a_fleet_numbered, alive, called, end, pid_of, reap, spoke,
    started,
};
use crate::tests::daemon::{a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;

use crate::tests::adopting::Fixture;

/// A Drone that lives exactly as long as the Fleet holding its stdin.
fn a_drone_that_ends_with_its_fleet() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo CALLED; cat >/dev/null"])
        .reading("CALLED", vec![called()])
}

/// A Fleet with a Job on its step and a Drone, and that Drone's pid.
async fn first_fleet(home: &TempDir) -> (Fixture, JobId, u32) {
    let first = a_fleet(home, a_drone_that_ends_with_its_fleet());
    let job = started(&first, home).await;
    assert!(spoke(&first, 1).await, "the Drone never said anything");
    let pid = pid_of(&first).await;
    (first, job, pid)
}

/// The Fleet thrown away, and the Drone gone with it.
async fn gone(first: Fixture, pid: u32) {
    drop(first);
    reap(pid).await;
    assert!(!alive(pid), "the Drone was meant to have finished");
}

async fn interrupted(home: &TempDir) -> JobId {
    let (first, job, pid) = first_fleet(home).await;
    gone(first, pid).await;
    job
}

/// Every Job-level mark in the log, oldest first.
async fn marks(fleet: &Fixture, job: &JobId) -> Vec<Mark> {
    fleet
        .store()
        .lock()
        .await
        .events_for(job)
        .expect("the log reads")
        .iter()
        .map(mark)
        .collect()
}

async fn running_pid_ended(fleet: &Fixture) {
    end(pid_of(fleet).await).await;
}

#[tokio::test]
async fn a_job_whose_drone_is_gone_has_its_step_restarted_and_the_record_says_so() {
    let home = TempDir::new();
    let job = interrupted(&home).await;

    let second = a_fleet(&home, a_drone_that_keeps_working());
    let reconciled = second.reconcile().await.expect("the boot read");
    assert_eq!(reconciled.interrupted, vec![job.clone()]);
    assert_eq!(reconciled.restarted, vec![job.clone()]);
    assert_ne!(
        second.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
    assert_eq!(
        marks(&second, &job)
            .await
            .iter()
            .filter(|mark| **mark == Mark::Restarted)
            .count(),
        1,
        "the restart is a Fleet-signed move out of `escalated`, which a person's is not"
    );
    running_pid_ended(&second).await;
}

#[tokio::test]
async fn a_job_already_escalated_before_the_boot_is_left_alone() {
    let home = TempDir::new();
    let (first, job, pid) = first_fleet(&home).await;
    let record = first.load(&job).await.unwrap();
    first
        .move_job(
            &record,
            Target::Escalated(EscalationTrigger::Stalled),
            Actor::Fleet,
        )
        .await
        .unwrap();
    gone(first, pid).await;

    let second = a_fleet(&home, a_drone_that_keeps_working());
    let reconciled = second.reconcile().await.expect("the boot read");
    assert!(reconciled.interrupted.is_empty(), "{:?}", reconciled);
    assert!(reconciled.restarted.is_empty());
    assert_eq!(
        second.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
    assert_eq!(
        second.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Stalled))
    );
}

#[tokio::test]
async fn a_job_at_a_gate_and_a_queued_job_are_not_restarted() {
    let home = TempDir::new();
    let (first, gated, pid) = first_fleet(&home).await;
    let record = first.load(&gated).await.unwrap();
    first
        .move_job(&record, Target::AwaitingReview, Actor::Fleet)
        .await
        .unwrap();
    gone(first, pid).await;

    let second = a_fleet_numbered(&home, a_drone_that_keeps_working(), 500);
    let waiting = second.propose(a_proposal("a second Job")).await.unwrap();
    worktree_directory(&home, &waiting);
    second.approve(waiting.id()).await.unwrap();
    assert_eq!(
        second.load(waiting.id()).await.unwrap().status(),
        JobStatus::Queued
    );

    let reconciled = second.reconcile().await.expect("the boot read");
    assert!(reconciled.restarted.is_empty(), "{:?}", reconciled);
    assert!(!reconciled.interrupted.contains(waiting.id()));
    assert_eq!(
        second.load(&gated).await.unwrap().status(),
        JobStatus::AwaitingReview
    );
    // Admission, as before, may start the queued Job; this change does not.
    if let Some(pid) = pid_of_if_any(&second).await {
        end(pid).await;
    }
}

async fn pid_of_if_any(fleet: &Fixture) -> Option<u32> {
    fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .map(|working| working.session().pid())
}

#[tokio::test]
async fn a_job_past_its_cost_cap_stays_escalated_and_the_log_says_why() {
    let home = TempDir::new();
    let (first, job, pid) = first_fleet(&home).await;
    let record = first.load(&job).await.unwrap();
    first
        .store()
        .lock()
        .await
        .record_cost_cap(&record.cost_capped(Some(0)))
        .unwrap();
    gone(first, pid).await;

    let second = a_fleet(&home, a_drone_that_keeps_working());
    let reconciled = second.reconcile().await.expect("the boot read");
    assert_eq!(reconciled.interrupted, vec![job.clone()]);
    assert!(reconciled.restarted.is_empty());
    assert_eq!(
        second.last_reason(&job).await.unwrap(),
        Some(TransitionReason::Escalation(EscalationTrigger::Interrupted))
    );
    let log = log_of(&second, &job);
    assert!(
        log.contains("did not restart this step") && log.contains("cost cap is spent"),
        "{log}"
    );
}

/// The Job's log as text.
fn log_of(fleet: &Fixture, job: &JobId) -> String {
    let handle = fleet.name_of(job).expect("the Job has a name");
    let served = fleet.served_by_id(job).expect("served");
    std::fs::read_to_string(crate::transcript::log_of(served.records_root(), &handle))
        .unwrap_or_default()
}

/// **The bound, and what settles whether a spawn counts as something done.**
/// Three Fleets in a row each lose the Drone the one before started: the
/// first two restart the step, the third finds it twice restarted with nothing
/// advanced and leaves it for a person.
#[tokio::test]
async fn a_fleet_that_keeps_losing_the_drone_restarts_the_step_twice_and_stops() {
    let home = TempDir::new();
    let job = interrupted(&home).await;

    for boot in 1..=2 {
        let next = a_fleet_numbered(&home, a_drone_that_ends_with_its_fleet(), 1000 * boot);
        let reconciled = next.reconcile().await.expect("the boot read");
        assert_eq!(reconciled.restarted, vec![job.clone()], "boot {boot}");
        let pid = pid_of(&next).await;
        drop(next);
        reap(pid).await;
    }

    let last = a_fleet_numbered(&home, a_drone_that_keeps_working(), 9000);
    let reconciled = last.reconcile().await.expect("the boot read");
    assert_eq!(reconciled.interrupted, vec![job.clone()]);
    assert!(reconciled.restarted.is_empty());
    assert_eq!(
        last.load(&job).await.unwrap().status(),
        JobStatus::Escalated
    );
    assert!(log_of(&last, &job).contains("restarted this way 2 times"));
}

#[test]
fn only_what_came_after_the_last_thing_done_is_counted() {
    use Mark::{Other, Progress, Restarted};
    assert_eq!(restarted_since_progress(&[]), 0);
    assert_eq!(
        restarted_since_progress(&[Restarted, Progress, Other, Restarted, Other]),
        1
    );
    assert_eq!(restarted_since_progress(&[Restarted, Restarted, Other]), 2);
}
