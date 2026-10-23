//! The questions about where taught words open: put once a mod and repository, read back by
//! Session, and kept across a reopen.

use crate::tests::{open, TempDir};

#[test]
fn a_question_is_put_once_for_a_mod_and_a_repository_whoever_asks_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.ask_word_address("jira-key", "armada", "s1", "2026-10-23T10:00:00Z").expect("kept"));
    assert!(!store.ask_word_address("jira-key", "armada", "s2", "2026-10-23T11:00:00Z").expect("kept"), "a second Session does not ask again");
    assert!(store.ask_word_address("jira-key", "other", "s1", "2026-10-23T12:00:00Z").expect("kept"), "another repository is its own question");
    assert!(store.ask_word_address("ticket", "armada", "s1", "2026-10-23T13:00:00Z").expect("kept"), "so is another pattern");

    let asks = store.word_asks_of("s1").expect("reads");
    assert_eq!(asks.iter().map(|one| (one.mod_name.as_str(), one.repository.as_str())).collect::<Vec<_>>(), [("jira-key", "armada"), ("jira-key", "other"), ("ticket", "armada")]);
    assert!(store.word_asks_of("s2").expect("reads").is_empty(), "the question stays with the Session it was put to");
    assert_eq!(store.word_ask("jira-key", "armada").expect("reads").map(|one| one.session_id), Some("s1".to_string()));
    assert_eq!(store.word_ask("jira-key", "nowhere").expect("reads"), None);
}

#[test]
fn a_question_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.ask_word_address("jira-key", "armada", "s1", "2026-10-23T10:00:00Z").expect("kept");
    drop(store);
    let mut store = open(&dir);
    assert_eq!(store.word_asks_of("s1").expect("reads").len(), 1);
    assert!(!store.ask_word_address("jira-key", "armada", "s1", "2026-10-23T14:00:00Z").expect("kept"));
}
