//! An Epic's waves: a member entering at its gate with the pass that made it,
//! when a pull request merged, and what finishes a Job. Spike 022, slice 6.

use core_model::{
    Actor, CompleteWhen, DispatchOrigin, Job, JobStatus, Landing, StepId, Target, Timestamp,
};

use crate::tests::{at, created_at, full_new_job, job_id, open, TempDir};

fn member(id: &str, pass: u32) -> Job {
    Job::create_proposed_member(
        full_new_job(id),
        DispatchOrigin {
            job_id: job_id("01PARENT"),
            step_id: Some(StepId::new("plan")),
            pass: Some(pass),
        },
        created_at(),
    )
}

/// A member is rebuilt at its gate with its pass, and released from it: the
/// column is what tells the rebuild which constructor made it.
#[test]
fn a_proposed_member_keeps_its_pass_and_its_gate_through_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = member("01MEMBER", 2);
    store.insert_job(&job, &created_at()).expect("inserted");
    drop(store);

    let read = open(&dir).load_job(job.id()).expect("it rebuilds");
    assert_eq!(read.status(), JobStatus::AwaitingApproval);
    assert_eq!(read.dispatched_by().and_then(|by| by.pass), Some(2));

    let released = read
        .transition(Target::Queued, Actor::Human, at("2026-10-02T10:01:00.000Z"))
        .expect("released with its wave");
    let mut store = open(&dir);
    store.record_transition(&released).expect("kept");
    drop(store);
    assert_eq!(
        open(&dir).load_job(job.id()).expect("it rebuilds").status(),
        JobStatus::Queued,
        "the log replays from the gate it entered at"
    );
}

/// A child made before V101 has no pass and still rebuilds at `queued`.
#[test]
fn a_child_made_before_waves_still_rebuilds_queued() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = crate::tests::sub_dispatched("01OLDCHILD");
    store.insert_job(&job, &created_at()).expect("inserted");
    let read = store.load_job(job.id()).expect("it rebuilds");
    assert_eq!(read.status(), JobStatus::Queued);
    assert_eq!(read.dispatched_by().and_then(|by| by.pass), None);
}

#[test]
fn when_a_pull_request_merged_is_kept_and_read_for_the_board() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = member("01MERGED", 1);
    store.insert_job(&job, &created_at()).expect("inserted");
    assert_eq!(store.merged_at(job.id()).expect("read"), None);
    let merged = Timestamp::from_rfc3339("2026-10-02T12:00:00Z");
    store.record_merged_at(job.id(), &merged).expect("kept");
    drop(store);
    let store = open(&dir);
    assert_eq!(
        store.merged_at(job.id()).expect("read"),
        Some(merged.clone())
    );
    assert_eq!(
        store.merged_at_by_job().expect("read").get(job.id()),
        Some(&merged)
    );
}

#[test]
fn what_finishes_a_job_is_kept_with_its_landing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = crate::tests::top_level("01PARENTJOB");
    store.insert_job(&job, &created_at()).expect("inserted");
    let landing = Landing {
        complete_when: CompleteWhen::AllMembersLanded,
        ..Landing::as_ever()
    };
    store.set_landing(job.id(), &landing).expect("kept");
    assert_eq!(store.landing(job.id()).expect("read"), Some(landing));
}

#[test]
fn a_step_nothing_returned_to_is_on_its_first_pass() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = crate::tests::top_level("01FIRSTPASS");
    store.insert_job(&job, &created_at()).expect("inserted");
    assert_eq!(
        store
            .pass_over(job.id(), &StepId::new("plan"))
            .expect("counted"),
        1
    );
}
