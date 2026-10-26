//! The thumbs on a follow-up: each sentence kept for good, one row, of no Session; a confirmed one reads so on its row.

use crate::tests::{open, TempDir};

const SLOW: &str = "I will file the slow fixture load once this lands.";
const FIRST: &str = "2026-10-21T10:00:00.000Z";
const MARKED: &str = "2026-10-21T10:09:00.000Z";

#[test]
fn a_sentence_marked_not_deferred_is_kept_once_across_reopens() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.later_misreads().unwrap().is_empty());
    assert!(store.keep_later_misread("s1", SLOW, MARKED).unwrap());
    assert!(!store.keep_later_misread("s2", SLOW, MARKED).unwrap(), "one row a sentence, of no Session");
    drop(store);
    assert_eq!(open(&dir).later_misreads().unwrap(), [SLOW]);
}

#[test]
fn a_confirmed_sentence_reads_confirmed_on_its_open_row_and_is_kept_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_later("s1", SLOW, FIRST).unwrap();
    assert!(!store.later_of("s1").unwrap()[0].confirmed);
    assert!(store.keep_later_confirmed("s1", SLOW, MARKED).unwrap());
    assert!(!store.keep_later_confirmed("s1", SLOW, MARKED).unwrap());
    assert!(store.later_of("s1").unwrap()[0].confirmed);
    assert_eq!(store.later_confirmed().unwrap(), [SLOW]);
}
