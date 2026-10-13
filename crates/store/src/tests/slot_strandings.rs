//! What Fleet did about a stranded slot survives a reopen, is of one stranding, and goes with it.

use crate::tests::{open, TempDir};
use crate::KeptStranding;

fn told(branch: &str, since: &str) -> KeptStranding {
    KeptStranding {
        manifest_id: String::from("armada"),
        slot: 2,
        branch: branch.to_string(),
        since: since.to_string(),
        told_at: Some(String::from("2026-10-13T10:00:00.000Z")),
        scouted: false,
    }
}

#[test]
fn a_telling_outlives_a_reopen_and_belongs_to_one_stranding() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_stranding(&told("fleet/a", "2026-10-13T09:00:00.000Z")).unwrap();
    drop(store);

    let mut store = open(&dir);
    let kept = store.strandings().unwrap();
    assert_eq!(kept, [told("fleet/a", "2026-10-13T09:00:00.000Z")]);
    assert!(kept[0].is_of("fleet/a", "2026-10-13T09:00:00.000Z"));
    assert!(!kept[0].is_of("fleet/a", "2026-10-13T11:00:00.000Z"), "a later lease is another");

    store.forget_stranding("armada", 2).unwrap();
    assert!(store.strandings().unwrap().is_empty());
}
