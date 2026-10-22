//! A follow-up is kept once a sentence, across reopens, until it is settled; a settled one stays off.

use crate::tests::{open, TempDir};
use crate::KeptLater;

const FIRST: &str = "2026-10-21T10:00:00.000Z";
const SECOND: &str = "2026-10-21T10:05:00.000Z";
const SETTLED: &str = "2026-10-21T10:09:00.000Z";
const SLOW: &str = "I will file the slow fixture load once this lands.";
const DOCS: &str = "I will tidy the docs after the merge.";

fn open_ones(store: &crate::Store, session: &str) -> Vec<(String, String, String)> {
    store.later_of(session).unwrap().into_iter().map(|one| (one.id, one.sentence, one.said_at)).collect()
}

#[test]
fn a_sentence_said_again_is_kept_once_and_both_survive_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.keep_later("s1", SLOW, FIRST).unwrap());
    assert!(store.keep_later("s1", DOCS, FIRST).unwrap());
    assert!(!store.keep_later("s1", SLOW, SECOND).unwrap(), "the same sentence is not a second item");
    drop(store);

    let store = open(&dir);
    assert_eq!(store.later_of("s1").unwrap(), vec![
        KeptLater { id: "later-1".into(), sentence: SLOW.into(), said_at: FIRST.into() },
        KeptLater { id: "later-2".into(), sentence: DOCS.into(), said_at: FIRST.into() },
    ]);
    assert_eq!(store.later_of("s2").unwrap(), Vec::new());
}

#[test]
fn a_settled_item_leaves_the_list_and_the_sentence_is_not_kept_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_later("s1", SLOW, FIRST).unwrap();
    store.keep_later("s1", DOCS, FIRST).unwrap();
    assert_eq!(store.settle_later("s1", "later-1", "filed", Some("2710"), SETTLED).unwrap().as_deref(), Some(SLOW));
    assert_eq!(open_ones(&store, "s1"), vec![("later-2".into(), DOCS.into(), FIRST.into())]);
    assert!(!store.keep_later("s1", SLOW, SECOND).unwrap(), "a reply read again does not bring it back");
    assert_eq!(store.settle_later("s1", "later-1", "done", None, SETTLED).unwrap(), None, "already settled");
    assert_eq!(store.settle_later("s1", "later-9", "done", None, SETTLED).unwrap(), None, "no such item");
    assert_eq!(store.settle_later("s2", "later-2", "done", None, SETTLED).unwrap(), None, "another session's item");
    assert_eq!(open_ones(&store, "s1").len(), 1);
}
