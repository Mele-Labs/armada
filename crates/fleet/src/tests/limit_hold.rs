//! A Drone the plan's limit stopped is held until the limit resets, not failed.
//! `docs/concepts/job.md`, *Held for the plan's limit*; fixture
//! `crates/testkit/fixtures/ndjson/plan-limit.ndjson`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use config::ResolvedWorkflow;
use core_model::{JobId, JobStatus, QueuedReason, StepId, StepState, Timestamp};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::clock::Clock;
use crate::daemon::Fleet;
use crate::drone::{aftermath, limited_until, Aftermath, Ending, Left};
use crate::tests::admitted::{admit, dispatched};
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::waiting::until;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// 05:00 on the clock's day, in unix seconds.
const RESETS: i64 = 1_788_325_200;

/// A clock that ticks a second per reading, and jumps when a test says so.
struct Held {
    ticks: AtomicU64,
    pushed: AtomicU64,
}

impl Clock for Held {
    fn now(&self) -> Timestamp {
        let at = self.ticks.fetch_add(1, Ordering::SeqCst) + self.pushed.load(Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-09-02T{:02}:{:02}:{:02}.000Z",
            (at / 3_600) % 24,
            (at / 60) % 60,
            at % 60
        ))
    }
}

fn quota(status: &str, resets_at: Option<i64>) -> DroneEvent {
    DroneEvent::QuotaMoved {
        window: String::from("five_hour"),
        status: String::from(status),
        resets_at,
    }
}

fn called() -> DroneEvent {
    DroneEvent::Called {
        tool: String::from("Read"),
        call: String::from("a-call"),
        detail: CallDetail::of("a file"),
    }
}

fn ended() -> DroneEvent {
    DroneEvent::Ended {
        turns: 8,
        cost_micros: 1_000_000,
        refusals: 0,
    }
}

#[test]
fn a_rejected_window_that_ends_the_run_is_a_limit() {
    let events = [called(), quota("rejected", Some(RESETS)), ended()];
    assert_eq!(limited_until(&events), Some(RESETS));
    assert_eq!(Ending::of(&events), Ending::Limited { until: RESETS });
    assert_eq!(
        aftermath(JobStatus::Running, &Ending::of(&events), Left::Nothing),
        Aftermath::HeldUntil { until: RESETS }
    );
}

#[test]
fn what_is_not_a_limit_ending_is_read_as_before() {
    let allowed = [called(), quota("allowed", Some(RESETS)), ended()];
    assert_eq!(limited_until(&allowed), None);
    let no_reset = [called(), quota("rejected", None), ended()];
    assert_eq!(limited_until(&no_reset), None, "nothing to hold it until");
    let carried_on = [quota("rejected", Some(RESETS)), called(), ended()];
    assert_eq!(limited_until(&carried_on), None, "it got past the rejection");
    let reopened = [quota("rejected", Some(RESETS)), quota("allowed", None), ended()];
    assert_eq!(limited_until(&reopened), None);
}

#[test]
fn evidence_waiting_still_goes_to_the_gate_and_a_stopped_job_stays_stopped() {
    let ending = Ending::Limited { until: RESETS };
    assert_eq!(
        aftermath(JobStatus::Running, &ending, Left::Evidence),
        Aftermath::TheGateDecides
    );
    assert_eq!(
        aftermath(JobStatus::Escalated, &ending, Left::Nothing),
        Aftermath::AlreadyStopped
    );
}

fn the_step() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// A Drone that worked, was refused by the plan's limit, and ended its run. `stays` keeps the
/// process up, so the vigil's road reaches it and not the reaper's.
fn a_drone_the_limit_stops(stays: bool) -> FakeHarness {
    let script = if stays {
        "echo CALLED; echo LIMIT; echo ENDED; sleep 30"
    } else {
        "echo CALLED; echo LIMIT; echo ENDED"
    };
    FakeHarness::running("/bin/sh", &["-c", script])
        .reading("CALLED", vec![called()])
        .reading("LIMIT", vec![quota("rejected", Some(RESETS))])
        .reading("ENDED", vec![ended()])
}

fn fleet_over(home: &TempDir, harness: FakeHarness, clock: Arc<Held>) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    ok();\n"),
        harness,
    );
    fittings.starting().workflows = one(the_step());
    fittings.clock = clock;
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a limit"));
    Fleet::assembled(fittings)
}

async fn held(stays: bool) -> (TempDir, Fixture, Arc<Held>, JobId) {
    let home = TempDir::new();
    let clock = Arc::new(Held {
        ticks: AtomicU64::new(0),
        pushed: AtomicU64::new(0),
    });
    let fleet = fleet_over(&home, a_drone_the_limit_stops(stays), Arc::clone(&clock));
    let job = fleet.propose(a_proposal("make the parser take it")).await.unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();
    let id = job.id().clone();
    until("the Job was never held", 600, Duration::from_millis(20), || async {
        fleet.turn().await.expect("a turn");
        (fleet.load(&id).await.unwrap().status() != JobStatus::Running).then_some(())
    })
    .await;
    (home, fleet, clock, id)
}

async fn a_held_job_is_parked_not_failed(stays: bool) {
    let (_home, fleet, clock, id) = held(stays).await;
    let record = fleet.load(&id).await.unwrap();

    assert_eq!(record.status(), JobStatus::Queued, "not escalated, not failed");
    let pause = record.pause().expect("held by a pause marker");
    assert_eq!(pause.by, core_model::PausedBy::Fleet);
    assert_eq!(
        pause.until.as_ref().map(Timestamp::as_str),
        Some("2026-09-02T05:00:00.000Z"),
        "the reset itself, which admission waits a minute past"
    );
    assert_eq!(record.worktree_slot(), None, "the slot is given back");
    let step = record.step(&StepId::new("implement")).expect("the step");
    assert_eq!(step.state(), StepState::Running, "the step never stopped");
    let spent = fleet.store().lock().await.step_spent(&id, &StepId::new("implement")).unwrap();
    assert_eq!(
        fleet.queued_reason(&record).await.unwrap().reason,
        Some(QueuedReason::Paused)
    );
    assert!(fleet.working_on().await.is_empty());

    admit(&fleet).await.unwrap();
    assert_eq!(
        fleet.load(&id).await.unwrap().status(),
        JobStatus::Queued,
        "admission leaves it alone before the reset"
    );

    clock.pushed.fetch_add(6 * 3_600, Ordering::SeqCst);
    admit(&fleet).await.unwrap();
    let back = fleet.load(&id).await.unwrap();
    assert_eq!(back.status(), JobStatus::Running, "resumed once the limit reset");
    assert!(back.pause().is_none());
    assert_eq!(
        back.step(&StepId::new("implement")).map(|row| row.state()),
        Some(StepState::Running),
        "the same step, taken again"
    );
    assert_eq!(
        fleet.store().lock().await.step_spent(&id, &StepId::new("implement")).unwrap(),
        spent,
        "no retry was spent on the wait"
    );
}

#[tokio::test]
async fn a_drone_the_limit_stopped_mid_vigil_holds_its_job_until_the_reset() {
    a_held_job_is_parked_not_failed(true).await;
}

#[tokio::test]
async fn a_drone_the_limit_stopped_that_already_exited_holds_its_job_too() {
    a_held_job_is_parked_not_failed(false).await;
}
