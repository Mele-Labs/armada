//! A Session's rail, kept and read back: what the rail did survives a reopen.
//! `docs/concepts/session.md`, *The rail*.

use crate::tests::{open, TempDir};
use crate::{KeptRail, KeptStarted};

#[test]
fn what_the_rail_did_survives_a_reopen_and_a_session_with_none_reads_as_default() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.session_rail("s1").unwrap(), KeptRail::default());
    let rail = KeptRail { step: Some(2), nudged: vec![0, 1], judged: vec![1], stalled: vec![2], visits: Vec::new(), skipped: vec![1, 3] };
    store.keep_session_rail("s1", &rail).unwrap();
    store.keep_session_rail("s2", &KeptRail { step: None, ..KeptRail::default() }).unwrap();
    drop(store);
    let store = open(&dir);
    assert_eq!(store.session_rail("s1").unwrap(), rail);
    assert_eq!(store.session_rail("s2").unwrap(), KeptRail::default());
}

#[test]
fn the_steps_a_session_began_are_kept_in_order_and_a_step_it_went_back_to_appears_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let visits = vec![(0, "2026-10-27T10:00:00Z".to_string()), (1, "2026-10-27T10:05:00Z".to_string()), (0, "2026-10-27T10:09:00Z".to_string())];
    store.keep_session_rail("s1", &KeptRail { step: Some(0), visits: visits.clone(), ..KeptRail::default() }).unwrap();
    drop(store);
    assert_eq!(open(&dir).session_rail("s1").unwrap().visits, visits);
}

#[test]
fn how_a_session_started_is_kept_once_and_a_later_start_does_not_rewrite_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.session_started("s1").unwrap(), None);
    let first = KeptStarted { workflow: Some("Bug".into()), delivery: Some("open_pr".into()), model: Some("opus".into()), effort: None, mode: "auto".into() };
    store.keep_session_started("s1", &first).unwrap();
    store.keep_session_started("s1", &KeptStarted { model: Some("sonnet".into()), mode: "plan".into(), ..first.clone() }).unwrap();
    drop(store);
    assert_eq!(open(&dir).session_started("s1").unwrap(), Some(first));
}
