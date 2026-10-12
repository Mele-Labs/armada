//! How many times a pass went back to its Drone on a refusal the Judges agreed
//! on, counted off the log by who signed the hand-back. #206.
use core_model::{Actor, EscalationTrigger, Job, StepId, StepLevelTrigger, StepTarget};

use crate::tests::attempt::on_its_first_run;
use crate::tests::{at, job_id, open, TempDir};
use crate::Store;

fn fix() -> StepId {
    StepId::new("fix")
}

fn moved(store: &mut Store, job: &Job, to: StepTarget, by: Actor, when: &str) -> Job {
    let moved = job
        .transition_step(&fix(), to, by, at(&format!("2026-10-11T{when}:00.000Z")))
        .unwrap_or_else(|cause| panic!("moving fix: {cause}"));
    store
        .record_step_transition(&moved)
        .expect("the step move is recorded");
    moved.job
}

/// One hand-back, signed by `by`: `retrying` and straight back to `running`.
fn handed_back(store: &mut Store, job: &Job, by: Actor, when: &str) -> Job {
    let why = StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("step-level");
    let job = moved(store, job, StepTarget::Retrying(why), by, when);
    moved(store, &job, StepTarget::Running, Actor::Fleet, when)
}

#[test]
fn only_the_judges_hand_backs_are_counted() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let id = "01SENTBACK";
    let job = job_id(id);
    let running = on_its_first_run(&mut store, id);
    let count = |store: &Store| {
        store
            .step_sent_back_by_judges(&job, &fix())
            .expect("counted")
    };
    assert_eq!(count(&store), 0, "nothing went back yet");

    let running = handed_back(&mut store, &running, Actor::Check, "10:01");
    assert_eq!(
        count(&store),
        0,
        "a failed Check's hand-back is not the Judges'"
    );

    let running = handed_back(&mut store, &running, Actor::Judge, "10:02");
    handed_back(&mut store, &running, Actor::Judge, "10:03");
    assert_eq!(count(&store), 2);
}
