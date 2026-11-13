//! A draft is read, kept and thrown away at one path by three methods, and published by an act under it: each method
//! reaches its own handler with the key its query or body carries, for a workflow and for a piece.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{
    LifecyclePieceKind, PieceDraft, PieceDraftDiscarded, PiecePublished, PieceScope, WorkflowDraft, WorkflowDraftDiscarded, WorkflowPublished,
    WorkflowScope,
};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn a_workflow_draft_is_put_read_discarded_and_published_by_the_method_and_path_that_name_each() {
    let app = wired();
    let (status, bytes) = call(&app, "PUT", "/workflows/draft", r#"{"workflow_id":"feature","source":"project","project":"storefront","text":"{half"}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let put: WorkflowDraft = ipc::decode("a draft", &bytes).expect("reads");
    assert_eq!((put.workflow_id.as_str(), put.source.as_str(), put.project.as_deref(), put.text.as_str()), ("feature", "project", Some("storefront"), "{half"));

    let (status, bytes) = call(&app, "GET", "/workflows/draft?workflow_id=feature&source=kit", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let read: WorkflowDraft = ipc::decode("a draft", &bytes).expect("reads");
    assert_eq!((read.source.as_str(), read.project), ("kit", None));

    let (status, bytes) = call(&app, "DELETE", "/workflows/draft?workflow_id=feature&source=project&project=storefront", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let discarded: WorkflowDraftDiscarded = ipc::decode("a discard", &bytes).expect("reads");
    assert_eq!((discarded.source.as_str(), discarded.project.as_deref()), ("project", Some("storefront")));

    let (status, bytes) = call(&app, "POST", "/workflows/draft/publish", r#"{"workflow_id":"feature","source":"armada","scope":"kit"}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let published: WorkflowPublished = ipc::decode("a publish", &bytes).expect("reads");
    assert_eq!(published.saved.scope, WorkflowScope::Kit);

    let (status, _) = call(&app, "POST", "/workflows/draft", "{}").await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "a draft is not posted to");
}

#[tokio::test]
async fn a_piece_draft_is_keyed_by_kind_id_and_the_scope_it_is_within_and_is_everywhere_without_one() {
    let app = wired();
    let (status, bytes) = call(&app, "PUT", "/lifecycles/pieces/draft", r#"{"kind":"rule","id":"ticket_to_review","text":"{}"}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let put: PieceDraft = ipc::decode("a draft", &bytes).expect("reads");
    assert_eq!((put.kind, put.scope), (LifecyclePieceKind::Rule, PieceScope::Everywhere));

    let (status, bytes) = call(&app, "GET", "/lifecycles/pieces/draft?kind=lifecycle&id=ticket&within=my-store", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let read: PieceDraft = ipc::decode("a draft", &bytes).expect("reads");
    assert_eq!((read.kind, read.scope), (LifecyclePieceKind::Lifecycle, PieceScope::Within { id: String::from("my-store") }));

    let (status, bytes) = call(&app, "DELETE", "/lifecycles/pieces/draft?kind=autonomy&id=mine", "").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let discarded: PieceDraftDiscarded = ipc::decode("a discard", &bytes).expect("reads");
    assert_eq!((discarded.kind, discarded.scope), (LifecyclePieceKind::Autonomy, PieceScope::Everywhere));

    let (status, bytes) = call(&app, "POST", "/lifecycles/pieces/draft/publish", r#"{"kind":"rule","id":"ticket_to_review"}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let published: PiecePublished = ipc::decode("a publish", &bytes).expect("reads");
    assert_eq!(published.saved.file, "/kit/ticket_to_review.yml");

    let (status, _) = call(&app, "GET", "/lifecycles/pieces/draft?kind=template&id=x", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a kind that is none of the three is the caller's to correct");
}
