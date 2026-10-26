//! The saved-query routes over the wire: each takes its body, answers its shape, and carries a
//! tracker's refusal through as the 422 it is.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{IssueList, IssueQueries, SavedIssueQuery, WrittenIssueQuery};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    router(Served::sharing(daemon, run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let response = app
        .clone()
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

#[tokio::test]
async fn the_five_routes_take_their_bodies_and_answer_their_shapes() {
    let app = wired();

    let (status, body) = call(&app, "POST", "/issue_queries/armada/search", r#"{"query":"state = open"}"#).await;
    assert_eq!(status, StatusCode::OK);
    ipc::decode::<IssueList>("a list", &body).expect("an issue list");

    let (status, body) = call(&app, "GET", "/issue_queries/armada", "").await;
    assert_eq!(status, StatusCode::OK);
    let queries: IssueQueries = ipc::decode("queries", &body).expect("queries");
    assert_eq!((queries.saved.len(), queries.guide.map(|one| one.language)), (1, Some("Fake search".to_string())));

    let (status, body) = call(&app, "POST", "/issue_queries/armada/write", r#"{"words":"mine"}"#).await;
    assert_eq!(status, StatusCode::OK);
    let written: WrittenIssueQuery = ipc::decode("a query", &body).expect("a written query");
    assert_eq!((written.row_fields, written.cost_micros), (vec!["Labels".to_string()], Some(1200)));

    let save = r#"{"name":"Mine","words":"mine","query":"assignee = me","row_fields":["Labels"]}"#;
    let (status, body) = call(&app, "POST", "/issue_queries/armada", save).await;
    assert_eq!(status, StatusCode::OK);
    let saved: SavedIssueQuery = ipc::decode("a saved query", &body).expect("a saved query");
    assert_eq!((saved.id.as_str(), saved.name.as_str()), ("q2", "Mine"));

    let (status, body) = call(&app, "POST", "/issue_queries/armada/q1/forget", "").await;
    assert_eq!(status, StatusCode::OK);
    let left: IssueQueries = ipc::decode("queries", &body).expect("queries");
    assert!(left.saved.is_empty());
}

#[tokio::test]
async fn a_tracker_refusal_is_a_422_with_its_words_a_missing_query_a_404_and_a_bad_body_a_400() {
    let app = wired();

    let (status, body) = call(&app, "POST", "/issue_queries/armada/search", r#"{"query":"refuse"}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("fleet.issue_query_refused") && text.contains("unknown field"), "{text}");

    let save = r#"{"name":"Bad","words":"","query":"refuse","row_fields":[]}"#;
    let (status, _) = call(&app, "POST", "/issue_queries/armada", save).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, body) = call(&app, "POST", "/issue_queries/armada/nope/forget", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("fleet.no_such_issue_query"));

    let (status, _) = call(&app, "POST", "/issue_queries/armada/write", "not json").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn installing_a_tool_answers_whether_it_worked_and_refuses_a_tool_it_does_not_know() {
    let app = wired();

    let (status, body) = call(&app, "POST", "/issue_queries/armada/install_tool", r#"{"tool":"bd"}"#).await;
    assert_eq!(status, StatusCode::OK);
    let installed: ipc::InstalledTool = ipc::decode("an install", &body).expect("an install");
    assert_eq!((installed.tool.as_str(), installed.done), ("bd", true));

    let (status, body) = call(&app, "POST", "/issue_queries/armada/install_tool", r#"{"tool":"rm"}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(String::from_utf8_lossy(&body).contains("fleet.tool_not_installable"));

    let (status, _) = call(&app, "POST", "/issue_queries/armada/install_tool", "not json").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
