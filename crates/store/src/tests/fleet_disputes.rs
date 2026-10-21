//! A dispute is a row its review is named on later, found by its notice and by its key; a notice is
//! read once.

use crate::fleet_disputes::{KeptDispute, KeptNotice};
use crate::tests::{open, TempDir};

fn dispute(id: &str, notice_row: &str, at: &str) -> KeptDispute {
    KeptDispute {
        id: id.into(),
        session_id: "s1".into(),
        notice_row: notice_row.into(),
        notice: "Subagents burned 5.0 units".into(),
        why: "the readers are on sonnet".into(),
        spotted: "called".into(),
        at: at.into(),
        key: "subagents burned # units".into(),
        review: None,
    }
}

#[test]
fn a_dispute_is_kept_found_by_its_notice_and_its_review_by_its_key() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_dispute(&dispute("d1", "r1", "2026-10-19T09:00:00.000Z")).unwrap();
    assert!(store.notice_disputed("s1", "r1").unwrap());
    assert!(!store.notice_disputed("s1", "r2").unwrap());
    assert!(store.dispute_reviews("subagents burned # units").unwrap().is_empty());

    store.keep_dispute_review("d1", "review-1").unwrap();
    store.keep_dispute(&dispute("d2", "r2", "2026-10-19T09:05:00.000Z")).unwrap();
    store.keep_dispute_review("d2", "review-1").unwrap();

    assert_eq!(store.dispute("d1").unwrap().unwrap().review.as_deref(), Some("review-1"));
    assert_eq!(store.dispute_reviews("subagents burned # units").unwrap(), vec!["review-1".to_string()]);
    assert!(store.dispute_reviews("another").unwrap().is_empty());
}

#[test]
fn a_notice_is_listed_until_it_is_read_and_read_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let notice = KeptNotice { session_id: "s1".into(), row_id: "r1".into(), at: "2026-10-19T09:00:00.000Z".into(), text: "Fix it".into() };
    store.keep_fleet_notice(&notice).unwrap();
    store.keep_fleet_notice(&notice).unwrap();
    assert_eq!(store.unread_fleet_notices().unwrap(), vec![notice]);

    assert!(store.fleet_notice_read("s1", "r1", "2026-10-19T09:10:00.000Z").unwrap());
    assert!(!store.fleet_notice_read("s1", "r1", "2026-10-19T09:11:00.000Z").unwrap());
    assert!(store.unread_fleet_notices().unwrap().is_empty());
}
