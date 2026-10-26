//! A Job started with project rows holds the written tasks among them: each task reads as held by
//! that Job, an issue row keeps no state, and a start naming a row that is not there or is another
//! repository's is refused before a Job is made.

use api::{HostedSessions, Projects};
use ipc::{
    ManifestId, NameProject, ProjectHolder, ProjectRow, ProjectRowRef, ProjectTask, ProjectTaskState, StartSession,
};

use super::pull_notices::{a_rig, Rig};

fn task(title: &str) -> ProjectTask {
    ProjectTask { title: title.to_string(), notes: String::new() }
}

pub(super) fn rows_of(projects: &ipc::Projects, project: usize) -> &[ProjectRow] {
    &projects.projects[project].rows
}

pub(super) fn id_of(row: &ProjectRow) -> String {
    match row {
        ProjectRow::Task { id, .. } | ProjectRow::Issue { id, .. } => id.clone(),
    }
}

/// What a task row says: its state and the Job holding it.
pub(super) fn held(row: &ProjectRow) -> (ProjectTaskState, Option<ProjectHolder>) {
    match row {
        ProjectRow::Task { state, held_by, .. } => (*state, held_by.clone()),
        ProjectRow::Issue { .. } => panic!("a task"),
    }
}

/// One project `Pocket launch` with a task per title, then a linked issue: its id and row ids.
pub(super) async fn a_project(rig: &Rig, titles: &[&str]) -> (String, Vec<String>) {
    let at = ManifestId::carried(rig.manifest());
    let projects = rig.fleet.add_project(at.clone(), NameProject { name: "Pocket launch".into() }).await.expect("added");
    let project = projects.projects.last().expect("the project").id.clone();
    for title in titles {
        rig.fleet.add_project_task(at.clone(), project.clone(), task(title)).await.expect("added");
    }
    let read = rig
        .fleet
        .link_project_issue(at, project.clone(), ipc::LinkProjectIssue { issue: "ABC-12".into() })
        .await
        .expect("linked");
    (project, rows_of(&read, read.projects.len() - 1).iter().map(id_of).collect())
}

pub(super) fn start(rig: &Rig, title: &str, rows: Vec<ProjectRowRef>) -> StartSession {
    StartSession {
        manifest_id: ManifestId::carried(rig.manifest()),
        title: Some(title.to_string()),
        model: None,
        effort: None,
        mode: None,
        root: None,
        fork: None,
        request: None,
        after: None,
        held: false,
        project_rows: rows,
    }
}

pub(super) fn row_ref(project: &str, row: &str) -> ProjectRowRef {
    ProjectRowRef { project: project.to_string(), row: row.to_string() }
}

#[tokio::test]
async fn a_job_started_with_rows_holds_its_tasks_and_an_issue_row_keeps_no_state() {
    let rig = a_rig();
    let (project, rows) = a_project(&rig, &["Sign the build", "Ship it"]).await;
    let (sign, ship, issue) = (&rows[0], &rows[1], &rows[2]);

    let job = rig
        .fleet
        .start_session(start(&rig, "Sign the build", vec![row_ref(&project, sign), row_ref(&project, issue)]))
        .await
        .expect("started");

    let read = rig.fleet.list_projects(ManifestId::carried(rig.manifest())).await.expect("listed");
    let rows = rows_of(&read, 0);
    assert_eq!(
        held(&rows[0]),
        (
            ProjectTaskState::Held,
            Some(ProjectHolder { session_id: job.id.as_str().to_string(), title: Some("Sign the build".into()) })
        ),
        "the task the Job started with reads as held by it, with its title"
    );
    assert_eq!(held(&rows[1]), (ProjectTaskState::Open, None), "a task it did not name stays open");
    assert_eq!(id_of(&rows[1]), *ship);
    assert!(matches!(rows[2], ProjectRow::Issue { .. }), "the issue row is as it was: {:?}", rows[2]);
}

#[tokio::test]
async fn a_start_naming_a_row_of_another_repository_or_one_that_is_missing_is_refused_and_makes_no_job() {
    let rig = a_rig();
    let (project, rows) = a_project(&rig, &["Sign the build"]).await;
    {
        let mut store = rig.fleet.store().lock().await;
        store.add_project("some-other-repository", "foreign", "Theirs", "2026-10-26T10:00:00.000Z").unwrap();
    }
    let jobs = || async { rig.fleet.store().lock().await.find_sessions(&Default::default()).unwrap().len() };
    let before = jobs().await;

    let elsewhere = rig.fleet.start_session(start(&rig, "x", vec![row_ref("foreign", "anything")])).await.expect_err("refused");
    assert_eq!((elsewhere.status(), elsewhere.error().code.as_str()), (422, "fleet.project_row_other_repository"));
    let no_project = rig.fleet.start_session(start(&rig, "x", vec![row_ref("nope", "r")])).await.expect_err("refused");
    assert_eq!((no_project.status(), no_project.error().code.as_str()), (404, "fleet.no_such_project"));
    let no_row = rig.fleet.start_session(start(&rig, "x", vec![row_ref(&project, "nope")])).await.expect_err("refused");
    assert_eq!((no_row.status(), no_row.error().code.as_str()), (404, "fleet.no_such_project_row"));
    let half = rig
        .fleet
        .start_session(start(&rig, "x", vec![row_ref(&project, &rows[0]), row_ref(&project, "nope")]))
        .await
        .expect_err("refused");
    assert_eq!(half.status(), 404);

    assert_eq!(jobs().await, before, "no Job was made");
    let read = rig.fleet.list_projects(ManifestId::carried(rig.manifest())).await.expect("listed");
    assert_eq!(held(&rows_of(&read, 0)[0]), (ProjectTaskState::Open, None), "and the valid row was not held");
}
