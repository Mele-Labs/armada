//! What a Job was told of other holders' claims, kept across the store being opened again, as a
//! Fleet restart opens it.

use core_model::Timestamp;

use crate::tests::{open, TempDir};

#[test]
fn a_told_claim_is_kept_across_an_open_until_forgotten() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let at = Timestamp::from_rfc3339("2026-11-16T20:00:00.000Z".to_string());
    assert!(!store.claims_told_before("j1", "file:src/a.rs", "home/j2").unwrap());
    store.keep_claims_told("j1", "file:src/a.rs", "home/j2", &at).unwrap();
    store.keep_claims_told("j1", "item:armada-1", "home/j2", &at).unwrap();
    drop(store);

    let mut store = open(&dir);
    assert!(store.claims_told_before("j1", "file:src/a.rs", "home/j2").unwrap());
    assert!(!store.claims_told_before("j1", "file:src/a.rs", "work/j3").unwrap(), "another holder is news");
    store.forget_claims_told("j1", Some("item:armada-1")).unwrap();
    assert!(!store.claims_told_before("j1", "item:armada-1", "home/j2").unwrap());
    assert!(store.claims_told_before("j1", "file:src/a.rs", "home/j2").unwrap(), "only that claim is forgotten");
    store.forget_claims_told("j1", None).unwrap();
    assert!(!store.claims_told_before("j1", "file:src/a.rs", "home/j2").unwrap());
}
