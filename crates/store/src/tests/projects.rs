//! What a person keeps in a project survives a reopen: the projects in order under their
//! repository, each row in its place, a task's state, and an issue row's reference alone.

use crate::tests::{open, TempDir};
use crate::{KeptHolder, KeptProjectRow, KeptRowItem, KeptSession, KeptTaskState, SessionState};

fn task(id: &str, title: &str) -> KeptProjectRow {
    KeptProjectRow {
        id: id.to_string(),
        item: KeptRowItem::Task {
            title: title.to_string(),
            notes: format!("notes of {title}"),
            state: KeptTaskState::Open,
            parent: None,
            issues: Vec::new(),
        },
    }
}

fn subtask(id: &str, title: &str, parent: &str) -> KeptProjectRow {
    let mut row = task(id, title);
    if let KeptRowItem::Task { parent: slot, .. } = &mut row.item {
        *slot = Some(parent.to_string());
    }
    row
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
    done.item = KeptRowItem::Task {
        title: "Ship it".to_string(),
        notes: "notes of Ship it".to_string(),
        state: KeptTaskState::Done,
        parent: None,
        issues: Vec::new(),
    };
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
    assert!(store.move_project_row("p1", "d", 0, None).unwrap());
    assert_eq!(row_ids(&store, "m1", 0), ["d", "a", "b", "c"]);
    assert!(store.move_project_row("p1", "d", 99, None).unwrap(), "past the end is the end");
    assert_eq!(row_ids(&store, "m1", 0), ["a", "b", "c", "d"]);
    assert!(store.remove_project_row("p1", "b").unwrap());
    store.add_project_row("p1", &task("e", "e")).unwrap();
    assert!(store.move_project_row("p1", "e", 1, None).unwrap());
    drop(store);

    let mut store = open(&dir);
    assert_eq!(row_ids(&store, "m1", 0), ["a", "e", "c", "d"]);
    assert!(!store.move_project_row("p1", "b", 0, None).unwrap(), "a removed row is gone");
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

#[test]
fn a_held_task_reads_back_with_its_job_and_the_job_title_and_a_tick_lets_it_go() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store
        .keep_session(&KeptSession {
            id: "s1".into(),
            harness: "a_harness".into(),
            origin: "bridge".into(),
            manifest_id: Some("m1".into()),
            cwd: "/repos/m1".into(),
            title: Some("Sign the build".into()),
            state: SessionState::Live,
            started_at: "2026-10-26T10:00:00.000Z".into(),
            last_seen_at: "2026-10-26T10:00:00.000Z".into(),
            last_turn_at: None,
            ended_at: None,
            end_reason: None,
            figures: Default::default(),
            mod_version: None,
        })
        .unwrap();
    store.add_project("m1", "p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &task("r1", "Sign the build")).unwrap();
    store.add_project_row("p1", &task("r2", "Ship it")).unwrap();
    store.hold_project_tasks("s1", &[("p1".to_string(), "r1".to_string())]).unwrap();
    drop(store);

    let mut store = open(&dir);
    let state = |store: &crate::Store, at: usize| match &store.projects("m1").unwrap()[0].rows[at].item {
        KeptRowItem::Task { state, .. } => state.clone(),
        KeptRowItem::Issue { .. } => panic!("a task"),
    };
    assert_eq!(
        state(&store, 0),
        KeptTaskState::Held(KeptHolder { session: "s1".into(), title: Some("Sign the build".into()) }),
        "held by the Job, with its title, after a reopen"
    );
    assert_eq!(state(&store, 1), KeptTaskState::Open, "a task no Job named stays open");
    assert_eq!(store.project_manifest("p1").unwrap().as_deref(), Some("m1"));
    assert_eq!(store.project_manifest("nope").unwrap(), None);

    assert!(store.set_project_task_state("p1", "r1", KeptTaskState::Done).unwrap());
    assert_eq!(state(&store, 0), KeptTaskState::Done, "a tick drops the hold");
}

#[test]
fn subtasks_sit_under_their_parent_count_their_places_among_siblings_and_go_with_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("m1", "p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &task("p", "Parent")).unwrap();
    store.add_project_row("p1", &subtask("a", "A", "p")).unwrap();
    store.add_project_row("p1", &task("q", "Other")).unwrap();
    store.add_project_row("p1", &subtask("b", "B", "p")).unwrap();
    assert_eq!(row_ids(&store, "m1", 0), ["p", "a", "b", "q"], "a parent is followed by its subtasks, in their order");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(row_ids(&store, "m1", 0), ["p", "a", "b", "q"], "and so after a reopen");
    assert!(store.move_project_row("p1", "b", 0, None).unwrap(), "within its parent by default");
    assert_eq!(row_ids(&store, "m1", 0), ["p", "b", "a", "q"]);
    assert!(store.move_project_row("p1", "b", 0, Some("")).unwrap(), "out to the top level");
    assert_eq!(row_ids(&store, "m1", 0), ["b", "p", "a", "q"]);
    assert!(store.move_project_row("p1", "a", 99, Some("q")).unwrap(), "to another parent, past the end is the end");
    assert_eq!(row_ids(&store, "m1", 0), ["b", "p", "q", "a"]);
    assert!(store.move_project_row("p1", "q", 0, None).unwrap(), "a top-level move counts top-level rows only");
    assert_eq!(row_ids(&store, "m1", 0), ["q", "a", "b", "p"]);

    assert!(store.remove_project_row("p1", "q").unwrap());
    assert_eq!(row_ids(&store, "m1", 0), ["b", "p"], "its subtask went with it");
    assert!(store.delete_project("m1", "p1").unwrap());
}

#[test]
fn a_task_keeps_the_issues_linked_to_it_by_reference_and_loses_them_with_itself() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("m1", "p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &task("t", "Task")).unwrap();
    store.add_project_row("p1", &issue("i", "7")).unwrap();
    let linked = |store: &crate::Store| match &store.projects("m1").unwrap()[0].rows[0].item {
        KeptRowItem::Task { issues, .. } => issues.clone(),
        KeptRowItem::Issue { .. } => panic!("a task"),
    };
    assert!(store.link_task_issue("p1", "t", "ABC-12").unwrap());
    assert!(store.link_task_issue("p1", "t", "7").unwrap());
    assert!(store.link_task_issue("p1", "t", "ABC-12").unwrap(), "linking twice is the same link");
    assert!(!store.link_task_issue("p1", "i", "7").unwrap(), "an issue row takes none");
    assert!(!store.link_task_issue("p2", "t", "7").unwrap(), "not another project's task");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(linked(&store), ["ABC-12", "7"], "in the order they were linked, after a reopen");
    assert!(store.unlink_task_issue("p1", "t", "ABC-12").unwrap());
    assert_eq!(linked(&store), ["7"]);
    assert!(store.remove_project_row("p1", "t").unwrap());
    store.add_project_row("p1", &task("t", "Task again")).unwrap();
    assert!(linked(&store).is_empty(), "a removed task leaves no link behind for the next of its id");
}
