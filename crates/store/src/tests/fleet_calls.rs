//! The calls a Fleet rule raised: kept by Session in the order raised, with an id each, across a reopen.

use crate::tests::{open, TempDir};

#[test]
fn a_call_is_kept_for_its_session_with_an_id_of_its_own_and_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = store.keep_call("s1", "write_clash", "Another Job wrote a.rs.", "2026-10-23T10:00:00Z").expect("kept");
    let again = store.keep_call("s1", "write_clash", "Another Job wrote a.rs.", "2026-10-23T10:00:00Z").expect("kept");
    store.keep_call("s2", "pr_merged", "#4 merged.", "2026-10-23T11:00:00Z").expect("kept");
    assert_ne!(first.id, again.id, "the same words at the same instant are two calls");

    drop(store);
    let store = open(&dir);
    let calls = store.calls_of("s1").expect("reads");
    assert_eq!(calls.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [first.id.as_str(), again.id.as_str()]);
    assert_eq!((calls[0].notice.as_str(), calls[0].text.as_str()), ("write_clash", "Another Job wrote a.rs."));
    assert_eq!(store.calls_of("s2").expect("reads").len(), 1);
    assert!(store.calls_of("s3").expect("reads").is_empty());
}
