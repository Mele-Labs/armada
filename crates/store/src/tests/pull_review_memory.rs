//! What the person remembers of a pull request between visits, kept across a restart.

use crate::tests::{open, TempDir};
use crate::{KeptPullOpened, KeptReviewLayout, KeptReviewRequest, KeptSeenHead, KeptViewedFile, KeptWrittenReview};

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

fn written(number: u64, said: &str, at: &str) -> KeptWrittenReview {
    KeptWrittenReview {
        manifest_id: "harbour".to_string(),
        repository: "example-org/harbour".to_string(),
        number,
        said: said.to_string(),
        commit: Some("a1b2c3d4".to_string()),
        submitted_at: at.to_string(),
    }
}

#[test]
fn the_review_fleet_wrote_is_kept_per_pull_request_and_a_later_one_replaces_it() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        store.keep_written_review(&written(41, "commented", "2026-11-13T09:00:00Z")).expect("kept");
        store.keep_written_review(&written(41, "approved", "2026-11-13T10:00:00Z")).expect("kept");
        store.keep_written_review(&written(42, "changes_requested", "2026-11-13T09:30:00Z")).expect("kept");
    }
    let mut kept = open(&dir).written_reviews().expect("read");
    kept.sort_by_key(|it| it.number);
    assert_eq!(kept, [written(41, "approved", "2026-11-13T10:00:00Z"), written(42, "changes_requested", "2026-11-13T09:30:00Z")]);
}

fn asked(number: u64, after_review: &str, at: &str) -> KeptReviewRequest {
    KeptReviewRequest { repository: "example-org/harbour".to_string(), number, after_review: after_review.to_string(), told_at: at.to_string() }
}

#[test]
fn a_review_request_told_is_kept_by_its_pull_request_and_the_review_before_it() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        store.keep_review_request_told(&asked(41, "", "2026-11-16T09:00:00Z")).expect("kept");
        store.keep_review_request_told(&asked(41, "", "2026-11-16T09:05:00Z")).expect("kept");
        store.keep_review_request_told(&asked(41, "2026-11-16T10:00:00Z", "2026-11-16T10:30:00Z")).expect("kept");
    }
    let mut kept = open(&dir).review_requests_told().expect("read");
    kept.sort_by(|a, b| a.after_review.cmp(&b.after_review));
    assert_eq!(kept, [asked(41, "", "2026-11-16T09:00:00Z"), asked(41, "2026-11-16T10:00:00Z", "2026-11-16T10:30:00Z")], "told again keeps the first instant");
}

fn opened(number: u64, at: &str) -> KeptPullOpened {
    KeptPullOpened { manifest_id: "harbour".to_string(), number, opened_at: at.to_string() }
}

#[test]
fn a_pull_request_opened_is_kept_at_its_latest_read() {
    let dir = TempDir::new();
    {
        let mut store = open(&dir);
        store.keep_pull_request_opened(&opened(41, "2026-11-16T09:00:00Z")).expect("kept");
        store.keep_pull_request_opened(&opened(41, "2026-11-16T09:05:00Z")).expect("kept");
        store.keep_pull_request_opened(&opened(42, "2026-11-16T09:01:00Z")).expect("kept");
    }
    let mut kept = open(&dir).pull_requests_opened().expect("read");
    kept.sort_by_key(|it| it.number);
    assert_eq!(kept, [opened(41, "2026-11-16T09:05:00Z"), opened(42, "2026-11-16T09:01:00Z")], "read again moves the instant");
}

#[test]
fn requests_and_opens_no_longer_asked_of_him_are_forgotten() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_review_request_told(&asked(41, "", "2026-11-16T09:00:00Z")).expect("kept");
    store.keep_review_request_told(&asked(41, "2026-11-16T10:00:00Z", "2026-11-16T10:30:00Z")).expect("kept");
    store.keep_review_request_told(&asked(42, "", "2026-11-16T09:00:00Z")).expect("kept");
    store.keep_pull_request_opened(&opened(41, "2026-11-16T09:05:00Z")).expect("kept");
    store.keep_pull_request_opened(&opened(42, "2026-11-16T09:05:00Z")).expect("kept");

    store.forget_review_requests_but(&[("example-org/harbour".to_string(), 42)]).expect("forgot");
    store.forget_pull_requests_opened_but(&[("harbour".to_string(), 42)]).expect("forgot");

    assert_eq!(store.review_requests_told().expect("read"), [asked(42, "", "2026-11-16T09:00:00Z")]);
    assert_eq!(store.pull_requests_opened().expect("read"), [opened(42, "2026-11-16T09:05:00Z")]);
}
