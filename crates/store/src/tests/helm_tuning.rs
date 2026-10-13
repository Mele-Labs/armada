//! A Helm conversation's chosen model and effort: absent until kept, kept
//! across a reopen, replaced whole, and untouched by forgetting the session.

use core_model::Timestamp;

use crate::tests::{open, TempDir};

#[test]
fn a_conversation_nobody_tuned_runs_on_the_machines_own() {
    let dir = TempDir::new();
    let store = open(&dir);
    assert_eq!(store.helm_tuning("armada").expect("reads"), (None, None));
}

#[test]
fn a_choice_survives_a_reopen_and_each_conversation_keeps_its_own() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .keep_helm_tuning("armada", Some("opus"), Some("high"))
        .expect("kept");
    store
        .keep_helm_tuning("elsewhere", None, Some("low"))
        .expect("kept");
    drop(store);

    let store = open(&dir);
    assert_eq!(
        store.helm_tuning("armada").expect("reads"),
        (Some("opus".to_string()), Some("high".to_string()))
    );
    assert_eq!(
        store.helm_tuning("elsewhere").expect("reads"),
        (None, Some("low".to_string()))
    );
}

#[test]
fn a_second_keep_replaces_the_first_and_two_nones_clear_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .keep_helm_tuning("armada", Some("opus"), Some("high"))
        .expect("kept");
    store
        .keep_helm_tuning("armada", Some("sonnet"), None)
        .expect("kept");
    assert_eq!(
        store.helm_tuning("armada").expect("reads"),
        (Some("sonnet".to_string()), None)
    );
    store.keep_helm_tuning("armada", None, None).expect("kept");
    assert_eq!(store.helm_tuning("armada").expect("reads"), (None, None));
}

#[test]
fn forgetting_the_session_keeps_the_choice() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .keep_helm_session(
            "armada",
            "4e0fb413",
            &Timestamp::from_rfc3339("2026-09-13T09:00:00.000Z".to_string()),
        )
        .expect("kept");
    store
        .keep_helm_tuning("armada", Some("opus"), None)
        .expect("kept");
    store.forget_helm_session("armada").expect("forgot");
    assert_eq!(
        store.helm_tuning("armada").expect("reads"),
        (Some("opus".to_string()), None)
    );
}
