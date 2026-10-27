//! The sentences of a reply that set off a walk or done reminder a job disputed: kept for good, one
//! row a sentence and claim, of no Session.

use crate::tests::{open, TempDir};

const LATER: &str = "Once you approve the walk, Fleet keeps one task list per repo.";
const MARKED: &str = "2026-10-27T00:49:26.000Z";

#[test]
fn a_disputed_sentence_is_kept_once_for_its_claim_across_reopens() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.claim_misreads("walk").unwrap().is_empty());
    assert!(store.keep_claim_misread("s1", "walk", LATER, MARKED).unwrap());
    assert!(!store.keep_claim_misread("s2", "walk", LATER, MARKED).unwrap(), "one row a sentence, of no Session");
    drop(store);
    let store = open(&dir);
    assert_eq!(store.claim_misreads("walk").unwrap(), [LATER]);
    assert!(store.claim_misreads("done").unwrap().is_empty(), "a walk misread holds no done reminder off");
}
