//! What a person keeps in a project survives a reopen: his projects in order, spanning repositories,
//! each row in its place and naming its repository, the tasks in no project, a task's state, and an
//! issue row's reference alone. And the migration that took projects off their repository.

use rusqlite::Connection;

use crate::tests::{open, TempDir};
use crate::{KeptHolder, KeptProjectRow, KeptRowItem, KeptSession, KeptTaskState, SessionState, LOOSE_PROJECT};

fn task_in(manifest: &str, id: &str, title: &str) -> KeptProjectRow {
    KeptProjectRow {
        id: id.to_string(),
        manifest_id: manifest.to_string(),
        item: KeptRowItem::Task {
            title: title.to_string(),
            notes: format!("notes of {title}"),
            state: KeptTaskState::Open,
            parent: None,
            issues: Vec::new(),
            pulls: Vec::new(),
        },
    }
}

fn task(id: &str, title: &str) -> KeptProjectRow {
    task_in("m1", id, title)
}

fn subtask(id: &str, title: &str, parent: &str) -> KeptProjectRow {
    let mut row = task(id, title);
    if let KeptRowItem::Task { parent: slot, .. } = &mut row.item {
        *slot = Some(parent.to_string());
    }
    row
}

fn issue(id: &str, reference: &str) -> KeptProjectRow {
    KeptProjectRow { id: id.to_string(), manifest_id: "m1".to_string(), item: KeptRowItem::Issue { reference: reference.to_string() } }
}

fn row_ids(store: &crate::Store, project: usize) -> Vec<String> {
    store.projects().unwrap().projects[project].rows.iter().map(|one| one.id.clone()).collect()
}

fn loose_ids(store: &crate::Store) -> Vec<String> {
    store.projects().unwrap().loose.iter().map(|one| one.id.clone()).collect()
}

fn find(store: &crate::Store, id: &str) -> KeptProjectRow {
    let kept = store.projects().unwrap();
    kept.projects.into_iter().flat_map(|one| one.rows).chain(kept.loose).find(|row| row.id == id).unwrap()
}

#[test]
fn what_is_kept_in_a_project_reads_back_after_the_store_is_reopened() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project("p2", "Later", "2026-10-26T10:00:02.000Z").unwrap();
    store.add_project_row("p1", &task("r1", "Write the page")).unwrap();
    store.add_project_row("p1", &issue("r2", "ABC-12")).unwrap();
    store.add_project_row("p1", &task("r3", "Ship it")).unwrap();
    assert!(store.set_project_task_state("r3", KeptTaskState::Done).unwrap());
    assert!(store.rename_project("p2", "Someday").unwrap());
    drop(store);

    let store = open(&dir);
    let kept = store.projects().unwrap();
    assert_eq!(kept.projects.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["Launch", "Someday"]);
    let mut done = task("r3", "Ship it");
    done.item = KeptRowItem::Task {
        title: "Ship it".to_string(),
        notes: "notes of Ship it".to_string(),
        state: KeptTaskState::Done,
        parent: None,
        issues: Vec::new(),
        pulls: Vec::new(),
    };
    assert_eq!(kept.projects[0].rows, [task("r1", "Write the page"), issue("r2", "ABC-12"), done]);
    assert!(kept.projects[1].rows.is_empty());
    assert!(kept.loose.is_empty());
}

#[test]
fn a_project_spans_repositories_and_a_loose_task_sits_in_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &task_in("m1", "a", "In the app")).unwrap();
    store.add_project_row("p1", &task_in("m2", "b", "In the site")).unwrap();
    store.add_project_row("p1", &task_in("m1", "c", "In the app again")).unwrap();
    store.add_project_row(LOOSE_PROJECT, &task_in("m2", "x", "Loose in the site")).unwrap();
    store.add_project_row(LOOSE_PROJECT, &task_in("m1", "y", "Loose in the app")).unwrap();
    drop(store);

    let mut store = open(&dir);
    assert_eq!(row_ids(&store, 0), ["a", "b", "c"], "one order across repositories, his");
    let manifests: Vec<String> = store.projects().unwrap().projects[0].rows.iter().map(|one| one.manifest_id.clone()).collect();
    assert_eq!(manifests, ["m1", "m2", "m1"]);
    assert_eq!(loose_ids(&store), ["x", "y"], "the loose rows have an order of their own");
    assert!(store.move_project_row("b", None, 0, None).unwrap());
    assert_eq!(row_ids(&store, 0), ["b", "a", "c"]);
    assert_eq!(find(&store, "b").manifest_id, "m2");
}

#[test]
fn a_row_moved_between_projects_or_out_of_one_keeps_its_repository_and_takes_its_subtasks() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project("p2", "Later", "2026-10-26T10:00:02.000Z").unwrap();
    store.add_project_row("p1", &task_in("m2", "p", "Parent")).unwrap();
    let mut sub = subtask("s", "Sub", "p");
    sub.manifest_id = "m2".to_string();
    store.add_project_row("p1", &sub).unwrap();
    store.add_project_row("p1", &task("q", "Other")).unwrap();
    store.add_project_row("p2", &task("z", "Waiting")).unwrap();

    assert!(store.move_project_row("p", Some("p2"), 0, None).unwrap());
    assert_eq!(row_ids(&store, 0), ["q"], "the old project closes the gap");
    assert_eq!(row_ids(&store, 1), ["p", "s", "z"], "in at the place named, its subtask with it");
    assert_eq!(find(&store, "p").manifest_id, "m2", "the repository stays");
    assert_eq!(find(&store, "s").manifest_id, "m2");

    assert!(store.move_project_row("p", Some(LOOSE_PROJECT), 0, None).unwrap(), "out of every project");
    assert_eq!(row_ids(&store, 1), ["z"]);
    assert_eq!(loose_ids(&store), ["p", "s"]);
    assert_eq!(find(&store, "s").manifest_id, "m2");

    assert!(store.move_project_row("s", Some("p1"), 99, None).unwrap(), "a subtask sent elsewhere goes to the top level");
    assert_eq!(row_ids(&store, 0), ["q", "s"]);
    assert_eq!(loose_ids(&store), ["p"]);
}

#[test]
fn a_moved_row_lands_where_it_was_put_and_a_removed_one_closes_the_gap() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    for id in ["a", "b", "c", "d"] {
        store.add_project_row("p1", &task(id, id)).unwrap();
    }
    assert!(store.move_project_row("d", None, 0, None).unwrap());
    assert_eq!(row_ids(&store, 0), ["d", "a", "b", "c"]);
    assert!(store.move_project_row("d", None, 99, None).unwrap(), "past the end is the end");
    assert_eq!(row_ids(&store, 0), ["a", "b", "c", "d"]);
    assert!(store.remove_project_row("b").unwrap());
    store.add_project_row("p1", &task("e", "e")).unwrap();
    assert!(store.move_project_row("e", None, 1, None).unwrap());
    drop(store);

    let mut store = open(&dir);
    assert_eq!(row_ids(&store, 0), ["a", "e", "c", "d"]);
    assert!(!store.move_project_row("b", None, 0, None).unwrap(), "a removed row is gone");
    assert!(!store.remove_project_row("b").unwrap());
}

#[test]
fn a_task_edit_and_tick_reach_no_issue_row_and_a_deleted_project_takes_its_rows() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &issue("r1", "7")).unwrap();
    store.add_project_row(LOOSE_PROJECT, &task("l", "Loose")).unwrap();
    assert!(!store.edit_project_task("r1", "Title", "").unwrap());
    assert!(!store.set_project_task_state("r1", KeptTaskState::Done).unwrap());
    assert!(store.delete_project("p1").unwrap());
    assert!(!store.delete_project("p1").unwrap());
    assert!(store.projects().unwrap().projects.is_empty());
    assert_eq!(loose_ids(&store), ["l"], "a loose task is no project's to lose");
    store.add_project("p1", "Again", "2026-10-26T10:00:02.000Z").unwrap();
    assert!(store.projects().unwrap().projects[0].rows.is_empty(), "its rows went with it");
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
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &task("r1", "Sign the build")).unwrap();
    store.add_project_row("p1", &task("r2", "Ship it")).unwrap();
    store.add_project_row(LOOSE_PROJECT, &task("r3", "Loose")).unwrap();
    store.hold_project_tasks("s1", &["r1".to_string(), "r3".to_string()]).unwrap();
    drop(store);

    let mut store = open(&dir);
    let state = |store: &crate::Store, id: &str| match find(store, id).item {
        KeptRowItem::Task { state, .. } => state,
        KeptRowItem::Issue { .. } => panic!("a task"),
    };
    let held = KeptTaskState::Held(KeptHolder { session: "s1".into(), title: Some("Sign the build".into()) });
    assert_eq!(state(&store, "r1"), held, "held by the Job, with its title, after a reopen");
    assert_eq!(state(&store, "r3"), held, "a loose task is held as any is");
    assert_eq!(state(&store, "r2"), KeptTaskState::Open, "a task no Job named stays open");

    assert!(store.set_project_task_state("r1", KeptTaskState::Done).unwrap());
    assert_eq!(state(&store, "r1"), KeptTaskState::Done, "a tick drops the hold");
}

#[test]
fn what_a_job_asked_about_is_grouped_by_project_and_the_loose_ones_come_last() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row(LOOSE_PROJECT, &task("l", "Loose")).unwrap();
    store.add_project_row("p1", &task("a", "A")).unwrap();
    store.hold_project_tasks("s1", &["l".to_string(), "a".to_string()]).unwrap();
    assert_eq!(store.ask_held_project_tasks("s1", "2026-10-26T11:00:00.000Z").unwrap(), 2);

    let asked = store.asked_project_tasks("s1").unwrap();
    assert_eq!(asked.iter().map(|one| (one.project_id.as_str(), one.project_name.as_str())).collect::<Vec<_>>(), [("p1", "Launch"), ("", "")]);
    assert_eq!(store.settle_asked_project_tasks("s1", LOOSE_PROJECT, true).unwrap(), 1);
    assert_eq!(state_of(&store, "l"), KeptTaskState::Done);
}

#[test]
fn subtasks_sit_under_their_parent_count_their_places_among_siblings_and_go_with_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &task("p", "Parent")).unwrap();
    store.add_project_row("p1", &subtask("a", "A", "p")).unwrap();
    store.add_project_row("p1", &task("q", "Other")).unwrap();
    store.add_project_row("p1", &subtask("b", "B", "p")).unwrap();
    assert_eq!(row_ids(&store, 0), ["p", "a", "b", "q"], "a parent is followed by its subtasks, in their order");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(row_ids(&store, 0), ["p", "a", "b", "q"], "and so after a reopen");
    assert!(store.move_project_row("b", None, 0, None).unwrap(), "within its parent by default");
    assert_eq!(row_ids(&store, 0), ["p", "b", "a", "q"]);
    assert!(store.move_project_row("b", None, 0, Some("")).unwrap(), "out to the top level");
    assert_eq!(row_ids(&store, 0), ["b", "p", "a", "q"]);
    assert!(store.move_project_row("a", None, 99, Some("q")).unwrap(), "to another parent, past the end is the end");
    assert_eq!(row_ids(&store, 0), ["b", "p", "q", "a"]);
    assert!(store.move_project_row("q", None, 0, None).unwrap(), "a top-level move counts top-level rows only");
    assert_eq!(row_ids(&store, 0), ["q", "a", "b", "p"]);

    assert!(store.remove_project_row("q").unwrap());
    assert_eq!(row_ids(&store, 0), ["b", "p"], "its subtask went with it");
    assert!(store.delete_project("p1").unwrap());
}

#[test]
fn a_task_keeps_the_issues_linked_to_it_by_reference_and_loses_them_with_itself() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &task("t", "Task")).unwrap();
    store.add_project_row("p1", &issue("i", "7")).unwrap();
    let linked = |store: &crate::Store| match find(store, "t").item {
        KeptRowItem::Task { issues, .. } => issues,
        KeptRowItem::Issue { .. } => panic!("a task"),
    };
    assert!(store.link_task_issue("t", "ABC-12").unwrap());
    assert!(store.link_task_issue("t", "7").unwrap());
    assert!(store.link_task_issue("t", "ABC-12").unwrap(), "linking twice is the same link");
    assert!(!store.link_task_issue("i", "7").unwrap(), "an issue row takes none");
    assert!(!store.link_task_issue("nope", "7").unwrap(), "nor a row there is not");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(linked(&store), ["ABC-12", "7"], "in the order they were linked, after a reopen");
    assert!(store.unlink_task_issue("t", "ABC-12").unwrap());
    assert_eq!(linked(&store), ["7"]);
    assert!(store.remove_project_row("t").unwrap());
    store.add_project_row("p1", &task("t", "Task again")).unwrap();
    assert!(linked(&store).is_empty(), "a removed task leaves no link behind for the next of its id");
}

fn pulls_of(store: &crate::Store, task: &str) -> Vec<u64> {
    match find(store, task).item {
        KeptRowItem::Task { pulls, .. } => pulls,
        KeptRowItem::Issue { .. } => panic!("a task"),
    }
}

fn state_of(store: &crate::Store, task: &str) -> KeptTaskState {
    match find(store, task).item {
        KeptRowItem::Task { state, .. } => state,
        KeptRowItem::Issue { .. } => panic!("a task"),
    }
}

#[test]
fn a_task_keeps_the_pull_requests_linked_to_it_by_number_and_loses_them_with_itself() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &task("t", "Task")).unwrap();
    store.add_project_row("p1", &issue("i", "7")).unwrap();
    assert!(store.link_task_pull("t", 42).unwrap());
    assert!(store.link_task_pull("t", 7).unwrap());
    assert!(store.link_task_pull("t", 42).unwrap(), "linking twice is the same link");
    assert!(!store.link_task_pull("i", 7).unwrap(), "an issue row takes none");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(pulls_of(&store, "t"), [42, 7], "in the order they were linked, after a reopen");
    assert!(store.unlink_task_pull("t", 42).unwrap());
    assert_eq!(pulls_of(&store, "t"), [7]);
    assert!(store.remove_project_row("t").unwrap());
    store.add_project_row("p1", &task("t", "Task again")).unwrap();
    assert!(pulls_of(&store, "t").is_empty(), "a removed task leaves no link behind for the next of its id");
}

#[test]
fn a_merge_asks_about_an_open_or_held_task_once_and_leaves_a_done_one_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    for id in ["open", "held", "done", "other"] {
        store.add_project_row("p1", &task(id, id)).unwrap();
        store.link_task_pull(id, 42).unwrap();
    }
    store.add_project_row(LOOSE_PROJECT, &task("loose", "loose")).unwrap();
    store.link_task_pull("loose", 42).unwrap();
    store.link_task_pull("other", 43).unwrap();
    store.hold_project_tasks("s1", &["held".to_string()]).unwrap();
    store.set_project_task_state("done", KeptTaskState::Done).unwrap();

    let moved = store.ask_pulled_project_tasks("m1", 42, "2026-10-26T11:00:00.000Z").unwrap();
    assert_eq!(moved.len(), 4, "the open, held, other and loose tasks moved: {moved:?}");
    assert!(moved.contains(&"s1".to_string()) && moved.iter().filter(|one| one.is_empty()).count() == 3, "{moved:?}");
    assert_eq!(state_of(&store, "open"), KeptTaskState::Asked(None), "asked with no Job to name");
    assert_eq!(state_of(&store, "loose"), KeptTaskState::Asked(None), "a loose task is asked as any is");
    assert!(matches!(state_of(&store, "held"), KeptTaskState::Asked(Some(holder)) if holder.session == "s1"));
    assert_eq!(state_of(&store, "done"), KeptTaskState::Done);

    store.set_project_task_state("open", KeptTaskState::Open).unwrap();
    assert!(store.ask_pulled_project_tasks("m1", 42, "2026-10-26T11:05:00.000Z").unwrap().is_empty(), "the same merge asks once");
    assert_eq!(state_of(&store, "open"), KeptTaskState::Open, "so a task kept open stays open");
    assert!(store.ask_pulled_project_tasks("m2", 43, "2026-10-26T11:05:00.000Z").unwrap().is_empty(), "not another repository's");
    assert_eq!(store.ask_pulled_project_tasks("m1", 43, "2026-10-26T11:06:00.000Z").unwrap().len(), 0, "its other pull request already asked about it");
}

#[test]
fn letting_go_of_a_task_frees_it_and_the_subtasks_the_same_job_held_and_no_others() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.add_project("p1", "Launch", "2026-10-26T10:00:01.000Z").unwrap();
    store.add_project_row("p1", &task("p", "Parent")).unwrap();
    for (id, parent) in [("a", "p"), ("b", "p")] {
        store.add_project_row("p1", &subtask(id, id, parent)).unwrap();
    }
    let hold = |store: &mut crate::Store, job: &str, rows: &[&str]| {
        let rows: Vec<String> = rows.iter().map(|one| one.to_string()).collect();
        store.hold_project_tasks(job, &rows).unwrap();
    };
    hold(&mut store, "s1", &["p", "a"]);
    hold(&mut store, "s2", &["b"]);

    assert_eq!(store.free_held_project_task("p").unwrap(), 2);
    assert_eq!(state_of(&store, "p"), KeptTaskState::Open);
    assert_eq!(state_of(&store, "a"), KeptTaskState::Open);
    assert!(matches!(state_of(&store, "b"), KeptTaskState::Held(holder) if holder.session == "s2"), "another Job's subtask stays held");
    assert_eq!(store.free_held_project_task("p").unwrap(), 0, "a task nobody holds changes nothing");
}

/// The store as it stood before projects spanned repositories: the five migrations that made its tables.
fn before_spanning() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    for sql in [
        include_str!("../../migrations/20261026T1200Z-projects.table.sql"),
        include_str!("../../migrations/20261026T1300Z-projects.task_holds.sql"),
        include_str!("../../migrations/20261026T1400Z-project_task_issues.table.sql"),
        include_str!("../../migrations/20261026T1400Z-projects.task_parent.sql"),
        include_str!("../../migrations/20261026T1500Z-project_task_pulls.table.sql"),
    ] {
        conn.execute_batch(sql).unwrap();
    }
    conn
}

fn span_repositories(conn: &Connection) {
    conn.execute_batch(include_str!("../../migrations/20261027T0200Z-projects.span_repositories.sql")).unwrap();
}

fn old_project(conn: &Connection, id: &str, manifest: &str, name: &str, position: i64, created: &str) {
    conn.execute(
        "INSERT INTO projects (id, manifest_id, name, position, created) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, manifest, name, position, created],
    )
    .unwrap();
}

fn old_row(conn: &Connection, id: &str, project: &str, position: i64, parent: &str) {
    conn.execute(
        "INSERT INTO project_rows (id, project_id, position, kind, title, parent) VALUES (?1, ?2, ?3, 'task', ?1, ?4)",
        rusqlite::params![id, project, position, parent],
    )
    .unwrap();
}

fn strings(conn: &Connection, sql: &str) -> Vec<String> {
    let mut statement = conn.prepare(sql).unwrap();
    statement.query_map([], |row| row.get::<_, String>(0)).unwrap().map(Result::unwrap).collect()
}

#[test]
fn migrating_gives_each_row_its_projects_repository_and_the_project_none() {
    let conn = before_spanning();
    old_project(&conn, "p1", "m1", "Launch", 0, "2026-10-26T10:00:01.000Z");
    old_project(&conn, "p2", "m2", "Launch", 0, "2026-10-26T10:00:02.000Z");
    old_project(&conn, "p3", "m1", "Later", 1, "2026-10-26T10:00:03.000Z");
    old_row(&conn, "a", "p1", 0, "");
    old_row(&conn, "a1", "p1", 0, "a");
    old_row(&conn, "b", "p2", 0, "");
    old_row(&conn, "c", "p3", 0, "");
    conn.execute("INSERT INTO project_task_pulls (task_id, pull, position) VALUES ('a', 7, 0)", []).unwrap();

    span_repositories(&conn);

    assert_eq!(strings(&conn, "SELECT id || '@' || manifest_id FROM project_rows ORDER BY id"), ["a@m1", "a1@m1", "b@m2", "c@m1"]);
    assert_eq!(
        strings(&conn, "SELECT name || '#' || position FROM projects ORDER BY position"),
        ["Launch#0", "Launch#1", "Later#2"],
        "two projects of one name in two repositories stay two, in the order they were made"
    );
    assert_eq!(strings(&conn, "SELECT id FROM projects ORDER BY position"), ["p1", "p2", "p3"]);
    assert!(conn.prepare("SELECT manifest_id FROM projects").is_err(), "a project has no repository now");
    assert_eq!(strings(&conn, "SELECT project_id FROM project_rows WHERE id IN ('a', 'b') ORDER BY id"), ["p1", "p2"], "rows stay in their project");
    assert_eq!(strings(&conn, "SELECT CAST(pull AS TEXT) FROM project_task_pulls"), ["7"], "links are left alone");
}

#[test]
fn migrating_turns_the_rows_of_every_own_project_into_loose_rows_of_that_repository() {
    let conn = before_spanning();
    old_project(&conn, "own:m1", "m1", "", 0, "2026-10-26T10:00:00.000Z");
    old_project(&conn, "own:m2", "m2", "", 0, "2026-10-26T10:00:00.500Z");
    old_project(&conn, "p1", "m1", "Launch", 1, "2026-10-26T10:00:01.000Z");
    old_row(&conn, "x", "own:m1", 0, "");
    old_row(&conn, "x1", "own:m1", 0, "x");
    old_row(&conn, "y", "own:m1", 1, "");
    old_row(&conn, "z", "own:m2", 0, "");
    old_row(&conn, "a", "p1", 0, "");

    span_repositories(&conn);

    assert_eq!(strings(&conn, "SELECT id FROM projects"), ["p1"], "the own: projects are deleted");
    assert_eq!(strings(&conn, "SELECT name || '#' || position FROM projects"), ["Launch#0"], "and the others keep their places counted from 0");
    assert_eq!(
        strings(&conn, "SELECT id || '@' || manifest_id || '#' || position || ':' || parent FROM project_rows WHERE project_id = '' ORDER BY parent, position"),
        ["x@m1#0:", "y@m1#1:", "z@m2#2:", "x1@m1#0:x"],
        "loose in their repository, one order among the loose rows, a subtask beside its parent"
    );
    assert_eq!(strings(&conn, "SELECT project_id FROM project_rows WHERE id = 'a'"), ["p1"]);
}
