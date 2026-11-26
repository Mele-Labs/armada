//! An addition survives a reopen, a removed one is not proposed again, and an answer changes its state.

use crate::tests::{open, TempDir};
use crate::{addition_state, KeptAddition};

fn addition(id: &str) -> KeptAddition {
    KeptAddition {
        project: "p1".to_string(),
        id: id.to_string(),
        name: "Worker".to_string(),
        kind: "service".to_string(),
        summary: "A worker no manifest named".to_string(),
        why: "It holds 12 files and nothing reaches it.".to_string(),
        paths: vec!["worker/".to_string()],
        manifest: "armada".to_string(),
        state: addition_state::PROPOSED.to_string(),
        note: String::new(),
        added_at: "2026-11-24T10:00:00.000Z".to_string(),
    }
}

#[test]
fn an_addition_is_kept_answered_and_not_proposed_again_once_removed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.system_additions("p1").unwrap().is_empty());
    assert!(store.propose_system_addition(&addition("worker")).unwrap());
    assert!(!store.propose_system_addition(&addition("worker")).unwrap(), "the same id is not added twice");
    assert!(store.answer_system_addition("p1", "worker", addition_state::REWORK, "it is a queue").unwrap());
    let held = store.system_additions("p1").unwrap();
    assert_eq!((held[0].state.as_str(), held[0].note.as_str()), (addition_state::REWORK, "it is a queue"));
    assert!(store.redo_system_addition("p1", "worker", "A queue worker", "Reworked.", &["worker/".to_string(), "queue/".to_string()]).unwrap());
    let held = store.system_additions("p1").unwrap();
    assert_eq!((held[0].state.as_str(), held[0].note.as_str(), held[0].paths.len()), (addition_state::PROPOSED, "", 2));
    store.answer_system_addition("p1", "worker", addition_state::REMOVED, "").unwrap();
    drop(store);
    let mut store = open(&dir);
    assert!(!store.propose_system_addition(&addition("worker")).unwrap(), "a removed part is not proposed again after a reopen");
    assert_eq!(store.system_additions("p1").unwrap()[0].state, addition_state::REMOVED);
    assert_eq!(store.system_librarian_at("p1").unwrap(), None);
    store.keep_system_librarian_at("p1", "2026-11-24T11:00:00.000Z").unwrap();
    assert_eq!(store.system_librarian_at("p1").unwrap().as_deref(), Some("2026-11-24T11:00:00.000Z"));
}
