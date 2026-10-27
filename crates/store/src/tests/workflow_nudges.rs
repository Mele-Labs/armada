//! The nudges kept against a workflow's step: one row for each Job, step and fact, newest first, and a
//! mark that a nudge was not a drift, which stops it counting and keeps the row.

use crate::tests::{open, TempDir};

const FACT: &str = "a test before the walk was approved";

#[test]
fn a_nudge_is_kept_once_for_a_job_and_step_and_listed_newest_first_across_reopens() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.workflow_nudges("feature", "armada", "walk", 10).unwrap().is_empty());
    assert!(store.keep_workflow_nudge("feature", "armada", "walk", "s1", FACT, "2026-10-27T09:00:00.000Z").unwrap());
    assert!(!store.keep_workflow_nudge("feature", "armada", "walk", "s1", FACT, "2026-10-27T09:05:00.000Z").unwrap(), "once");
    assert!(store.keep_workflow_nudge("feature", "armada", "walk", "s2", FACT, "2026-10-27T10:00:00.000Z").unwrap());
    drop(store);
    let store = open(&dir);
    let kept = store.workflow_nudges("feature", "armada", "walk", 10).unwrap();
    assert_eq!(kept.iter().map(|one| one.session_id.as_str()).collect::<Vec<_>>(), ["s2", "s1"]);
    assert_eq!(kept[0].fact, FACT);
    assert!(kept.iter().all(|one| one.counts()));
    assert_eq!(store.workflow_nudges("feature", "armada", "walk", 1).unwrap().len(), 1, "the limit holds");
}

#[test]
fn a_nudge_belongs_to_the_step_and_the_copy_of_the_workflow_it_was_sent_for() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_workflow_nudge("feature", "armada", "walk", "s1", FACT, "2026-10-27T09:00:00.000Z").unwrap();
    assert!(store.workflow_nudges("feature", "repository", "walk", 10).unwrap().is_empty(), "the repository's copy is another workflow");
    assert!(store.workflow_nudges("feature", "armada", "refine", 10).unwrap().is_empty());
    assert!(store.workflow_nudges("bug", "armada", "walk", 10).unwrap().is_empty());
}

#[test]
fn a_nudge_marked_not_a_drift_stops_counting_and_stays_listed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_workflow_nudge("feature", "armada", "walk", "s1", FACT, "2026-10-27T09:00:00.000Z").unwrap();
    let id = store.workflow_nudges("feature", "armada", "walk", 10).unwrap()[0].id;
    assert!(store.dismiss_workflow_nudge(id, "2026-10-27T11:00:00.000Z").unwrap());
    assert!(!store.dismiss_workflow_nudge(id + 99, "2026-10-27T11:00:00.000Z").unwrap(), "no such nudge");
    assert!(store.dismiss_workflow_nudge(id, "2026-10-27T12:00:00.000Z").unwrap(), "marking twice is not a fault");
    let kept = store.workflow_nudges("feature", "armada", "walk", 10).unwrap();
    assert_eq!(kept.len(), 1);
    assert!(!kept[0].counts());
    assert_eq!(kept[0].dismissed_at.as_deref(), Some("2026-10-27T11:00:00.000Z"), "the first mark stays");
}
