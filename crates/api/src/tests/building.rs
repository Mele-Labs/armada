//! `get_fleet_build` and `change_fleet_build` over the router: the read is the
//! report as the daemon gave it, a change is a 202 that names the build it moves
//! onto, `adopt` arrives as sent and defaults to no, and a build outside the two
//! is a 400 that starts nothing.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{BuildSource, BuildStage, FleetBuildChanging, FleetBuildReport};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::{self, run_id};
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(
        FakeDaemon::new(events.clone()),
        run_id(),
        events,
    ))
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
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    (status, bytes.to_vec())
}

async fn read(app: &Router) -> FleetBuildReport {
    let (status, body) = call(app, "GET", "/fleet/build", "").await;
    assert_eq!(status, StatusCode::OK);
    ipc::decode("build", &body).expect("the build decodes")
}

#[tokio::test]
async fn the_read_is_the_report_the_daemon_gave() {
    let app = wired();
    assert_eq!(read(&app).await, shapes::fleet_build());
}

#[tokio::test]
async fn a_change_is_accepted_and_the_next_read_shows_the_restart() {
    let app = wired();
    let (status, body) = call(
        &app,
        "POST",
        "/fleet/build/change",
        r#"{"build":"preview","adopt":true}"#,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let changing: FleetBuildChanging = ipc::decode("answer", &body).expect("decodes");
    assert_eq!(changing.build, BuildSource::Preview);
    assert_eq!(read(&app).await.restarting, Some(BuildSource::Preview));
}

#[tokio::test]
async fn the_stage_of_a_restart_under_way_crosses_the_wire_as_a_word() {
    let app = wired();
    let (_, before) = call(&app, "GET", "/fleet/build", "").await;
    assert!(!String::from_utf8_lossy(&before).contains("stage"), "no restart, no stage");
    call(&app, "POST", "/fleet/build/change", r#"{"build":"main"}"#).await;
    let (_, body) = call(&app, "GET", "/fleet/build", "").await;
    assert!(String::from_utf8_lossy(&body).contains(r#""stage":"building_fleet""#));
    assert_eq!(read(&app).await.stage, Some(BuildStage::BuildingFleet));
}

#[tokio::test]
async fn a_build_that_is_neither_main_nor_the_preview_is_a_400_and_starts_nothing() {
    let app = wired();
    let (status, _) = call(&app, "POST", "/fleet/build/change", r#"{"build":"feature"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(read(&app).await.restarting, None);
}
