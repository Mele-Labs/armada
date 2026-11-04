//! The Operator's Log, kept a row an arrival and read back newest first, with its calls counted.

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
