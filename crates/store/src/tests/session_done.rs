//! The agent's `all_done` is kept with when it said so, and with what it said the Job achieved: the
//! first instant stands, a later summary replaces the kept one, and none or an empty one leaves it.

use crate::tests::{open, TempDir};

const FIRST: &str = "2026-10-30T10:00:00.000Z";
const SECOND: &str = "2026-10-30T10:05:00.000Z";

#[test]
fn a_done_kept_with_a_summary_reads_back_and_a_second_call_keeps_the_first_instant() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.done_summary("s1").unwrap(), None);
    store.keep_done("s1", FIRST, Some("Shipped the done summary.")).unwrap();
    store.keep_done("s1", SECOND, Some("Shipped it, and walked it.")).unwrap();
    assert_eq!(store.done_since("s1").unwrap().as_deref(), Some(FIRST));
    assert_eq!(store.done_summary("s1").unwrap().as_deref(), Some("Shipped it, and walked it."), "a later one replaces");
}

#[test]
fn a_done_said_again_without_a_summary_keeps_the_one_it_has() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_done("s1", FIRST, Some("Shipped the done summary.")).unwrap();
    store.keep_done("s1", SECOND, None).unwrap();
    assert_eq!(store.done_summary("s1").unwrap().as_deref(), Some("Shipped the done summary."));
    assert_eq!(store.done_since("s1").unwrap().as_deref(), Some(FIRST));

    store.keep_done("s2", FIRST, None).unwrap();
    assert_eq!(store.done_summary("s2").unwrap(), None, "a done kept bare has no summary");
    assert!(store.done_since("s2").unwrap().is_some());
}

#[test]
fn clearing_the_done_takes_the_summary_with_it_and_a_new_done_starts_bare() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_done("s1", FIRST, Some("Shipped the done summary.")).unwrap();
    assert!(store.clear_done("s1").unwrap());
    assert_eq!(store.done_summary("s1").unwrap(), None);
    store.keep_done("s1", SECOND, None).unwrap();
    assert_eq!(store.done_summary("s1").unwrap(), None, "the old summary does not come back");
    assert_eq!(store.done_since("s1").unwrap().as_deref(), Some(SECOND));
}
