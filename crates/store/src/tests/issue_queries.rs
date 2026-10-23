//! A saved issue query survives a reopen, an edit by id lands in place, and a forgotten one is gone.

use crate::tests::{open, TempDir};
use crate::KeptIssueQuery;

fn saved(id: &str, manifest: &str, name: &str, query: &str) -> KeptIssueQuery {
    KeptIssueQuery {
        id: id.to_string(),
        manifest_id: manifest.to_string(),
        name: name.to_string(),
        words: format!("words of {name}"),
        query: query.to_string(),
        row_fields: vec!["Labels".to_string(), "Sprint".to_string()],
    }
}

#[test]
fn saved_queries_read_back_after_a_reopen_each_under_its_repository() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_issue_query(&saved("q2", "m1", "Second", "assignee = me"), "2026-10-23T10:00:02.000Z").unwrap();
    store.keep_issue_query(&saved("q1", "m1", "First", "state = open"), "2026-10-23T10:00:01.000Z").unwrap();
    store.keep_issue_query(&saved("q3", "m2", "Other", "bug"), "2026-10-23T10:00:03.000Z").unwrap();
    drop(store);

    let store = open(&dir);
    let names: Vec<String> = store.issue_queries("m1").unwrap().into_iter().map(|one| one.name).collect();
    assert_eq!(names, ["First", "Second"], "oldest first, and m2's is not here");
    assert_eq!(store.issue_query("m1", "q1").unwrap(), Some(saved("q1", "m1", "First", "state = open")));
    assert_eq!(store.issue_query("m1", "q3").unwrap(), None, "another repository's id is none here");
}

#[test]
fn a_save_by_an_id_already_kept_edits_it_in_place() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_issue_query(&saved("q1", "m1", "First", "state = open"), "2026-10-23T10:00:01.000Z").unwrap();
    store.keep_issue_query(&saved("q2", "m1", "Second", "assignee = me"), "2026-10-23T10:00:02.000Z").unwrap();

    let mut edited = saved("q1", "m1", "Renamed", "state = closed");
    edited.row_fields = Vec::new();
    store.keep_issue_query(&edited, "2026-10-23T10:00:09.000Z").unwrap();

    let kept = store.issue_queries("m1").unwrap();
    assert_eq!(kept.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["q1", "q2"], "it keeps its place");
    assert_eq!(kept[0], edited);
}

#[test]
fn a_forgotten_query_is_gone_and_forgetting_twice_says_so() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_issue_query(&saved("q1", "m1", "First", "state = open"), "2026-10-23T10:00:01.000Z").unwrap();
    assert!(store.forget_issue_query("m2", "q1").unwrap() == false, "not another repository's");
    assert!(store.forget_issue_query("m1", "q1").unwrap());
    assert!(!store.forget_issue_query("m1", "q1").unwrap());
    assert!(store.issue_queries("m1").unwrap().is_empty());
}
