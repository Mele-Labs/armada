//! A Drone asking a person a question, giving its place back, and a fresh
//! Drone carrying on from the answer.
//!
//! # What these prove, and what they deliberately do not
//!
//! The owner's decision of 11 Oct 2026, as cases: the asking Drone stands
//! down and the Job waits at `queued` reading `awaiting_answer`; its place
//! goes to the next queued Job; the question survives a Fleet restart; and
//! the answer reaches the next Drone's opening brief. Then the refusals.
//!
//! Not proved here: that the Drone *acts* on the answer — nothing in this
//! workspace reads what a Drone said. `Fleet::ask_question` is called with the
//! Job id the peer lookup would have produced, so what is skipped is the socket
//! and nothing about the binding.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use api::Queries;
use config::ResolvedWorkflow;
use core_model::{
    Actor, EscalationTrigger, JobId, JobStatus, StepId, StepState, Target, Timestamp,
};
use ipc::mcp::{AskQuestion, AskedOption};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::clock::Clock;
use crate::converging::StepNorms;
use crate::daemon::Fleet;
use crate::questioning::{NotAnswered, NotAsked};
use crate::silence::Liveness;
use crate::slots::Concurrency;
use crate::tests::admitted::{dispatched, started};
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::restarting::{on_it, the_handle, until_spoken};
use crate::tests::tmp::TempDir;

mod interview;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";

/// How long a Drone may say nothing before the vigil speaks. The production
/// number, so the two cases below are measured against what ships.
const QUIET_AFTER: Duration = Duration::from_secs(120);

/// A clock that ticks a second per reading, and jumps when a test says so.
/// `crate::tests::silence`'s, carried rather than shared because that module's
/// is private to it.
struct Held {
    ticks: AtomicU64,
    pushed: AtomicU64,
}

impl Held {
    fn started() -> Held {
        Held {
            ticks: AtomicU64::new(0),
            pushed: AtomicU64::new(0),
        }
    }

    /// Move the clock on. **Never backwards.**
    fn on(&self, seconds: u64) {
        self.pushed.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for Held {
    fn now(&self) -> Timestamp {
        let at = self.ticks.fetch_add(1, Ordering::SeqCst) + self.pushed.load(Ordering::SeqCst);
        Timestamp::from_rfc3339(format!(
            "2026-08-26T{:02}:{:02}:{:02}.000Z",
            (at / 3_600) % 24,
            (at / 60) % 60,
            at % 60
        ))
    }
}

/// A well-formed question: two answers, each saying what it commits to.
fn a_question() -> AskQuestion {
    AskQuestion {
        question: "Should the store schema change be its own Job?".to_string(),
        options: vec![
            AskedOption {
                label: "Its own Job".to_string(),
                consequence: "dispatch a migration Job first and make the rest depend on it"
                    .to_string(),
            },
            AskedOption {
                label: "Fold it in".to_string(),
                consequence: "the first Job that needs the column adds it".to_string(),
            },
        ],
        decisions: Vec::new(),
    }
}

fn one_step() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: IMPLEMENT,
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn called() -> Vec<DroneEvent> {
    vec![DroneEvent::Called {
        tool: String::from("Read"),
        call: String::from("a-call"),
        detail: CallDetail::of("a file"),
    }]
}

/// A Drone that says one thing and then works on, for as long as it is let.
fn a_drone_that_works() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"]).reading("BUSY", called())
}

/// A Fleet on one step, bounded at `places`. **The Judge fails every call**:
/// nothing here may ask a model anything.
fn watching(home: &TempDir, harness: FakeHarness, clock: Arc<Held>, places: usize) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    panic!();\n"),
        harness,
    );
    fittings.starting().workflows = one(one_step());
    fittings.clock = clock;
    fittings.liveness = Liveness::of(QUIET_AFTER, 2);
    fittings.norms = StepNorms::of(60, Duration::from_secs(1_500), Duration::from_secs(120));
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a question"));
    fittings.concurrency = Concurrency::of(places);
    Fleet::assembled(fittings)
}

fn a_fleet(home: &TempDir) -> Fixture {
    watching(home, a_drone_that_works(), Arc::new(Held::started()), 2)
}

/// Approve a Job, let it take a place, and wait for its Drone to speak.
async fn started_one(fleet: &Fixture, home: &TempDir, title: &str) -> JobId {
    let job = fleet.propose(a_proposal(title)).await.unwrap();
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.unwrap();
    assert_eq!(
        fleet.load(job.id()).await.unwrap().status(),
        JobStatus::Running,
        "the Job took a place"
    );
    job.id().clone()
}

/// Whether any Drone is still held on this Job.
async fn holds_a_drone(fleet: &Fixture, job: &JobId) -> bool {
    match fleet.slot_of(job).await {
        Some(slot) => slot.lock().await.is_some(),
        None => false,
    }
}

/// What the Board says about a Job that has not started.
async fn queued_reason(fleet: &Fixture, job: &JobId) -> Option<String> {
    fleet
        .get_job(ipc::JobId::from(job))
        .await
        .expect("the Job reads")
        .job
        .queued_reason
        .map(|why| why.as_wire().to_string())
}

/// **The requirement, as a case.** A working Drone asks: the question is on
/// the Job, the Drone is gone, and the Job waits at `queued` saying why. The
/// step is still `running`, because the run is not over.
#[tokio::test]
async fn a_drone_that_asks_stands_down_and_the_job_waits_on_the_answer() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = started_one(&fleet, &home, "make the parser take it").await;

    let asked = fleet
        .ask_question(&job, a_question())
        .await
        .expect("a working Drone may ask");

    let record = fleet.load(&job).await.unwrap();
    assert_eq!(
        record.status(),
        JobStatus::Queued,
        "the Drone gave its place back"
    );
    assert_eq!(
        record.step(&StepId::new(IMPLEMENT)).unwrap().state(),
        StepState::Running,
        "and the step is still running: no new status and no new step state"
    );
    assert_eq!(
        record.question(),
        Some(&asked),
        "the question is on the record"
    );
    assert!(
        record.assigned_drone().is_none(),
        "no Drone is recorded on it"
    );
    assert!(!holds_a_drone(&fleet, &job).await, "and none is held");
    assert_eq!(
        queued_reason(&fleet, &job).await.as_deref(),
        Some("awaiting_answer")
    );
    let board = Queries::list_jobs(&fleet, None).await.expect("a board");
    assert!(
        board
            .jobs
            .iter()
            .any(|row| row.id.as_str() == job.as_str() && row.asking),
        "the row says it is waiting on a person, which puts it under Needs you"
    );
}

/// **The point of the decision.** At a bound of one, a Job that asks lets the
/// next queued Job start at once, and is not started again itself.
#[tokio::test]
async fn its_place_goes_to_the_next_queued_job() {
    let home = TempDir::new();
    let fleet = watching(&home, a_drone_that_works(), Arc::new(Held::started()), 1);
    let asking = started_one(&fleet, &home, "make the parser take it").await;
    let waiting = fleet.propose(a_proposal("and the writer")).await.unwrap();
    worktree_directory(&home, &waiting);
    dispatched(&fleet, waiting.id()).await.unwrap();
    assert_eq!(
        fleet.load(waiting.id()).await.unwrap().status(),
        JobStatus::Queued,
        "the one place is taken"
    );

    fleet.ask_question(&asking, a_question()).await.unwrap();
    started(&fleet, waiting.id()).await.unwrap();

    assert_eq!(
        fleet.load(waiting.id()).await.unwrap().status(),
        JobStatus::Running,
        "the place went to the Job waiting for it"
    );
    assert_eq!(
        fleet.load(&asking).await.unwrap().status(),
        JobStatus::Queued,
        "and the asking Job was not started again"
    );
}

/// The wire says so too: `get_job` carries the question, its options and the
/// step, and a client can draw the controls from it without asking anything
/// else.
#[tokio::test]
async fn the_question_reaches_the_wire_whole() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = started_one(&fleet, &home, "make the parser take it").await;
    fleet.ask_question(&job, a_question()).await.unwrap();

    let served = fleet
        .get_job(ipc::JobId::from(&job))
        .await
        .expect("the Job reads")
        .asking
        .expect("a question outstanding is served");
    assert_eq!(served.options.len(), 2);
    assert_eq!(served.options[0].label, "Its own Job");
    assert!(
        !served.options[0].consequence.is_empty(),
        "a label with no consequence is a button whose effect has to be guessed"
    );
    assert_eq!(served.step_id.as_str(), IMPLEMENT);
}

/// **The definition of done.** A Drone asks, Fleet restarts, the person
/// answers hours later, and a fresh Drone picks up from the answer.
#[tokio::test]
async fn the_question_survives_a_restart_and_the_answer_opens_the_next_drones_brief() {
    let home = TempDir::new();
    let first = a_fleet(&home);
    let job = started_one(&first, &home, "make the parser take it").await;
    let asked = first.ask_question(&job, a_question()).await.unwrap();
    drop(first);

    let second = watching(
        &home,
        FakeHarness::that_echoes_its_first_turn(),
        Arc::new(Held::started()),
        2,
    );
    second.reconcile().await.expect("the boot read");
    let record = second.load(&job).await.unwrap();
    assert_eq!(
        record.status(),
        JobStatus::Queued,
        "nothing escalated it as interrupted"
    );
    assert_eq!(
        record.question(),
        Some(&asked),
        "the question outlived the Fleet"
    );
    assert_eq!(
        queued_reason(&second, &job).await.as_deref(),
        Some("awaiting_answer"),
        "and nothing started it while it waited"
    );

    second
        .answer_question(&job, asked.id.as_str(), "Fold it in")
        .await
        .expect("a label the Drone offered");
    started(&second, &job).await.unwrap();

    let record = second.load(&job).await.unwrap();
    assert_eq!(
        record.status(),
        JobStatus::Running,
        "a fresh Drone is on it"
    );
    assert_eq!(
        record.current_step_id().map(StepId::as_str),
        Some(IMPLEMENT),
        "on the same step"
    );
    let said = until_spoken(
        &home,
        &the_handle(&second, &job).await,
        &on_it(&second, &job).await,
    )
    .await;
    assert!(
        said.contains("WHAT A PERSON ANSWERED"),
        "no answer block: {said}"
    );
    assert!(
        said.contains("Should the store schema change be its own Job?"),
        "the question was not restated: {said}"
    );
    assert!(
        said.contains("Fold it in"),
        "the answer was dropped: {said}"
    );
    assert!(
        second.load(&job).await.unwrap().question().is_none(),
        "delivered once, and gone from the record"
    );
}

/// A question on a Job a person has already stopped is a second thing to
/// answer about a Job they have taken over.
#[tokio::test]
async fn a_job_that_is_not_running_takes_no_question() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = started_one(&fleet, &home, "make the parser take it").await;
    let record = fleet.load(&job).await.unwrap();
    fleet
        .move_job(
            &record,
            Target::Escalated(EscalationTrigger::Stalled),
            Actor::Fleet,
        )
        .await
        .unwrap();

    let refused = fleet
        .ask_question(&job, a_question())
        .await
        .expect_err("an escalated Job is a person's");
    assert!(matches!(refused, NotAsked::NotRunning { .. }));
    assert!(
        fleet.load(&job).await.unwrap().question().is_none(),
        "and nothing was kept"
    );
}

/// **A label the Drone never offered is refused rather than passed through.**
/// There is no free-text answer, and this is where that is enforced.
#[tokio::test]
async fn a_label_that_was_not_offered_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = started_one(&fleet, &home, "make the parser take it").await;
    let asked = fleet.ask_question(&job, a_question()).await.unwrap();

    let refused = fleet
        .answer_question(&job, asked.id.as_str(), "do whatever you think best")
        .await
        .expect_err("that is not one of the answers");
    assert!(matches!(refused, NotAnswered::NotOffered { .. }));
    assert!(
        refused.to_string().contains("Its own Job"),
        "and the refusal names what was offered: {refused}"
    );
    assert!(
        fleet.load(&job).await.unwrap().waiting_on_an_answer(),
        "the question still stands"
    );
}

/// **A window left open names a question that is not the one on the Job.**
/// Refused rather than applied to whatever is outstanding: a label matching by
/// coincidence would start work nobody chose.
#[tokio::test]
async fn an_answer_to_a_question_that_is_not_the_one_outstanding_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = started_one(&fleet, &home, "make the parser take it").await;
    fleet.ask_question(&job, a_question()).await.unwrap();

    let refused = fleet
        .answer_question(&job, "01SOMEOTHERQUESTION", "Fold it in")
        .await
        .expect_err("that is not the question on the Job");
    assert!(matches!(refused, NotAnswered::Superseded { .. }));
    assert!(fleet.load(&job).await.unwrap().waiting_on_an_answer());
}

/// Answered once: a second answer, from a second window, finds nothing to
/// answer, and the first stands.
#[tokio::test]
async fn a_question_is_answered_once() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = started_one(&fleet, &home, "make the parser take it").await;
    let asked = fleet.ask_question(&job, a_question()).await.unwrap();
    fleet
        .answer_question(&job, asked.id.as_str(), "Fold it in")
        .await
        .unwrap();

    let refused = fleet
        .answer_question(&job, asked.id.as_str(), "Its own Job")
        .await
        .expect_err("already answered");
    assert!(matches!(refused, NotAnswered::NothingIsAsking { .. }));
    assert_ne!(
        queued_reason(&fleet, &job).await.as_deref(),
        Some("awaiting_answer"),
        "and the Job is no longer held for it"
    );
}

/// Answering a Job whose Drone asked nothing.
#[tokio::test]
async fn a_job_with_nothing_outstanding_takes_no_answer() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = started_one(&fleet, &home, "make the parser take it").await;

    let refused = fleet
        .answer_question(&job, "01WHATEVER", "Fold it in")
        .await
        .expect_err("nothing asked");
    assert!(matches!(refused, NotAnswered::NothingIsAsking { .. }));
}

/// **A person's wait has no budget.** Turn after turn with the clock pushed
/// well past every vigil's threshold, the Job waits at `queued` and nothing
/// starts or escalates it.
#[tokio::test]
async fn a_job_waiting_on_an_answer_is_left_waiting_by_every_turn() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = watching(&home, a_drone_that_works(), Arc::clone(&clock), 2);
    let job = started_one(&fleet, &home, "make the parser take it").await;
    fleet.ask_question(&job, a_question()).await.unwrap();

    for _ in 0..4 {
        clock.on(QUIET_AFTER.as_secs() * 20);
        fleet.turn().await.expect("a turn");
    }

    assert_eq!(fleet.load(&job).await.unwrap().status(), JobStatus::Queued);
    assert_eq!(
        queued_reason(&fleet, &job).await.as_deref(),
        Some("awaiting_answer")
    );
}
