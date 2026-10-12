//! The Commander's claim: **When a Job's Drone ends with nothing handed in, or
//! a group is red with its retries spent, the Job's Commander steers it
//! without the owner. It reads the Job, then raises the cap inside its
//! ceiling, splits or restarts a task, redirects a Drone, or sends a repair
//! Drone onto the Job's branch. The owner sees in the Job's thread and on each
//! task's mark that the Commander did it, and he can write to the Commander
//! and get an answer. Where the fix is out of its reach (skip or edit a Check,
//! edit the Manifest, pass a gate, attest, merge, push), it puts one card to
//! him. The Job never advances on anything the Commander said.** #2163.

//! Merged green. Each step below cannot be asserted yet; the issue that builds
//! it adds its assertion beside the code and deletes its row.
//!
//! | Step of the claim | Carried by |
//! |---|---|
//! | It reaches a Job, and is offered nothing it may not do (a Check, the Manifest, a gate, attest, merge, push) | #2164 |
//! | A cap it raises stops at three times the workflow's caps, and a raise cannot claim a person made it | #2165 |
//! | The owner writes to it and gets an answer, a message queues, and one card goes to him | #2166 |
//! | A Drone with nothing handed in, or a red group, wakes it; three wakes on one problem stop it | #2167 |
//! | Every change to a plan is signed by whoever made it | #2168 |
//! | The owner sees in the thread and on each task's mark that it acted, and answers a card | #2169 |
//! | A repair Drone goes onto the Job's own branch | #2170 |

//! What is asserted today is the fourth signer: the record a Commander's act
//! would leave, made by moving the machines as `Actor::Commander`, through
//! `ipc::encode` and back. The fake hosting that scripts its tool calls comes
//! with #2164, the first issue with a tool to call.

// Shared with the other milestones' tests, and none of them uses all of it.
#[allow(dead_code)]
mod bench;

use core_model::{Actor, Attempt, JobStatus, PlanAuthor, StepId, Via};
use testkit::FakeWorkProduct;

use bench::commander::{
    a_plan_recorded_by, a_step_the_commander_stopped, status_moves_received,
    stopped_by_fleet_and_resumed_by_the_commander,
};
use bench::plan::PLAN;
use bench::Bench;

fn a_bench() -> Bench {
    Bench::with(FakeWorkProduct::untouched())
}

#[test]
fn a_commanders_move_on_a_job_reaches_a_bridge_as_the_commanders() {
    let bench = a_bench();
    stopped_by_fleet_and_resumed_by_the_commander(&bench);

    let received = status_moves_received(&bench);

    let signers: Vec<&str> = received.iter().map(|moved| moved.actor.as_wire()).collect();
    assert_eq!(
        signers,
        vec!["human", "fleet", "fleet", "commander"],
        "approved by a person, run and stopped by Fleet, resumed by the Commander"
    );
    let resumed = received.last().expect("the Commander moved the Job");
    assert_eq!(
        (resumed.from.domain(), resumed.to.domain()),
        (JobStatus::Escalated, JobStatus::Running)
    );
}

#[test]
fn a_step_the_commander_stopped_reads_as_the_commanders_beside_fleets_own() {
    let bench = a_bench();
    let mut run = bench.created("raise the bound");
    bench.approved_and_dispatched(&mut run);
    let step = bench.step(0);

    let (moved, bytes) = a_step_the_commander_stopped(&bench, &mut run, &step);

    assert_eq!(moved.actor.as_wire(), "commander");
    assert!(
        bytes.contains(r#""actor":"commander""#),
        "the spelling a client dispatches on is on the wire: {bytes}"
    );
    assert_eq!(
        bench::arc::step_signers(&bench),
        vec![Actor::Fleet, Actor::Commander],
        "the dispatch was Fleet's, the stop the Commander's"
    );
}

#[test]
fn a_plan_the_commander_recorded_is_told_apart_from_a_steps_and_a_persons() {
    let by_commander = a_plan_recorded_by(PlanAuthor::Commander);
    let by_person = a_plan_recorded_by(PlanAuthor::Person);
    let by_step = a_plan_recorded_by(PlanAuthor::Step {
        step_id: StepId::new(PLAN),
        attempt: Attempt::stored(1).expect("one-based"),
    });

    assert_eq!(by_commander.0.recorded_by, ipc::ChangedBy::Commander);
    assert!(
        by_commander.1.contains(r#""by":"commander""#),
        "{}",
        by_commander.1
    );
    assert_eq!(by_person.0.recorded_by, ipc::ChangedBy::Person);
    assert!(matches!(
        by_step.0.recorded_by,
        ipc::ChangedBy::Step { ref step_id, attempt: 1 } if step_id.as_str() == PLAN
    ));
}

#[test]
fn the_commander_is_one_more_signer_and_one_more_door_and_not_a_person() {
    let signers: Vec<&str> = Actor::ALL.iter().map(Actor::as_wire).collect();
    let mut distinct = signers.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), signers.len(), "no two signers share a spelling");
    assert_eq!(Actor::from_wire("commander"), Some(Actor::Commander));

    assert_eq!(Via::from_wire("commander"), Some(Via::Commander));
    assert!(!Via::Commander.is_a_person());
    assert_ne!(Via::Commander, Via::Helm, "Helm's door is not the Commander's");
}
