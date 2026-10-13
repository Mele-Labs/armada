//! What wakes a Job's Commander is written down once per fact, and counted per
//! problem, so a tick or a restart never wakes it twice and three wakes that
//! moved nothing can be told from three that did. `#2167`.

use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{CommanderProblem, CommanderWake, Store, WriteError};

const JOB: &str = "01COMMANDER00000000000002";

fn a_job(store: &mut Store) {
    store
        .insert_job(&top_level(JOB), &created_at())
        .expect("the job is stored");
}

fn stalled(attempt: u64) -> CommanderProblem {
    CommanderProblem {
        trigger: String::from("nothing_handed_in"),
        step: String::from("build"),
        attempt,
    }
}

#[test]
fn a_fact_wakes_once_however_often_it_is_seen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);

    let first = store
        .note_commander_wake(&job_id(JOB), &stalled(1), "escalated")
        .expect("noted");
    let again = store
        .note_commander_wake(&job_id(JOB), &stalled(1), "escalated")
        .expect("noted");

    assert_eq!(first, CommanderWake::New { run: 1 });
    assert_eq!(again, CommanderWake::Already);
}

#[test]
fn a_fact_noted_before_a_restart_is_still_noted_after_it() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        a_job(&mut store);
        store
            .note_commander_wake(&job_id(JOB), &stalled(1), "escalated")
            .expect("noted");
    }
    let mut reopened = open(&dir);
    assert_eq!(
        reopened
            .note_commander_wake(&job_id(JOB), &stalled(1), "escalated")
            .expect("noted"),
        CommanderWake::Already
    );
}

#[test]
fn the_same_problem_again_with_nothing_moved_is_the_next_run() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);

    let runs: Vec<CommanderWake> = (1..=4)
        .map(|attempt| {
            store
                .note_commander_wake(&job_id(JOB), &stalled(attempt), "escalated|steps=stopped")
                .expect("noted")
        })
        .collect();

    assert_eq!(
        runs,
        [1, 2, 3, 4].map(|run| CommanderWake::New { run }),
        "each is new, and each counts the ones before it"
    );
}

#[test]
fn a_job_that_moved_between_two_wakes_starts_the_count_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);

    store
        .note_commander_wake(&job_id(JOB), &stalled(1), "escalated|tasks=open:3")
        .expect("noted");
    store
        .note_commander_wake(&job_id(JOB), &stalled(2), "escalated|tasks=open:3")
        .expect("noted");
    let after_a_task_was_handed_in = store
        .note_commander_wake(&job_id(JOB), &stalled(3), "escalated|tasks=open:2,done:1")
        .expect("noted");

    assert_eq!(after_a_task_was_handed_in, CommanderWake::New { run: 1 });
}

#[test]
fn another_trigger_or_another_step_is_another_problem() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    store
        .note_commander_wake(&job_id(JOB), &stalled(1), "escalated")
        .expect("noted");

    let other_step = CommanderProblem {
        step: String::from("review"),
        ..stalled(2)
    };
    let other_trigger = CommanderProblem {
        trigger: String::from("drift"),
        ..stalled(2)
    };

    assert_eq!(
        store
            .note_commander_wake(&job_id(JOB), &other_step, "escalated")
            .expect("noted"),
        CommanderWake::New { run: 1 }
    );
    assert_eq!(
        store
            .note_commander_wake(&job_id(JOB), &other_trigger, "escalated")
            .expect("noted"),
        CommanderWake::New { run: 1 }
    );
}

#[test]
fn a_job_that_is_forgotten_takes_its_wakes_with_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    store
        .note_commander_wake(&job_id(JOB), &stalled(1), "escalated")
        .expect("noted");
    store.forget_job(&job_id(JOB)).expect("forgotten");
    assert!(matches!(
        store.note_commander_wake(&job_id(JOB), &stalled(1), "escalated"),
        Err(WriteError::NoSuchJob { .. })
    ));
}

/// A trouble that does not move the step's attempt is counted by what has been
/// noted of it, so each of its facts is new and none reads as one already seen.
#[test]
fn the_wakes_noted_for_a_trigger_on_a_step_are_counted_whatever_their_attempt() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    let repair = |attempt| CommanderProblem {
        trigger: String::from("repair_handed_nothing_in"),
        step: String::from("build"),
        attempt,
    };
    let count = |store: &Store| {
        store
            .commander_wakes_noted(&job_id(JOB), "repair_handed_nothing_in", "build")
            .expect("counted")
    };

    assert_eq!(count(&store), 0);
    store
        .note_commander_wake(&job_id(JOB), &repair(count(&store) + 1), "escalated")
        .expect("noted");
    store
        .note_commander_wake(&job_id(JOB), &repair(count(&store) + 1), "escalated")
        .expect("noted");
    store
        .note_commander_wake(&job_id(JOB), &stalled(1), "escalated")
        .expect("another trigger's");

    assert_eq!(count(&store), 2, "two repairs, and the other trigger not among them");
}
