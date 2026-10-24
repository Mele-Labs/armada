//! The beads a plan was mirrored as: kept in the order they were filed, rewritten where the mirror
//! wrote to one again, forgotten once closed for good, and kept apart by Job and by issue.

use crate::tests::{open, TempDir};
use crate::KeptBead;

fn bead(session: &str, issue: &str, plan_id: &str, bead: &str) -> KeptBead {
    KeptBead {
        session_id: session.to_string(),
        issue: issue.to_string(),
        plan_id: plan_id.to_string(),
        bead: bead.to_string(),
        title: format!("title of {plan_id}"),
        status: "open".to_string(),
        blocked_by: String::new(),
    }
}

#[test]
fn a_bead_is_kept_once_per_plan_id_and_what_was_written_to_it_again_replaces_the_row() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_plan_bead(&bead("s1", "armada-1", "g1", "armada-1.1")).expect("kept");
    store.keep_plan_bead(&bead("s1", "armada-1", "t1", "armada-1.1.1")).expect("kept");
    let mut moved = bead("s1", "armada-1", "g1", "armada-1.9");
    moved.status = "closed".to_string();
    moved.title = "Renamed".to_string();
    moved.blocked_by = "g0".to_string();
    store.keep_plan_bead(&moved).expect("kept again");

    let kept = store.plan_beads("s1", "armada-1").expect("reads");
    let ids: Vec<&str> = kept.iter().map(|one| one.plan_id.as_str()).collect();
    assert_eq!(ids, ["g1", "t1"], "one row a plan id, in the order they were filed");
    assert_eq!(kept[0].bead, "armada-1.1", "the bead it was filed as is never replaced");
    assert_eq!((kept[0].status.as_str(), kept[0].title.as_str(), kept[0].blocked_by.as_str()), ("closed", "Renamed", "g0"));
}

#[test]
fn rows_are_kept_apart_by_job_and_by_issue_and_forgotten_one_at_a_time() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_plan_bead(&bead("s1", "armada-1", "g1", "armada-1.1")).expect("kept");
    store.keep_plan_bead(&bead("s2", "armada-1", "g1", "armada-1.2")).expect("kept");
    store.keep_plan_bead(&bead("s1", "armada-7", "g1", "armada-7.1")).expect("kept");

    store.forget_plan_bead("s1", "armada-1", "g1").expect("forgotten");

    assert!(store.plan_beads("s1", "armada-1").expect("reads").is_empty());
    assert_eq!(store.plan_beads("s2", "armada-1").expect("reads").len(), 1);
    assert_eq!(store.plan_beads("s1", "armada-7").expect("reads").len(), 1);
}

#[test]
fn rows_survive_a_reopen() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        store.keep_plan_bead(&bead("s1", "armada-1", "g1", "armada-1.1")).expect("kept");
    }
    let store = open(&dir);
    assert_eq!(store.plan_beads("s1", "armada-1").expect("reads")[0].bead, "armada-1.1");
}
