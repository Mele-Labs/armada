//! The shell each Job's agent last saw, kept across the store being opened again.

use core_model::Timestamp;

use crate::tests::{open, TempDir};

fn at(when: &str) -> Timestamp {
    Timestamp::from_rfc3339(when.to_string())
}

#[test]
fn the_shell_an_agent_last_saw_is_kept_across_an_open_and_replaced_by_the_next() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.job_shell_seen("s1").unwrap(), None);
    store.keep_job_shell_seen("s1", "session-run-A", &at("2026-10-26T09:00:00.000Z")).unwrap();
    store.keep_job_shell_seen("s2", "session-run-B", &at("2026-10-26T09:00:00.000Z")).unwrap();
    drop(store);

    let mut store = open(&dir);
    assert_eq!(store.job_shell_seen("s1").unwrap().as_deref(), Some("session-run-A"));
    store.keep_job_shell_seen("s1", "session-run-C", &at("2026-10-26T09:05:00.000Z")).unwrap();
    assert_eq!(store.job_shell_seen("s1").unwrap().as_deref(), Some("session-run-C"));
    assert_eq!(store.job_shell_seen("s2").unwrap().as_deref(), Some("session-run-B"));
}
