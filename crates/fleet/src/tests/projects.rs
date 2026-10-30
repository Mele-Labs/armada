//! Projects through a real Fleet, its store and a scripted tracker: an issue row's state and title
//! are the tracker's and never kept, a task's state is its own, every write publishes the
//! projects whole, and each refusal is the status its neighbours use.

use std::sync::Arc;
use std::time::Duration;

use api::{Next, Projects, Refusal};
use ipc::{
    Event, IssueState, LinkProjectIssue, LinkTaskPull, ManifestId, MoveProjectRow, NameProject, NewProjectIssue, NewProjectTask,
    ProjectRow, ProjectTask, ProjectTaskState, TickProjectTask,
};
use testkit::{FakeHarness, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::repositories::{Located, SetUp};
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

pub(super) type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

pub(super) fn manifest(fleet: &Hosted) -> ManifestId {
    let served = fleet.repositories().first().expect("a served repository");
    ManifestId::carried(served.manifest().id().as_str())
}

pub(super) fn a_fleet(home: &TempDir, tracker: &Arc<FakeTracker>) -> Hosted {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.trackers = Arc::new(FakeTrackers::empty().tracker(Arc::clone(tracker)));
    Fleet::assembled(fittings)
}

pub(super) fn name(name: &str) -> NameProject {
    NameProject { name: name.to_string() }
}

pub(super) fn task(title: &str) -> ProjectTask {
    ProjectTask { title: title.to_string(), notes: String::new() }
}

pub(super) fn link(issue: &str) -> LinkProjectIssue {
    LinkProjectIssue { issue: issue.to_string() }
}

/// A task in `manifest`, in `project` or, with none, loose.
pub(super) fn new_task(manifest: ManifestId, project: Option<String>, title: &str) -> NewProjectTask {
    NewProjectTask { manifest_id: manifest, project, title: title.to_string(), notes: String::new() }
}

/// An issue of `manifest`, a row of `project` or, with none, a loose one.
pub(super) fn new_issue(manifest: ManifestId, project: Option<String>, issue: &str) -> NewProjectIssue {
    NewProjectIssue { manifest_id: manifest, project, issue: issue.to_string() }
}

/// One project, `Launch`, and its id.
pub(super) async fn launched(fleet: &Hosted) -> String {
    let projects = fleet.add_project(name("Launch")).await.expect("added");
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

pub(super) fn code_of(refusal: Refusal) -> (u16, String) {
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
        fleet.link_project_issue(new_issue(manifest(&fleet), Some(project.clone()), issue)).await.expect("linked");
    }

    let read = fleet.list_projects().await.expect("listed");
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
    let read = fleet.list_projects().await.expect("listed though the tracker refuses");
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
    fleet.add_project_task(new_task(manifest(&fleet), Some(project.clone()), "Write the page")).await.expect("added");
    let read = fleet.link_project_issue(new_issue(manifest(&fleet), Some(project.clone()), "ABC-12")).await.expect("linked");
    let (task_row, issue_row) = (row_ids(&read)[0].clone(), row_ids(&read)[1].clone());

    let ticked = fleet
        .tick_project_task(task_row.clone(), TickProjectTask { done: true })
        .await
        .expect("ticked");
    assert!(matches!(&ticked.projects[0].rows[0], ProjectRow::Task { state: ProjectTaskState::Done, .. }));
    let unticked = fleet
        .tick_project_task(task_row, TickProjectTask { done: false })
        .await
        .expect("unticked");
    assert!(matches!(&unticked.projects[0].rows[0], ProjectRow::Task { state: ProjectTaskState::Open, .. }));

    let tick = fleet.tick_project_task(issue_row.clone(), TickProjectTask { done: true });
    assert_eq!(code_of(tick.await.expect_err("refused")), (422, "fleet.project_row_not_a_task".to_string()));
    let edit = fleet.edit_project_task(issue_row, task("A title of its own"));
    assert_eq!(code_of(edit.await.expect_err("refused")), (422, "fleet.project_row_not_a_task".to_string()));
}

#[tokio::test]
async fn a_task_is_edited_moved_and_removed_and_a_project_renamed_and_deleted() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let project = launched(&fleet).await;
    for title in ["a", "b", "c"] {
        fleet.add_project_task(new_task(manifest(&fleet), Some(project.clone()), title)).await.expect("added");
    }
    let ids = row_ids(&fleet.list_projects().await.expect("listed"));

    let edited = fleet
        .edit_project_task(ids[0].clone(),
            ProjectTask { title: "  A  ".to_string(), notes: " why ".to_string() },
        )
        .await
        .expect("edited");
    assert!(matches!(&edited.projects[0].rows[0], ProjectRow::Task { title, notes, .. } if title == "A" && notes == "why"));
    let moved = fleet
        .move_project_row(ids[2].clone(), MoveProjectRow { project: None, position: 0, parent: None })
        .await
        .expect("moved");
    assert_eq!(row_ids(&moved), [ids[2].clone(), ids[0].clone(), ids[1].clone()]);
    let removed = fleet.remove_project_row(ids[0].clone()).await.expect("removed");
    assert_eq!(row_ids(&removed), [ids[2].clone(), ids[1].clone()]);

    let renamed = fleet.rename_project(project.clone(), name("Later")).await.expect("renamed");
    assert_eq!(renamed.projects[0].name, "Later");
    let left = fleet.delete_project(project).await.expect("deleted");
    assert!(left.projects.is_empty());
}

#[tokio::test]
async fn a_write_publishes_the_projects_whole() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let mut watching = fleet.events().subscribe();

    let answered = fleet.add_project(name("Launch")).await.expect("added");

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

    let nowhere = fleet.add_project_task(new_task(ManifestId::carried("01NOTSERVEDHERE"), Some(project.clone()), "x")).await;
    assert_eq!(nowhere.expect_err("refused").status(), 422, "a repository Fleet does not serve");
    let no_project = fleet.add_project_task(new_task(at.clone(), Some("nope".to_string()), "x")).await.expect_err("refused");
    assert_eq!(code_of(no_project), (404, "fleet.no_such_project".to_string()));
    let sent_nowhere = fleet.move_project_row("nope".to_string(), MoveProjectRow { project: Some("nope".to_string()), position: 0, parent: None });
    assert_eq!(code_of(sent_nowhere.await.expect_err("refused")), (404, "fleet.no_such_project_row".to_string()));
    let blank = fleet.add_project(name("   ")).await.expect_err("refused");
    assert_eq!(code_of(blank), (422, "fleet.project_blank".to_string()));
    let blank = fleet.add_project_task(new_task(at.clone(), Some(project.clone()), " ")).await.expect_err("refused");
    assert_eq!(code_of(blank), (422, "fleet.project_blank".to_string()));
    let missing = fleet.rename_project("nope".to_string(), name("Later")).await.expect_err("refused");
    assert_eq!(code_of(missing), (404, "fleet.no_such_project".to_string()));
    let missing = fleet.remove_project_row("nope".to_string()).await.expect_err("refused");
    assert_eq!(code_of(missing), (404, "fleet.no_such_project_row".to_string()));
    let not_an_issue = fleet.link_project_issue(new_issue(at.clone(), Some(project), "the login bug")).await.expect_err("refused");
    assert_eq!(code_of(not_an_issue), (422, "fleet.project_issue_unreadable".to_string()));
    assert!(fleet.list_projects().await.expect("listed").projects[0].rows.is_empty(), "nothing refused was kept");
}

/// Serves a second repository beside the first, and its id.
pub(super) fn a_second_repository(fleet: &Hosted, home: &TempDir) -> ManifestId {
    let root = home.path().join("second").to_string_lossy().to_string();
    std::fs::create_dir_all(&root).expect("a folder");
    let text = "version: 1\nid: 01SECONDMANIFEST\n";
    let manifest = config::Manifest::parse(&std::path::Path::new(&root).join("armada.yml"), text).expect("a Manifest");
    fleet
        .repositories()
        .add(Located { root: root.clone(), records_root: root, set_up: Some(SetUp::of(manifest, Default::default())) })
        .expect("served");
    ManifestId::carried("01SECONDMANIFEST")
}

fn named_in(rows: &[ProjectRow], title: &str) -> Option<String> {
    rows.iter().find_map(|row| match row {
        ProjectRow::Task { title: one, manifest_id, .. } if one == title => Some(manifest_id.as_str().to_string()),
        _ => None,
    })
}

#[tokio::test]
async fn a_project_holds_tasks_of_two_repositories_in_his_order_and_a_task_in_none_is_loose() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let (first, second) = (manifest(&fleet), a_second_repository(&fleet, &home));
    let project = launched(&fleet).await;

    fleet.add_project_task(new_task(first.clone(), Some(project.clone()), "In the first")).await.expect("added");
    let both = fleet.add_project_task(new_task(second.clone(), Some(project.clone()), "In the second")).await.expect("added");
    assert_eq!(both.projects.len(), 1, "one project, whatever repositories its rows are in");
    assert_eq!(named_in(&both.projects[0].rows, "In the first").as_deref(), Some(first.as_str()));
    assert_eq!(named_in(&both.projects[0].rows, "In the second").as_deref(), Some(second.as_str()));

    let loose = fleet.add_project_task(new_task(second.clone(), None, "Loose in the second")).await.expect("added");
    assert_eq!(row_ids(&loose).len(), 2, "a loose task is in no project");
    assert_eq!(named_in(&loose.loose, "Loose in the second").as_deref(), Some(second.as_str()));
    let empty = fleet.add_project_task(new_task(first.clone(), Some(String::new()), "Loose in the first")).await.expect("an empty project is none");
    assert_eq!(empty.loose.len(), 2);

    let ids = row_ids(&fleet.list_projects().await.expect("listed"));
    let moved = fleet
        .move_project_row(ids[1].clone(), MoveProjectRow { project: None, position: 0, parent: None })
        .await
        .expect("moved");
    assert_eq!(row_ids(&moved), [ids[1].clone(), ids[0].clone()], "one order across repositories, his");
}

#[tokio::test]
async fn moving_a_row_between_projects_or_out_of_every_one_keeps_its_repository() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let (first, second) = (manifest(&fleet), a_second_repository(&fleet, &home));
    let one = launched(&fleet).await;
    let other = fleet.add_project(name("Later")).await.expect("added").projects[1].id.clone();
    let added = fleet.add_project_task(new_task(second.clone(), Some(one.clone()), "Travels")).await.expect("added");
    let travels = row_ids(&added)[0].clone();
    fleet.add_project_subtask(travels.clone(), task("With a subtask")).await.expect("added");
    fleet.add_project_task(new_task(first.clone(), Some(other.clone()), "Waits")).await.expect("added");

    let sent = fleet
        .move_project_row(travels.clone(), MoveProjectRow { project: Some(other.clone()), position: 0, parent: None })
        .await
        .expect("moved");
    assert!(sent.projects[0].rows.is_empty());
    assert_eq!(sent.projects[1].rows.len(), 3, "the task, its subtask and the one waiting");
    assert_eq!(named_in(&sent.projects[1].rows, "Travels").as_deref(), Some(second.as_str()), "still the second repository's");
    assert_eq!(named_in(&sent.projects[1].rows, "With a subtask").as_deref(), Some(second.as_str()), "and so is its subtask");

    let out = fleet
        .move_project_row(travels.clone(), MoveProjectRow { project: Some(String::new()), position: 0, parent: None })
        .await
        .expect("moved");
    assert_eq!(out.projects[1].rows.len(), 1);
    assert_eq!(out.loose.len(), 2, "the task and its subtask are loose now");
    assert_eq!(named_in(&out.loose, "Travels").as_deref(), Some(second.as_str()));

    let under_nothing = fleet
        .move_project_row(travels, MoveProjectRow { project: Some(one), position: 0, parent: Some("nope".to_string()) })
        .await
        .expect_err("refused");
    assert_eq!(code_of(under_nothing), (404, "fleet.no_such_project_row".to_string()));
}

#[tokio::test]
async fn a_loose_task_ticks_and_is_linked_and_removed_like_any() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(7, "Open on the tracker")]));
    let fleet = a_fleet(&home, &tracker);
    let added = fleet.add_project_task(new_task(manifest(&fleet), None, "Loose")).await.expect("added");
    let id = match &added.loose[0] {
        ProjectRow::Task { id, .. } => id.clone(),
        ProjectRow::Issue { .. } => panic!("a task"),
    };
    fleet.link_task_issue(id.clone(), link("7")).await.expect("linked");
    let ticked = fleet.tick_project_task(id.clone(), TickProjectTask { done: true }).await.expect("ticked");
    assert!(matches!(&ticked.loose[0], ProjectRow::Task { state: ProjectTaskState::Done, issues, .. } if issues[0].state == Some(IssueState::Open)));
    let issue = fleet.link_project_issue(new_issue(manifest(&fleet), None, "7")).await.expect("linked");
    assert!(matches!(&issue.loose[1], ProjectRow::Issue { issue, .. } if issue == "7"), "a loose issue row too");
    let gone = fleet.remove_project_row(id).await.expect("removed");
    assert_eq!(gone.loose.len(), 1);
    assert!(gone.projects.is_empty());
}

/// The repositories the store keeps for task `row`'s issue links and pull request links, in the order linked.
async fn link_repositories(fleet: &Hosted, row: &str) -> (Vec<String>, Vec<String>) {
    let kept = fleet.store().lock().await.projects().expect("read");
    let found = kept.projects.into_iter().flat_map(|one| one.rows).chain(kept.loose).find(|one| one.id == row).expect("the row");
    match found.item {
        store::KeptRowItem::Task { issues, pulls, .. } => {
            (issues.into_iter().map(|one| one.manifest_id).collect(), pulls.into_iter().map(|one| one.manifest_id).collect())
        }
        store::KeptRowItem::Issue { .. } => panic!("a task"),
    }
}

fn state_in(rows: &[ProjectRow], title: &str) -> Option<ProjectTaskState> {
    rows.iter().find_map(|row| match row {
        ProjectRow::Task { title: one, state, .. } if one == title => Some(*state),
        _ => None,
    })
}

#[tokio::test]
async fn moving_a_task_to_another_repository_takes_its_subtasks_and_leaves_its_links_in_the_one_they_were_linked_in() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let (first, second) = (manifest(&fleet), a_second_repository(&fleet, &home));
    let project = launched(&fleet).await;
    let travels = row_ids(&fleet.add_project_task(new_task(first.clone(), Some(project.clone()), "Travels")).await.expect("added"))[0].clone();
    let sub = row_ids(&fleet.add_project_subtask(travels.clone(), task("Sub")).await.expect("added"))[1].clone();
    fleet.add_project_task(new_task(first.clone(), Some(project), "Stays")).await.expect("added");
    fleet.link_task_issue(travels.clone(), link("7")).await.expect("linked");
    fleet.link_task_pull(travels.clone(), LinkTaskPull { number: 5 }).await.expect("linked");
    fleet.link_task_issue(sub.clone(), link("8")).await.expect("linked");
    fleet.store().lock().await.hold_project_tasks("s1", std::slice::from_ref(&travels)).expect("held");
    let before = row_ids(&fleet.list_projects().await.expect("listed"));
    let mut watching = fleet.events().subscribe();

    let moved = fleet.move_project_task_repository(travels.clone(), second.clone()).await.expect("moved");

    let rows = &moved.projects[0].rows;
    assert_eq!(named_in(rows, "Travels").as_deref(), Some(second.as_str()));
    assert_eq!(named_in(rows, "Sub").as_deref(), Some(second.as_str()), "its subtask goes with it");
    assert_eq!(named_in(rows, "Stays").as_deref(), Some(first.as_str()), "and no other row");
    assert_eq!(row_ids(&moved), before, "each in the place it stood");
    assert_eq!(state_in(rows, "Travels"), Some(ProjectTaskState::Held), "a Job holding it keeps it");
    let first_only = vec![first.as_str().to_string()];
    assert_eq!(link_repositories(&fleet, &travels).await, (first_only.clone(), first_only.clone()), "its links stay the first's");
    assert_eq!(link_repositories(&fleet, &sub).await, (first_only, Vec::new()));
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
    assert_eq!(published, moved, "what was answered is what was published");

    let again = fleet.move_project_task_repository(travels, second).await.expect("to the repository it is in");
    assert_eq!(again, moved, "changes nothing");
}

#[tokio::test]
async fn a_linked_issue_and_pull_request_say_the_repository_they_were_linked_in_on_the_wire_after_their_task_moved() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let (first, second) = (manifest(&fleet), a_second_repository(&fleet, &home));
    let project = launched(&fleet).await;
    let travels = row_ids(&fleet.add_project_task(new_task(first.clone(), Some(project), "Travels")).await.expect("added"))[0].clone();
    fleet.link_task_issue(travels.clone(), link("7")).await.expect("linked");
    let linked = fleet.link_task_pull(travels.clone(), LinkTaskPull { number: 5 }).await.expect("linked");
    let links_of = |projects: &ipc::Projects| match &projects.projects[0].rows[0] {
        ProjectRow::Task { manifest_id, issues, pulls, .. } => (
            manifest_id.as_str().to_string(),
            issues.iter().map(|one| one.manifest_id.as_ref().map(|id| id.as_str().to_string())).collect::<Vec<_>>(),
            pulls.iter().map(|one| one.manifest_id.as_ref().map(|id| id.as_str().to_string())).collect::<Vec<_>>(),
        ),
        ProjectRow::Issue { .. } => panic!("a task"),
    };
    let first_only = vec![Some(first.as_str().to_string())];
    assert_eq!((links_of(&linked).1, links_of(&linked).2), (first_only.clone(), first_only.clone()), "a link says where it was linked, in the task's own repository too");

    let moved = fleet.move_project_task_repository(travels, second.clone()).await.expect("moved");
    assert_eq!(
        links_of(&moved),
        (second.as_str().to_string(), first_only.clone(), first_only),
        "the task is the second's, and each link still the first's"
    );
}

#[tokio::test]
async fn moving_a_subtask_an_issue_row_or_a_row_there_is_not_or_to_a_repository_not_served_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, &Arc::new(FakeTracker::empty()));
    let (first, second) = (manifest(&fleet), a_second_repository(&fleet, &home));
    let project = launched(&fleet).await;
    let travels = row_ids(&fleet.add_project_task(new_task(first.clone(), Some(project.clone()), "Travels")).await.expect("added"))[0].clone();
    let sub = row_ids(&fleet.add_project_subtask(travels.clone(), task("Sub")).await.expect("added"))[1].clone();
    let issue = row_ids(&fleet.link_project_issue(new_issue(first, Some(project), "7")).await.expect("linked"))[2].clone();
    let before = fleet.list_projects().await.expect("listed");

    let refused = |row: String, to: ManifestId| {
        let fleet = &fleet;
        async move { code_of(fleet.move_project_task_repository(row, to).await.expect_err("refused")) }
    };
    assert_eq!(refused(sub, second.clone()).await, (422, "fleet.project_subtask_too_deep".to_string()), "a subtask moves with its task");
    assert_eq!(refused(issue, second.clone()).await, (422, "fleet.project_row_not_a_task".to_string()));
    assert_eq!(refused("nope".to_string(), second).await, (404, "fleet.no_such_project_row".to_string()));
    let nowhere = refused(travels, ManifestId::carried("01NOTSERVEDHERE")).await;
    assert_eq!(nowhere, (422, "fleet.unacceptable_proposal".to_string()), "a repository Fleet does not serve");
    assert_eq!(fleet.list_projects().await.expect("listed"), before, "nothing refused was kept");
}
