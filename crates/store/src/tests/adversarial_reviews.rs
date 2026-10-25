//! A Job's adversarial reviews: one a plan version, replaced whole on a run again, the latest read
//! first, and a finding settled once.

use crate::tests::{open, TempDir};
use crate::{KeptAdversarialFinding, KeptAdversarialReview};

fn finding(id: &str, anchor: &str, anchor_id: &str) -> KeptAdversarialFinding {
    KeptAdversarialFinding {
        id: id.into(),
        anchor: anchor.into(),
        anchor_id: anchor_id.into(),
        part: if anchor == "plan" { String::new() } else { "Write the rotation behind a lock".into() },
        consequence: "Two rotations can land on one day.".into(),
        expected: "A lock.".into(),
        found: "None.".into(),
        cites: vec!["crates/store/src/keys.rs:2".into(), "crates/store/src/lib.rs".into()],
        settled: None,
    }
}

fn review(version: u32, state: &str, findings: Vec<KeptAdversarialFinding>) -> KeptAdversarialReview {
    KeptAdversarialReview {
        version,
        state: state.into(),
        started_at: "2026-10-25T09:00:00.000Z".into(),
        finished_at: (state != "running").then(|| "2026-10-25T09:01:00.000Z".into()),
        failed: (state == "failed").then(|| "the answer was refused".into()),
        cost_micros: Some(400_000),
        findings,
    }
}

#[test]
fn a_review_is_kept_per_version_replaced_whole_and_the_latest_is_read_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.latest_adversarial_review("s1").unwrap(), None);

    let one = review(1, "done", vec![finding("f1", "task", "t2"), finding("f2", "plan", "")]);
    store.keep_adversarial_review("s1", &one).unwrap();
    assert_eq!(store.adversarial_review("s1", 1).unwrap(), Some(one.clone()), "it reads back as it was kept");

    store.keep_adversarial_review("s1", &review(2, "running", Vec::new())).unwrap();
    assert_eq!(store.latest_adversarial_review("s1").unwrap().map(|kept| kept.version), Some(2));
    assert_eq!(store.adversarial_review("s1", 1).unwrap(), Some(one), "the older version's stays");

    // Run again on version 1: its findings are replaced, not added to.
    let again = review(1, "failed", Vec::new());
    store.keep_adversarial_review("s1", &again).unwrap();
    assert_eq!(store.adversarial_review("s1", 1).unwrap(), Some(again));
    assert_eq!(store.latest_adversarial_review("s2").unwrap(), None, "another Job's are its own");
}

#[test]
fn a_finding_is_settled_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_adversarial_review("s1", &review(1, "done", vec![finding("f1", "group", "g1")])).unwrap();

    assert!(store.settle_adversarial_finding("s1", 1, "f1", "sent", "2026-10-25T09:02:00.000Z").unwrap());
    assert!(!store.settle_adversarial_finding("s1", 1, "f1", "dismissed", "2026-10-25T09:03:00.000Z").unwrap());
    assert!(!store.settle_adversarial_finding("s1", 1, "f9", "sent", "2026-10-25T09:03:00.000Z").unwrap());
    let kept = store.adversarial_review("s1", 1).unwrap().expect("kept");
    assert_eq!(kept.findings[0].settled, Some(("sent".into(), "2026-10-25T09:02:00.000Z".into())));
}
