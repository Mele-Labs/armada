//! The repairs Fleet hosted beside a broken Job, kept apart from sessions.

use crate::tests::{open, TempDir};

const AT: &str = "2026-11-10T10:00:00.000Z";
const LATER: &str = "2026-11-10T10:05:00.000Z";

#[test]
fn a_repair_is_running_until_its_end_is_kept_and_its_end_is_kept_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_repair_started("s1", "r1", "hung", AT).expect("keeps");
    let running = store.repair("r1").expect("reads").expect("kept");
    assert_eq!((running.session_id.as_str(), running.broke.as_str(), running.ended_at), ("s1", "hung", None));
    assert!(!store.repair_over("r1").expect("reads"));

    store.keep_repair_ended("r1", LATER, "repaired", "stopped sleep 9999").expect("keeps");
    store.keep_repair_ended("r1", AT, "failed", "later word").expect("keeps");
    let over = store.repair("r1").expect("reads").expect("kept");
    assert_eq!((over.ended_at.as_deref(), over.outcome.as_deref(), over.said.as_deref()), (Some(LATER), Some("repaired"), Some("stopped sleep 9999")));
    assert!(store.repair_over("r1").expect("reads"));
}

#[test]
fn a_repair_this_store_never_kept_reads_as_over() {
    let dir = TempDir::new();
    assert!(open(&dir).repair_over("nobody").expect("reads"), "nothing is running it");
}

#[test]
fn a_repair_left_open_when_fleet_stopped_ends_as_abandoned_and_one_over_is_left_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_repair_started("s1", "r1", "died", AT).expect("keeps");
    store.keep_repair_started("s2", "r2", "quiet", AT).expect("keeps");
    store.keep_repair_ended("r2", AT, "repaired", "it landed").expect("keeps");
    assert_eq!(store.end_open_repairs(LATER).expect("ends"), 1);
    assert_eq!(store.repair("r1").expect("reads").expect("kept").outcome.as_deref(), Some("abandoned"));
    assert_eq!(store.repair("r2").expect("reads").expect("kept").outcome.as_deref(), Some("repaired"));
}
