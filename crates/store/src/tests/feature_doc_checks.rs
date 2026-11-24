//! What a press of Check on a feature's docs found survives a reopen, a dismissal holds only while the stamp is the same,
//! and the calls a press spent are counted by night.

use crate::tests::{open, TempDir};
use crate::KeptDocCheck;

fn doc_check(target: &str, code_at: &str, stamp: &str) -> KeptDocCheck {
    KeptDocCheck {
        feature_id: "f1".to_string(),
        target: target.to_string(),
        checked_at: "2026-11-18T09:00:00.000Z".to_string(),
        doc_quote: if code_at.is_empty() { String::new() } else { "Ranks are kept for later.".to_string() },
        code_at: code_at.to_string(),
        code_quote: if code_at.is_empty() { String::new() } else { "sends on Return".to_string() },
        stamp: stamp.to_string(),
        dismissed: None,
    }
}

#[test]
fn a_presss_calls_are_counted_by_project_and_night() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_librarian_check_calls("p1", "2026-11-18", "2026-11-18T10:00:00.000Z", 3).unwrap();
    store.keep_librarian_check_calls("p1", "2026-11-18", "2026-11-18T11:00:00.000Z", 1).unwrap();
    store.keep_librarian_check_calls("p2", "2026-11-18", "2026-11-18T10:00:00.000Z", 5).unwrap();
    drop(store);
    let store = open(&dir);
    assert_eq!(store.librarian_check_calls("p1", "2026-11-18").unwrap(), 4, "a night sums its presses");
    assert_eq!(store.librarian_check_calls("p1", "2026-11-19").unwrap(), 0);
}

#[test]
fn a_dismissal_holds_only_while_a_check_reads_the_same_stamp() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_doc_check(&doc_check("docs/a.md", "src/send.rs:88", "s1")).unwrap();
    store.keep_doc_check(&doc_check("docs/b.md", "", "s1")).unwrap();
    assert!(!store.dismiss_doc_check("f1", "docs/b.md").unwrap(), "a doc that agrees has nothing to dismiss");
    assert!(store.dismiss_doc_check("f1", "docs/a.md").unwrap());
    assert!(!store.dismiss_doc_check("f1", "docs/a.md").unwrap(), "dismissed once");
    store.keep_doc_check(&doc_check("docs/a.md", "src/send.rs:90", "s1")).unwrap();
    drop(store);

    let mut store = open(&dir);
    let held = store.doc_checks_of("f1").unwrap();
    assert_eq!(held.iter().map(|one| (one.target.as_str(), one.flagged())).collect::<Vec<_>>(), [("docs/a.md", false), ("docs/b.md", false)], "the same stamp stays quiet");
    assert_eq!(held[0].code_at, "src/send.rs:90");
    store.keep_doc_check(&doc_check("docs/a.md", "src/send.rs:90", "s2")).unwrap();
    let held = store.doc_checks_of("f1").unwrap();
    assert!(held[0].flagged() && held[0].dismissed.is_none(), "another stamp flags it again: {held:?}");
    assert!(store.doc_checks_of("f2").unwrap().is_empty());
}
