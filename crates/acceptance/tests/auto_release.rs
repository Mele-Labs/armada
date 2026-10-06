//! Auto-release's claim: **when other work is waiting for a worktree slot and
//! the pool is full, Fleet frees one by pausing the parked Job that has waited
//! longest, leaves alone one a person may be looking at, and gives nothing back
//! on its own — a Job paused for a waiter comes back, gate and all, when a
//! person resumes it.**
//!
//! Asserted without a repository: the victim rule is `fleet::release_order`,
//! a function over instants, and the gate is a Job the two machines moved to
//! `awaiting_review`, then paused and resumed through the record's writers.
//!
//! | Proved | Not proved |
//! |---|---|
//! | Which parked Job is taken first, and that one inside the grace window is skipped for the next | That the pass finds a waiter, pauses and starts it in the freed slot. That is `Fleet::release_for_waiters` over a store and a pool, and `fleet`'s `tests::releasing` drives it through fakes |
//! | That a pause by Fleet keeps the status, the steps and the gate, and a resume restores them | That Fleet never resumes one by itself, and that a person's Resume forces nobody out. Both are what `seat_resuming` and the waiter rule do, in the same tests |
//! | That nothing is chosen while every parked Job is inside the window | That `setup.auto_release: false` stops the pass. `config` tests the key and `fleet` tests the pass |

// The bench is shared with the other milestones' tests and none of them uses
// all of it.
#[allow(dead_code)]
mod bench;

use core_model::{Job, JobId, JobStatus, PausedBy, Ulid};
use fleet::{release_order, Parked};
use testkit::{FakeJudge, FakeWorkProduct};

use bench::board::on_its_branch;
use bench::landing::{a_handoff_note, sends_it_out};
use bench::{a_fix_diff, a_root_cause_note, states, Bench, Run};

const MINUTE: i64 = 60_000;
const GRACE: i64 = 15 * MINUTE;

fn parked(id: &str, in_status_since: i64, last_moved: i64) -> Parked {
    Parked {
        job: JobId::carried(Ulid::carried(id)),
        in_status_since,
        last_moved,
    }
}

fn id_of(parked: &Parked) -> JobId {
    parked.job.clone()
}

// ---------------------------------------------------------------------------
// Which one
// ---------------------------------------------------------------------------

#[test]
fn the_job_that_has_waited_longest_at_its_status_is_taken_first() {
    let old = parked("01AAAAAAAAAAAAAAAAAAAAAAAA", 0, 0);
    let newer = parked("01BBBBBBBBBBBBBBBBBBBBBBBB", 5 * MINUTE, 5 * MINUTE);
    let order = release_order(&[newer.clone(), old.clone()], 30 * MINUTE, GRACE);
    assert_eq!(order, vec![id_of(&old), id_of(&newer)]);
}

#[test]
fn a_tie_goes_to_the_lower_job_id() {
    let low = parked("01AAAAAAAAAAAAAAAAAAAAAAAA", 0, 0);
    let high = parked("01BBBBBBBBBBBBBBBBBBBBBBBB", 0, 0);
    let order = release_order(&[high.clone(), low.clone()], 30 * MINUTE, GRACE);
    assert_eq!(order, vec![id_of(&low), id_of(&high)]);
}

#[test]
fn a_job_inside_the_grace_window_is_skipped_for_the_next() {
    let oldest_but_just_looked_at = parked("01AAAAAAAAAAAAAAAAAAAAAAAA", 0, 20 * MINUTE);
    let next = parked("01BBBBBBBBBBBBBBBBBBBBBBBB", 5 * MINUTE, 5 * MINUTE);
    let order = release_order(
        &[oldest_but_just_looked_at.clone(), next.clone()],
        30 * MINUTE,
        GRACE,
    );
    assert_eq!(order, vec![id_of(&next)]);
}

#[test]
fn nothing_is_taken_while_every_parked_job_is_inside_the_window() {
    let one = parked("01AAAAAAAAAAAAAAAAAAAAAAAA", 0, 20 * MINUTE);
    let two = parked("01BBBBBBBBBBBBBBBBBBBBBBBB", 0, 29 * MINUTE);
    assert!(release_order(&[one, two], 30 * MINUTE, GRACE).is_empty());
}

// ---------------------------------------------------------------------------
// What a pause leaves
// ---------------------------------------------------------------------------

/// A Job the machines moved to `awaiting_review`, holding a slot.
async fn a_job_at_the_gate() -> (Run, Bench) {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        sends_it_out(),
        FakeJudge::that_fails("a Judge that should never be asked"),
    );
    let mut run = bench.created("fix the cursor that reads past the end");
    on_its_branch(&mut run);
    bench.approved_and_dispatched(&mut run);
    for (at, note) in [
        (0, a_root_cause_note()),
        (1, a_fix_diff()),
        (2, a_handoff_note()),
    ] {
        let step = bench.step(at);
        let ruling = bench.gate(&run, &step, &note).await;
        bench.settled(&mut run, &step, &ruling);
    }
    assert_eq!(run.job.status(), JobStatus::AwaitingReview);
    run.job = run.job.in_slot(1);
    (run, bench)
}

#[tokio::test]
async fn a_pause_by_fleet_keeps_the_gate_and_a_resume_restores_it() {
    let (run, _) = a_job_at_the_gate().await;
    let before: Job = run.job.clone();
    let at = core_model::Timestamp::from_rfc3339("2026-10-06T09:00:00.000Z");

    let paused = before.paused(PausedBy::Fleet, at);

    assert_eq!(paused.status(), JobStatus::AwaitingReview, "its gate");
    assert_eq!(states(&paused), states(&before), "its steps");
    assert_eq!(paused.worktree_slot(), None, "its slot is given back");
    assert!(paused
        .pause()
        .is_some_and(|pause| pause.by == PausedBy::Fleet));
    assert!(
        !paused.pause().is_some_and(|pause| pause.resuming),
        "and nothing is waiting to take it back"
    );

    let back = paused.in_slot(2).unpaused();

    assert_eq!(back.status(), JobStatus::AwaitingReview);
    assert_eq!(states(&back), states(&before));
    assert!(back.pause().is_none());
}
