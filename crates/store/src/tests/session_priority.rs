//! The one prioritised Job: none until kept, replaced by the next, cleared by none, and still there
//! after a reopen.

use crate::tests::{open, TempDir};

#[test]
fn a_store_nothing_was_kept_in_prioritises_no_job() {
    let dir = TempDir::new();
    assert_eq!(open(&dir).prioritised_session().expect("reads"), None);
}

#[test]
fn the_prioritised_job_survives_a_reopen_and_the_next_replaces_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_prioritised_session(Some("s1")).expect("kept");
    store.keep_prioritised_session(Some("s2")).expect("kept");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(store.prioritised_session().expect("reads").as_deref(), Some("s2"));
    store.keep_prioritised_session(None).expect("cleared");
    drop(store);
    assert_eq!(open(&dir).prioritised_session().expect("reads"), None);
}
