//! Projects through a real Fleet, its store and a scripted tracker: an issue row's state and title
//! are the tracker's and never kept, a task's state is its own, every write publishes the
//! projects whole, and each refusal is the status its neighbours use.

use std::sync::Arc;
use std::time::Duration;

use api::{Next, Projects, Refusal};
use ipc::{
    Event, IssueState, LinkProjectIssue, ManifestId, MoveProjectRow, NameProject, ProjectRow, ProjectTask,
    ProjectTaskState, TickProjectTask,
};
use testkit::{FakeHarness, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn manifest(fleet: &Hosted) -> ManifestId {
    let served = fleet.repositories().first().expect("a served repository");
    ManifestId::carried(served.manifest().id().as_str())
}

fn a_fleet(home: &TempDir, tracker: &Arc<FakeTracker>) -> Hosted {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.trackers = Arc::new(FakeTrackers::empty().tracker(Arc::clone(tracker)));
    Fleet::assembled(fittings)
}

fn name(name: &str) -> NameProject {
    NameProject { name: name.to_string() }
}

fn task(title: &str) -> ProjectTask {
    ProjectTask { title: title.to_string(), notes: String::new() }
}

fn link(issue: &str) -> LinkProjectIssue {
    LinkProjectIssue { issue: issue.to_string() }
}

/// One project, `Launch`, and its id.
async fn launched(fleet: &Hosted) -> String {
    let projects = fleet.add_project(manifest(fleet), name("Launch")).await.expect("added");
    projects.projects.last().expect("the project").id.clone()
}

fn row_ids(projects: &ipc::Projects) -> Vec<String> {
    projects.projects[0]
        .rows
        .iter()
        .map(|row| match row {
            ProjectRow::Task { id, .. } | ProjectRow::Issue { id, .. } => id.clone(),
        })
        .collect()
}

/// What each issue row of the first project says: reference, state, title.
fn issues_of(projects: &ipc::Projects) -> Vec<(String, Option<IssueState>, Option<String>)> {
    projects.projects[0]
        .rows
        .iter()
        .filter_map(|row| match row {
            ProjectRow::Issue { issue, state, title, .. } => Some((issue.clone(), *state, title.clone())),
            ProjectRow::Task { .. } => None,
        })
        .collect()
}

fn code_of(refusal: Refusal) -> (u16, String) {
    (refusal.status(), refusal.error().code.as_str().to_string())
}

#[tokio::test]
async fn an_issue_row_reads_its_state_and_title_from_the_tracker_and_keeps_neither() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(7, "Open on the tracker")]));
    tracker.knows_closed(8, "Closed on the tracker");
    let fleet = a_fleet(&home, &tracker);
    let project = launched(&fleet).await;
    for issue in ["7", "#8", "9"] {
        fleet.link_project_issue(manifest(&fleet), project.clone(), link(issue)).await.expect("linked");
    }

    let read = fleet.list_projects(manifest(&fleet)).await.expect("listed");
    assert_eq!(
        issues_of(&read),
        [
            ("7".to_string(), Some(IssueState::Open), Some("Open on the tracker".to_string())),
            ("8".to_string(), Some(IssueState::Closed), Some("Closed on the tracker".to_string())),
            ("9".to_string(), None, None),
        ],
        "open from the list, closed read by reference, and one the tracker has none of says nothing"
    );
    drop(fleet);

    let silent = Arc::new(FakeTracker::empty());
    silent.will_not_list("nobody is signed in");
    let fleet = a_fleet(&home, &silent);
    let read = fleet.list_projects(manifest(&fleet)).await.expect("listed though the tracker refuses");
    assert_eq!(
        issues_of(&read),
        [("7".to_string(), None, None), ("8".to_string(), None, None), ("9".to_string(), None, None)],
        "after a restart with a tracker that cannot answer, nothing was kept to answer with"
    );
}

#[tokio::test]
async fn a_task_ticks_and_unticks_and_an_issue_row_refuses_both_tick_and_edit() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let project = launched(&fleet).await;
    fleet.add_project_task(manifest(&fleet), project.clone(), task("Write the page")).await.expect("added");
    let read = fleet.link_project_issue(manifest(&fleet), project.clone(), link("ABC-12")).await.expect("linked");
    let (task_row, issue_row) = (row_ids(&read)[0].clone(), row_ids(&read)[1].clone());

    let ticked = fleet
        .tick_project_task(manifest(&fleet), project.clone(), task_row.clone(), TickProjectTask { done: true })
        .await
        .expect("ticked");
    assert!(matches!(&ticked.projects[0].rows[0], ProjectRow::Task { state: ProjectTaskState::Done, .. }));
    let unticked = fleet
        .tick_project_task(manifest(&fleet), project.clone(), task_row, TickProjectTask { done: false })
        .await
        .expect("unticked");
    assert!(matches!(&unticked.projects[0].rows[0], ProjectRow::Task { state: ProjectTaskState::Open, .. }));

    let tick = fleet.tick_project_task(manifest(&fleet), project.clone(), issue_row.clone(), TickProjectTask { done: true });
    assert_eq!(code_of(tick.await.expect_err("refused")), (422, "fleet.project_row_not_a_task".to_string()));
    let edit = fleet.edit_project_task(manifest(&fleet), project, issue_row, task("A title of its own"));
    assert_eq!(code_of(edit.await.expect_err("refused")), (422, "fleet.project_row_not_a_task".to_string()));
}

#[tokio::test]
async fn a_task_is_edited_moved_and_removed_and_a_project_renamed_and_deleted() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let project = launched(&fleet).await;
    for title in ["a", "b", "c"] {
        fleet.add_project_task(manifest(&fleet), project.clone(), task(title)).await.expect("added");
    }
    let ids = row_ids(&fleet.list_projects(manifest(&fleet)).await.expect("listed"));

    let edited = fleet
        .edit_project_task(
            manifest(&fleet),
            project.clone(),
            ids[0].clone(),
            ProjectTask { title: "  A  ".to_string(), notes: " why ".to_string() },
        )
        .await
        .expect("edited");
    assert!(matches!(&edited.projects[0].rows[0], ProjectRow::Task { title, notes, .. } if title == "A" && notes == "why"));
    let moved = fleet
        .move_project_row(manifest(&fleet), project.clone(), ids[2].clone(), MoveProjectRow { position: 0 })
        .await
        .expect("moved");
    assert_eq!(row_ids(&moved), [ids[2].clone(), ids[0].clone(), ids[1].clone()]);
    let removed = fleet.remove_project_row(manifest(&fleet), project.clone(), ids[0].clone()).await.expect("removed");
    assert_eq!(row_ids(&removed), [ids[2].clone(), ids[1].clone()]);

    let renamed = fleet.rename_project(manifest(&fleet), project.clone(), name("Later")).await.expect("renamed");
    assert_eq!(renamed.projects[0].name, "Later");
    let left = fleet.delete_project(manifest(&fleet), project).await.expect("deleted");
    assert!(left.projects.is_empty());
}

#[tokio::test]
async fn a_write_publishes_the_projects_whole() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let mut watching = fleet.events().subscribe();

    let answered = fleet.add_project(manifest(&fleet), name("Launch")).await.expect("added");

    let published = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match watching.next().await {
                Some(Next::Send(delivered)) => {
                    if let Event::ProjectsChanged(projects) = delivered.event {
                        return projects;
                    }
                }
                other => panic!("the stream ended or dropped: {other:?}"),
            }
        }
    })
    .await
    .expect("a projects.changed arrives");
    assert_eq!(published, answered, "what was answered is what was published");
}

#[tokio::test]
async fn each_refusal_is_its_neighbours_status() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let project = launched(&fleet).await;
    let at = manifest(&fleet);

    let nowhere = fleet.list_projects(ManifestId::carried("01NOTSERVEDHERE")).await.expect_err("refused");
    assert_eq!(nowhere.status(), 422, "a repository Fleet does not serve");
    let blank = fleet.add_project(at.clone(), name("   ")).await.expect_err("refused");
    assert_eq!(code_of(blank), (422, "fleet.project_blank".to_string()));
    let blank = fleet.add_project_task(at.clone(), project.clone(), task(" ")).await.expect_err("refused");
    assert_eq!(code_of(blank), (422, "fleet.project_blank".to_string()));
    let missing = fleet.rename_project(at.clone(), "nope".to_string(), name("Later")).await.expect_err("refused");
    assert_eq!(code_of(missing), (404, "fleet.no_such_project".to_string()));
    let missing = fleet.remove_project_row(at.clone(), project.clone(), "nope".to_string()).await.expect_err("refused");
    assert_eq!(code_of(missing), (404, "fleet.no_such_project_row".to_string()));
    let not_an_issue = fleet.link_project_issue(at.clone(), project, link("the login bug")).await.expect_err("refused");
    assert_eq!(code_of(not_an_issue), (422, "fleet.project_issue_unreadable".to_string()));
    assert!(fleet.list_projects(at).await.expect("listed").projects[0].rows.is_empty(), "nothing refused was kept");
}
