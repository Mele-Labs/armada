//! Fleet pauses a parked Job by itself when work waits for a full pool.
//! `docs/concepts/fleet.md`, *A paused Job gives its slot back*.
//!
//! The clock is [`Held`], so the fifteen minutes are a push and not a wait.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::SlotParkRefused;
use config::Manifest;
use core_model::{Job, JobId, JobStatus, PausedBy, StepState};
use testkit::FakeWorkProduct;

use crate::slots::Concurrency;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, note_evidence, one, two_steps_gated_on_a_person,
    worktree_directory,
};
use crate::tests::planted::Held;
use crate::tests::reviewing::at_the_gate;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = crate::daemon::Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct>;

pub(super) const GRACE_SECONDS: u64 = 15 * 60;

fn root(home: &TempDir) -> String {
    home.path().to_string_lossy().to_string()
}

/// A Fleet whose pool is `size` slots, the rest an agent's, over a Manifest
/// saying `setup`.
pub(super) fn a_pool_of(
    home: &TempDir,
    size: u32,
    setup: &str,
    clock: &Arc<Held>,
    gate_on: &str,
) -> Fixture {
    a_bounded_pool_of(home, size, 4, setup, clock, gate_on)
}

/// The same with `drones` Drones at once, which the fixtures otherwise hold to one.
fn a_bounded_pool_of(
    home: &TempDir,
    size: u32,
    drones: u32,
    setup: &str,
    clock: &Arc<Held>,
    gate_on: &str,
) -> Fixture {
    let mut fitted = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fitted.concurrency = Concurrency::of(drones as usize);
    fitted.clock = Arc::clone(clock) as _;
    fitted.starting().workflows = one(two_steps_gated_on_a_person(
        gate_on,
        None,
        Some("summarise"),
    ));
    fitted.starting().manifest = Manifest::parse(
        Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\n{setup}"),
    )
    .expect("a manifest");
    let fleet = crate::daemon::Fleet::assembled(fitted);
    for n in (size + 1)..=8 {
        fleet.vcs().hold_slot(&root(home), n, "an agent's session");
    }
    fleet
}

/// Three Jobs at a gate, a minute apart: the first is the oldest.
pub(super) async fn three_at_a_gate(
    fleet: &Fixture,
    home: &TempDir,
    clock: &Held,
) -> (JobId, JobId, JobId) {
    let first = at_the_gate(fleet, home).await;
    clock.on(60);
    let second = at_the_gate(fleet, home).await;
    clock.on(60);
    let third = at_the_gate(fleet, home).await;
    (first, second, third)
}

/// An approved Job with no slot to go to.
pub(super) async fn a_waiter(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("proposed");
    worktree_directory(home, &job);
    fleet.approve(job.id()).await.expect("approved");
    job.id().clone()
}

fn paused_by_fleet(job: &Job) -> bool {
    job.pause().is_some_and(|pause| pause.by == PausedBy::Fleet)
}

fn steps_of(job: &Job) -> Vec<(String, StepState)> {
    job.steps()
        .iter()
        .map(|row| (row.step_id().as_str().to_string(), row.state()))
        .collect()
}

#[tokio::test]
async fn the_oldest_parked_job_is_paused_for_a_waiter_that_then_starts_in_its_slot() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(&home, 3, "", &clock, "implement");
    let (first, second, third) = three_at_a_gate(&fleet, &home, &clock).await;
    let waiter = a_waiter(&fleet, &home, "the one that waits").await;

    // Inside the window nobody is taken, though the pool is full.
    let turned = fleet.turn().await.expect("a turn");
    assert!(turned.released.is_empty(), "inside the grace window");
    assert_eq!(
        fleet.load(&waiter).await.unwrap().status(),
        JobStatus::Queued
    );

    clock.on(GRACE_SECONDS);
    let turned = fleet.turn().await.expect("a turn");
    assert_eq!(turned.released, vec![first.clone()], "the oldest, once");
    let taken = fleet.load(&first).await.unwrap();
    assert_eq!(
        taken.status(),
        JobStatus::AwaitingReview,
        "it keeps its gate"
    );
    assert!(paused_by_fleet(&taken));
    assert_eq!(taken.worktree_slot(), None);
    assert_eq!(
        fleet.vcs().parked_slots(),
        vec![(1, first.as_str().to_string())],
        "its work is on its branch"
    );

    let turned = fleet.turn().await.expect("a turn");
    assert_eq!(turned.admitted, vec![waiter.clone()]);
    assert_eq!(fleet.load(&waiter).await.unwrap().worktree_slot(), Some(1));

    // Nothing is paused twice, and nobody else is touched.
    let pause_at = fleet.load(&first).await.unwrap().pause().cloned();
    for _ in 0..3 {
        let turned = fleet.turn().await.expect("a turn");
        assert!(turned.released.is_empty());
    }
    assert_eq!(fleet.load(&first).await.unwrap().pause().cloned(), pause_at);
    for other in [&second, &third] {
        assert!(fleet.load(other).await.unwrap().pause().is_none());
    }
}

#[tokio::test]
async fn the_job_log_names_the_waiter_that_needed_the_slot() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(&home, 3, "", &clock, "implement");
    let (first, _, _) = three_at_a_gate(&fleet, &home, &clock).await;
    let waiter = a_waiter(&fleet, &home, "the one that waits").await;
    clock.on(GRACE_SECONDS);
    fleet.turn().await.expect("a turn");

    let handle = fleet.load(&first).await.unwrap().handle();
    let log = std::fs::read_to_string(crate::transcript::log_of(&root(&home), &handle))
        .expect("the Job's log");
    assert!(log.contains("paused by Fleet"), "{log}");
    assert!(log.contains("the one that waits"), "{log}");
    assert!(log.contains(waiter.as_str()), "{log}");
}

#[tokio::test]
async fn a_job_inside_the_grace_window_is_skipped_and_the_next_is_taken() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(&home, 3, "", &clock, "implement");
    let (first, second, third) = three_at_a_gate(&fleet, &home, &clock).await;
    clock.on(GRACE_SECONDS);
    // A person takes the oldest: pauses it and resumes it.
    fleet.pause_job(&first).await.expect("paused");
    fleet.resume_job(&first).await.expect("resumed");
    let waiter = a_waiter(&fleet, &home, "the one that waits").await;

    let turned = fleet.turn().await.expect("a turn");

    assert_eq!(
        turned.released,
        vec![second.clone()],
        "not the one just resumed"
    );
    assert!(fleet.load(&first).await.unwrap().pause().is_none());
    assert!(fleet.load(&third).await.unwrap().pause().is_none());
    let turned = fleet.turn().await.expect("a turn");
    assert_eq!(turned.admitted, vec![waiter]);
}

#[tokio::test]
async fn with_auto_release_off_nothing_is_paused() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(
        &home,
        3,
        "setup:\n  auto_release: false\n",
        &clock,
        "implement",
    );
    let (first, second, third) = three_at_a_gate(&fleet, &home, &clock).await;
    let waiter = a_waiter(&fleet, &home, "the one that waits").await;
    clock.on(GRACE_SECONDS * 4);

    for _ in 0..3 {
        let turned = fleet.turn().await.expect("a turn");
        assert!(turned.released.is_empty());
    }

    for job in [&first, &second, &third] {
        assert!(fleet.load(job).await.unwrap().pause().is_none());
    }
    assert_eq!(
        fleet.load(&waiter).await.unwrap().status(),
        JobStatus::Queued
    );
}

#[tokio::test]
async fn the_window_is_the_manifests() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(
        &home,
        3,
        "setup:\n  auto_release_grace_minutes: 60\n",
        &clock,
        "implement",
    );
    let (first, _, _) = three_at_a_gate(&fleet, &home, &clock).await;
    a_waiter(&fleet, &home, "the one that waits").await;

    clock.on(GRACE_SECONDS * 2);
    assert!(
        fleet.turn().await.unwrap().released.is_empty(),
        "thirty minutes is inside sixty"
    );
    clock.on(GRACE_SECONDS * 3);
    assert_eq!(fleet.turn().await.unwrap().released, vec![first]);
}

#[tokio::test]
async fn a_finished_job_and_a_running_one_are_never_taken() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(&home, 2, "", &clock, "summarise");
    let done = fleet.propose(a_proposal("the finished one")).await.unwrap();
    worktree_directory(&home, &done);
    dispatched(&fleet, done.id()).await.expect("dispatched");
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.expect("advances");
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet.turn().await.expect("held for review");
    let done = fleet.approve_review(done.id()).await.expect("taken");
    assert_eq!(done.status(), JobStatus::CompletedSuccess);
    let running = fleet.propose(a_proposal("the running one")).await.unwrap();
    worktree_directory(&home, &running);
    let running = dispatched(&fleet, running.id()).await.expect("dispatched");
    assert_eq!(running.status(), JobStatus::Running);
    a_waiter(&fleet, &home, "the one that waits").await;
    clock.on(GRACE_SECONDS * 4);

    for _ in 0..3 {
        assert!(fleet.turn().await.unwrap().released.is_empty());
    }

    assert!(fleet.load(done.id()).await.unwrap().pause().is_none());
    assert!(fleet.load(running.id()).await.unwrap().pause().is_none());
}

#[tokio::test]
async fn fleet_never_resumes_what_it_paused_and_a_person_takes_it_back_with_its_gate() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(&home, 3, "", &clock, "implement");
    let (first, _, _) = three_at_a_gate(&fleet, &home, &clock).await;
    let before = fleet.load(&first).await.unwrap();
    let waiter = a_waiter(&fleet, &home, "the one that waits").await;
    clock.on(GRACE_SECONDS);
    fleet.turn().await.unwrap();
    fleet.turn().await.unwrap();
    // The waiter ends and its slot is free, with a Fleet-paused Job parked.
    fleet.kill_job(&waiter).await.expect("the waiter ends");

    for _ in 0..3 {
        let turned = fleet.turn().await.unwrap();
        assert!(turned.reseated.is_empty() && turned.released.is_empty());
    }
    assert!(
        paused_by_fleet(&fleet.load(&first).await.unwrap()),
        "still paused"
    );

    let back = fleet.resume_job(&first).await.expect("a person resumes it");

    assert_eq!(back.status(), JobStatus::AwaitingReview);
    assert_eq!(steps_of(&back), steps_of(&before));
    assert!(back.worktree_slot().is_some() && back.pause().is_none());
    fleet
        .approve_review(&first)
        .await
        .expect("and it can be answered");
}

#[tokio::test]
async fn a_resume_never_forces_another_out_and_a_resumed_job_is_not_taken_back_at_once() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(&home, 3, "", &clock, "implement");
    let (first, second, third) = three_at_a_gate(&fleet, &home, &clock).await;
    let waiter = a_waiter(&fleet, &home, "the one that waits").await;
    clock.on(GRACE_SECONDS);
    fleet.turn().await.unwrap();
    fleet.turn().await.unwrap();
    assert!(paused_by_fleet(&fleet.load(&first).await.unwrap()));

    // The pool is full again and a person resumes the Job: it waits.
    let waiting = fleet.resume_job(&first).await.expect("a resume is taken");
    assert!(waiting.pause().is_some_and(|pause| pause.resuming));
    for _ in 0..3 {
        let turned = fleet.turn().await.unwrap();
        assert!(turned.released.is_empty(), "a resume forces nobody out");
    }
    for other in [&second, &third] {
        assert!(fleet.load(other).await.unwrap().pause().is_none());
    }

    // A slot frees and it comes back. Another waiter then finds it inside its
    // window, so the next oldest is the one taken, not this one again.
    fleet.kill_job(&waiter).await.expect("the waiter ends");
    let turned = fleet.turn().await.unwrap();
    assert_eq!(turned.reseated, vec![first.clone()]);
    let again = a_waiter(&fleet, &home, "the next one").await;
    let turned = fleet.turn().await.unwrap();
    assert_eq!(turned.released, vec![second.clone()]);
    assert!(fleet.load(&first).await.unwrap().pause().is_none());
    let turned = fleet.turn().await.unwrap();
    assert_eq!(turned.admitted, vec![again]);
}

#[tokio::test]
async fn a_refused_park_skips_the_job_and_is_not_asked_again_inside_the_window() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(&home, 3, "", &clock, "implement");
    let (first, second, third) = three_at_a_gate(&fleet, &home, &clock).await;
    let waiter = a_waiter(&fleet, &home, "the one that waits").await;
    clock.on(GRACE_SECONDS);
    fleet.vcs().refuse_next_park(SlotParkRefused::Busy);

    let turned = fleet.turn().await.unwrap();

    assert_eq!(turned.released, vec![second.clone()], "the next is taken");
    assert!(fleet.load(&first).await.unwrap().pause().is_none());
    fleet.turn().await.unwrap();
    assert_eq!(
        fleet.load(&waiter).await.unwrap().status(),
        JobStatus::Running
    );

    // Another waiter, and the refused Job is still inside its window.
    a_waiter(&fleet, &home, "another").await;
    let turned = fleet.turn().await.unwrap();
    assert_eq!(
        turned.released,
        vec![third.clone()],
        "never the refused one"
    );
    assert!(fleet.load(&first).await.unwrap().pause().is_none());
}

#[tokio::test]
async fn a_full_drone_bound_pauses_nobody_since_a_slot_would_not_start_the_waiter() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_bounded_pool_of(&home, 2, 1, "", &clock, "implement");
    let gated = at_the_gate(&fleet, &home).await;
    let running = fleet.propose(a_proposal("the running one")).await.unwrap();
    worktree_directory(&home, &running);
    dispatched(&fleet, running.id()).await.expect("dispatched");
    a_waiter(&fleet, &home, "the one that waits").await;
    clock.on(GRACE_SECONDS * 4);

    for _ in 0..3 {
        assert!(fleet.turn().await.unwrap().released.is_empty());
    }

    assert!(fleet.load(&gated).await.unwrap().pause().is_none());
}

#[tokio::test]
async fn a_persons_resume_of_a_running_job_waits_for_a_slot_and_forces_nobody_out() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = a_pool_of(&home, 2, "", &clock, "implement");
    let gated = at_the_gate(&fleet, &home).await;
    let running = fleet.propose(a_proposal("the running one")).await.unwrap();
    worktree_directory(&home, &running);
    let running = dispatched(&fleet, running.id()).await.expect("dispatched");
    fleet
        .pause_job(running.id())
        .await
        .expect("a person pauses it");
    // An agent takes the slot it freed, then the person resumes it.
    fleet.vcs().hold_slot(&root(&home), 2, "an agent's session");
    let waiting = fleet.resume_job(running.id()).await.expect("resumed");
    assert!(waiting.pause().is_some_and(|pause| pause.resuming));
    clock.on(GRACE_SECONDS * 4);

    for _ in 0..3 {
        assert!(fleet.turn().await.unwrap().released.is_empty());
    }

    assert!(fleet.load(&gated).await.unwrap().pause().is_none());
}
