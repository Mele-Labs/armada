//! A session's Loadout, kept and read back: the list is replaced whole and a repeat changes nothing,
//! a use repeated in its turn is one, and the uses are a window of the latest turns.

use crate::tests::{open, TempDir};

fn turn(n: u32) -> String {
    format!("2026-10-17T09:{n:02}:00.000Z")
}

#[test]
fn the_list_is_replaced_whole_and_a_repeat_changes_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.session_loadout("s1").unwrap(), None);
    assert!(store.keep_session_loadout("s1", r#"{"items":[]}"#).unwrap());
    assert!(!store.keep_session_loadout("s1", r#"{"items":[]}"#).unwrap(), "the same list again");
    assert!(store.keep_session_loadout("s1", r#"{"items":[1]}"#).unwrap());
    drop(store);
    let store = open(&dir);
    assert_eq!(store.session_loadout("s1").unwrap().as_deref(), Some(r#"{"items":[1]}"#));
    assert_eq!(store.session_loadout("s2").unwrap(), None);
}

#[test]
fn a_use_repeated_in_its_turn_is_kept_once_and_turns_read_in_the_order_heard() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.session_latest_uses("s1").unwrap(), None);
    assert!(store.keep_session_use("s1", &turn(1), "a", 50).unwrap());
    assert!(store.keep_session_use("s1", &turn(1), "b", 50).unwrap());
    assert!(!store.keep_session_use("s1", &turn(1), "a", 50).unwrap(), "the same use in the same turn");
    assert!(store.keep_session_use("s1", &turn(2), "a", 50).unwrap(), "the same use in another turn");
    // A late use of the first turn stays in it.
    assert!(store.keep_session_use("s1", &turn(1), "c", 50).unwrap());
    store.keep_session_use("s2", &turn(3), "z", 50).unwrap();

    let all = store.session_uses("s1").unwrap();
    let read: Vec<(&str, &str)> = all.iter().map(|(at, used, _)| (at.as_str(), used.as_str())).collect();
    assert_eq!(read, [(turn(1).as_str(), "a"), (turn(1).as_str(), "b"), (turn(1).as_str(), "c"), (turn(2).as_str(), "a")]);
    assert_eq!(store.session_latest_uses("s1").unwrap(), Some((turn(2), vec![("a".to_string(), 1)])));
}

#[test]
fn a_repeat_raises_the_count_of_its_use_in_its_turn_only() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for _ in 0..3 {
        store.keep_session_use("s1", &turn(1), "a", 50).unwrap();
    }
    store.keep_session_use("s1", &turn(1), "b", 50).unwrap();
    store.keep_session_use("s1", &turn(2), "a", 50).unwrap();
    let counts: Vec<u32> = store.session_uses("s1").unwrap().into_iter().map(|(_, _, times)| times).collect();
    assert_eq!(counts, [3, 1, 1]);
    assert_eq!(store.session_latest_uses("s1").unwrap(), Some((turn(2), vec![("a".to_string(), 1)])));
}

#[test]
fn only_the_latest_turns_are_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for n in 0..5 {
        store.keep_session_use("s1", &turn(n), "a", 3).unwrap();
        store.keep_session_use("s1", &turn(n), "b", 3).unwrap();
    }
    let turns: Vec<String> = store.session_uses("s1").unwrap().into_iter().map(|(at, _, _)| at).collect();
    assert_eq!(turns, [turn(2), turn(2), turn(3), turn(3), turn(4), turn(4)]);
}
