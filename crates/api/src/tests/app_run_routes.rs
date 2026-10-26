//! App runs over the router: each route reaches the daemon with the repository and the Job it
//! named, a body of the wrong shape is the caller's fault, and a name that is no run is a 422.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{AppRun, AppRunCheckout, AppRunDraft, AppRunList, SavedAppRun};
use tower::ServiceExt;

use crate::tests::fake::{FakeDaemon, THE_RUN};
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
async fn a_list_names_the_repository_or_the_job_on_the_query_string() {
    let app = wired();
    let (status, bytes) = call(&app, "GET", "/app_runs?manifest_id=01OTHER", "").await;
    assert_eq!(status, StatusCode::OK);
    let list: AppRunList = ipc::decode("runs", &bytes).expect("a list");
    assert_eq!(list.runs[0].manifest_id.as_str(), "01OTHER");
    assert!(matches!(list.runs[0].checkout, AppRunCheckout::Main { .. }));

    let (status, bytes) = call(&app, "GET", "/app_runs?job_id=job-7", "").await;
    assert_eq!(status, StatusCode::OK);
    let list: AppRunList = ipc::decode("runs", &bytes).expect("a list");
    assert!(matches!(&list.runs[0].checkout, AppRunCheckout::Job { job_id, .. } if job_id == "job-7"));

    // An empty `job_id` is no Job: it is the main checkout.
    let (_, bytes) = call(&app, "GET", "/app_runs?job_id=", "").await;
    let list: AppRunList = ipc::decode("runs", &bytes).expect("a list");
    assert!(matches!(list.runs[0].checkout, AppRunCheckout::Main { .. }));
}

#[tokio::test]
async fn a_start_is_a_202_and_a_stop_a_200_each_with_the_run() {
    let app = wired();
    let body = format!(r#"{{"name":"{THE_RUN}","job_id":"job-7"}}"#);
    let (status, bytes) = call(&app, "POST", "/app_runs/start?manifest_id=01MF", &body).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{}", String::from_utf8_lossy(&bytes));
    let run: AppRun = ipc::decode("a run", &bytes).expect("a run");
    assert!(matches!(&run.checkout, AppRunCheckout::Job { job_id, .. } if job_id == "job-7"));

    let (status, _) = call(&app, "POST", "/app_runs/restart", &body).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (status, _) = call(&app, "POST", "/app_runs/stop", &body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_name_that_is_no_run_is_a_422_and_a_body_of_no_shape_a_400() {
    let app = wired();
    for route in ["start", "stop", "restart"] {
        let (status, bytes) = call(&app, "POST", &format!("/app_runs/{route}"), r#"{"name":"nope"}"#).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{route}");
        assert!(String::from_utf8_lossy(&bytes).contains("fleet.not_an_app_run"));
        let (status, _) = call(&app, "POST", &format!("/app_runs/{route}"), r#"{"job_id":"x"}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{route}");
    }
}

#[tokio::test]
async fn a_draft_is_read_and_a_save_starts_what_it_saved() {
    let app = wired();
    let (status, bytes) = call(&app, "GET", "/app_runs/draft", "").await;
    assert_eq!(status, StatusCode::OK);
    let draft: AppRunDraft = ipc::decode("a draft", &bytes).expect("a draft");
    assert_eq!(draft.root, "/repos/armada");

    let body = r#"{"name":"Web app","processes":[{"name":"web","command":"vite --port ${port}"}]}"#;
    let (status, bytes) = call(&app, "POST", "/app_runs/save", body).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{}", String::from_utf8_lossy(&bytes));
    let saved: SavedAppRun = ipc::decode("a saved run", &bytes).expect("saved");
    assert_eq!(saved.run.name, "Web app");

    let (status, _) = call(&app, "POST", "/app_runs/save", r#"{"name":"x"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
