//! The Operator's Log, kept a row an arrival and read back newest first, with its calls counted,
//! one Job's rows, and the latest notice a Job was told of a pull request.

use crate::tests::{open, TempDir};
use crate::KeptRuling;

fn ruled(arrival: &str, call: Option<&str>, at: &str) -> KeptRuling {
    KeptRuling {
        arrival: arrival.to_string(),
        arrived: "start".to_string(),
        source: "job".to_string(),
        source_session: Some("01PARENT".to_string()),
        session_id: format!("01{arrival}"),
        manifest_id: "01REPOSITORY".to_string(),
        subject: "Agent Opens Terminal in Tab".to_string(),
        outcome: "merge".to_string(),
        other_session: Some("01OTHER".to_string()),
        shared: vec!["panes/terminal.tsx".to_string(), "panes/host.tsx".to_string()],
        because: "the same group is in the other's plan".to_string(),
        call: call.map(str::to_string),
        by: "operator".to_string(),
        at: at.to_string(),
        place: None,
        first_session: None,
        refused: Vec::new(),
        answer: None,
        issue: None,
    }
}

#[test]
fn a_ruling_reads_back_whole_with_its_shared_paths_and_signer() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_operator_ruling(&ruled("a", Some("c1"), "2026-11-04T10:00:00.000Z")).expect("kept");
    let passed = KeptRuling {
        outcome: "passed_on".to_string(),
        other_session: None,
        shared: Vec::new(),
        source_session: None,
        ..ruled("b", None, "2026-11-04T10:01:00.000Z")
    };
    store.add_operator_ruling(&passed).expect("kept");
    let placed = KeptRuling {
        outcome: "keep_in_touch".to_string(),
        place: Some("ahead:01WAITING".to_string()),
        first_session: Some("01OTHER".to_string()),
        refused: vec!["ahead of 01GONE, which was not waiting".to_string(), "a second".to_string()],
        ..ruled("c", Some("c2"), "2026-11-04T10:02:00.000Z")
    };
    store.add_operator_ruling(&placed).expect("kept");

    let log = store.operator_rulings(10).expect("read");
    assert_eq!(log, vec![placed, passed, ruled("a", Some("c1"), "2026-11-04T10:00:00.000Z")], "newest first, every column");
    assert_eq!(store.operator_rulings(1).expect("read").len(), 1);
}

#[test]
fn the_calls_of_a_day_are_counted_once_each_and_a_row_with_no_call_is_not_one() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (arrival, call, at) in [
        ("yesterday", Some("c0"), "2026-11-03T23:59:00.000Z"),
        ("a", Some("c1"), "2026-11-04T09:00:00.000Z"),
        ("b", Some("c1"), "2026-11-04T09:00:00.000Z"),
        ("c", Some("c2"), "2026-11-04T09:05:00.000Z"),
        ("ceiling", None, "2026-11-04T09:06:00.000Z"),
    ] {
        store.add_operator_ruling(&ruled(arrival, call, at)).expect("kept");
    }
    assert_eq!(store.operator_calls_since("2026-11-04T00:00:00.000Z").expect("counted"), 2);
    assert_eq!(store.operator_calls_since("2026-11-03T00:00:00.000Z").expect("counted"), 3);
}

#[test]
fn a_jobs_rows_read_back_oldest_first_and_an_arrival_in_the_log_reads_as_ruled_and_since_when() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let nudged = KeptRuling { arrived: "stopped".to_string(), outcome: "nudge".to_string(), ..ruled("stopped:01a:t1", Some("c1"), "2026-11-04T10:00:00.000Z") };
    let again = KeptRuling { session_id: "01a".to_string(), ..ruled("stopped:01a:t2", Some("c2"), "2026-11-04T11:00:00.000Z") };
    let nudged = KeptRuling { session_id: "01a".to_string(), ..nudged };
    for one in [&nudged, &ruled("other", None, "2026-11-04T10:30:00.000Z"), &again] {
        store.add_operator_ruling(one).expect("kept");
    }

    assert_eq!(store.operator_rulings_of("01a").expect("read"), vec![nudged, again]);
    assert!(store.operator_ruled("stopped:01a:t1").expect("read"));
    assert!(!store.operator_ruled("stopped:01a:t3").expect("read"));
    assert!(store.operator_ruled_since("stopped:01a:t1", "2026-11-04T10:00:00.000Z").expect("read"), "at the instant counts");
    assert!(!store.operator_ruled_since("stopped:01a:t1", "2026-11-04T10:00:01.000Z").expect("read"));
}

#[test]
fn the_latest_notice_a_job_was_told_of_a_pull_request_names_its_commit_and_when() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let at = |when: &str| core_model::Timestamp::from_rfc3339(when);
    for (head, recipient, when) in [
        ("1111111", "session:01a", "2026-11-04T09:00:00.000Z"),
        ("2222222", "session:01a", "2026-11-04T10:00:00.000Z"),
        ("3333333", "session:01b", "2026-11-04T11:00:00.000Z"),
    ] {
        store.keep_pull_notice("/repo", 7, head, "checks_failed", recipient, &at(when)).expect("kept");
    }
    store.keep_pull_notice("/repo", 7, "4444444", "conflicts", "session:01a", &at("2026-11-04T12:00:00.000Z")).expect("kept");

    assert_eq!(
        store.pull_notice_latest(7, "checks_failed", "session:01a").expect("read"),
        Some(("2222222".to_string(), "2026-11-04T10:00:00.000Z".to_string()))
    );
    assert_eq!(store.pull_notice_latest(8, "checks_failed", "session:01a").expect("read"), None);
}

#[test]
fn the_persons_answers_to_one_question_read_back_newest_first_and_the_operators_are_not_among_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let asked = "Keep the scrollback per tab?";
    let answered = |arrival: &str, by: &str, answer: &str, at: &str| KeptRuling {
        arrived: "question".to_string(),
        subject: asked.to_string(),
        outcome: if by == "human" { "answered" } else { "answer" }.to_string(),
        by: by.to_string(),
        answer: Some(answer.to_string()),
        ..ruled(arrival, None, at)
    };
    store.add_operator_ruling(&answered("first", "human", "Per tab", "2026-11-04T10:00:00.000Z")).expect("kept");
    store.add_operator_ruling(&answered("second", "operator", "Shared", "2026-11-04T10:30:00.000Z")).expect("kept");
    store.add_operator_ruling(&answered("third", "human", "Shared", "2026-11-04T11:00:00.000Z")).expect("kept");
    let elsewhere = KeptRuling { manifest_id: "01ELSEWHERE".to_string(), ..answered("fourth", "human", "Per tab", "2026-11-04T12:00:00.000Z") };
    store.add_operator_ruling(&elsewhere).expect("kept");

    let read = store.operator_answers_to("01REPOSITORY", asked).expect("read");
    let said: Vec<(&str, Option<&str>)> = read.iter().map(|row| (row.session_id.as_str(), row.answer.as_deref())).collect();
    assert_eq!(said, vec![("01third", Some("Shared")), ("01first", Some("Per tab"))]);
    assert!(store.operator_answers_to("01REPOSITORY", "Something else?").expect("read").is_empty());
}
