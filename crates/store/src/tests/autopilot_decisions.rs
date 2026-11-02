//! What a running Autopilot answered, kept once an item and read back for review.

use crate::tests::{open, TempDir};
use crate::KeptDecision;

fn decision(id: &str, manifest: &str) -> KeptDecision {
    KeptDecision {
        id: id.to_string(),
        manifest_id: manifest.to_string(),
        session_id: Some("01SESSION".to_string()),
        who: "Fix the clock".to_string(),
        kind: "recommended".to_string(),
        asked: "Which clock?".to_string(),
        chose: "Frozen (Recommended)".to_string(),
        because: "the agent marked it".to_string(),
        at: "2026-11-02T09:00:00.000Z".to_string(),
        reviewed: false,
        corrected: None,
    }
}

#[test]
fn an_item_is_answered_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.add_autopilot_decision(&decision("01SESSION:a", "one")).expect("kept"));
    let again = KeptDecision { chose: "Fake".to_string(), ..decision("01SESSION:a", "one") };
    assert!(!store.add_autopilot_decision(&again).expect("kept"), "the id is already there");
    let read = store.autopilot_decision("01SESSION:a").expect("reads").expect("held");
    assert_eq!(read.chose, "Frozen (Recommended)", "the first answer stands");
}

#[test]
fn a_kept_or_corrected_decision_leaves_the_review_and_stays_for_the_helper() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for id in ["a", "b", "c"] {
        store.add_autopilot_decision(&decision(id, "one")).expect("kept");
    }
    assert!(store.review_autopilot_decision("a").expect("reviewed"));
    assert!(store.correct_autopilot_decision("b", "Use the fake one").expect("corrected"));
    assert!(!store.review_autopilot_decision("ghost").expect("asked"));
    let left: Vec<String> = store.autopilot_decisions_to_review().expect("reads").into_iter().map(|one| one.id).collect();
    assert_eq!(left, ["c"]);
    let earlier = store.autopilot_decisions_of("one", 10).expect("reads");
    assert_eq!(earlier.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["c", "b", "a"], "newest first");
    assert_eq!(earlier[1].corrected.as_deref(), Some("Use the fake one"));
}

#[test]
fn a_repositorys_earlier_decisions_are_its_own_and_bounded() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_autopilot_decision(&decision("a", "one")).expect("kept");
    store.add_autopilot_decision(&decision("b", "two")).expect("kept");
    store.add_autopilot_decision(&decision("c", "one")).expect("kept");
    let ids = |limit| -> Vec<String> {
        store.autopilot_decisions_of("one", limit).expect("reads").into_iter().map(|one| one.id).collect()
    };
    assert_eq!(ids(10), ["c", "a"]);
    assert_eq!(ids(1), ["c"]);
}
