//! A Session's rail, kept and read back: what the rail did survives a reopen.
//! `docs/concepts/session.md`, *The rail*.

use crate::tests::{open, TempDir};
use crate::{KeptRail, KeptStarted};

fn ids(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[test]
fn what_the_rail_did_survives_a_reopen_and_a_session_with_none_reads_as_default() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let steps = ids(&["a", "b", "c", "d"]);
    assert_eq!(store.session_rail("s1", &steps).unwrap(), KeptRail::default());
    let rail = KeptRail { step: Some(2), judged: vec![1], stalled: vec![2], visits: Vec::new(), skipped: vec![1, 3] };
    store.keep_session_rail("s1", &rail, &steps).unwrap();
    store.keep_session_rail("s2", &KeptRail { step: None, ..KeptRail::default() }, &steps).unwrap();
    drop(store);
    let store = open(&dir);
    assert_eq!(store.session_rail("s1", &steps).unwrap(), rail);
    assert_eq!(store.session_rail("s2", &steps).unwrap(), KeptRail::default());
}

#[test]
fn the_steps_a_session_began_are_kept_in_order_and_a_step_it_went_back_to_appears_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let visits = vec![(0, "2026-10-27T10:00:00Z".to_string()), (1, "2026-10-27T10:05:00Z".to_string()), (0, "2026-10-27T10:09:00Z".to_string())];
    let steps = ids(&["a", "b"]);
    store.keep_session_rail("s1", &KeptRail { step: Some(0), visits: visits.clone(), ..KeptRail::default() }, &steps).unwrap();
    drop(store);
    assert_eq!(open(&dir).session_rail("s1", &steps).unwrap().visits, visits);
}

#[test]
fn a_rail_kept_against_some_steps_reads_back_shifted_when_a_step_is_added_before_them() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let rail = KeptRail { step: Some(1), judged: vec![0], stalled: vec![1], visits: vec![(0, "t0".into()), (1, "t1".into())], skipped: vec![2] };
    store.keep_session_rail("s1", &rail, &ids(&["a", "b", "c"])).unwrap();
    let read = store.session_rail("s1", &ids(&["x", "a", "b", "c"])).unwrap();
    assert_eq!(
        read,
        KeptRail { step: Some(2), judged: vec![1], stalled: vec![2], visits: vec![(1, "t0".into()), (2, "t1".into())], skipped: vec![3] }
    );
}

#[test]
fn a_step_the_workflow_no_longer_has_is_left_out_of_what_the_rail_reads() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let rail = KeptRail { step: Some(1), skipped: vec![0, 2], visits: vec![(0, "t0".into()), (1, "t1".into())], ..KeptRail::default() };
    store.keep_session_rail("s1", &rail, &ids(&["a", "b", "c"])).unwrap();
    let read = store.session_rail("s1", &ids(&["b", "c"])).unwrap();
    assert_eq!(read, KeptRail { step: Some(0), skipped: vec![1], visits: vec![(0, "t1".into())], ..KeptRail::default() });
    assert_eq!(store.session_rail("s1", &ids(&["a", "c"])).unwrap().step, None);
}

#[test]
fn a_step_id_with_a_comma_or_a_tab_in_it_is_kept_whole() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let steps = ids(&["a,b", "c\td\ne"]);
    let rail = KeptRail { step: Some(1), skipped: vec![0, 1], visits: vec![(1, "t".into())], ..KeptRail::default() };
    store.keep_session_rail("s1", &rail, &steps).unwrap();
    assert_eq!(store.session_rail("s1", &steps).unwrap(), rail);
}

#[test]
fn a_row_kept_by_index_reads_as_indexes_until_it_is_kept_again_by_id() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .conn
        .execute("INSERT INTO session_rail (session_id, step, judged, visits, stalled, skipped) VALUES ('old', 2, '1', '2\tt', '2', '0,1')", [])
        .unwrap();
    let steps = ids(&["x", "y", "z"]);
    let expected = KeptRail { step: Some(2), judged: vec![1], stalled: vec![2], visits: vec![(2, "t".into())], skipped: vec![0, 1] };
    assert_eq!(store.session_rail("old", &steps).unwrap(), expected);
    assert_eq!(store.session_rail("old", &[]).unwrap(), expected);
    assert_eq!(store.rails_kept_by_index().unwrap(), vec!["old".to_string()]);
    store.keep_session_rail("old", &expected, &steps).unwrap();
    assert!(store.rails_kept_by_index().unwrap().is_empty());
    assert_eq!(store.session_rail("old", &ids(&["w", "x", "y", "z"])).unwrap().step, Some(3));
}

#[test]
fn clearing_a_sessions_skips_leaves_the_rest_of_its_rail() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let steps = ids(&["a", "b"]);
    store.keep_session_rail("s1", &KeptRail { step: Some(1), skipped: vec![0], ..KeptRail::default() }, &steps).unwrap();
    store.clear_session_skips("s1").unwrap();
    assert_eq!(store.session_rail("s1", &steps).unwrap(), KeptRail { step: Some(1), ..KeptRail::default() });
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
