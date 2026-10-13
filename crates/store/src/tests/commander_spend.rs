//! What a Job's Commander spent is kept apart from what its Drones did, so the
//! cap, which reads the Drones', cannot be spent by the agent that raises it.
//! `#2166`.

use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{Store, WriteError};

const JOB: &str = "01COMMANDER00000000000001";

fn a_job(store: &mut Store) {
    store
        .insert_job(&top_level(JOB), &created_at())
        .expect("the job is stored");
}

#[test]
fn a_job_whose_commander_never_woke_has_cost_nothing_for_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    assert_eq!(store.commander_cost_for(&job_id(JOB)).expect("reads"), 0);
}

#[test]
fn each_turn_adds_to_the_commanders_cost_and_none_to_the_drones() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    store
        .record_commander_spend(&job_id(JOB), "turn-1", Some(400_000), 3)
        .expect("recorded");
    store
        .record_commander_spend(&job_id(JOB), "turn-2", Some(250_000), 2)
        .expect("recorded");

    assert_eq!(
        store.commander_cost_for(&job_id(JOB)).expect("reads"),
        650_000
    );
    let drones = store.spend_for(&job_id(JOB)).expect("reads");
    assert_eq!(
        (drones.cost_micros, drones.turns, drones.drones),
        (0, 0, 0),
        "the cap sums this, and a Commander is in none of it"
    );
}

#[test]
fn recording_a_turn_again_replaces_it_and_inflates_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    for _ in 0..3 {
        store
            .record_commander_spend(&job_id(JOB), "turn-1", Some(400_000), 3)
            .expect("recorded");
    }
    assert_eq!(
        store.commander_cost_for(&job_id(JOB)).expect("reads"),
        400_000
    );
}

#[test]
fn a_turn_that_named_no_price_is_a_floor_and_not_a_price_of_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    store
        .record_commander_spend(&job_id(JOB), "turn-1", Some(100_000), 1)
        .expect("recorded");
    store
        .record_commander_spend(&job_id(JOB), "turn-2", None, 0)
        .expect("recorded");
    assert_eq!(
        store.commander_cost_for(&job_id(JOB)).expect("reads"),
        100_000
    );
}

#[test]
fn a_job_that_is_forgotten_takes_its_commanders_spend_with_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    store
        .record_commander_spend(&job_id(JOB), "turn-1", Some(1), 1)
        .expect("recorded");
    store.forget_job(&job_id(JOB)).expect("forgotten");
    assert_eq!(store.commander_cost_for(&job_id(JOB)).expect("reads"), 0);
    assert!(matches!(
        store.record_commander_spend(&job_id(JOB), "turn-2", Some(1), 1),
        Err(WriteError::NoSuchJob { .. })
    ));
}
