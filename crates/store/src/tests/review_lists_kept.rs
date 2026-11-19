//! The Reviews lists last read, one row a project, kept across a reopen.

use crate::tests::{open, TempDir};
use crate::review_lists_kept::KeptReviewLists;

fn lists(project: &str, fetched_at: &str) -> KeptReviewLists {
    KeptReviewLists { project: project.to_string(), fetched_at: fetched_at.to_string(), lists: format!(r#"{{"me":"{project}"}}"#) }
}

#[test]
fn the_lists_of_a_project_are_kept_across_a_reopen_and_replaced_by_the_next_read() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.kept_review_lists("").unwrap(), None, "nothing read yet");
    store.keep_review_lists(&lists("", "2026-11-18T09:00:00Z")).unwrap();
    store.keep_review_lists(&lists("harbour", "2026-11-18T09:01:00Z")).unwrap();
    store.keep_review_lists(&lists("", "2026-11-18T09:05:00Z")).unwrap();
    drop(store);
    let store = open(&dir);
    assert_eq!(store.kept_review_lists("").unwrap(), Some(lists("", "2026-11-18T09:05:00Z")), "the later read replaced the earlier");
    assert_eq!(store.kept_review_lists("harbour").unwrap(), Some(lists("harbour", "2026-11-18T09:01:00Z")), "one row a project");
    assert_eq!(store.kept_review_lists("other").unwrap(), None);
}
