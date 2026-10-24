//! The issues Jobs worked: kept once, marked merged and closed by address, forgotten on a lost claim.

use crate::tests::{open, TempDir};

#[test]
fn an_issue_is_kept_once_marked_merged_then_closed_and_forgotten_on_a_lost_claim() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_job_issue("s1", "m", "armada-1").expect("kept");
    store.keep_job_issue("s1", "m", "armada-1").expect("kept again");
    store.keep_job_issue("s2", "m", "armada-1").expect("another job");
    store.keep_job_issue("s1", "other", "armada-9").expect("another repository");

    store.job_issue_merged("s1", "m", "armada-1", "https://forge.invalid/pull/1").expect("merged");
    let rows = store.job_issues("m").expect("reads");
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0].merged_in.as_str(), rows[0].closed_in.as_str()), ("https://forge.invalid/pull/1", ""));
    assert_eq!(rows[1].merged_in, "", "the other Job's is not marked");

    store.job_issue_closed("s1", "m", "armada-1", "https://forge.invalid/pull/1").expect("closed");
    store.forget_job_issue("s2", "m", "armada-1").expect("forgotten");
    let rows = store.job_issues("m").expect("reads");
    assert_eq!((rows.len(), rows[0].closed_in.as_str()), (1, "https://forge.invalid/pull/1"));
}
