//! What this Mac syncs with the shared home: a switch a kind, kept across a reopen.

use crate::tests::{open, TempDir};

/// A kind nobody switched has no row, and a second switch replaces the first.
#[test]
fn a_home_kind_switch_is_kept_across_a_reopen_and_a_second_one_replaces_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.home_kinds().expect("reads").is_empty());
    store.switch_home_kind("skills", true).expect("on");
    store.switch_home_kind("mods", true).expect("on");
    store.switch_home_kind("mods", false).expect("off again");
    drop(store);

    let switched = open(&dir).home_kinds().expect("reads");
    assert_eq!(switched.len(), 2);
    assert!(switched["skills"]);
    assert!(!switched["mods"]);
}
