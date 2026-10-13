//! Which bay a Job held and when, kept as one row a stint.

use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{KeptBay, Store};

const JOB: &str = "01BAYS00000000000000000001";

fn a_job(store: &mut Store) {
    store
        .insert_job(&top_level(JOB), &created_at())
        .expect("the job is stored");
}

fn bay(slot: u32, since: &str, until: Option<&str>) -> KeptBay {
    KeptBay {
        slot,
        since: since.to_string(),
        until: until.map(str::to_string),
    }
}

#[test]
fn a_bay_taken_and_given_back_is_one_stint_with_both_times() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);

    store.bay_taken(&job_id(JOB), 3, "2026-10-13T09:10:00Z").unwrap();
    assert_eq!(
        store.bays_of(&job_id(JOB)).unwrap(),
        [bay(3, "2026-10-13T09:10:00Z", None)]
    );
    store.bay_given_back(&job_id(JOB), "2026-10-13T11:40:00Z").unwrap();

    assert_eq!(
        store.bays_of(&job_id(JOB)).unwrap(),
        [bay(3, "2026-10-13T09:10:00Z", Some("2026-10-13T11:40:00Z"))]
    );
}

#[test]
fn coming_back_to_the_same_bay_after_a_pause_is_a_second_stint_and_the_first_stays() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);

    store.bay_taken(&job_id(JOB), 3, "2026-10-13T09:10:00Z").unwrap();
    store.bay_given_back(&job_id(JOB), "2026-10-13T10:00:00Z").unwrap();
    store.bay_taken(&job_id(JOB), 3, "2026-10-13T11:00:00Z").unwrap();

    assert_eq!(
        store.bays_of(&job_id(JOB)).unwrap(),
        [
            bay(3, "2026-10-13T09:10:00Z", Some("2026-10-13T10:00:00Z")),
            bay(3, "2026-10-13T11:00:00Z", None),
        ]
    );
}

#[test]
fn taking_the_bay_already_held_changes_nothing_and_taking_another_closes_the_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);

    store.bay_taken(&job_id(JOB), 3, "2026-10-13T09:10:00Z").unwrap();
    store.bay_taken(&job_id(JOB), 3, "2026-10-13T09:20:00Z").unwrap();
    store.bay_taken(&job_id(JOB), 5, "2026-10-13T09:30:00Z").unwrap();

    assert_eq!(
        store.bays_of(&job_id(JOB)).unwrap(),
        [
            bay(3, "2026-10-13T09:10:00Z", Some("2026-10-13T09:30:00Z")),
            bay(5, "2026-10-13T09:30:00Z", None),
        ]
    );
}

#[test]
fn a_job_that_held_no_bay_has_none_and_giving_back_is_not_a_fault() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);

    store.bay_given_back(&job_id(JOB), "2026-10-13T09:10:00Z").unwrap();

    assert!(store.bays_of(&job_id(JOB)).unwrap().is_empty());
}

#[test]
fn the_stints_survive_a_restart_and_go_with_the_job_when_it_is_forgotten() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        a_job(&mut store);
        store.bay_taken(&job_id(JOB), 2, "2026-10-13T09:10:00Z").unwrap();
    }
    let mut store = open(&dir);
    assert_eq!(store.bays_of(&job_id(JOB)).unwrap().len(), 1);

    store.forget_job(&job_id(JOB)).expect("forgotten");

    assert!(store.bays_of(&job_id(JOB)).unwrap().is_empty());
}
