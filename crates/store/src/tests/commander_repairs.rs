//! A repair Drone a Commander sent is written down while it is out, so a Fleet
//! that restarts can tell a Job held over one from a Job nobody is working on.
//! `#2170`.

use crate::tests::{created_at, job_id, open, top_level, TempDir};
use crate::{RepairOut, Store, WriteError};

const JOB: &str = "01COMMANDER00000000000003";

fn a_job(store: &mut Store) {
    store
        .insert_job(&top_level(JOB), &created_at())
        .expect("the job is stored");
}

fn out(drone: &str) -> RepairOut {
    RepairOut {
        job_id: job_id(JOB),
        step: String::from("implement"),
        broke: String::from("dependencies"),
        drone_id: drone.to_string(),
    }
}

#[test]
fn a_repair_noted_out_is_still_out_after_a_restart() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        a_job(&mut store);
        store.note_repair_out(&out("01DRONE")).expect("noted");
    }

    let store = open(&dir);

    assert_eq!(store.repairs_out().expect("read"), [out("01DRONE")]);
}

#[test]
fn a_job_has_one_repair_out_and_the_second_is_the_one_held() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);

    store.note_repair_out(&out("01FIRST")).expect("noted");
    store.note_repair_out(&out("01SECOND")).expect("noted");

    assert_eq!(store.repairs_out().expect("read"), [out("01SECOND")]);
}

#[test]
fn a_repair_dealt_with_is_given_back_and_giving_back_twice_is_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    store.note_repair_out(&out("01DRONE")).expect("noted");

    store.repair_dealt_with(&job_id(JOB), "01DRONE").expect("given back");
    store.repair_dealt_with(&job_id(JOB), "01DRONE").expect("and again");

    assert!(store.repairs_out().expect("read").is_empty());
}

#[test]
fn the_row_of_a_repair_sent_since_is_not_given_back_with_the_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    a_job(&mut store);
    store.note_repair_out(&out("01FIRST")).expect("noted");
    store.note_repair_out(&out("01SECOND")).expect("noted");

    store.repair_dealt_with(&job_id(JOB), "01FIRST").expect("the first came back");

    assert_eq!(store.repairs_out().expect("read"), [out("01SECOND")]);
}

#[test]
fn a_repair_for_a_job_the_store_does_not_hold_is_refused() {
    let dir = TempDir::new();
    let mut store = open(&dir);

    let refused = store.note_repair_out(&out("01DRONE")).expect_err("no such Job");

    assert!(matches!(refused, WriteError::NoSuchJob { .. }), "{refused:?}");
}
