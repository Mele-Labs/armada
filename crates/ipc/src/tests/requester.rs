//! Each kind of requester spells the ids it needs and no others, and a record
//! from before the field reads as outside.

use crate::{decode, encode, Requester};

fn spelled(requester: &Requester) -> String {
    encode(requester).expect("plain data")
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
fn outside_is_a_value_and_is_what_nothing_reads_as() {
    assert_eq!(spelled(&Requester::outside()), r#"{"kind":"outside"}"#);
    assert_eq!(Requester::default(), Requester::outside());
}

#[test]
fn a_requester_reads_back_whole() {
    let session = Requester::session("s9", 10);
    assert_eq!(
        decode::<Requester>("a requester", spelled(&session).as_bytes()).expect("reads"),
        session
    );
}

#[test]
fn a_record_written_before_the_jobs_went_still_reads() {
    let old = br#"{"kind":"gate","job_id":"01JOB","step":"implement","handle":"7-a-job"}"#;
    let read = decode::<Requester>("a requester", old).expect("reads");
    assert_eq!(read.kind, "gate");
}
