//! What this Mac syncs with each profile's shared home: a switch a kind, kept across a reopen.

use crate::tests::{open, TempDir};
use crate::KeptHomeItem;

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

/// An item switched on is kept for its profile alone, a memory note by its repository too, and
/// switching it off takes it away.
#[test]
fn an_item_switched_on_by_itself_is_kept_across_a_reopen_for_its_profile_and_switching_it_off_takes_it_away() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let impeccable = KeptHomeItem { kind: "plugins".to_string(), name: "impeccable@impeccable".to_string(), repository: None };
    let note = KeptHomeItem { kind: "memory".to_string(), name: "walks".to_string(), repository: Some("armada".to_string()) };
    let github = KeptHomeItem { kind: "plugins".to_string(), name: "github@claude-plugins-official".to_string(), repository: None };
    assert!(store.home_items("nick").expect("reads").is_empty());
    store.switch_home_item("nick", &impeccable, true).expect("on");
    store.switch_home_item("nick", &impeccable, true).expect("on twice is on");
    store.switch_home_item("nick", &note, true).expect("on");
    store.switch_home_item("nick", &github, true).expect("on");
    store.switch_home_item("nick", &github, false).expect("off again");
    store.switch_home_item("nick-work", &github, true).expect("on");
    drop(store);

    let store = open(&dir);
    assert_eq!(store.home_items("nick").expect("reads"), vec![note, impeccable]);
    assert_eq!(store.home_items("nick-work").expect("reads"), vec![github], "the work profile's own");
}
