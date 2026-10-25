//! How the person chose to land work: kept per repository and workflow, read newest first, and
//! across a reopen.

use crate::tests::{open, TempDir};

#[test]
fn picks_are_read_newest_first_for_their_repository_and_workflow_alone_and_survive_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.delivery_picks("armada", "feature", 10).expect("reads").is_empty());
    for (manifest, workflow, choice) in [
        ("armada", "feature", "auto_land"),
        ("armada", "bug", "keep_local"),
        ("armada", "feature", "open_pr"),
        ("other", "feature", "keep_local"),
        ("armada", "feature", "open_pr"),
    ] {
        store.keep_delivery_pick(manifest, workflow, choice, "2026-10-23T09:00:00Z").expect("kept");
    }
    drop(store);

    let store = open(&dir);
    assert_eq!(store.delivery_picks("armada", "feature", 10).expect("reads"), ["open_pr", "open_pr", "auto_land"]);
    assert_eq!(store.delivery_picks("armada", "feature", 2).expect("reads"), ["open_pr", "open_pr"], "the limit takes the newest");
    assert_eq!(store.delivery_picks("armada", "bug", 10).expect("reads"), ["keep_local"]);
    assert_eq!(store.delivery_picks("other", "feature", 10).expect("reads"), ["keep_local"]);
}

#[test]
fn a_hosting_is_held_on_its_start_card_across_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mut hosting = crate::tests::session_ledger::hosting("s1", "armada", false);
    assert!(!hosting.start_held, "a hosting is not held unless it is put so");
    hosting.start_held = true;
    store.keep_hosting(&hosting).expect("kept");
    drop(store);
    assert!(open(&dir).hosting("s1").expect("reads").expect("kept").start_held);
}

#[test]
fn a_held_jobs_first_turn_is_kept_once_survives_a_reopen_and_is_taken_whole() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.held_first_turn("s1").expect("reads"), None);
    assert!(store.keep_held_first_turn("s1", "the first").expect("kept"));
    assert!(!store.keep_held_first_turn("s1", "a second").expect("refused, not failed"), "the first one stands");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(store.held_first_turn("s1").expect("reads").as_deref(), Some("the first"));
    assert_eq!(store.held_first_turn("s2").expect("reads"), None, "another Job's is its own");
    assert_eq!(store.take_held_first_turn("s1").expect("taken").as_deref(), Some("the first"));
    assert_eq!(store.take_held_first_turn("s1").expect("taken"), None, "taken once");
    assert_eq!(store.held_first_turn("s1").expect("reads"), None);
}
