//! A Job started with project rows holds the written tasks among them: each task reads as held by
//! that Job, an issue row keeps no state, and a start naming a row that is not there or is another
//! repository's is refused before a Job is made.

use std::sync::Arc;

use adapter_traits::{FromOutside, RecentlyMerged};
use api::{HostedSessions, Projects, Sessions};
use ipc::{
    AllDone, AnswerWaiting, CloseSession, DismissWaiting, ManifestId, NameProject, ProjectHolder, ProjectRow,
    ProjectRowRef, ProjectTask, ProjectTaskState, SessionId, StartSession, TickProjectTask, WaitingItem,
    WaitingSource,
};
use store::{AttachmentState, Holder};

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

/// The first project's task rows as `(title, state)`.
fn states(projects: &ipc::Projects) -> Vec<(String, ProjectTaskState)> {
    rows_of(projects, 0)
        .iter()
        .filter_map(|row| match row {
            ProjectRow::Task { title, state, .. } => Some((title.clone(), *state)),
            ProjectRow::Issue { .. } => None,
        })
        .collect()
}

async fn read(rig: &Rig) -> ipc::Projects {
    rig.fleet.list_projects(ManifestId::carried(rig.manifest())).await.expect("listed")
}

/// The `task_done` items on the Job's list.
async fn asks(rig: &Rig, job: &SessionId) -> Vec<WaitingItem> {
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session;
    record.waiting_for.into_iter().filter(|one| one.source == WaitingSource::TaskDone).collect()
}

/// A Job holding both written tasks of a project of three: `Sign the build`, `Ship it` and `Rest`.
async fn a_job_holding_two(rig: &Rig) -> (String, Vec<String>, SessionId) {
    let (project, rows) = a_project(rig, &["Sign the build", "Ship it", "Rest"]).await;
    let job = rig
        .fleet
        .start_session(start(rig, "Launch", vec![row_ref(&project, &rows[0]), row_ref(&project, &rows[1])]))
        .await
        .expect("started")
        .id;
    (project, rows, job)
}

fn answer(job: &SessionId, item: &str, choice: u32) -> AnswerWaiting {
    AnswerWaiting { session_id: job.clone(), item_id: item.to_string(), choice: Some(choice), text: None, mode: None }
}

async fn says_done(rig: &Rig, job: &SessionId) {
    rig.fleet.all_done(None, AllDone { session_id: Some(job.clone()) }).await.expect("done");
}

#[tokio::test]
async fn a_job_saying_it_is_done_asks_about_its_held_tasks_and_one_that_has_not_is_not_asked() {
    let rig = a_rig();
    let (project, _, job) = a_job_holding_two(&rig).await;
    assert!(asks(&rig, &job).await.is_empty(), "nothing is asked while the Job works");

    says_done(&rig, &job).await;

    let read = read(&rig).await;
    assert_eq!(
        states(&read),
        [
            ("Sign the build".to_string(), ProjectTaskState::Asked),
            ("Ship it".to_string(), ProjectTaskState::Asked),
            ("Rest".to_string(), ProjectTaskState::Open)
        ]
    );
    let [ask] = asks(&rig, &job).await.try_into().expect("one ask, for the one project");
    assert_eq!(ask.id, format!("task_done:{project}"));
    assert_eq!(ask.text, "Mark the Pocket launch tasks done?");
    assert_eq!(ask.options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Mark done", "Keep open"]);
    assert_eq!(ask.act.map(|act| act.kind), Some(ipc::WaitingActKind::Answer));

    let refused = Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: job.clone(), item_id: ask.id, reason: None, misread: false })
        .await
        .expect_err("answered, not dismissed");
    assert_eq!(refused.status(), 422);
}

#[tokio::test]
async fn a_held_tasks_pull_request_merging_asks_about_them_by_the_merge_alone() {
    let rig = a_rig();
    let (_, _, job) = a_job_holding_two(&rig).await;
    rig.holds(Holder::session(job.as_str()), "pr", "5", AttachmentState::Standing).await;
    rig.fleet.vcs().main_ci.followed_are(Some(Vec::new()));
    rig.fleet.vcs().main_ci.recently_merged_are(Some(vec![RecentlyMerged {
        number: 5,
        title: FromOutside::verbatim("Sign the build"),
        branch: FromOutside::verbatim("armada/5"),
        url: FromOutside::verbatim("https://forge.invalid/armada/pull/5"),
        author: None,
        merged_at: FromOutside::verbatim(rig.fleet.now().as_str()),
        commit: None,
    }]));
    let served = rig.fleet.repositories().served().remove(0);

    rig.fleet.notice_merged(&served).await;
    rig.reads().await;

    assert_eq!(states(&read(&rig).await)[..2], [("Sign the build".to_string(), ProjectTaskState::Asked), ("Ship it".to_string(), ProjectTaskState::Asked)]);
    assert_eq!(asks(&rig, &job).await.len(), 1, "and the Job's list carries the ask");
}

#[tokio::test]
async fn mark_done_makes_the_asked_tasks_done_and_tells_the_agent_nothing() {
    let rig = a_rig();
    let (project, _, job) = a_job_holding_two(&rig).await;
    says_done(&rig, &job).await;

    let record = Arc::clone(&rig.fleet).answer_waiting(answer(&job, &format!("task_done:{project}"), 0)).await.expect("answered");

    assert!(record.waiting_for.iter().all(|one| one.source != WaitingSource::TaskDone), "the item left");
    let read = read(&rig).await;
    assert_eq!(
        states(&read),
        [
            ("Sign the build".to_string(), ProjectTaskState::Done),
            ("Ship it".to_string(), ProjectTaskState::Done),
            ("Rest".to_string(), ProjectTaskState::Open)
        ]
    );
    assert!(matches!(&rows_of(&read, 0)[0], ProjectRow::Task { held_by: None, .. }), "no longer held");
    assert!(rig.runs_sent_to_the_session().is_empty(), "no message went to the agent");
    let again = Arc::clone(&rig.fleet).answer_waiting(answer(&job, &format!("task_done:{project}"), 0)).await.expect_err("gone");
    assert_eq!(again.status(), 409, "a second answer finds it settled");
}

#[tokio::test]
async fn keep_open_opens_the_asked_tasks_again_and_lets_them_go() {
    let rig = a_rig();
    let (project, _, job) = a_job_holding_two(&rig).await;
    says_done(&rig, &job).await;

    Arc::clone(&rig.fleet).answer_waiting(answer(&job, &format!("task_done:{project}"), 1)).await.expect("answered");

    let read = read(&rig).await;
    assert_eq!(states(&read).iter().map(|(_, state)| *state).collect::<Vec<_>>(), [ProjectTaskState::Open; 3]);
    assert!(matches!(&rows_of(&read, 0)[1], ProjectRow::Task { held_by: None, .. }));
    assert!(asks(&rig, &job).await.is_empty());
    assert!(rig.runs_sent_to_the_session().is_empty(), "no message went to the agent");
}

#[tokio::test]
async fn the_tick_route_settles_an_asked_row_and_the_item_names_what_is_left() {
    let rig = a_rig();
    let (project, rows, job) = a_job_holding_two(&rig).await;
    says_done(&rig, &job).await;
    let at = ManifestId::carried(rig.manifest());

    rig.fleet.tick_project_task(at.clone(), project.clone(), rows[0].clone(), TickProjectTask { done: true }).await.expect("ticked");
    let [ask] = asks(&rig, &job).await.try_into().expect("the other task is still asked");
    assert_eq!(ask.text, "Mark Ship it done?", "one task left, so it is named");

    let read = rig.fleet.tick_project_task(at, project, rows[1].clone(), TickProjectTask { done: false }).await.expect("unticked");
    assert_eq!(
        states(&read)[..2],
        [("Sign the build".to_string(), ProjectTaskState::Done), ("Ship it".to_string(), ProjectTaskState::Open)]
    );
    assert!(asks(&rig, &job).await.is_empty(), "the item left with the last task");
}

#[tokio::test]
async fn ending_a_job_that_asked_leaves_the_ask_on_the_rows_for_the_projects_tab() {
    let rig = a_rig();
    let (project, rows, job) = a_job_holding_two(&rig).await;
    says_done(&rig, &job).await;

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: job.clone() }).await.expect("ended");

    assert_eq!(
        states(&read(&rig).await)[..2],
        [("Sign the build".to_string(), ProjectTaskState::Asked), ("Ship it".to_string(), ProjectTaskState::Asked)],
        "the ask outlived the Job"
    );
    assert!(asks(&rig, &job).await.is_empty(), "the ended Job lists nothing");
    let at = ManifestId::carried(rig.manifest());
    let read = rig.fleet.tick_project_task(at, project, rows[0].clone(), TickProjectTask { done: true }).await.expect("ticked");
    assert_eq!(states(&read)[0].1, ProjectTaskState::Done, "and the tab can still answer it");
}

#[tokio::test]
async fn a_job_ended_without_merging_or_saying_done_frees_its_held_tasks_to_open() {
    let rig = a_rig();
    let (_, _, job) = a_job_holding_two(&rig).await;
    assert_eq!(states(&read(&rig).await)[0].1, ProjectTaskState::Held);

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: job }).await.expect("ended");

    let read = read(&rig).await;
    assert_eq!(states(&read).iter().map(|(_, state)| *state).collect::<Vec<_>>(), [ProjectTaskState::Open; 3]);
    assert!(matches!(&rows_of(&read, 0)[0], ProjectRow::Task { held_by: None, .. }), "and none is held");
}
