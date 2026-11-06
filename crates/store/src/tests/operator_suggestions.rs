//! What the Operator would have done, waiting on one press: the newest for an arrival stands alone,
//! and the first press settles it.

use crate::tests::{open, TempDir};
use crate::KeptSuggestion;

fn suggestion(arrival: &str, session: &str, because: &str) -> KeptSuggestion {
    KeptSuggestion {
        n: 0,
        arrival: arrival.to_string(),
        arrived: "start".to_string(),
        way: "existing_job".to_string(),
        session_id: session.to_string(),
        manifest_id: "01REPOSITORY".to_string(),
        subject: "Fix the clock".to_string(),
        words: "The clock drifts".to_string(),
        group: Some("g1".to_string()),
        other_session: Some("other".to_string()),
        shared: vec!["a.rs".to_string(), "b.rs".to_string()],
        first_session: None,
        step: None,
        item: None,
        pick: None,
        because: because.to_string(),
        what: None,
        text: "The Operator would hand this work over".to_string(),
        raised_at: "2026-11-06T09:00:00Z".to_string(),
    }
}

#[test]
fn a_suggestion_keeps_what_it_would_do_and_stands_on_its_job_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_operator_suggestion(&suggestion("start:s1", "s1", "same work")).expect("kept");
    store.keep_operator_suggestion(&suggestion("start:s2", "s2", "other work")).expect("kept");

    let read = store.operator_suggestions_of("s1").expect("read");
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].shared, ["a.rs", "b.rs"]);
    assert_eq!((read[0].group.as_deref(), read[0].other_session.as_deref()), (Some("g1"), Some("other")));
    assert_eq!(store.operator_suggestion(read[0].n).expect("read"), Some(read[0].clone()));
}

#[test]
fn an_arrival_ruled_again_stands_as_its_newest() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_operator_suggestion(&suggestion("start:s1", "s1", "first read")).expect("kept");
    store.keep_operator_suggestion(&suggestion("start:s1", "s1", "second read")).expect("kept");

    let read = store.operator_suggestions_of("s1").expect("read");
    assert_eq!(read.iter().map(|one| one.because.as_str()).collect::<Vec<_>>(), ["second read"]);
}

#[test]
fn the_first_press_settles_it_and_a_second_finds_it_settled() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_operator_suggestion(&suggestion("start:s1", "s1", "same work")).expect("kept");
    let n = store.operator_suggestions_of("s1").expect("read")[0].n;

    assert!(store.settle_operator_suggestion(n, true, "2026-11-06T09:01:00Z").expect("settled"));
    assert!(!store.settle_operator_suggestion(n, false, "2026-11-06T09:02:00Z").expect("read"), "the first press won");
    assert!(store.operator_suggestions_of("s1").expect("read").is_empty());
    assert_eq!(store.operator_suggestion(n).expect("read"), None);
}
