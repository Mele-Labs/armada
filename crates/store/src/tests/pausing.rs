//! A pause marker and the slot it gave up survive a reopen, and lifting it
//! writes both back.

use core_model::{Branch, PausedBy};

use crate::tests::{at, job_id, open, top_level, TempDir};

#[test]
fn a_parked_job_reads_back_paused_with_no_slot_and_a_resumed_one_with_its_new_slot() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01PAUSED")
        .on_branch(Branch::new("armada/job-1").expect("a branch"))
        .in_slot(3);
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
    store.record_slot(&job).expect("its slot is recorded");

    let parked = job.paused(PausedBy::Person, at("2026-10-05T10:00:00.000Z"));
    store.record_pause(&parked).expect("the pause is recorded");
    drop(store);

    let mut store = open(&dir);
    let read = store.load_job(&job_id("01PAUSED")).expect("it reads back");
    assert_eq!(read.worktree_slot(), None, "the slot was given up");
    let pause = read.pause().expect("the marker survived the reopen");
    assert_eq!(pause.by, PausedBy::Person);
    assert_eq!(pause.at, at("2026-10-05T10:00:00.000Z"));
    assert!(!pause.resuming);
    assert!(read.is_parked());

    let back = read.resuming().in_slot(5).unpaused();
    store.record_pause(&back).expect("the resume is recorded");
    let read = store.load_job(&job_id("01PAUSED")).expect("it reads back");
    assert_eq!(read.worktree_slot(), Some(5));
    assert!(read.pause().is_none());
}

#[test]
fn a_job_nobody_paused_reads_with_no_marker() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = top_level("01PLAIN");
    store
        .insert_job(&job, &crate::tests::created_at())
        .expect("the job is stored");
    let read = store.load_job(&job_id("01PLAIN")).expect("it reads back");
    assert!(read.pause().is_none());
    assert!(!read.is_parked());
}
