//! `list_import_folders`, `get_import_runs` and `install_agent_mod` over the router: the routes
//! exist, the folder travels in the query, and an install body that does not say `requested` is a
//! 400 rather than a request.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{ImportFolders, ImportRuns, ModInstalled};
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
async fn the_folders_and_one_folders_runs_are_read() {
    let app = wired();

    let (status, body) = call(&app, "GET", "/import/folders", "").await;
    assert_eq!(status, StatusCode::OK);
    let folders: ImportFolders = ipc::decode("folders", &body).expect("decodes");
    assert!(folders.folders.is_empty());

    let (status, body) = call(&app, "GET", "/import/runs?path=/work/shop", "").await;
    assert_eq!(status, StatusCode::OK);
    let runs: ImportRuns = ipc::decode("runs", &body).expect("decodes");
    assert_eq!(runs.path, "/work/shop");

    let (status, _) = call(&app, "GET", "/import/runs", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a folder is asked for");
}

#[tokio::test]
async fn an_install_is_a_request_only_when_the_body_says_so() {
    let app = wired();

    let (status, body) = call(&app, "POST", "/import/install_mod", r#"{"requested":true}"#).await;
    assert_eq!(status, StatusCode::OK);
    let installed: ModInstalled = ipc::decode("installed", &body).expect("decodes");
    assert!(matches!(installed, ModInstalled::Installed { .. }));

    for empty in ["", "{}", r#"{"requested":null}"#] {
        let (status, _) = call(&app, "POST", "/import/install_mod", empty).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "`{empty}` is not a request");
    }
}
