//! An aside survives a reopen, answers in place, is told once, and is listed oldest first for its Job alone.

use crate::tests::{open, TempDir};
use crate::{aside_state, KeptAside};

fn aside(id: &str, session: &str, at: &str) -> KeptAside {
    KeptAside {
        aside_id: id.to_string(),
        session_id: session.to_string(),
        question: "why a mutex here?".to_string(),
        quote: Some("let held = lock.lock()".to_string()),
        answer: String::new(),
        state: aside_state::ASKING.to_string(),
        failed: None,
        asked_at: at.to_string(),
        answered_at: None,
        told_at: None,
        reads: Vec::new(),
    }
}

#[test]
fn an_aside_reads_back_after_a_reopen_answered_in_place_and_listed_oldest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_aside(&aside("b", "s1", "2026-11-15T10:00:02.000Z")).unwrap();
    store.keep_aside(&aside("a", "s1", "2026-11-15T10:00:01.000Z")).unwrap();
    store.keep_aside(&aside("c", "s2", "2026-11-15T10:00:00.000Z")).unwrap();
    let mut answered = aside("a", "s1", "2026-11-15T10:00:01.000Z");
    answered.answer = "Because two tasks share it.".to_string();
    answered.state = aside_state::ANSWERED.to_string();
    answered.answered_at = Some("2026-11-15T10:00:05.000Z".to_string());
    answered.reads = vec!["Read crates/store/migrations".to_string(), "Grep keep_aside in crates".to_string()];
    store.keep_aside(&answered).unwrap();
    drop(store);

    let store = open(&dir);
    let held = store.asides_of("s1").unwrap();
    assert_eq!(held.iter().map(|one| one.aside_id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(held[0], answered);
    assert_eq!(held[0].reads, ["Read crates/store/migrations", "Grep keep_aside in crates"], "the reads come back in order");
    assert!(held[1].reads.is_empty());
    assert_eq!(held[1].state, aside_state::ASKING);
    assert!(store.aside_of("s2", "a").unwrap().is_none(), "an aside is read through its own Job");
    assert!(store.aside_of("s1", "zz").unwrap().is_none());
}

#[test]
fn an_aside_is_told_once_and_a_later_answer_keeps_the_time() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_aside(&aside("a", "s1", "2026-11-15T10:00:01.000Z")).unwrap();
    assert!(store.keep_aside_told("s1", "a", "2026-11-15T10:01:00.000Z").unwrap());
    assert!(!store.keep_aside_told("s1", "a", "2026-11-15T10:02:00.000Z").unwrap());
    // The answer's last write arrives after the telling, and carries no `told_at`.
    let mut later = aside("a", "s1", "2026-11-15T10:00:01.000Z");
    later.answer = "done".to_string();
    store.keep_aside(&later).unwrap();
    let held = store.aside_of("s1", "a").unwrap().unwrap();
    assert_eq!(held.told_at.as_deref(), Some("2026-11-15T10:01:00.000Z"));
    assert_eq!(held.answer, "done");
}
