//! What the person remembers of a pull request between visits, kept across a restart.

use crate::tests::{open, TempDir};
use crate::{KeptReviewLayout, KeptSeenHead, KeptViewedFile};

fn seen(number: u64, head: &str, at: &str) -> KeptSeenHead {
    KeptSeenHead {
        manifest_id: "harbour".to_string(),
        repository: "example-org/harbour".to_string(),
        number,
        head: head.to_string(),
        seen_at: at.to_string(),
    }
}

fn viewed(path: &str, commit: &str, blob: Option<&str>) -> KeptViewedFile {
    KeptViewedFile {
        manifest_id: "harbour".to_string(),
        repository: "example-org/harbour".to_string(),
        number: 41,
        path: path.to_string(),
        commit: commit.to_string(),
        blob: blob.map(str::to_string),
        viewed_at: "2026-11-13T09:00:00Z".to_string(),
    }
}

#[test]
fn a_seen_head_is_kept_per_pull_request_and_a_later_look_replaces_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_seen_head(&seen(41, "aaa", "2026-11-13T09:00:00Z")).unwrap();
    store.keep_seen_head(&seen(42, "ccc", "2026-11-13T09:01:00Z")).unwrap();
    store.keep_seen_head(&seen(41, "bbb", "2026-11-13T10:00:00Z")).unwrap();

    let kept = store.seen_head("harbour", "example-org/harbour", 41).unwrap().expect("kept");
    assert_eq!((kept.head.as_str(), kept.seen_at.as_str()), ("bbb", "2026-11-13T10:00:00Z"));
    assert_eq!(store.seen_head("harbour", "example-org/harbour", 43).unwrap(), None);
    assert_eq!(store.seen_heads().unwrap().len(), 2);
}

#[test]
fn a_viewed_file_is_kept_with_its_commit_and_blob_and_unmarking_forgets_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_viewed_file(&viewed("src/dock.rs", "aaa", Some("b1"))).unwrap();
    store.keep_viewed_file(&viewed("src/quay.rs", "aaa", None)).unwrap();
    store.keep_viewed_file(&viewed("src/dock.rs", "bbb", Some("b2"))).unwrap();

    let files = store.viewed_files("harbour", "example-org/harbour", 41).unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!((files[0].commit.as_str(), files[0].blob.as_deref()), ("bbb", Some("b2")));

    store.forget_viewed_file("harbour", "example-org/harbour", 41, "src/dock.rs").unwrap();
    store.forget_viewed_file("harbour", "example-org/harbour", 41, "src/never.rs").unwrap();
    let files = store.viewed_files("harbour", "example-org/harbour", 41).unwrap();
    assert_eq!(files.iter().map(|it| it.path.as_str()).collect::<Vec<_>>(), ["src/quay.rs"]);
}

#[test]
fn a_named_layout_is_saved_picked_and_deleting_it_drops_the_pick() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let wide = KeptReviewLayout {
        name: "Wide".to_string(),
        arrangement: "{\"root\":{}}".to_string(),
        saved_at: "2026-11-13T09:00:00Z".to_string(),
    };
    store.keep_review_layout(&wide).unwrap();
    store.pick_review_layout("Wide").unwrap();
    assert_eq!(store.picked_review_layout().unwrap().as_deref(), Some("Wide"));
    assert_eq!(store.review_layouts().unwrap(), vec![wide]);

    store.forget_review_layout("Wide").unwrap();
    assert_eq!(store.review_layouts().unwrap(), vec![]);
    assert_eq!(store.picked_review_layout().unwrap(), None);
}
