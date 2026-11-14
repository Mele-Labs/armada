//! The project routes over the wire: each takes its path and body to the call it names, answers
//! his projects and the loose tasks whole, and carries a refusal through as its status.

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
    let project = projects.projects.into_iter().next().expect("one project");
    (project.name, project.id)
}

async fn refused(method: &str, uri: &str, body: &str) -> (StatusCode, String) {
    let (status, answer) = call(method, uri, body).await;
    (status, String::from_utf8_lossy(&answer).into_owned())
}

#[tokio::test]
async fn list_projects_answers_each_row_by_its_kind() {
    let (status, answer) = call("GET", "/projects", "").await;
    assert_eq!(status, StatusCode::OK);
    let projects: Projects = ipc::decode("projects", &answer).expect("projects");
    let rows = &projects.projects[0].rows;
    assert!(matches!(&rows[0], ProjectRow::Task { title, .. } if title == "Write the page"));
    assert!(matches!(&rows[1], ProjectRow::Issue { issue, manifest_id, state: Some(IssueState::Open), .. } if issue == "214" && manifest_id.as_str() == "site"));
    assert!(matches!(&rows[0], ProjectRow::Task { manifest_id, .. } if manifest_id.as_str() == "armada"), "a row names its repository");
    assert!(matches!(&projects.loose[0], ProjectRow::Task { title, .. } if title == "Not in a project"), "and the loose tasks come beside the projects");
    let text = String::from_utf8_lossy(&answer);
    assert!(text.contains(r#""kind":"task""#) && text.contains(r#""kind":"issue""#), "{text}");
}

#[tokio::test]
async fn add_project_takes_a_name_and_a_blank_one_is_a_422() {
    assert_eq!(reached("POST", "/projects", r#"{"name":"Launch"}"#).await.0, "add_project Launch 0 ThisMac");
    let with = r#"{"name":"Launch","members":[{"manifest_id":"api"},{"manifest_id":"web","folder":"apps/web"}],"place":{"place":"synced","synced_folder":"/tmp/x"}}"#;
    assert_eq!(reached("POST", "/projects", with).await.0, "add_project Launch 2 Synced", "members and a place come with the name");
    let (status, text) = refused("POST", "/projects", r#"{"name":"Launch","place":{"place":"repository"}}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_place_refused"), "{text}");
    let (status, text) = refused("POST", "/projects", r#"{"name":"  "}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_blank"), "{text}");
}

#[tokio::test]
async fn set_project_workflow_takes_the_project_the_workflow_and_whether_it_is_on() {
    let (said, id) = reached("POST", "/projects/project/p7/workflow", r#"{"workflow":"bug","on":false}"#).await;
    assert_eq!((said.as_str(), id.as_str()), ("set_project_workflow bug off", "p7"));
    let (status, text) = refused("POST", "/projects/project/nope/workflow", r#"{"workflow":"bug","on":true}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(text.contains("fleet.no_such_project"), "{text}");
    let (status, _) = refused("POST", "/projects/project/p7/workflow", r#"{"workflow":"bug"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "on is required");
}

#[tokio::test]
async fn edit_project_takes_the_project_and_what_changes_and_a_missing_one_is_a_404() {
    let (said, id) = reached("POST", "/projects/project/p7/edit", r#"{"name":"Later"}"#).await;
    assert_eq!((said.as_str(), id.as_str()), ("edit_project Later - -", "p7"));
    let (said, _) = reached("POST", "/projects/project/p7/edit", r#"{"members":[],"place":{"place":"this_mac"}}"#).await;
    assert_eq!(said, "edit_project - 0 ThisMac", "an empty list clears the members, and absent ones are left");
    let (status, text) = refused("POST", "/projects/project/nope/edit", r#"{"name":"Later"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(text.contains("fleet.no_such_project"), "{text}");
    let (status, text) = refused("POST", "/projects/project/p7/edit", r#"{"members":[{"manifest_id":"api","folder":""}]}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_malformed"), "{text}");
    let (status, _) = refused("POST", "/projects/project/p7/rename", r#"{"name":"Later"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "the old route is gone");
}

#[tokio::test]
async fn the_projects_holding_a_repository_are_read_by_manifest_id() {
    let (status, answer) = call("GET", "/projects/holding?manifest_id=armada", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&answer));
    let projects: Projects = ipc::decode("projects", &answer).expect("projects");
    assert_eq!(projects.projects[0].name, "list_holding_projects armada");
    let (status, _) = call("GET", "/workspaces/all", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "the Workspace reads are gone");
}

#[tokio::test]
async fn delete_project_takes_the_project() {
    let (said, id) = reached("POST", "/projects/project/p7/delete", "").await;
    assert_eq!((said.as_str(), id.as_str()), ("delete_project", "p7"));
}

#[tokio::test]
async fn add_project_task_takes_a_title_and_notes_which_may_be_left_out() {
    let body = r#"{"manifest_id":"armada","project":"p7","title":"Write the page","notes":"two lines"}"#;
    assert_eq!(reached("POST", "/projects/tasks", body).await.0, "add_project_task armada p7 Write the page / two lines");
    let bare = r#"{"manifest_id":"site","title":"Ship it"}"#;
    assert_eq!(reached("POST", "/projects/tasks", bare).await.0, "add_project_task site - Ship it / ", "no project is a loose task");
    let (status, _) = refused("POST", "/projects/tasks", r#"{"title":"No repository"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a task is in a repository");
    let (status, text) = refused("POST", "/projects/tasks", r#"{"manifest_id":"armada","project":"nope","title":"x"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(text.contains("fleet.no_such_project"), "{text}");
}

#[tokio::test]
async fn edit_project_task_takes_the_row_and_the_task_whole() {
    let body = r#"{"title":"Rewritten","notes":"n"}"#;
    assert_eq!(reached("POST", "/projects/rows/r1/edit", body).await.0, "edit_project_task r1 Rewritten / n");
}

#[tokio::test]
async fn link_project_issue_takes_the_reference() {
    let body = r#"{"manifest_id":"site","project":"p7","issue":"ABC-12"}"#;
    assert_eq!(reached("POST", "/projects/issues", body).await.0, "link_project_issue site p7 ABC-12");
    assert_eq!(reached("POST", "/projects/issues", r#"{"manifest_id":"site","issue":"7"}"#).await.0, "link_project_issue site - 7");
}

#[tokio::test]
async fn remove_project_row_takes_the_row() {
    assert_eq!(reached("POST", "/projects/rows/r2/remove", "").await.0, "remove_project_row r2");
}

#[tokio::test]
async fn move_project_row_takes_the_row_and_its_position_and_a_bad_body_is_a_400() {
    assert_eq!(reached("POST", "/projects/rows/r1/move", r#"{"position":3}"#).await.0, "move_project_row r1 - 3 -");
    assert_eq!(
        reached("POST", "/projects/rows/r1/move", r#"{"position":0,"parent":"r9"}"#).await.0,
        "move_project_row r1 - 0 r9",
        "a parent to go under"
    );
    assert_eq!(
        reached("POST", "/projects/rows/r1/move", r#"{"position":0,"parent":""}"#).await.0,
        "move_project_row r1 - 0 ",
        "and an empty one is the top level"
    );
    assert_eq!(
        reached("POST", "/projects/rows/r1/move", r#"{"project":"p2","position":0}"#).await.0,
        "move_project_row r1 p2 0 -",
        "into another project"
    );
    assert_eq!(
        reached("POST", "/projects/rows/r1/move", r#"{"project":"","position":2}"#).await.0,
        "move_project_row r1  2 -",
        "and an empty one is out of every project"
    );
    let (status, _) = refused("POST", "/projects/rows/r1/move", r#"{"position":-1}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn move_project_task_repository_takes_the_row_and_the_repository_and_each_refusal_is_its_status() {
    assert_eq!(
        reached("POST", "/projects/rows/r1/repository", r#"{"manifest_id":"site"}"#).await.0,
        "move_project_task_repository r1 site"
    );
    let body = r#"{"manifest_id":"site"}"#;
    let (status, text) = refused("POST", "/projects/rows/r2/repository", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_row_not_a_task"), "{text}");
    let (status, text) = refused("POST", "/projects/rows/r3/repository", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_subtask_too_deep"), "{text}");
    let (status, text) = refused("POST", "/projects/rows/nope/repository", body).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(text.contains("fleet.no_such_project_row"), "{text}");
    let (status, _) = refused("POST", "/projects/rows/r1/repository", "{}").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a repository is named");
}

#[tokio::test]
async fn tick_project_task_takes_done_and_an_issue_row_is_a_422() {
    assert_eq!(reached("POST", "/projects/rows/r1/tick", r#"{"done":true}"#).await.0, "tick_project_task r1 true");
    let (status, text) = refused("POST", "/projects/rows/r2/tick", r#"{"done":true}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_row_not_a_task"), "{text}");
}

#[tokio::test]
async fn add_project_subtask_takes_the_parent_and_the_task_and_a_subtask_of_a_subtask_is_a_422() {
    let body = r#"{"title":"Sub","notes":"n"}"#;
    assert_eq!(reached("POST", "/projects/rows/r1/subtasks", body).await.0, "add_project_subtask r1 Sub / n");
    let (status, text) = refused("POST", "/projects/rows/r3/subtasks", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_subtask_too_deep"), "{text}");
}

#[tokio::test]
async fn link_and_unlink_task_issue_take_the_task_and_the_reference() {
    assert_eq!(
        reached("POST", "/projects/rows/r1/issues", r#"{"issue":"ABC-12"}"#).await.0,
        "link_task_issue r1 ABC-12"
    );
    assert_eq!(
        reached("POST", "/projects/rows/r1/issues/unlink", r#"{"issue":"ABC-12"}"#).await.0,
        "unlink_task_issue r1 ABC-12"
    );
}

#[tokio::test]
async fn link_and_unlink_task_job_and_pull_take_the_task_and_what_is_linked() {
    let base = "/projects/rows/r1";
    assert_eq!(reached("POST", &format!("{base}/job"), r#"{"session_id":"s9"}"#).await.0, "link_task_job r1 s9");
    assert_eq!(reached("POST", &format!("{base}/job/unlink"), "").await.0, "unlink_task_job r1");
    assert_eq!(reached("POST", &format!("{base}/pulls"), r#"{"number":42}"#).await.0, "link_task_pull r1 42");
    assert_eq!(reached("POST", &format!("{base}/pulls/unlink"), r#"{"number":42}"#).await.0, "unlink_task_pull r1 42");
    let (status, _) = refused("POST", &format!("{base}/pulls"), r#"{"number":-1}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn values_are_read_for_a_project_a_repository_or_both_and_a_missing_project_is_a_404() {
    let reached = |answer: &[u8]| ipc::decode::<ipc::ResolvedValues>("values", answer).expect("values").review_model.value.expect("a word");
    let (status, answer) = call("GET", "/projects/values?project=p7&manifest_id=armada", "").await;
    assert_eq!((status, reached(&answer).as_str()), (StatusCode::OK, "p7 armada"));
    let (_, answer) = call("GET", "/projects/values?project=p7", "").await;
    assert_eq!(reached(&answer), "p7 -", "the repository is optional");
    let (_, answer) = call("GET", "/projects/values", "").await;
    assert_eq!(reached(&answer), "- -", "so is the project");
    let (status, text) = refused("GET", "/projects/values?project=nope", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(text.contains("fleet.no_such_project"), "{text}");
}

#[tokio::test]
async fn a_projects_issues_are_listed_by_its_route_and_a_missing_project_is_a_404() {
    let (status, answer) = call("GET", "/projects/project/p7/issues?refresh=true", "").await;
    let listed = ipc::decode::<ipc::ProjectIssueList>("issues", &answer).expect("issues");
    assert_eq!((status, listed.project.as_str()), (StatusCode::OK, "p7"));
    let (status, text) = refused("GET", "/projects/project/nope/issues", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(text.contains("fleet.no_such_project"), "{text}");
}

#[tokio::test]
async fn kit_values_are_saved_whole_and_a_word_the_key_does_not_take_is_a_422() {
    let (status, answer) = call("POST", "/kit/values", r#"{"values":{"auto_merge":"always"}}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&answer));
    let saved: ipc::KitValuesSaved = ipc::decode("saved", &answer).expect("saved");
    assert_eq!(saved.values.auto_merge.as_deref(), Some("always"));
    let (status, text) = refused("POST", "/kit/values", r#"{"values":{"auto_merge":"sometimes"}}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(text.contains("fleet.project_values_refused"), "{text}");
}

#[tokio::test]
async fn a_projects_store_is_read_joined_and_deployed_on_its_own_routes() {
    let (status, answer) = call("GET", "/projects/project/p7/store", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&answer));
    let state: ipc::ProjectStoreState = ipc::decode("project store", &answer).expect("a store state");
    assert_eq!((state.project.as_str(), state.store, state.deploy), ("p7", None, ipc::StoreDeploy::TryTunnel));

    let (status, answer) = call("POST", "/projects/project/p7/store", r#"{"store":{"address":"https://store.test","team":"mele-labs"}}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&answer));
    let state: ipc::ProjectStoreState = ipc::decode("project store", &answer).expect("a store state");
    let store = state.store.expect("joined");
    assert_eq!((store.address.as_str(), store.team.as_deref()), ("https://store.test", Some("mele-labs")));
    let (status, answer) = call("POST", "/projects/project/p7/store", "{}").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ipc::decode::<ipc::ProjectStoreState>("project store", &answer).expect("a store state").store, None, "an empty body is none");

    let (status, answer) = call("POST", "/projects/project/p7/store/deploy", "{}").await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8_lossy(&answer);
    assert!(text.contains(r#""outcome":"needs_token""#) && text.contains("dash.example/tokens"), "{text}");
    let (_, answer) = call("POST", "/projects/project/p7/store/deploy", r#"{"cloudflare_token":"t0k3n"}"#).await;
    let deployed: ipc::StoreDeployed = ipc::decode("store deployed", &answer).expect("deployed");
    assert!(matches!(&deployed, ipc::StoreDeployed::Deployed { state, .. } if state.store.as_ref().and_then(|store| store.worker.as_deref()) == Some("armada-store")));
    assert!(!String::from_utf8_lossy(&answer).contains("t0k3n"), "the token is never answered back");

    for (method, path) in [("GET", "/projects/project/nope/store"), ("POST", "/projects/project/nope/store"), ("POST", "/projects/project/nope/store/deploy")] {
        let (status, text) = refused(method, path, "{}").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {path}");
        assert!(text.contains("fleet.no_such_project"), "{text}");
    }
}
