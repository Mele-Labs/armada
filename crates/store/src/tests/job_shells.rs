//! A Job's terminals, kept across the store being opened again: their numbers, names, and the shell
//! its agent last saw in each.

use core_model::Timestamp;

use crate::tests::{open, TempDir};

fn at(when: &str) -> Timestamp {
    Timestamp::from_rfc3339(when.to_string())
}

#[test]
fn a_jobs_terminals_are_numbered_from_one_and_a_number_is_never_given_twice() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.job_terminals("s1").unwrap().is_empty());
    assert_eq!(store.mint_job_terminal("s1").unwrap(), 1);
    assert_eq!(store.mint_job_terminal("s1").unwrap(), 2);
    assert_eq!(store.mint_job_terminal("s2").unwrap(), 1, "each Job counts its own");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(store.mint_job_terminal("s1").unwrap(), 3, "a row stays, so a restart does not start over");
    let numbers: Vec<u32> = store.job_terminals("s1").unwrap().iter().map(|one| one.number).collect();
    assert_eq!(numbers, [1, 2, 3]);
}

#[test]
fn the_shell_an_agent_last_saw_is_kept_for_each_terminal_across_an_open_and_replaced_by_the_next() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for _ in 0..2 {
        store.mint_job_terminal("s1").unwrap();
    }
    store.keep_job_terminal_seen("s1", 1, "session-run-A", &at("2026-10-26T09:00:00.000Z")).unwrap();
    store.keep_job_terminal_seen("s1", 2, "session-run-B", &at("2026-10-26T09:01:00.000Z")).unwrap();
    drop(store);

    let mut store = open(&dir);
    store.keep_job_terminal_seen("s1", 1, "session-run-C", &at("2026-10-26T09:05:00.000Z")).unwrap();
    let kept = store.job_terminals("s1").unwrap();
    let seen: Vec<(Option<&str>, Option<&str>)> = kept.iter().map(|one| (one.seen_run.as_deref(), one.seen_at.as_deref())).collect();
    assert_eq!(
        seen,
        [(Some("session-run-C"), Some("2026-10-26T09:05:00.000Z")), (Some("session-run-B"), Some("2026-10-26T09:01:00.000Z"))]
    );
}

#[test]
fn a_terminal_never_seen_has_no_run_and_a_name_is_kept_until_blanked() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.mint_job_terminal("s1").unwrap();
    let fresh = store.job_terminals("s1").unwrap().remove(0);
    assert_eq!((fresh.name.as_str(), fresh.seen_run, fresh.seen_at), ("", None, None));

    store.keep_job_terminal_name("s1", 1, "build").unwrap();
    drop(store);
    let mut store = open(&dir);
    assert_eq!(store.job_terminals("s1").unwrap()[0].name, "build");
    store.keep_job_terminal_name("s1", 1, "").unwrap();
    assert_eq!(store.job_terminals("s1").unwrap()[0].name, "");
}
