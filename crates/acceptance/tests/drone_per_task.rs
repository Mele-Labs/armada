//! Arc's claim: **a Job's plan is worked by a Drone per task, group by group,
//! and I can see and act on each task, each group and each Drone.** Spike 022,
//! written before slice 1a and added to slice by slice. 1a carries the first
//! step: a Judge's refusal and a Check's failure are signed by them, on the
//! step's stop and on the Job's move as Bridge is served it.
//!
//! | Not asserted here | Why not, and where |
//! |---|---|
//! | A Fleet built before 1a refuses the store | Hermetic, and `store` has no in-memory constructor. `crates/store/src/tests/signers.rs` holds it |
//! | A Drone per task, `handed_in` at its hand-in | Slice 1b, `#1762` |
//! | Groups, and a task that failed | Slice 2 |
//! | A model per task; what a Job will be; several Drones; Jobs under a Job | Slices 3 to 6 |
//!
//! The apparatus is [`bench::arc`].

#[allow(dead_code)]
mod bench;

use core_model::{Actor, JobEvent, JobStatus};
use fleet::Ruling;
use testkit::{FakeJudge, FakeWorkProduct};

use bench::arc::step_signers;
use bench::{a_fix_diff, a_root_cause_note, bug_workflow_with_the_fix_judged, Bench};

/// The actor on a Job's move, after a round trip through the wire.
fn served(event: &JobEvent) -> ipc::Actor {
    let wire = ipc::JobStateChanged::from(event);
    let text = ipc::encode(&wire).expect("a Job's move encodes");
    let back: ipc::JobStateChanged =
        ipc::decode("a Job's move", text.as_bytes()).expect("and decodes");
    back.actor
}

/// A Judge refuses the fix, and both rows the refusal writes are the Judge's.
#[tokio::test]
async fn a_judges_refusal_is_signed_by_the_judge() {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        bug_workflow_with_the_fix_judged(),
        FakeJudge::refusing(
            "a fix addressing the cause the note named",
            "a change to an unrelated bound",
            "the reported symptom still occurs",
        ),
    );
    let mut run = bench.created("fix the cursor that reads one row past the end");
    bench.approved_and_dispatched(&mut run);
    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);

    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    assert!(
        matches!(ruling, Ruling::Refused { .. }),
        "the Judge refuses the fix, and got {ruling:?}"
    );
    bench.settled(&mut run, &bench.step(1), &ruling);

    assert_eq!(run.job.status(), JobStatus::Escalated);
    assert_eq!(
        step_signers(&bench).last(),
        Some(&Actor::Judge),
        "the step the Judge stopped says the Judge stopped it"
    );
    assert_eq!(
        bench.actors(),
        vec![Actor::Human, Actor::Fleet, Actor::Judge],
        "approval is a person's, dispatch Fleet's, and the escalation the Judge's"
    );
    let moves = bench.moves.borrow();
    let escalated = moves.last().expect("the refusal moved the Job");
    assert_eq!(
        served(escalated),
        ipc::Actor::from(Actor::Judge),
        "and Bridge is told the Judge signed it, not Fleet"
    );
}

/// A Check fails with no retry left, and both rows the failure writes are the
/// Check's.
#[tokio::test]
async fn a_failed_check_is_signed_by_the_check() {
    // Nothing changed, so `diff_nonempty` fails.
    let bench = Bench::with(FakeWorkProduct::untouched());
    let mut run = bench.created("change nothing");
    bench.approved_and_dispatched(&mut run);
    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);

    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    assert!(
        matches!(ruling, Ruling::Failed { .. }),
        "an empty diff is a failed Check, and got {ruling:?}"
    );
    bench.settled(&mut run, &bench.step(1), &ruling);

    assert_eq!(run.job.status(), JobStatus::AwaitingRepair);
    assert_eq!(
        step_signers(&bench).last(),
        Some(&Actor::Check),
        "the step the Check stopped says the Check stopped it"
    );
    assert_eq!(
        bench.actors(),
        vec![Actor::Human, Actor::Fleet, Actor::Check],
        "approval is a person's, dispatch Fleet's, and the hold the Check's"
    );
    let moves = bench.moves.borrow();
    let held = moves.last().expect("the failure moved the Job");
    assert_eq!(
        served(held),
        ipc::Actor::from(Actor::Check),
        "and Bridge is told the Check signed it, not Fleet"
    );
}
