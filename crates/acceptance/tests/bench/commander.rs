//! The apparatus the Commander's claim is asserted against, and none of it
//! asserts anything.
//!
//! Separate from `mod.rs` for `arc`'s reason: Commander asks who signed what
//! a Job's record holds, which is a question of the same machinery.
//!
//! **No Commander exists yet to drive.** What this builds is the record as a
//! Commander's act would leave it, by moving the machines with
//! [`Actor::Commander`] as the signer, and the wire as a Bridge receives it.
//! The hosting that runs a Commander, and so scripts its tool calls, arrives
//! with #2166; it is added here then.

use core_model::{
    Actor, DropReason, EscalationTrigger, NewTask, PlanAuthor, PlanChange, PlanEntry, StepId,
    StepLevelTrigger, StepTarget, TaskEdit, TaskId, TaskUpdate, Timestamp, WorkPlan,
};
use ipc::{Event, JobStateChanged, JobStepAdvanced, JobSummary};

use super::plan::{called, received_event, PLAN};
use super::{a_root_cause_note, Bench, Run};

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

/// A Job whose `fix` step stopped with its Drone gone and the Job held over
/// it: where a Commander's repair is sent from. The first step passed.
pub async fn stopped_on_the_fix(bench: &Bench) -> Run {
    let mut run = bench.created("clear the conflict");
    bench.approved_and_dispatched(&mut run);
    let first = bench.step(0);
    let ruling = bench.gate(&run, &first, &a_root_cause_note()).await;
    bench.settled(&mut run, &first, &ruling);
    let stopped = StepLevelTrigger::of(EscalationTrigger::NoReport)
        .expect("a step-level trigger in the registry");
    bench.step_moved(&mut run, &bench.step(1), StepTarget::Stopped(stopped));
    bench.moved(
        &mut run,
        core_model::Target::Escalated(EscalationTrigger::NoReport),
        Actor::Fleet,
    );
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

const A_PLAN_OF_THREE: &str = r#"{"approach":"Cover the bound, then widen the reader",
    "tasks":[{"title":"Cover the last row","note":"","scope":[],"expects":""},
             {"title":"Stop the reader at the end","note":"","scope":[],"expects":""},
             {"title":"Note the bound","note":"","scope":[],"expects":""}]}"#;

/// What a client holds after `by` added a task, edited one, moved the added one
/// and dropped another of a plan a step recorded, T3 left alone: the plan as the wire carries it, the bytes
/// it arrived in, and each `job.plan_changed` as received, with the actor
/// that signed it. `actor` is the signer the caller was placed as.
pub fn a_plan_four_acts_were_made_on(
    by: PlanAuthor,
    actor: Actor,
) -> (ipc::WorkPlan, String, Vec<ipc::JobPlanChanged>) {
    let job = core_model::JobId::carried(core_model::Ulid::carried("01PLANJOB"));
    let step = PlanAuthor::Step {
        step_id: StepId::new(PLAN),
        attempt: core_model::Attempt::stored(1).expect("one-based"),
    };
    let task = |id: &str| TaskId::read(id).expect("an id");
    let changes = [
        PlanChange::Added {
            task: NewTask::new("Cover the empty file", "", &[], "").expect("a title"),
            after: None,
        },
        PlanChange::Edited {
            task: task("T1"),
            edit: TaskEdit::new(Some("Stop one row early"), None, None, None, None)
                .expect("an edit"),
        },
        PlanChange::MovedTask {
            task: task("T4"),
            group: core_model::GroupId::FIRST,
            after: Some(task("T1")),
        },
        PlanChange::Updated {
            task: task("T2"),
            to: TaskUpdate::Dropped(DropReason::new("covered by T1").expect("a reason")),
            shown: None,
        },
    ];
    let recorded = PlanEntry {
        change: called("record_plan", A_PLAN_OF_THREE).change,
        by: step,
        at: Timestamp::from_rfc3339("2026-10-12T10:00:00.000Z"),
    };
    let mut plan = WorkPlan::after(None, &recorded).expect("a plan that records");
    let mut told = Vec::new();
    for (change, second) in changes.into_iter().zip(1u32..) {
        let at = Timestamp::from_rfc3339(&format!("2026-10-12T10:00:{second:02}.000Z"));
        let entry = PlanEntry {
            change,
            by: by.clone(),
            at: at.clone(),
        };
        plan = WorkPlan::after(Some(&plan), &entry).expect("a change the plan takes");
        let event = Event::JobPlanChanged(ipc::JobPlanChanged::changed(
            &job,
            &entry.change,
            &plan,
            actor,
            &at,
        ));
        match received_event(&event) {
            Event::JobPlanChanged(received) => told.push(received),
            other => panic!("a plan change came back as {other:?}"),
        }
    }
    let wire = ipc::WorkPlan::from(&plan);
    let bytes = ipc::encode(&wire).expect("a plan that serialises");
    let read = ipc::decode("a work plan", bytes.as_bytes()).expect("a plan that reads back");
    (read, bytes, told)
}
