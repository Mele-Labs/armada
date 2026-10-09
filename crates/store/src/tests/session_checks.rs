//! A Session's Check runs: begun running, ended once with their log, and listed newest first.

use crate::tests::{open, TempDir};

#[test]
fn a_run_ends_once_and_keeps_its_log() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let run = store
        .start_session_check("s1", "armada", 6, "components_test", "2026-10-09T10:00:00.000Z")
        .expect("begins");
    assert_eq!(store.session_check(run).expect("reads").expect("kept").state, "running");

    assert!(store
        .end_session_check(run, "failed", "2026-10-09T10:00:21.000Z", 21_000, "$ vitest\nFAIL")
        .expect("ends"));
    assert!(!store
        .end_session_check(run, "passed", "2026-10-09T10:05:00.000Z", 1, "later")
        .expect("ends"));

    let ended = store.session_check(run).expect("reads").expect("kept");
    assert_eq!((ended.state.as_str(), ended.took_ms), ("failed", Some(21_000)));
    assert_eq!(store.session_check_log(run).expect("reads").as_deref(), Some("$ vitest\nFAIL"));
    assert_eq!(store.session_check_log(run + 1).expect("reads"), None);
}

#[test]
fn runs_list_newest_first_for_one_repository() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = store.start_session_check("s1", "armada", 6, "a", "t1").expect("begins");
    store.start_session_check("s2", "other", 2, "b", "t2").expect("begins");
    let third = store.start_session_check("s1", "armada", 6, "c", "t3").expect("begins");

    let ids = |manifest| -> Vec<i64> {
        store.session_checks(manifest, 10).expect("reads").iter().map(|run| run.id).collect()
    };
    assert_eq!(ids(Some("armada")), vec![third, first]);
    assert_eq!(ids(None).len(), 3);
}
