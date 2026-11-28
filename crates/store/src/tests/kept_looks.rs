//! A look he approved is kept through a reopen, and a newer approval of the same walk replaces it.

use crate::tests::{open, TempDir};
use crate::KeptLook;

fn look(session: &str, walk: &str, commit: &str) -> KeptLook {
    KeptLook {
        session_id: session.into(),
        walk: walk.into(),
        url: format!("armada://walk/{walk}"),
        commit: commit.into(),
        at: "2026-11-27T10:00:00.000Z".into(),
        folder: format!("/looks/{session}/{walk}"),
    }
}

#[test]
fn a_look_is_kept_across_a_reopen_and_a_newer_approval_of_the_same_walk_replaces_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_look(&look("s1", "sidebar", "aaa")).unwrap();
    store.keep_look(&look("s1", "footer", "aaa")).unwrap();
    store.keep_look(&look("s2", "sidebar", "zzz")).unwrap();
    store.keep_look(&look("s1", "sidebar", "bbb")).unwrap();
    drop(store);
    let store = open(&dir);
    let kept = store.kept_looks("s1").unwrap();
    assert_eq!(kept.iter().map(|one| (one.walk.as_str(), one.commit.as_str())).collect::<Vec<_>>(), [("footer", "aaa"), ("sidebar", "bbb")]);
    assert_eq!(store.kept_looks("s2").unwrap().len(), 1, "another Job's is its own");
    assert!(store.kept_looks("s3").unwrap().is_empty());
}
