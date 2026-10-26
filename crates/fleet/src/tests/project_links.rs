//! A task linked to a Job already running is held by it as if the Job had been started with it, and
//! one linked to a pull request is asked about when that merges, with no Job to list the ask on.
//! A pull request's title and state are the forge's, read when the rows are, and never kept.

use std::sync::Arc;

use adapter_traits::{FromOutside, PullRequestFacts, PullRequestStanding, RecentlyMerged};
use api::{HostedSessions, Projects, Sessions};
use ipc::{
    AllDone, CloseSession, LinkTaskJob, LinkTaskPull, ManifestId, ProjectHolder, ProjectRow, ProjectTask,
    ProjectTaskState, SessionId, TaskPull, TaskPullState, TickProjectTask, WaitingItem, WaitingSource,
};
use store::{KeptSession, SessionState};

use super::project_holds::{a_project, held, id_of, rows_of, start};
use super::projects::code_of;
use super::pull_notices::{a_rig, Rig};

fn at(rig: &Rig) -> ManifestId {
    ManifestId::carried(rig.manifest())
}

fn job_link(job: &SessionId) -> LinkTaskJob {
    LinkTaskJob { session_id: job.clone() }
}

async fn read(rig: &Rig) -> ipc::Projects {
    rig.fleet.list_projects(at(rig)).await.expect("listed")
}

/// A running Job that was started with no rows, titled `title`.
async fn a_job(rig: &Rig, title: &str) -> SessionId {
    rig.fleet.start_session(start(rig, title, Vec::new())).await.expect("started").id
}

/// A project of `Parent`, `Other` and an issue row, with a subtask `Sub` under `Parent`; the ids of
/// the project, `Parent`, `Sub`, `Other` and the issue.
async fn a_parent_with_a_subtask(rig: &Rig) -> (String, String, String, String, String) {
    let (project, rows) = a_project(rig, &["Parent", "Other"]).await;
    let sub = rig
        .fleet
        .add_project_subtask(at(rig), project.clone(), rows[0].clone(), ProjectTask { title: "Sub".into(), notes: String::new() })
        .await
        .expect("added");
    let sub = id_of(&rows_of(&sub, 0)[1]);
    (project, rows[0].clone(), sub, rows[1].clone(), rows[2].clone())
}

fn holder(job: &SessionId, title: &str) -> Option<ProjectHolder> {
    Some(ProjectHolder { session_id: job.as_str().to_string(), title: Some(title.to_string()) })
}

#[tokio::test]
async fn linking_a_running_job_holds_the_task_and_its_subtasks_and_another_job_replaces_it() {
    let rig = a_rig();
    let (project, parent, sub, _, _) = a_parent_with_a_subtask(&rig).await;
    let (first, second) = (a_job(&rig, "First").await, a_job(&rig, "Second").await);

    let read_after = rig.fleet.link_task_job(at(&rig), project.clone(), parent.clone(), job_link(&first)).await.expect("linked");
    let rows = rows_of(&read_after, 0);
    assert_eq!(held(&rows[0]), (ProjectTaskState::Held, holder(&first, "First")));
    assert_eq!(held(&rows[1]), (ProjectTaskState::Held, holder(&first, "First")), "its subtask with it");
    assert_eq!(held(&rows[2]).0, ProjectTaskState::Open, "a task it was not linked to stays open");

    let replaced = rig.fleet.link_task_job(at(&rig), project.clone(), parent.clone(), job_link(&second)).await.expect("linked");
    assert_eq!(held(&rows_of(&replaced, 0)[0]), (ProjectTaskState::Held, holder(&second, "Second")), "a task holds one Job");
    assert_eq!(held(&rows_of(&replaced, 0)[1]).1, holder(&second, "Second"));

    let only_the_subtask = rig.fleet.link_task_job(at(&rig), project.clone(), sub.clone(), job_link(&first)).await.expect("linked");
    assert_eq!(held(&rows_of(&only_the_subtask, 0)[0]).1, holder(&second, "Second"), "a subtask links alone");
    assert_eq!(held(&rows_of(&only_the_subtask, 0)[1]).1, holder(&first, "First"));
}

#[tokio::test]
async fn unlinking_the_job_frees_the_task_and_its_subtasks_and_leaves_what_is_not_held() {
    let rig = a_rig();
    let (project, parent, _, other, _) = a_parent_with_a_subtask(&rig).await;
    let job = a_job(&rig, "Launch").await;
    rig.fleet.link_task_job(at(&rig), project.clone(), parent.clone(), job_link(&job)).await.expect("linked");

    let freed = rig.fleet.unlink_task_job(at(&rig), project.clone(), parent).await.expect("unlinked");
    for row in &rows_of(&freed, 0)[..2] {
        assert_eq!(held(row), (ProjectTaskState::Open, None));
    }
    let again = rig.fleet.unlink_task_job(at(&rig), project, other).await.expect("a task with no Job changes nothing");
    assert_eq!(held(&rows_of(&again, 0)[2]), (ProjectTaskState::Open, None));
}

#[tokio::test]
async fn a_linked_job_finishing_asks_about_the_task_as_a_job_started_with_it_would() {
    let rig = a_rig();
    let (project, parent, _, _, _) = a_parent_with_a_subtask(&rig).await;
    let job = a_job(&rig, "Launch").await;
    rig.fleet.link_task_job(at(&rig), project.clone(), parent, job_link(&job)).await.expect("linked");

    rig.fleet.all_done(None, AllDone { session_id: Some(job.clone()) }).await.expect("done");

    let rows = read(&rig).await;
    assert_eq!(held(&rows_of(&rows, 0)[0]).0, ProjectTaskState::Asked);
    assert_eq!(held(&rows_of(&rows, 0)[1]).0, ProjectTaskState::Asked);
    assert_eq!(asks(&rig, &job).await.len(), 1);
    let ended = a_job(&rig, "Another").await;
    let (project, rows) = a_project(&rig, &["Free"]).await;
    rig.fleet.link_task_job(at(&rig), project, rows[0].clone(), job_link(&ended)).await.expect("linked");
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: ended }).await.expect("ended");
    assert_eq!(held(&rows_of(&read(&rig).await, 1)[0]).0, ProjectTaskState::Open, "ending the Job frees what it held");
}

async fn asks(rig: &Rig, job: &SessionId) -> Vec<WaitingItem> {
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session;
    record.waiting_for.into_iter().filter(|one| one.source == WaitingSource::TaskDone).collect()
}

#[tokio::test]
async fn a_job_nobody_knows_one_of_another_repository_or_one_that_ended_is_refused_and_so_is_an_issue_row() {
    let rig = a_rig();
    let (project, parent, _, _, issue) = a_parent_with_a_subtask(&rig).await;
    let job = a_job(&rig, "Launch").await;
    let refuse = |who: SessionId, row: String| {
        let (rig, project) = (&rig, project.clone());
        async move { code_of(rig.fleet.link_task_job(at(rig), project, row, job_link(&who)).await.expect_err("refused")) }
    };

    assert_eq!(refuse(SessionId::carried("nobody"), parent.clone()).await, (404, "fleet.no_such_session".to_string()));
    assert_eq!(refuse(job.clone(), issue).await, (422, "fleet.project_row_not_a_task".to_string()));
    {
        let mut store = rig.fleet.store().lock().await;
        store
            .keep_session(&KeptSession {
                id: "elsewhere".into(),
                harness: "a_harness".into(),
                origin: "bridge".into(),
                manifest_id: Some("some-other-repository".into()),
                cwd: "/repos/other".into(),
                title: None,
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
    }
    assert_eq!(refuse(SessionId::carried("elsewhere"), parent.clone()).await, (422, "fleet.project_job_other_repository".to_string()));
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: job.clone() }).await.expect("ended");
    assert_eq!(refuse(job, parent).await, (422, "fleet.project_job_ended".to_string()));
    assert_eq!(held(&rows_of(&read(&rig).await, 0)[0]), (ProjectTaskState::Open, None), "and nothing was held");
}

fn facts(standing: PullRequestStanding, title: &str) -> PullRequestFacts {
    PullRequestFacts {
        standing,
        branch: "armada/5".into(),
        auto_merge: false,
        title: title.into(),
        url: "https://forge.invalid/armada/pull/5".into(),
        body: None,
        closes: Vec::new(),
    }
}

fn pulls_of(row: &ProjectRow) -> Vec<TaskPull> {
    match row {
        ProjectRow::Task { pulls, .. } => pulls.clone(),
        ProjectRow::Issue { .. } => panic!("a task"),
    }
}

#[tokio::test]
async fn a_linked_pull_request_reads_its_title_and_state_from_the_forge_and_keeps_neither() {
    let rig = a_rig();
    let (project, rows) = a_project(&rig, &["Sign the build"]).await;
    let forge = rig.fleet.vcs();
    forge.now_pull_request(Some(facts(PullRequestStanding::Draft, "Sign it")));

    rig.fleet.link_task_pull(at(&rig), project.clone(), rows[0].clone(), LinkTaskPull { number: 5 }).await.expect("linked");
    let listed = rig.fleet.link_task_pull(at(&rig), project.clone(), rows[0].clone(), LinkTaskPull { number: 9 }).await.expect("linked");
    let pulls = pulls_of(&rows_of(&listed, 0)[0]);
    assert_eq!(
        pulls,
        [
            TaskPull { number: 5, title: Some("Sign it".into()), state: Some(TaskPullState::Open) },
            TaskPull { number: 9, title: Some("Sign it".into()), state: Some(TaskPullState::Open) }
        ],
        "in the order linked, a draft read as open"
    );

    forge.now_pull_request(None);
    let silent = rig.fleet.list_projects(at(&rig)).await.expect("listed");
    assert_eq!(
        pulls_of(&rows_of(&silent, 0)[0]).iter().map(|one| one.number).collect::<Vec<_>>(),
        [5, 9],
        "a forge that says nothing still lists the links, as held for a few minutes"
    );

    let refused = rig.fleet.link_task_pull(at(&rig), project.clone(), rows[1].clone(), LinkTaskPull { number: 5 }).await.expect_err("an issue row");
    assert_eq!(code_of(refused), (422, "fleet.project_row_not_a_task".to_string()));
    let zero = rig.fleet.link_task_pull(at(&rig), project.clone(), rows[0].clone(), LinkTaskPull { number: 0 }).await.expect_err("not a number");
    assert_eq!(code_of(zero), (422, "fleet.project_pull_unreadable".to_string()));

    let after = rig.fleet.unlink_task_pull(at(&rig), project, rows[0].clone(), LinkTaskPull { number: 5 }).await.expect("unlinked");
    assert_eq!(pulls_of(&rows_of(&after, 0)[0]).iter().map(|one| one.number).collect::<Vec<_>>(), [9]);
}

#[tokio::test]
async fn a_pull_request_the_forge_cannot_read_is_listed_without_a_title_or_state() {
    let rig = a_rig();
    let (project, rows) = a_project(&rig, &["Sign the build"]).await;
    rig.fleet.vcs().now_pull_request(None);

    let listed = rig.fleet.link_task_pull(at(&rig), project, rows[0].clone(), LinkTaskPull { number: 5 }).await.expect("linked");

    assert_eq!(pulls_of(&rows_of(&listed, 0)[0]), [TaskPull { number: 5, title: None, state: None }]);
}

async fn merged(rig: &Rig, number: u64, author: Option<&str>) {
    let forge = &rig.fleet.vcs().main_ci;
    forge.followed_are(Some(Vec::new()));
    forge.recently_merged_are(Some(vec![RecentlyMerged {
        number,
        title: FromOutside::verbatim("Sign it"),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: author.map(FromOutside::verbatim),
        merged_at: FromOutside::verbatim(rig.fleet.now().as_str()),
        commit: None,
    }]));
    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.notice_merged(&served).await;
    rig.reads().await;
}

#[tokio::test]
async fn a_linked_pull_request_merging_asks_about_an_open_task_on_the_rows_and_asks_only_once() {
    let rig = a_rig();
    let (project, rows) = a_project(&rig, &["Sign the build", "Ship it"]).await;
    let (sign, ship) = (rows[0].clone(), rows[1].clone());
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open, "Sign it")));
    rig.fleet.link_task_pull(at(&rig), project.clone(), sign.clone(), LinkTaskPull { number: 5 }).await.expect("linked");
    rig.fleet.tick_project_task(at(&rig), project.clone(), ship.clone(), TickProjectTask { done: true }).await.expect("ticked");
    rig.fleet.link_task_pull(at(&rig), project.clone(), ship.clone(), LinkTaskPull { number: 5 }).await.expect("linked");

    merged(&rig, 5, Some("a-teammate")).await;

    let listed = read(&rig).await;
    assert_eq!(held(&rows_of(&listed, 0)[0]), (ProjectTaskState::Asked, None), "asked, with no Job to name");
    assert_eq!(held(&rows_of(&listed, 0)[1]), (ProjectTaskState::Done, None), "a task already done is left alone");
    assert_eq!(
        pulls_of(&rows_of(&listed, 0)[0]),
        [TaskPull { number: 5, title: Some("Sign it".into()), state: Some(TaskPullState::Merged) }],
        "and the pull request reads merged at once, whoever opened it"
    );

    let kept_open = rig.fleet.tick_project_task(at(&rig), project, sign, TickProjectTask { done: false }).await.expect("answered");
    assert_eq!(held(&rows_of(&kept_open, 0)[0]).0, ProjectTaskState::Open);
    merged(&rig, 5, Some("a-teammate")).await;
    assert_eq!(held(&rows_of(&read(&rig).await, 0)[0]).0, ProjectTaskState::Open, "the same merge does not ask again");
}

#[tokio::test]
async fn a_linked_pull_request_merging_asks_on_the_list_of_the_job_holding_the_task() {
    let rig = a_rig();
    let (project, rows) = a_project(&rig, &["Sign the build"]).await;
    let job = a_job(&rig, "Launch").await;
    rig.fleet.link_task_job(at(&rig), project.clone(), rows[0].clone(), job_link(&job)).await.expect("linked");
    rig.fleet.link_task_pull(at(&rig), project, rows[0].clone(), LinkTaskPull { number: 5 }).await.expect("linked");
    assert!(asks(&rig, &job).await.is_empty());

    merged(&rig, 5, None).await;

    let listed = read(&rig).await;
    assert_eq!(held(&rows_of(&listed, 0)[0]), (ProjectTaskState::Asked, holder(&job, "Launch")), "it keeps the Job it had");
    assert_eq!(asks(&rig, &job).await.len(), 1, "and the Job's list carries the ask");
}

#[tokio::test]
async fn removing_a_task_takes_its_pull_request_links_with_it() {
    let rig = a_rig();
    let (project, rows) = a_project(&rig, &["Sign the build"]).await;
    rig.fleet.link_task_pull(at(&rig), project.clone(), rows[0].clone(), LinkTaskPull { number: 5 }).await.expect("linked");

    rig.fleet.remove_project_row(at(&rig), project, rows[0].clone()).await.expect("removed");
    merged(&rig, 5, None).await;

    assert!(matches!(rows_of(&read(&rig).await, 0), [ProjectRow::Issue { .. }]), "nothing was asked, nothing is left");
}

#[tokio::test]
async fn linking_a_pull_request_that_already_merged_asks_about_an_open_or_held_task_at_once_and_once() {
    let rig = a_rig();
    let (project, rows) = a_project(&rig, &["Sign the build", "Ship it"]).await;
    let job = a_job(&rig, "Launch").await;
    rig.fleet.link_task_job(at(&rig), project.clone(), rows[1].clone(), job_link(&job)).await.expect("linked");
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open, "Sign it")));
    let still_open = rig.fleet.link_task_pull(at(&rig), project.clone(), rows[0].clone(), LinkTaskPull { number: 4 }).await.expect("linked");
    assert_eq!(held(&rows_of(&still_open, 0)[0]).0, ProjectTaskState::Open, "an open pull request asks nothing");

    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Merged, "Sign it")));
    let linked = rig.fleet.link_task_pull(at(&rig), project.clone(), rows[0].clone(), LinkTaskPull { number: 5 }).await.expect("linked");
    assert_eq!(held(&rows_of(&linked, 0)[0]), (ProjectTaskState::Asked, None), "asked in the answer itself, with no Job to name");
    assert_eq!(
        pulls_of(&rows_of(&linked, 0)[0])[1],
        TaskPull { number: 5, title: Some("Sign it".into()), state: Some(TaskPullState::Merged) }
    );

    let held_task = rig.fleet.link_task_pull(at(&rig), project.clone(), rows[1].clone(), LinkTaskPull { number: 5 }).await.expect("linked");
    assert_eq!(held(&rows_of(&held_task, 0)[1]), (ProjectTaskState::Asked, holder(&job, "Launch")), "a held task keeps its Job");
    assert_eq!(asks(&rig, &job).await.len(), 1, "and the Job's list carries the ask");

    rig.fleet.tick_project_task(at(&rig), project.clone(), rows[0].clone(), TickProjectTask { done: false }).await.expect("kept open");
    let again = rig.fleet.link_task_pull(at(&rig), project, rows[0].clone(), LinkTaskPull { number: 5 }).await.expect("linked again");
    assert_eq!(held(&rows_of(&again, 0)[0]).0, ProjectTaskState::Open, "linking it again changes nothing, so it does not ask twice");
    merged(&rig, 5, None).await;
    assert_eq!(held(&rows_of(&read(&rig).await, 0)[0]).0, ProjectTaskState::Open, "nor does the sweep");
}
