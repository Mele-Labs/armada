//! Where each profile's shared home is made: kept across a reopen, a profile at a time.

use crate::tests::{open, TempDir};
use crate::KeptHomeRepository;

fn kept(owner: &str, name: &str, found: bool) -> Option<KeptHomeRepository> {
    Some(KeptHomeRepository { owner: owner.to_string(), name: name.to_string(), found })
}

/// A login nobody chose for has no row, a second choice replaces the first, and one profile's
/// choice is not another's.
#[test]
fn a_home_repository_is_kept_across_a_reopen_for_its_profile_alone_and_a_second_choice_replaces_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.home_repository("nick").expect("reads"), None);
    store.set_home_repository("nick", "nick", "armada-home", false).expect("set");
    store.set_home_repository("nick", "mele-labs", "kit", true).expect("set again");
    store.set_home_repository("nick-work", "acme", "armada-home", false).expect("set");
    drop(store);

    let store = open(&dir);
    assert_eq!(store.home_repository("nick").expect("reads"), kept("mele-labs", "kit", true));
    assert_eq!(store.home_repository("nick-work").expect("reads"), kept("acme", "armada-home", false), "the work profile's own");
    assert_eq!(store.home_repository("someone").expect("reads"), None);
}
