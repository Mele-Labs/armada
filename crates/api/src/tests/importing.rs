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

#[tokio::test]
async fn the_runs_a_repository_remembers_are_read_and_their_place_chosen_and_one_dropped() {
    let app = wired();

    let (status, body) = call(&app, "GET", "/repositories/runs?repository=/work/shop", "").await;
    assert_eq!(status, StatusCode::OK);
    let memory: ipc::RunMemory = ipc::decode("runs", &body).expect("decodes");
    assert_eq!((memory.repository.as_str(), memory.place), ("/work/shop", ipc::RunPlace::ThisMac));
    let (status, _) = call(&app, "GET", "/repositories/runs", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a repository is asked for");

    let (status, body) = call(&app, "GET", "/repositories/runs/place?repository=/work/shop", "").await;
    assert_eq!(status, StatusCode::OK);
    let place: ipc::RunMemoryPlace = ipc::decode("place", &body).expect("decodes");
    assert_eq!(place.place, ipc::RunPlace::ThisMac);

    let chosen = r#"{"repository":"/work/shop","place":"synced","synced_folder":"/Users/user/Sync"}"#;
    let (status, body) = call(&app, "POST", "/repositories/runs/place", chosen).await;
    assert_eq!(status, StatusCode::OK);
    let place: ipc::RunMemoryPlace = ipc::decode("place", &body).expect("decodes");
    assert_eq!((place.place, place.synced_folder.as_deref()), (ipc::RunPlace::Synced, Some("/Users/user/Sync")));
    let (status, _) = call(&app, "POST", "/repositories/runs/place", r#"{"repository":"/work/shop","place":"nowhere"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a place Fleet does not know");

    let drop = r#"{"repository":"/work/shop","workspace":".","kind":"dev_server","command":"pnpm dev"}"#;
    let (status, _) = call(&app, "POST", "/repositories/runs/drop", drop).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&app, "POST", "/repositories/runs/drop", "{}").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
