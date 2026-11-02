//! A hosted Job's running summary: absent until one is kept, replaced by the next, and read back from
//! the file after a reopen, which is what a Fleet restart does.

use crate::tests::{open, TempDir};

const FIRST: &str = "2026-11-02T10:00:00.000Z";
const SECOND: &str = "2026-11-02T10:05:00.000Z";

#[test]
fn a_summary_reads_as_absent_is_kept_and_a_second_one_replaces_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.so_far("s1").unwrap(), None);

    store.keep_so_far("s1", "Found the reads happen once.", FIRST).unwrap();
    assert_eq!(store.so_far("s1").unwrap().as_deref(), Some("Found the reads happen once."));

    store.keep_so_far("s1", "Added a cache, not keyed yet.", SECOND).unwrap();
    assert_eq!(store.so_far("s1").unwrap().as_deref(), Some("Added a cache, not keyed yet."), "a later one replaces");
    assert_eq!(store.so_far("s2").unwrap(), None, "one Job's summary is not another's");
}

#[test]
fn a_summary_is_read_back_from_a_store_reopened_on_the_same_file() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_so_far("s1", "Found the reads happen once.", FIRST).unwrap();
    store.keep_so_far("s1", "Added a cache, not keyed yet.", SECOND).unwrap();
    drop(store);

    let store = open(&dir);
    assert_eq!(store.so_far("s1").unwrap().as_deref(), Some("Added a cache, not keyed yet."), "the replacement is what survives");
    assert_eq!(store.so_far("s2").unwrap(), None);
}
