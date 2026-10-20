//! The plan's last reading: absent until kept, replaced whole by the next, and still there after a
//! reopen.

use crate::tests::{open, TempDir};

#[test]
fn a_store_nothing_was_kept_in_holds_no_reading() {
    let dir = TempDir::new();
    assert_eq!(open(&dir).plan_reading().expect("reads"), None);
}

#[test]
fn a_kept_reading_survives_a_reopen_and_the_next_replaces_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_plan_reading("first").expect("kept");
    store.keep_plan_reading("second").expect("kept");
    drop(store);

    assert_eq!(open(&dir).plan_reading().expect("reads").as_deref(), Some("second"));
}
