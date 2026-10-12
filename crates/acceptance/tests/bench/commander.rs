//! The apparatus the Commander's claim is asserted against, and none of it
//! asserts anything.
//!
//! Separate from `mod.rs` for `arc`'s reason: Commander asks who signed what
//! a Job's record holds, which is a question of the same machinery.
//!
//! **No Commander exists yet to drive.** What this builds is the record as a
//! Commander's act would leave it, by moving the machines with
//! [`Actor::Commander`] as the signer, and the wire as a Bridge receives it.
//! The hosting that scripts a Commander's tool calls arrives with #2164, the
//! first issue that gives it something to call; it is added here then.

use core_model::{
    Actor, EscalationTrigger, PlanAuthor, PlanEntry, StepId, StepLevelTrigger,
    StepTarget, Timestamp, WorkPlan,
};
use ipc::{Event, JobStateChanged, JobStepAdvanced, JobSummary};

use super::plan::{called, received_event};
use super::{Bench, Run};

/// A Job dispatched and then stopped by Fleet, which its Commander then ran
/// again: **three signers on four moves** — a person approved it, Fleet ran it
/// and stopped it, and the Commander resumed it.
pub fn stopped_by_fleet_and_resumed_by_the_commander(bench: &Bench) -> Run {
    let mut run = bench.created("raise the bound");
    bench.approved_and_dispatched(&mut run);
    bench.moved(
        &mut run,
        core_model::Target::Escalated(EscalationTrigger::Stalled),
        Actor::Fleet,
    );
    bench.moved(&mut run, core_model::Target::Running, Actor::Commander);
    run
}

/// Every status move the bench recorded, as a connected client receives it.
pub fn status_moves_received(bench: &Bench) -> Vec<JobStateChanged> {
    bench
        .moves
        .borrow()
        .iter()
        .map(|event| {
            match received_event(&Event::JobStateChanged(JobStateChanged::from(event))) {
                Event::JobStateChanged(moved) => moved,
                other => panic!("a state change came back as {other:?}"),
            }
        })
        .collect()
}

/// The step a Commander stops, as a connected client receives the move and the
/// bytes it arrived in. The bytes are handed back for `board.rs`'s reason: the
/// spelling on the wire is what a client dispatches on.
pub fn a_step_the_commander_stopped(
    bench: &Bench,
    run: &mut Run,
    step: &StepId,
) -> (JobStepAdvanced, String) {
    let stopped = StepLevelTrigger::of(EscalationTrigger::NoReport)
        .expect("a step-level trigger in the registry");
    bench.step_moved_by(run, step, StepTarget::Stopped(stopped), Actor::Commander);
    let event = bench
        .step_moves
        .borrow()
        .last()
        .map(|moved| JobStepAdvanced::of(moved, JobSummary::from(&run.job)))
        .expect("the move was recorded");
    let event = Event::JobStepAdvanced(event);
    let bytes = ipc::encode(&event).expect("an event that serialises");
    match received_event(&event) {
        Event::JobStepAdvanced(moved) => (moved, bytes),
        other => panic!("a step move came back as {other:?}"),
    }
}

const A_PLAN: &str = r#"{"approach":"Cover the bound, then widen the reader",
    "tasks":[{"title":"Cover the last row","note":"","scope":[],"expects":""},
             {"title":"Stop the reader at the end","note":"","scope":[],"expects":""}]}"#;

/// The plan as it stands after `by` recorded it whole, as the wire carries it,
/// and the bytes it arrived in.
pub fn a_plan_recorded_by(by: PlanAuthor) -> (ipc::WorkPlan, String) {
    let entry = PlanEntry {
        change: called("record_plan", A_PLAN).change,
        by,
        at: Timestamp::from_rfc3339("2026-10-12T10:00:00.000Z"),
    };
    let plan = WorkPlan::after(None, &entry).expect("a plan that records");
    let wire = ipc::WorkPlan::from(&plan);
    let bytes = ipc::encode(&wire).expect("a plan that serialises");
    let read = ipc::decode("a work plan", bytes.as_bytes()).expect("a plan that reads back");
    (read, bytes)
}
