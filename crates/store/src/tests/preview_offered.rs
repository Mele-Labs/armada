//! The branches Jobs offer for the preview: one a (Job, branch), replaced by a re-offer, judged at the
//! tip it was offered at, and the Refresh row he skipped.

use crate::tests::{open, TempDir};

const B: &str = "armada/work";

#[test]
fn a_re_offer_replaces_the_offer_at_the_new_tip_as_checking_and_keeps_what_the_job_was_told() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_preview_offered("s1", B, "t1", "one line", "2026-11-13T09:00:00Z").expect("kept");
    store.keep_preview_checked("s1", B, "t1", "failed", Some("error[E0308]"), &["a.rs".to_string()], "2026-11-13T09:05:00Z").expect("checked");
    store.keep_preview_told("s1", B, "t1 failed").expect("told");

    store.keep_preview_offered("s1", B, "t2", "two lines", "2026-11-13T10:00:00Z").expect("kept");

    let kept = store.preview_offered_of("s1", B).expect("reads").expect("an offer");
    assert_eq!((kept.tip.as_str(), kept.about.as_str(), kept.state.as_str()), ("t2", "two lines", "checking"));
    assert!(kept.why.is_none() && kept.files.is_empty() && kept.checked_at.is_none(), "{kept:?}");
    assert_eq!(kept.told.as_deref(), Some("t1 failed"));
    assert_eq!(store.preview_offered_all().expect("reads").len(), 1, "one row a Job and branch");
}

#[test]
fn a_check_lands_only_on_the_tip_it_was_made_at_and_offers_list_oldest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_preview_offered("s2", "b2", "u1", "later", "2026-11-13T11:00:00Z").expect("kept");
    store.keep_preview_offered("s1", B, "t1", "earlier", "2026-11-13T09:00:00Z").expect("kept");
    store.keep_preview_offered("s1", B, "t2", "earlier", "2026-11-13T09:30:00Z").expect("re-offered while it was checked");

    assert!(!store.keep_preview_checked("s1", B, "t1", "ready", None, &[], "2026-11-13T09:40:00Z").expect("checked"), "t1 is not the offer now");
    assert!(store.keep_preview_checked("s1", B, "t2", "ready", None, &[], "2026-11-13T09:40:00Z").expect("checked"));

    let listed: Vec<(String, String)> = store.preview_offered_all().expect("reads").into_iter().map(|one| (one.branch, one.state)).collect();
    assert_eq!(listed, [(B.to_string(), "ready".to_string()), ("b2".to_string(), "checking".to_string())]);
    assert!(store.remove_preview_offered("s1", B).expect("removed"));
    assert!(!store.remove_preview_offered("s1", B).expect("removed"), "nothing left to take away");
}

#[test]
fn only_the_newest_skipped_refresh_is_kept_and_it_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.preview_refresh_skipped().expect("reads"), None);

    store.keep_preview_refresh_skipped("p-1", "2026-11-13T09:00:00Z").expect("kept");
    store.keep_preview_refresh_skipped("p-2", "2026-11-13T10:00:00Z").expect("kept");

    drop(store);
    let store = open(&dir);
    assert_eq!(store.preview_refresh_skipped().expect("reads").as_deref(), Some("p-2"));
}
