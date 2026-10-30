//! A written task may have subtasks, one level deep, and carry the issues linked to it: rows are
//! flat with a `parent`, a parent is followed by its subtasks, a move may change the parent, and a
//! linked issue's title and state are the tracker's and never kept.

use std::sync::Arc;

use api::Projects;
use ipc::{Event, IssueState, MoveProjectRow, ProjectRow, ProjectTask, TaskIssue};
use testkit::FakeTracker;

use super::projects::{a_fleet, code_of, launched, link, manifest, new_issue, new_task, Hosted};
use crate::tests::tmp::TempDir;

fn id_of(row: &ProjectRow) -> String {
    match row {
        ProjectRow::Task { id, .. } | ProjectRow::Issue { id, .. } => id.clone(),
    }
}

/// The first project's rows as `(id, parent)`, in the order they are listed.
fn shape(projects: &ipc::Projects) -> Vec<(String, Option<String>)> {
    projects.projects[0]
        .rows
        .iter()
        .map(|row| match row {
            ProjectRow::Task { id, parent, .. } => (id.clone(), parent.clone()),
            ProjectRow::Issue { id, .. } => (id.clone(), None),
        })
        .collect()
}

fn newest(projects: &ipc::Projects) -> String {
    id_of(projects.projects[0].rows.last().expect("a row"))
}

/// The id of the last subtask `parent` has.
fn last_sub(projects: &ipc::Projects, parent: &str) -> String {
    shape(projects).into_iter().rev().find(|(_, of)| of.as_deref() == Some(parent)).expect("a subtask").0
}

/// A project of two tasks `a` and `b`, and an issue row `i`; their ids.
async fn tasks_and_an_issue(fleet: &Hosted) -> (String, String, String, String) {
    let project = launched(fleet).await;
    let at = manifest(fleet);
    let a = newest(&fleet.add_project_task(new_task(at.clone(), Some(project.clone()), "A")).await.unwrap());
    let b = newest(&fleet.add_project_task(new_task(at.clone(), Some(project.clone()), "B")).await.unwrap());
    let i = newest(&fleet.link_project_issue(new_issue(at, Some(project.clone()), "ABC-1")).await.unwrap());
    (project, a, b, i)
}

fn sub(title: &str) -> ProjectTask {
    ProjectTask { title: title.to_string(), notes: String::new() }
}

fn move_to(position: u32, parent: Option<&str>) -> MoveProjectRow {
    MoveProjectRow { project: None, position, parent: parent.map(str::to_string) }
}

#[tokio::test]
async fn a_subtask_is_listed_straight_after_its_parent_and_counts_its_place_among_its_siblings() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let (_project, a, b, _) = tasks_and_an_issue(&fleet).await;

    let first = fleet.add_project_subtask(a.clone(), sub("A1")).await.expect("added");
    let a1 = last_sub(&first, &a);
    assert_eq!(first.projects[0].rows.len(), 4);
    let second = fleet.add_project_subtask(a.clone(), sub("A2")).await.expect("added");
    let a2 = last_sub(&second, &a);
    assert_eq!(
        shape(&second)[..4],
        [(a.clone(), None), (a1.clone(), Some(a.clone())), (a2.clone(), Some(a.clone())), (b.clone(), None)],
        "the parent, its subtasks in the order added, then the next top-level row"
    );
    let moved = fleet.move_project_row(a2.clone(), move_to(0, None)).await.expect("moved");
    assert_eq!(shape(&moved)[1].0, a2, "a move within the parent counts its subtasks only");
}

#[tokio::test]
async fn a_subtask_has_no_subtasks_and_an_issue_row_is_neither_parent_nor_subtask() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let (_project, a, b, issue) = tasks_and_an_issue(&fleet).await;
    let a1 = last_sub(&fleet.add_project_subtask(a.clone(), sub("A1")).await.unwrap(), &a);
    let before = fleet.list_projects().await.unwrap();

    let too_deep = (422, "fleet.project_subtask_too_deep".to_string());
    let not_a_task = (422, "fleet.project_row_not_a_task".to_string());
    let deeper = fleet.add_project_subtask(a1.clone(), sub("A1a")).await.expect_err("refused");
    assert_eq!(code_of(deeper), too_deep, "a subtask of a subtask");
    let of_issue = fleet.add_project_subtask(issue.clone(), sub("x")).await.expect_err("refused");
    assert_eq!(code_of(of_issue), not_a_task, "a subtask of an issue row");
    let blank = fleet.add_project_subtask(a.clone(), sub(" ")).await.expect_err("refused");
    assert_eq!(code_of(blank), (422, "fleet.project_blank".to_string()));
    let missing = fleet.add_project_subtask("nope".into(), sub("x")).await.expect_err("refused");
    assert_eq!(code_of(missing), (404, "fleet.no_such_project_row".to_string()));

    let under_a_subtask = fleet.move_project_row(b.clone(), move_to(0, Some(&a1))).await.expect_err("refused");
    assert_eq!(code_of(under_a_subtask), too_deep, "a task under a subtask");
    let with_subtasks = fleet.move_project_row(a.clone(), move_to(0, Some(&b))).await.expect_err("refused");
    assert_eq!(code_of(with_subtasks), too_deep, "a task that has subtasks under another");
    let itself = fleet.move_project_row(b.clone(), move_to(0, Some(&b))).await.expect_err("refused");
    assert_eq!(code_of(itself), too_deep, "a task under itself");
    let issue_under = fleet.move_project_row(issue.clone(), move_to(0, Some(&a))).await.expect_err("refused");
    assert_eq!(code_of(issue_under), not_a_task, "an issue row under a task");
    let under_issue = fleet.move_project_row(b.clone(), move_to(0, Some(&issue))).await.expect_err("refused");
    assert_eq!(code_of(under_issue), not_a_task, "a task under an issue row");
    let under_nothing = fleet.move_project_row(b, move_to(0, Some("nope"))).await.expect_err("refused");
    assert_eq!(code_of(under_nothing), (404, "fleet.no_such_project_row".to_string()));

    assert_eq!(fleet.list_projects().await.unwrap(), before, "nothing refused was kept");
}

#[tokio::test]
async fn a_subtask_moves_to_another_parent_and_out_to_the_top_level_and_a_removed_parent_takes_its_subtasks() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let (_project, a, b, issue) = tasks_and_an_issue(&fleet).await;
    let a1 = last_sub(&fleet.add_project_subtask(a.clone(), sub("A1")).await.unwrap(), &a);

    let to_b = fleet.move_project_row(a1.clone(), move_to(0, Some(&b))).await.expect("moved");
    assert_eq!(shape(&to_b), [(a.clone(), None), (b.clone(), None), (a1.clone(), Some(b.clone())), (issue.clone(), None)]);
    let out = fleet.move_project_row(a1.clone(), move_to(0, Some(""))).await.expect("moved");
    assert_eq!(shape(&out), [(a1.clone(), None), (a.clone(), None), (b.clone(), None), (issue.clone(), None)], "out to the top, at its place");
    fleet.move_project_row(a1.clone(), move_to(0, Some(&a))).await.expect("back under a");

    let removed = fleet.remove_project_row(a.clone()).await.expect("removed");
    assert_eq!(shape(&removed), [(b, None), (issue, None)], "its subtask went with it");
}

#[tokio::test]
async fn an_issue_linked_to_a_task_reads_its_state_and_title_from_the_tracker_and_keeps_neither() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(7, "Open on the tracker")]));
    tracker.knows_closed(8, "Closed on the tracker");
    let fleet = a_fleet(&home, &tracker);
    let (_project, a, _, issue) = tasks_and_an_issue(&fleet).await;
    let a1 = last_sub(&fleet.add_project_subtask(a.clone(), sub("A1")).await.unwrap(), &a);
    let issues_of = |projects: &ipc::Projects, id: &str| -> Vec<TaskIssue> {
        projects.projects[0]
            .rows
            .iter()
            .find_map(|row| match row {
                ProjectRow::Task { id: one, issues, .. } if one == id => Some(issues.clone()),
                _ => None,
            })
            .expect("the task")
    };
    for reference in ["7", "#8", "9"] {
        fleet.link_task_issue(a.clone(), link(reference)).await.expect("linked");
    }
    let linked = fleet.link_task_issue(a.clone(), link("7")).await.expect("linked again");
    let on_sub = fleet.link_task_issue(a1.clone(), link("ABC-12")).await.expect("on a subtask");

    let linked_in = manifest(&fleet);
    let issue_state = |issue: &str, title: Option<&str>, state: Option<IssueState>| TaskIssue {
        issue: issue.to_string(),
        manifest_id: Some(linked_in.clone()),
        title: title.map(str::to_string),
        state,
    };
    assert_eq!(
        issues_of(&linked, &a),
        [
            issue_state("7", Some("Open on the tracker"), Some(IssueState::Open)),
            issue_state("8", Some("Closed on the tracker"), Some(IssueState::Closed)),
            issue_state("9", None, None),
        ],
        "in the order linked, once each; one the tracker has none of says nothing"
    );
    assert_eq!(issues_of(&on_sub, &a1)[0].issue, "ABC-12");

    let unlinked = fleet.unlink_task_issue(a.clone(), link("#8")).await.expect("unlinked");
    assert_eq!(issues_of(&unlinked, &a).iter().map(|one| one.issue.as_str()).collect::<Vec<_>>(), ["7", "9"]);
    fleet.unlink_task_issue(a.clone(), link("42")).await.expect("one it did not have changes nothing");

    let not_a_task = fleet.link_task_issue(issue.clone(), link("7")).await.expect_err("refused");
    assert_eq!(code_of(not_a_task), (422, "fleet.project_row_not_a_task".to_string()));
    let unreadable = fleet.link_task_issue(a.clone(), link("the login bug")).await.expect_err("refused");
    assert_eq!(code_of(unreadable), (422, "fleet.project_issue_unreadable".to_string()));
    drop(fleet);

    let silent = Arc::new(FakeTracker::empty());
    silent.will_not_list("nobody is signed in");
    let fleet = a_fleet(&home, &silent);
    let read = fleet.list_projects().await.expect("listed though the tracker refuses");
    assert_eq!(
        issues_of(&read, &a),
        [issue_state("7", None, None), issue_state("9", None, None)],
        "after a restart with a tracker that cannot answer, only the references were kept"
    );
}

#[tokio::test]
async fn each_subtask_and_link_write_publishes_the_projects_whole() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let (_project, a, _, _) = tasks_and_an_issue(&fleet).await;
    let mut watching = fleet.events().subscribe();

    let answered = fleet.add_project_subtask(a.clone(), sub("A1")).await.expect("added");
    let linked = fleet.link_task_issue(a.clone(), link("7")).await.expect("linked");
    let unlinked = fleet.unlink_task_issue(a, link("7")).await.expect("unlinked");

    let mut published = Vec::new();
    while published.len() < 3 {
        match tokio::time::timeout(std::time::Duration::from_secs(5), watching.next()).await.expect("an event") {
            Some(api::Next::Send(delivered)) => {
                if let Event::ProjectsChanged(projects) = delivered.event {
                    published.push(projects);
                }
            }
            other => panic!("the stream ended or dropped: {other:?}"),
        }
    }
    assert_eq!(published, [answered, linked, unlinked]);
}
