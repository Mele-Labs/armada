//! Steps added to one Job: written beside the workflow, read back whole, and
//! removable only until their moment has come.

use core_model::{
    AddedKind, Fired, Kept, NotRun, OnTriggerFailure, Placed, StepId, Timestamp, TriggerState,
    TriggerWhen,
};

use crate::tests::{job_id, open, top_level, TempDir};
use crate::{NewAddition, Removal};

fn at(s: &str) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-10-07T09:00:{s}.000Z"))
}

fn script(command: &str, when: TriggerWhen, step: &str) -> NewAddition {
    NewAddition {
        kind: AddedKind::Script {
            command: command.to_string(),
        },
        when,
        step: StepId::new(step),
        on_failure: OnTriggerFailure {
            block: false,
            repair: true,
        },
    }
}

#[test]
fn an_addition_reads_back_whole_after_a_reopen_and_its_firing_is_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01ADDED"), &crate::tests::created_at())
        .expect("stored");
    let id = job_id("01ADDED");

    let placed = store
        .place_additions_at_approval(
            &id,
            &[
                script("fmt", TriggerWhen::StepPasses, "implement"),
                NewAddition {
                    kind: AddedKind::Drone {
                        brief: "read it twice".to_string(),
                    },
                    when: TriggerWhen::StepStarts,
                    step: StepId::new("summarise"),
                    on_failure: OnTriggerFailure::default(),
                },
            ],
            &at("01"),
        )
        .expect("placed");
    assert_eq!(
        placed.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(),
        ["a1", "a2"]
    );
    let running = Fired::running(at("02"));
    store
        .set_addition_fired(&id, "a1", &running)
        .expect("opened");
    store
        .set_addition_fired(&id, "a1", &running.clone().ended(Some(0), false, at("03")))
        .expect("settled");
    store
        .set_addition_kept(&id, "a1", Kept::Machine)
        .expect("kept");
    store
        .set_addition_fired(
            &id,
            "a2",
            &Fired::skipped(NotRun::DroneStepNotRun, at("04")),
        )
        .expect("skipped");
    drop(store);

    let store = open(&dir);
    let read = store.job_additions(&id).expect("read");
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].placed, Placed::AtApproval);
    assert_eq!(read[0].kept, Some(Kept::Machine));
    assert!(read[0].on_failure.repair);
    let fired = read[0].fired.as_ref().expect("fired");
    assert_eq!(
        (fired.state, fired.exit_code),
        (TriggerState::Passed, Some(0))
    );
    assert_eq!(fired.started_at, at("02"));
    assert_eq!(
        read[1]
            .fired
            .as_ref()
            .and_then(|fired| fired.not_run.clone()),
        Some(NotRun::DroneStepNotRun)
    );
}

#[test]
fn a_second_approval_replaces_the_draft_and_leaves_what_was_added_while_running() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01REPLACE"), &crate::tests::created_at())
        .expect("stored");
    let id = job_id("01REPLACE");
    store
        .place_additions_at_approval(
            &id,
            &[script("fmt", TriggerWhen::StepPasses, "a")],
            &at("01"),
        )
        .expect("placed");
    store
        .add_job_step(
            &id,
            &script("lint", TriggerWhen::StepPasses, "b"),
            Placed::WhileRunning,
            &at("02"),
        )
        .expect("added");
    let again = store
        .place_additions_at_approval(
            &id,
            &[script("tidy", TriggerWhen::StepStarts, "a")],
            &at("03"),
        )
        .expect("placed again");
    assert_eq!(again[0].id, "a3", "an id is not handed out twice");

    let read = store.job_additions(&id).expect("read");
    let held: Vec<_> = read.iter().map(|one| one.kind.text().to_string()).collect();
    assert_eq!(held, ["lint", "tidy"]);
}

#[test]
fn only_an_addition_that_has_not_fired_can_be_removed_and_its_row_stays() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .insert_job(&top_level("01REMOVE"), &crate::tests::created_at())
        .expect("stored");
    let id = job_id("01REMOVE");
    let one = script("fmt", TriggerWhen::StepPasses, "a");
    let waiting = store
        .add_job_step(&id, &one, Placed::WhileRunning, &at("01"))
        .expect("added");
    let fired = store
        .add_job_step(&id, &one, Placed::WhileRunning, &at("02"))
        .expect("added");
    store
        .set_addition_fired(&id, &fired.id, &Fired::running(at("03")))
        .expect("fired");

    assert_eq!(
        store.remove_job_step(&id, &fired.id, &at("04")).unwrap(),
        Removal::Fired
    );
    assert_eq!(
        store.remove_job_step(&id, &waiting.id, &at("04")).unwrap(),
        Removal::Removed
    );
    assert_eq!(
        store.remove_job_step(&id, &waiting.id, &at("05")).unwrap(),
        Removal::NoSuch
    );
    assert_eq!(
        store.remove_job_step(&id, "a9", &at("05")).unwrap(),
        Removal::NoSuch
    );

    let read = store.job_additions(&id).expect("read");
    assert_eq!(
        read.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(),
        ["a2"]
    );
    let next = store
        .add_job_step(&id, &one, Placed::WhileRunning, &at("06"))
        .expect("added");
    assert_eq!(next.id, "a3", "a removed one still holds its number");
}
