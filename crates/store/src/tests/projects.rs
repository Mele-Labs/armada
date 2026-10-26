//! What a person keeps in a project survives a reopen: the projects in order under their
//! repository, each row in its place, a task's state, and an issue row's reference alone.

use crate::tests::{open, TempDir};
use crate::{KeptProjectRow, KeptRowItem, KeptTaskState};

fn task(id: &str, title: &str) -> KeptProjectRow {
    KeptProjectRow {
        id: id.to_string(),
        item: KeptRowItem::Task { title: title.to_string(), notes: format!("notes of {title}"), state: KeptTaskState::Open },
    }
}

fn issue(id: &str, reference: &str) -> KeptProjectRow {
    KeptProjectRow { id: id.to_string(), item: KeptRowItem::Issue { reference: reference.to_string() } }
}

fn row_ids(store: &crate::Store, manifest: &str, project: usize) -> Vec<String> {
    store.projects(manifest).unwrap()[project].rows.iter().map(|one| one.id.clone()).collect()
}

#[test]
fn what_is_kept_in_a_project_reads_back_after_the_store_is_reopened() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("m1", "p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project("m1", "p2", "Later", "2026-10-26T10:00:02.000Z").unwrap();
    store.add_project("m2", "p3", "Other", "2026-10-26T10:00:03.000Z").unwrap();
    store.add_project_row("p1", &task("r1", "Write the page")).unwrap();
    store.add_project_row("p1", &issue("r2", "ABC-12")).unwrap();
    store.add_project_row("p1", &task("r3", "Ship it")).unwrap();
    assert!(store.set_project_task_state("p1", "r3", KeptTaskState::Done).unwrap());
    assert!(store.rename_project("m1", "p2", "Someday").unwrap());
    drop(store);

    let store = open(&dir);
    let projects = store.projects("m1").unwrap();
    assert_eq!(projects.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Launch", "Someday"]);
    let mut done = task("r3", "Ship it");
    done.item = KeptRowItem::Task { title: "Ship it".to_string(), notes: "notes of Ship it".to_string(), state: KeptTaskState::Done };
    assert_eq!(projects[0].rows, [task("r1", "Write the page"), issue("r2", "ABC-12"), done]);
    assert!(projects[1].rows.is_empty());
    assert_eq!(store.projects("m2").unwrap().len(), 1, "m2's is under m2 only");
}

#[test]
fn a_moved_row_lands_where_it_was_put_and_a_removed_one_closes_the_gap() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("m1", "p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    for id in ["a", "b", "c", "d"] {
        store.add_project_row("p1", &task(id, id)).unwrap();
    }
    assert!(store.move_project_row("p1", "d", 0).unwrap());
    assert_eq!(row_ids(&store, "m1", 0), ["d", "a", "b", "c"]);
    assert!(store.move_project_row("p1", "d", 99).unwrap(), "past the end is the end");
    assert_eq!(row_ids(&store, "m1", 0), ["a", "b", "c", "d"]);
    assert!(store.remove_project_row("p1", "b").unwrap());
    store.add_project_row("p1", &task("e", "e")).unwrap();
    assert!(store.move_project_row("p1", "e", 1).unwrap());
    drop(store);

    let mut store = open(&dir);
    assert_eq!(row_ids(&store, "m1", 0), ["a", "e", "c", "d"]);
    assert!(!store.move_project_row("p1", "b", 0).unwrap(), "a removed row is gone");
    assert!(!store.remove_project_row("p2", "a").unwrap(), "not another project's");
}

#[test]
fn a_task_edit_and_tick_reach_no_issue_row_and_a_deleted_project_takes_its_rows() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("m1", "p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &issue("r1", "7")).unwrap();
    assert!(!store.edit_project_task("p1", "r1", "Title", "").unwrap());
    assert!(!store.set_project_task_state("p1", "r1", KeptTaskState::Done).unwrap());
    assert!(!store.delete_project("m2", "p1").unwrap(), "not another repository's");
    assert!(store.delete_project("m1", "p1").unwrap());
    assert!(store.projects("m1").unwrap().is_empty());
    store.add_project("m1", "p1", "Again", "2026-10-26T10:00:02.000Z").unwrap();
    assert!(store.projects("m1").unwrap()[0].rows.is_empty(), "its rows went with it");
}
