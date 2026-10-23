//! What this Mac syncs with each profile's shared home: a switch a kind, kept across a reopen.

use crate::tests::{open, TempDir};

/// A kind nobody switched has no row, a second switch replaces the first, and one profile's
/// switches are not another's.
#[test]
fn a_home_kind_switch_is_kept_across_a_reopen_for_its_profile_alone_and_a_second_one_replaces_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.home_kinds("nick").expect("reads").is_empty());
    store.switch_home_kind("nick", "skills", true).expect("on");
    store.switch_home_kind("nick", "mods", true).expect("on");
    store.switch_home_kind("nick", "mods", false).expect("off again");
    store.switch_home_kind("nick-work", "memory", true).expect("on");
    drop(store);

    let store = open(&dir);
    let switched = store.home_kinds("nick").expect("reads");
    assert_eq!(switched.len(), 2, "{switched:?}");
    assert!(switched["skills"]);
    assert!(!switched["mods"]);
    let work = store.home_kinds("nick-work").expect("reads");
    assert_eq!(work.into_iter().collect::<Vec<_>>(), [("memory".to_string(), true)], "the work profile's own");
}
