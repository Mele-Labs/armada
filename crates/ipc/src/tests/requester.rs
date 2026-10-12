//! Each kind of requester spells the ids it needs and no others, and a record
//! from before the field reads as outside a Job.

use crate::{decode, encode, DroneId, JobId, Requester, StepId};

fn spelled(requester: &Requester) -> String {
    encode(requester).expect("plain data")
}

#[test]
fn a_gate_names_its_job_and_step() {
    let gate = Requester::gate(&JobId::carried("01JOB"), &StepId::carried("implement"));
    assert_eq!(
        spelled(&gate),
        r#"{"kind":"gate","job_id":"01JOB","step":"implement"}"#
    );
}

#[test]
fn a_drone_on_a_task_names_job_step_task_and_drone() {
    let asked = Requester::drone_on_task(
        &JobId::carried("01JOB"),
        &StepId::carried("implement"),
        "T3",
        &DroneId::carried("01DRONE"),
    );
    assert_eq!(
        spelled(&asked),
        r#"{"kind":"drone_task","job_id":"01JOB","step":"implement","task_id":"T3","drone_id":"01DRONE"}"#
    );
}

#[test]
fn a_drone_on_a_step_names_no_task() {
    let asked = Requester::drone_on_step(
        &JobId::carried("01JOB"),
        &StepId::carried("implement"),
        &DroneId::carried("01DRONE"),
    );
    assert_eq!(
        spelled(&asked),
        r#"{"kind":"drone_step","job_id":"01JOB","step":"implement","drone_id":"01DRONE"}"#
    );
}

#[test]
fn the_merge_line_names_the_branch() {
    assert_eq!(
        spelled(&Requester::merge_line("fleet/a-branch")),
        r#"{"kind":"merge_line","branch":"fleet/a-branch"}"#
    );
}

#[test]
fn a_session_names_itself_and_its_slot() {
    assert_eq!(
        spelled(&Requester::session("s9", 10)),
        r#"{"kind":"session","session_id":"s9","slot":10}"#
    );
}

#[test]
fn outside_a_job_is_a_value_and_is_what_nothing_reads_as() {
    assert_eq!(spelled(&Requester::outside()), r#"{"kind":"outside"}"#);
    assert_eq!(Requester::default(), Requester::outside());
}

#[test]
fn a_requester_reads_back_whole() {
    let gate = Requester::gate(&JobId::carried("01JOB"), &StepId::carried("implement"));
    assert_eq!(
        decode::<Requester>("a requester", spelled(&gate).as_bytes()).expect("reads"),
        gate
    );
}

#[test]
fn a_requester_carries_the_jobs_handle_where_it_is_given_one() {
    let gate = Requester::gate(&JobId::carried("01JOB"), &StepId::carried("implement"))
        .with_handle("7-a-job");
    assert_eq!(
        spelled(&gate),
        r#"{"kind":"gate","job_id":"01JOB","step":"implement","handle":"7-a-job"}"#
    );
}

#[test]
fn an_answer_cut_short_says_so_and_a_whole_one_says_nothing() {
    let whole = crate::ManifestChecks {
        rows: Vec::new(),
        total: 0,
        truncated: false,
    };
    assert_eq!(
        encode(&whole).expect("plain data"),
        r#"{"rows":[],"total":0}"#
    );
    let cut = crate::ManifestChecks {
        truncated: true,
        ..whole
    };
    assert!(encode(&cut)
        .expect("plain data")
        .contains(r#""truncated":true"#));
}
