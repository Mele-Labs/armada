//! A Job's open turn: what a second `turn_opened` keeps, what a death counts, and what closes it.

use crate::tests::{open, TempDir};

#[test]
fn opening_again_keeps_since_and_the_count_and_clears_the_death() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.turn_opened("s1", "2026-11-03T10:00:00Z").expect("opened");
    assert_eq!(store.turn_ended_dead("s1", "2026-11-03T10:05:00Z").expect("ended"), 0);
    store.turn_started_again("s1").expect("counted");
    let dead = store.open_turn("s1").expect("reads").expect("row");
    assert_eq!(dead.ended_at.as_deref(), Some("2026-11-03T10:05:00Z"));

    store.turn_opened("s1", "2026-11-03T10:06:00Z").expect("opened again");
    let alive = store.open_turn("s1").expect("reads").expect("row");
    assert_eq!(alive.since, "2026-11-03T10:00:00Z", "the turn is the same turn");
    assert_eq!(alive.started_again, 1);
    assert_eq!(alive.ended_at, None);
}

#[test]
fn a_death_answers_how_many_starts_the_turn_has_had() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.turn_opened("s1", "2026-11-03T10:00:00Z").expect("opened");
    assert_eq!(store.turn_ended_dead("s1", "2026-11-03T10:01:00Z").expect("ended"), 0);
    store.turn_started_again("s1").expect("counted");
    assert_eq!(store.turn_ended_dead("s1", "2026-11-03T10:02:00Z").expect("ended"), 1);
}

#[test]
fn closing_deletes_the_row_and_says_whether_there_was_one() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.turn_opened("s1", "2026-11-03T10:00:00Z").expect("opened");
    assert!(store.turn_closed("s1").expect("closed"));
    assert_eq!(store.open_turn("s1").expect("reads"), None);
    assert!(!store.turn_closed("s1").expect("closed"), "nothing left to close");
}
