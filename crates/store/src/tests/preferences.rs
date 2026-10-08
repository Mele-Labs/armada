//! A person's saved preferences: absent until saved, whole once saved, still
//! there after a reopen, and a name outside the closed set is refused rather
//! than stored.

use crate::tests::{open, TempDir};
use crate::{Preferences, WriteError};

#[test]
fn a_store_nobody_saved_into_reads_the_shipped_default() {
    let dir = TempDir::new();
    let store = open(&dir);
    assert_eq!(store.preferences().expect("reads"), Preferences::default());
}

#[test]
fn a_save_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .save_preference("where_things_are_open", true)
        .expect("saved");
    drop(store);

    let store = open(&dir);
    assert_eq!(
        store.preferences().expect("reads"),
        Preferences {
            where_things_are_open: true,
            ..Default::default()
        }
    );
}

/// A second save is the one row replaced, not a second row beside it.
#[test]
fn a_second_save_replaces_the_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .save_preference("where_things_are_open", true)
        .expect("first");
    store
        .save_preference("where_things_are_open", false)
        .expect("second");

    assert_eq!(
        store.preferences().expect("reads"),
        Preferences {
            where_things_are_open: false,
            ..Default::default()
        }
    );
}

#[test]
fn a_name_outside_the_closed_set_is_refused_by_name_and_nothing_is_stored() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let refused = store
        .save_preference("where_things_are_purple", true)
        .expect_err("not a preference this build reads");
    assert!(
        matches!(refused, WriteError::UnknownPreference { name } if name == "where_things_are_purple")
    );
    assert_eq!(store.preferences().expect("reads"), Preferences::default());
}

/// The draft default is a preference of its own, kept across a reopen, and
/// saving it moves nothing else.
#[test]
fn the_draft_default_is_saved_beside_the_other_preference() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .save_preference("where_things_are_open", true)
        .expect("first");
    store
        .save_preference("draft_pull_requests", true)
        .expect("second");
    drop(store);

    let store = open(&dir);
    assert_eq!(
        store.preferences().expect("reads"),
        Preferences {
            where_things_are_open: true,
            draft_pull_requests: true,
        }
    );
}
