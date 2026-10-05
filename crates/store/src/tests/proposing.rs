//! A Job dispatched at `proposing`, written and read back across each road
//! out of it. #1714.

use core_model::{
    Actor, EscalationTrigger, Facts, Job, JobNumber, JobStatus, ManifestId, ModelName, NewProposal,
    ProposalId, Target, TopLevelOrigin, Ulid, WorkflowId,
};

use crate::tests::{at, created_at, full_new_job, job_id, open, title, ulid, TempDir};
use crate::WriteError;

fn proposing(id: &str, number: u32) -> Job {
    Job::create_proposing(
        NewProposal {
            id: job_id(id),
            title: title("the log reader drops the last line of every file"),
            owner_manifest_id: ManifestId::carried(ulid("01OWNERMANIFEST")),
            model: ModelName::new("a-model-name").expect("a model name"),
            proposal_id: ProposalId::carried(ulid("01PROPOSALREAD")),
            number: JobNumber::carried(number),
            facts: Facts::new("the log reader drops the last line of every file"),
            attachments: Vec::new(),
        },
        TopLevelOrigin::Manual,
        created_at(),
    )
}

#[test]
fn a_job_at_proposing_reads_back_with_no_frozen_workflow() {
    let dir = TempDir::new();
    let stored = proposing("01PROPOSING", 9001);
    let mut store = open(&dir);
    store.insert_job(&stored, &created_at()).expect("stored");
    drop(store);

    let loaded = open(&dir).load_job(&job_id("01PROPOSING")).expect("loads");
    assert_eq!(loaded, stored);
    assert_eq!(loaded.status(), JobStatus::Proposing);
    assert_eq!(loaded.frozen_workflow(), None);
}

#[test]
fn a_settled_workflow_survives_a_call_that_died() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = proposing("01SETTLED", 9002);
    store.insert_job(&job, &created_at()).expect("stored");
    let settled = job.workflow_settled(WorkflowId::carried(Ulid::carried("bug")));
    store.record_workflow_settled(&settled).expect("settled");
    let escalated = settled
        .transition(
            Target::Escalated(EscalationTrigger::ProposerFailed),
            Actor::Fleet,
            at("2026-08-26T09:05:00.000Z"),
        )
        .expect("proposing -> escalated");
    store.record_transition(&escalated).expect("recorded");
    drop(store);

    let loaded = open(&dir).load_job(&job_id("01SETTLED")).expect("loads");
    assert_eq!(loaded, escalated.job);
    assert_eq!(loaded.workflow_id().as_str(), "bug");
    assert_eq!(loaded.frozen_workflow(), None);
}

#[test]
fn an_answered_job_reads_back_as_the_answer_froze_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = proposing("01ANSWERED", 9003);
    store.insert_job(&job, &created_at()).expect("stored");
    let answered = job
        .answered(
            full_new_job("01ANSWERED").into_answer(),
            Actor::Fleet,
            at("2026-08-26T09:05:00.000Z"),
        )
        .expect("proposing -> awaiting_approval");
    store.record_answered(&answered).expect("answered");
    drop(store);

    let loaded = open(&dir).load_job(&job_id("01ANSWERED")).expect("loads");
    assert_eq!(loaded, answered.job);
    assert_eq!(loaded.steps().len(), 2);
    assert!(loaded.frozen_workflow().is_some());
    // And the machine carries on from it like any other Job at the gate.
    assert!(loaded
        .transition(Target::Queued, Actor::Human, at("2026-08-26T09:06:00.000Z"))
        .is_ok());
}

#[test]
fn an_answer_lands_on_a_job_still_at_proposing_and_nowhere_else() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = proposing("01RACED", 9004);
    store.insert_job(&job, &created_at()).expect("stored");
    let answered = job
        .answered(
            full_new_job("01RACED").into_answer(),
            Actor::Fleet,
            at("2026-08-26T09:05:00.000Z"),
        )
        .expect("an answer");
    let stopped = job
        .transition(Target::Killed, Actor::Human, at("2026-08-26T09:04:00.000Z"))
        .expect("a stop");
    store
        .record_transition(&stopped)
        .expect("the stop landed first");

    assert!(matches!(
        store.record_answered(&answered),
        Err(WriteError::StatusChanged { .. })
    ));
}

#[test]
fn a_splits_extra_keeps_the_head_it_names() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let extra = Job::create_split(
        full_new_job("01EXTRA"),
        job_id("01HEAD"),
        TopLevelOrigin::Manual,
        created_at(),
    );
    store.insert_job(&extra, &created_at()).expect("stored");
    drop(store);

    let loaded = open(&dir).load_job(&job_id("01EXTRA")).expect("loads");
    assert_eq!(loaded, extra);
    let by = loaded.dispatched_by().expect("names the head");
    assert_eq!(by.job_id, job_id("01HEAD"));
    assert_eq!(by.step_id, None);
}

/// **A proposal sent back and answered again survives a reload**, whether the
/// Job was born at `proposing` or at the gate: the log holds the round trip, the
/// columns hold the last answer, and the rebuild replays the first through the
/// machine and reads the second from the columns.
#[test]
fn a_proposal_sent_back_and_answered_again_reads_back_with_the_last_answer() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = proposing("01REVISED", 9005);
    store.insert_job(&job, &created_at()).expect("stored");
    let first = job
        .answered(
            full_new_job("01REVISED").into_answer(),
            Actor::Fleet,
            at("2026-08-26T09:05:00.000Z"),
        )
        .expect("answered");
    store.record_answered(&first).expect("answered");
    let back = first
        .job
        .sent_back_to_the_proposer(Actor::Human, at("2026-08-26T09:06:00.000Z"))
        .expect("sent back");
    store.record_transition(&back).expect("recorded");
    assert_eq!(
        open_status(&mut store, "01REVISED"),
        JobStatus::Proposing,
        "mid-call it reloads as the Job it is"
    );

    let mut revised = full_new_job("01REVISED").into_answer();
    revised.title = title("a revised title");
    let second = back
        .job
        .answered(revised, Actor::Fleet, at("2026-08-26T09:07:00.000Z"))
        .expect("answered again");
    store.record_answered(&second).expect("answered again");
    drop(store);

    let loaded = open(&dir).load_job(&job_id("01REVISED")).expect("loads");
    assert_eq!(loaded.status(), JobStatus::AwaitingApproval);
    assert_eq!(loaded.title().as_str(), "a revised title");
    assert_eq!(
        loaded.steps().len(),
        2,
        "the rows were replaced, not doubled"
    );
}

fn open_status(store: &mut crate::Store, id: &str) -> JobStatus {
    store.load_job(&job_id(id)).expect("loads").status()
}

#[test]
fn a_job_created_at_the_gate_and_sent_back_reads_back() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = crate::tests::top_level("01GATEBORN");
    store.insert_job(&job, &created_at()).expect("stored");
    let back = job
        .sent_back_to_the_proposer(Actor::Human, at("2026-08-26T09:06:00.000Z"))
        .expect("sent back");
    store.record_transition(&back).expect("recorded");
    let again = back
        .job
        .answered(
            back.job.as_answered(),
            Actor::Fleet,
            at("2026-08-26T09:07:00.000Z"),
        )
        .expect("answered");
    store.record_answered(&again).expect("answered");
    drop(store);
    let loaded = open(&dir).load_job(&job_id("01GATEBORN")).expect("loads");
    assert_eq!(loaded.status(), JobStatus::AwaitingApproval);
    assert_eq!(loaded.steps().len(), job.steps().len());
}
