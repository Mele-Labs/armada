//! The project routes over the wire: each takes its path and body to the call it names, answers the
//! repository's projects whole, and carries a refusal through as its status.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{IssueState, ProjectRow, Projects};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    router(Served::sharing(daemon, run_id(), events))
}

async fn call(method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let response = wired()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    let status = response.status();
    (status, response.into_body().collect().await.expect("a body").to_bytes().to_vec())
}

/// What the fake named its one project for the call that reached it, and that project's id.
async fn reached(method: &str, uri: &str, body: &str) -> (String, String) {
    let (status, answer) = call(method, uri, body).await;
    assert_eq!(status, StatusCode::OK, "{uri}: {}", String::from_utf8_lossy(&answer));
    let projects: Projects = ipc::decode("projects", &answer).expect("the projects whole");
    assert_eq!(projects.manifest_id.as_str(), "armada");
    let project = projects.projects.into_iter().next().expect("one project");
    (project.name, project.id)
}

async fn refused(method: &str, uri: &str, body: &str) -> (StatusCode, String) {
    let (status, answer) = call(method, uri, body).await;
    (status, String::from_utf8_lossy(&answer).into_owned())
}

#[tokio::test]
async fn list_projects_answers_each_row_by_its_kind() {
    let (status, answer) = call("GET", "/projects/armada", "").await;
    assert_eq!(status, StatusCode::OK);
    let projects: Projects = ipc::decode("projects", &answer).expect("projects");
    let rows = &projects.projects[0].rows;
    assert!(matches!(&rows[0], ProjectRow::Task { title, .. } if title == "Write the page"));
    assert!(matches!(&rows[1], ProjectRow::Issue { issue, state: Some(IssueState::Open), .. } if issue == "214"));
    let text = String::from_utf8_lossy(&answer);
    assert!(text.contains(r#""kind":"task""#) && text.contains(r#""kind":"issue""#), "{text}");
}

#[tokio::test]
async fn add_project_takes_a_name_and_a_blank_one_is_a_422() {
    assert_eq!(reached("POST", "/projects/armada", r#"{"name":"Launch"}"#).await.0, "add_project Launch");
    let (status, text) = refused("POST", "/projects/armada", r#"{"name":"  "}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_blank"), "{text}");
}

#[tokio::test]
async fn rename_project_takes_the_project_and_a_name_and_a_missing_one_is_a_404() {
    let (said, id) = reached("POST", "/projects/armada/p7/rename", r#"{"name":"Later"}"#).await;
    assert_eq!((said.as_str(), id.as_str()), ("rename_project Later", "p7"));
    let (status, text) = refused("POST", "/projects/armada/nope/rename", r#"{"name":"Later"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(text.contains("fleet.no_such_project"), "{text}");
}

#[tokio::test]
async fn delete_project_takes_the_project() {
    let (said, id) = reached("POST", "/projects/armada/p7/delete", "").await;
    assert_eq!((said.as_str(), id.as_str()), ("delete_project", "p7"));
}

#[tokio::test]
async fn add_project_task_takes_a_title_and_notes_which_may_be_left_out() {
    let body = r#"{"title":"Write the page","notes":"two lines"}"#;
    assert_eq!(reached("POST", "/projects/armada/p7/tasks", body).await.0, "add_project_task Write the page / two lines");
    let bare = r#"{"title":"Ship it"}"#;
    assert_eq!(reached("POST", "/projects/armada/p7/tasks", bare).await.0, "add_project_task Ship it / ");
}

#[tokio::test]
async fn edit_project_task_takes_the_row_and_the_task_whole() {
    let body = r#"{"title":"Rewritten","notes":"n"}"#;
    assert_eq!(reached("POST", "/projects/armada/p7/rows/r1/edit", body).await.0, "edit_project_task r1 Rewritten / n");
}

#[tokio::test]
async fn link_project_issue_takes_the_reference() {
    assert_eq!(reached("POST", "/projects/armada/p7/issues", r#"{"issue":"ABC-12"}"#).await.0, "link_project_issue ABC-12");
}

#[tokio::test]
async fn remove_project_row_takes_the_row() {
    assert_eq!(reached("POST", "/projects/armada/p7/rows/r2/remove", "").await.0, "remove_project_row r2");
}

#[tokio::test]
async fn move_project_row_takes_the_row_and_its_position_and_a_bad_body_is_a_400() {
    assert_eq!(reached("POST", "/projects/armada/p7/rows/r1/move", r#"{"position":3}"#).await.0, "move_project_row r1 3 -");
    assert_eq!(
        reached("POST", "/projects/armada/p7/rows/r1/move", r#"{"position":0,"parent":"r9"}"#).await.0,
        "move_project_row r1 0 r9",
        "a parent to go under"
    );
    assert_eq!(
        reached("POST", "/projects/armada/p7/rows/r1/move", r#"{"position":0,"parent":""}"#).await.0,
        "move_project_row r1 0 ",
        "and an empty one is the top level"
    );
    let (status, _) = refused("POST", "/projects/armada/p7/rows/r1/move", r#"{"position":-1}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn tick_project_task_takes_done_and_an_issue_row_is_a_422() {
    assert_eq!(reached("POST", "/projects/armada/p7/rows/r1/tick", r#"{"done":true}"#).await.0, "tick_project_task r1 true");
    let (status, text) = refused("POST", "/projects/armada/p7/rows/r2/tick", r#"{"done":true}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_row_not_a_task"), "{text}");
}

#[tokio::test]
async fn add_project_subtask_takes_the_parent_and_the_task_and_a_subtask_of_a_subtask_is_a_422() {
    let body = r#"{"title":"Sub","notes":"n"}"#;
    assert_eq!(reached("POST", "/projects/armada/p7/rows/r1/subtasks", body).await.0, "add_project_subtask r1 Sub / n");
    let (status, text) = refused("POST", "/projects/armada/p7/rows/r3/subtasks", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_subtask_too_deep"), "{text}");
}

#[tokio::test]
async fn link_and_unlink_task_issue_take_the_task_and_the_reference() {
    assert_eq!(
        reached("POST", "/projects/armada/p7/rows/r1/issues", r#"{"issue":"ABC-12"}"#).await.0,
        "link_task_issue r1 ABC-12"
    );
    assert_eq!(
        reached("POST", "/projects/armada/p7/rows/r1/issues/unlink", r#"{"issue":"ABC-12"}"#).await.0,
        "unlink_task_issue r1 ABC-12"
    );
}
