//! A Session's Check runs: begun running, ended once with their log, and listed newest first.

use crate::tests::{open, TempDir};

#[test]
fn a_run_ends_once_and_keeps_its_log() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let run = store
        .start_session_check("s1", "armada", 6, "components_test", None, "2026-10-09T10:00:00.000Z")
        .expect("begins");
    assert_eq!(store.session_check(run).expect("reads").expect("kept").state, "running");

    assert!(store
        .end_session_check(run, "failed", "2026-10-09T10:00:21.000Z", 21_000, "$ vitest\nFAIL", None)
        .expect("ends"));
    assert!(!store
        .end_session_check(run, "passed", "2026-10-09T10:05:00.000Z", 1, "later", None)
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
    let first = store.start_session_check("s1", "armada", 6, "a", None, "t1").expect("begins");
    store.start_session_check("s2", "other", 2, "b", None, "t2").expect("begins");
    let third = store.start_session_check("s1", "armada", 6, "c", None, "t3").expect("begins");

    let ids = |manifest| -> Vec<i64> {
        store.session_checks(manifest, 10).expect("reads").iter().map(|run| run.id).collect()
    };
    assert_eq!(ids(Some("armada")), vec![third, first]);
    assert_eq!(ids(None).len(), 3);
}

#[test]
fn a_target_told_at_the_end_replaces_the_one_told_at_the_start() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let filtered = store.start_session_check("s1", "armada", 6, "test", Some("store::reads"), "t1").expect("begins");
    let changed = store.start_session_check("s1", "armada", 6, "test", None, "t2").expect("begins");
    let whole = store.start_session_check("s1", "armada", 6, "test", None, "t3").expect("begins");
    store.end_session_check(filtered, "passed", "t4", 1, "", None).expect("ends");
    store.end_session_check(changed, "passed", "t5", 1, "", Some("--changed fleet, ipc")).expect("ends");
    store.end_session_check(whole, "passed", "t6", 1, "", None).expect("ends");

    let target = |run| store.session_check(run).expect("reads").expect("kept").target;
    assert_eq!(target(filtered).as_deref(), Some("store::reads"));
    assert_eq!(target(changed).as_deref(), Some("--changed fleet, ipc"));
    assert_eq!(target(whole), None);
}

#[test]
fn the_running_runs_of_one_job_are_listed_and_an_ended_one_is_not() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let first = store.start_session_check("s1", "armada", 6, "a", None, "t1").expect("begins");
    let ended = store.start_session_check("s1", "armada", 6, "b", None, "t2").expect("begins");
    store.start_session_check("s2", "armada", 7, "c", None, "t3").expect("begins");
    store.end_session_check(ended, "passed", "t4", 1, "", None).expect("ends");

    let running: Vec<i64> = store.running_session_checks("s1").expect("reads").iter().map(|run| run.id).collect();
    assert_eq!(running, vec![first]);
}
