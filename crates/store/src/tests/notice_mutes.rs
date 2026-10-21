//! A mute survives a reopen, an hour mute ends at its time, and a lifted one is gone.

use crate::tests::{open, TempDir};

const NOW: &str = "2026-10-21T10:00:00.000Z";
const LATER: &str = "2026-10-21T11:00:00.000Z";
const PAST: &str = "2026-10-21T09:00:00.000Z";

#[test]
fn a_repo_mute_and_an_unexpired_hour_mute_read_back_after_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_mute("behind_base", "repo", "m1", None).unwrap();
    store.keep_mute("behind_base", "job", "s1", Some(LATER)).unwrap();
    drop(store);

    let store = open(&dir);
    assert!(store.is_muted("behind_base", "other", "m1", NOW).unwrap());
    assert!(store.is_muted("behind_base", "s1", "m2", NOW).unwrap());
    assert!(!store.is_muted("behind_base", "s2", "m2", NOW).unwrap());
    assert!(!store.is_muted("another_notice", "s1", "m1", NOW).unwrap());
}

#[test]
fn an_expired_mute_does_not_count_and_a_lifted_one_is_gone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_mute("behind_base", "job", "s1", Some(PAST)).unwrap();
    store.keep_mute("behind_base", "job", "s2", None).unwrap();
    assert!(!store.is_muted("behind_base", "s1", "m", NOW).unwrap());
    store.lift_mute("behind_base", "job", "s2").unwrap();
    assert!(!store.is_muted("behind_base", "s2", "m", NOW).unwrap());
    // A mute kept again replaces its end.
    store.keep_mute("behind_base", "job", "s1", Some(LATER)).unwrap();
    assert!(store.is_muted("behind_base", "s1", "m", NOW).unwrap());
}
