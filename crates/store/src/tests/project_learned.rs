//! What Fleet learned for a project: kept, read, replaced, cleared and dropped with its project, and
//! across a reopen.

use crate::tests::{open, TempDir};

#[test]
fn a_note_is_kept_read_replaced_and_cleared_for_its_project_and_key_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.project_learned("p1", "delivery").expect("reads"), None);
    store.keep_project_learned("p1", "delivery", "open_pr", "2026-11-13T11:00:00Z").expect("kept");
    store.keep_project_learned("p2", "delivery", "keep_local", "2026-11-13T11:00:00Z").expect("kept");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(store.project_learned("p1", "delivery").expect("reads").as_deref(), Some("open_pr"), "survives a reopen");
    store.keep_project_learned("p1", "delivery", "auto_land", "2026-11-13T12:00:00Z").expect("replaced");
    assert_eq!(store.project_learned("p1", "delivery").expect("reads").as_deref(), Some("auto_land"));
    store.clear_project_learned("p1", "delivery").expect("cleared");
    store.clear_project_learned("p1", "delivery").expect("clearing nothing is no fault");
    assert_eq!(store.project_learned("p1", "delivery").expect("reads"), None);
    assert_eq!(store.project_learned("p2", "delivery").expect("reads").as_deref(), Some("keep_local"), "another project's note stands");
}

#[test]
fn deleting_a_project_drops_its_notes() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Shop", "2026-11-13T11:00:00Z").expect("added");
    store.keep_project_learned("p1", "delivery", "open_pr", "2026-11-13T11:00:00Z").expect("kept");
    assert!(store.delete_project("p1").expect("deleted"));
    assert_eq!(store.project_learned("p1", "delivery").expect("reads"), None);
}
