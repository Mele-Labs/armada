//! The last tell a Session was sent survives a reopen, is replaced by the next, and marks only the first reply after it.

use crate::tests::{open, TempDir};
use crate::KeptTell;

const SENT: &str = "2026-10-20T10:00:00.000Z";
const FIRST: &str = "2026-10-20T10:01:00.000Z";
const SECOND: &str = "2026-10-20T10:05:00.000Z";

#[test]
fn only_the_first_reply_after_a_tell_is_marked_on_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_tell("s1", "context", SENT).unwrap();
    // A reply from before the tell is no answer to it.
    store.keep_turn_end("s1", "2026-10-20T09:59:00.000Z", "Earlier.").unwrap();
    assert_eq!(store.tell_of("s1").unwrap().unwrap().first_end, None);
    store.keep_turn_end("s1", FIRST, "No.").unwrap();
    store.keep_turn_end("s1", SECOND, "Done.").unwrap();
    drop(store);

    let store = open(&dir);
    let kept = store.tell_of("s1").unwrap().unwrap();
    assert_eq!(kept, KeptTell { kind: "context".into(), at: SENT.into(), first_end: Some(FIRST.into()) });
    assert_eq!(store.tell_of("s2").unwrap(), None);
}

#[test]
fn a_tell_sent_again_replaces_the_last_and_waits_for_its_own_reply() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_tell("s1", "context", SENT).unwrap();
    store.keep_turn_end("s1", FIRST, "No.").unwrap();
    store.keep_tell("s1", "subagents", "2026-10-20T10:03:00.000Z").unwrap();
    assert_eq!(store.tell_of("s1").unwrap().unwrap().first_end, None);
    store.keep_turn_end("s1", SECOND, "Fine.").unwrap();
    let kept = store.tell_of("s1").unwrap().unwrap();
    assert_eq!((kept.kind.as_str(), kept.first_end.as_deref()), ("subagents", Some(SECOND)));
}
