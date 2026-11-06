//! `get_fleet_build`, `change_fleet_build`, `revert_fleet_build` and the snapshot's `pick_fleet_build_job`,
//! `swap_fleet_build` and `keep_fleet_build` over the router: the read is the
//! report as the daemon gave it, a change is a 202 that names the build it moves
//! onto, `adopt` arrives as sent and defaults to no, and a build outside the two
//! is a 400 that starts nothing.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{
    BuildSource, BuildStage, FleetBuildChanging, FleetBuildKept, FleetBuildReport, FleetBuildReverting,
    FleetBuildSnapshotting, FleetBuildSwapping,
};
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

#[tokio::test]
async fn the_read_carries_the_kept_builds_as_the_daemon_gave_them() {
    let app = wired();
    let (_, body) = call(&app, "GET", "/fleet/build", "").await;
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains(r#""running":{"built_at":"2026-11-05T17:05:00Z""#), "{text}");
    assert!(text.contains(r#""takes_out":["Fleet core and mods"]"#), "{text}");
    assert!(!text.contains("brings_back"), "an empty list leaves no key: {text}");
    assert!(!text.contains("reverting"), "{text}");
    assert!(text.contains(r#""carried":["Kept builds"]"#), "{text}");
}

#[tokio::test]
async fn a_revert_is_accepted_with_no_body_and_the_next_read_says_reverting() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/fleet/build/revert", "").await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let reverting: FleetBuildReverting = ipc::decode("answer", &body).expect("decodes");
    assert_eq!(Some(reverting.commit), shapes::fleet_build().previous.map(|previous| previous.commit));
    assert!(read(&app).await.reverting);
}

#[tokio::test]
async fn the_read_carries_the_snapshot_as_the_daemon_gave_it() {
    let app = wired();
    let (_, body) = call(&app, "GET", "/fleet/build", "").await;
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains(r#""picked":[{"branch":"armada/kept-builds","title":"Kept builds"}]"#), "{text}");
    assert!(text.contains(r#""snapshot":{"state":"ready","built_at":"2026-11-05T18:10:00Z""#), "{text}");
    assert!(text.contains(r#""trying":["Kept builds"]"#), "{text}");
    assert!(!text.contains("swapping"), "false leaves no key: {text}");
}

#[tokio::test]
async fn a_pick_is_accepted_with_its_body_and_the_answer_and_next_read_carry_the_branches_picked() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/fleet/build/pick", r#"{"branch":"armada/a","picked":true}"#).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let snapshotting: FleetBuildSnapshotting = ipc::decode("answer", &body).expect("decodes");
    assert_eq!(snapshotting.picked, ["armada/kept-builds", "armada/a"]);
    assert_eq!(read(&app).await.picked.len(), 2);

    let (_, body) = call(&app, "POST", "/fleet/build/pick", r#"{"branch":"armada/kept-builds","picked":false}"#).await;
    let snapshotting: FleetBuildSnapshotting = ipc::decode("answer", &body).expect("decodes");
    assert_eq!(snapshotting.picked, ["armada/a"], "a branch taken back out leaves");
}

#[tokio::test]
async fn a_pick_without_its_flag_is_a_400_and_picks_nothing() {
    let app = wired();
    let (status, _) = call(&app, "POST", "/fleet/build/pick", r#"{"branch":"armada/a"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(read(&app).await.picked.len(), 1);
}

#[tokio::test]
async fn a_swap_is_accepted_with_no_body_and_the_next_read_says_swapping() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/fleet/build/swap", "").await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let swapping: FleetBuildSwapping = ipc::decode("answer", &body).expect("decodes");
    assert_eq!(Some(swapping.commit), shapes::fleet_build().snapshot.and_then(|snapshot| snapshot.commit));
    assert!(read(&app).await.swapping);
}

#[tokio::test]
async fn a_keep_is_a_200_naming_the_jobs_told_and_a_second_tells_none() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/fleet/build/keep", "").await;
    assert_eq!(status, StatusCode::OK);
    let kept: FleetBuildKept = ipc::decode("answer", &body).expect("decodes");
    assert_eq!(kept.told, ["Kept builds"]);
    assert!(read(&app).await.trying.is_empty());

    let (status, body) = call(&app, "POST", "/fleet/build/keep", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ipc::decode::<FleetBuildKept>("answer", &body).expect("decodes").told, Vec::<String>::new());
}
