//! The refreshes of the preview Fleet offered: once a (session, branch, tip, build), across a reopen,
//! and gone with their build.

use crate::tests::{open, TempDir};
use crate::KeptPreviewOffer;

fn offer(session: &str, tip: &str, build: &str) -> KeptPreviewOffer {
    KeptPreviewOffer {
        session_id: session.into(),
        branch: "feat/billing-export".into(),
        tip: tip.into(),
        build: build.into(),
        text: "feat/billing-export moved since the preview left it out: merge conflict with feat/ledger".into(),
        raised_at: "2026-10-28T10:00:00Z".into(),
    }
}

#[test]
fn an_offer_is_kept_once_a_tip_for_its_build_and_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.keep_preview_offer(&offer("s1", "t1", "b1")).expect("kept"));
    assert!(!store.keep_preview_offer(&offer("s1", "t1", "b1")).expect("kept"), "the same pair is not new");
    assert!(store.keep_preview_offer(&offer("s1", "t2", "b1")).expect("kept"), "a newer tip is");
    assert!(store.keep_preview_offer(&offer("s2", "t1", "b1")).expect("kept"), "another session's is");

    drop(store);
    let mut store = open(&dir);
    assert!(!store.keep_preview_offer(&offer("s1", "t1", "b1")).expect("kept"), "still not new after a reopen");
    let kept = store.preview_offers_of("s1").expect("reads");
    assert_eq!(kept.iter().map(|one| one.tip.as_str()).collect::<Vec<_>>(), ["t1", "t2"]);
    assert_eq!(kept[0], offer("s1", "t1", "b1"));
    assert!(store.preview_offers_of("s3").expect("reads").is_empty());
}

#[test]
fn a_build_change_drops_the_old_builds_offers_and_names_their_sessions_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_preview_offer(&offer("s1", "t1", "b1")).expect("kept");
    store.keep_preview_offer(&offer("s1", "t2", "b1")).expect("kept");
    store.keep_preview_offer(&offer("s2", "t1", "b2")).expect("kept");

    assert_eq!(store.drop_preview_offers_not_for(Some("b2")).expect("dropped"), ["s1"]);
    assert!(store.preview_offers_of("s1").expect("reads").is_empty());
    assert_eq!(store.preview_offers_of("s2").expect("reads").len(), 1);

    assert_eq!(store.drop_preview_offers_not_for(None).expect("dropped"), ["s2"], "no preview runs");
    assert!(store.drop_preview_offers_not_for(None).expect("dropped").is_empty());
}
