//! A gate's close is kept once: the first fact stands, and a plan's approval is kept per version.

use crate::tests::{open, TempDir};
use crate::KeptGate;

fn gate(group: &str, gate: &str, version: u32, fact: &str) -> KeptGate {
    KeptGate {
        group_id: group.into(),
        gate: gate.into(),
        version,
        closed_at: "2026-11-08T11:00:00.000Z".into(),
        fact: fact.into(),
        source: format!("source of {fact}"),
        commit: Some("1b500c4".into()),
        already_red: vec!["fleet tests::a".into(), "fleet tests::b".into()],
        skipped: false,
    }
}

#[test]
fn a_close_is_kept_once_and_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.close_session_gate("s1", &gate("g2", "checks_green", 0, "Checks passed at 1b500c4")).unwrap());
    assert!(!store.close_session_gate("s1", &gate("g2", "checks_green", 0, "Checks passed later")).unwrap(), "the first fact stands");
    assert!(store.close_session_gate("s1", &gate("g1", "plan_approved", 1, "You approved plan v1")).unwrap());
    assert!(store.close_session_gate("s1", &gate("g1", "plan_approved", 2, "You approved plan v2")).unwrap(), "each version is its own");
    drop(store);
    let read = open(&dir).session_gates("s1").unwrap();
    assert_eq!(read.len(), 3);
    assert_eq!(read[0], gate("g2", "checks_green", 0, "Checks passed at 1b500c4"));
    assert!(open(&dir).session_gates("s2").unwrap().is_empty(), "another Job's are its own");
}
