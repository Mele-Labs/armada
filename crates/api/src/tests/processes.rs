//! Pulse's two kills, over the router: the paths Bridge already sends, what
//! each answers, and the refusal a pid outside the Job's tree meets. `#1647`.
//!
//! The fake's tree is its resources reading, so the pid the panel draws is the
//! pid that is accepted, and the refusal is Fleet's code and field.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{JobSummary, WireError, WireValue};
use tower::ServiceExt;

use crate::tests::fake::{running, FakeDaemon};
use crate::tests::shapes::{self, run_id};
use crate::{router, Broadcaster, Served};

/// The Drone's pid in the fake's reading, `shapes::resources`.
const THE_DRONE: u32 = 41233;

fn wired() -> Router {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    running(&daemon, "01RUNNING");
    router(Served::by(daemon, run_id(), events))
}

async fn post(app: &Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .body(Body::empty())
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let body = response.into_body().collect().await.expect("a body");
    (status, body.to_bytes().to_vec())
}

#[tokio::test]
async fn the_drones_pid_is_killed_through_its_path_and_the_job_survives() {
    let app = wired();
    assert!(
        shapes::resources(ipc::JobId::carried("01RUNNING"))
            .processes
            .iter()
            .any(|one| one.pid == THE_DRONE && one.recorded),
        "the fixture's tree names the pid this sends"
    );

    let (status, body) = post(&app, &format!("/jobs/01RUNNING/processes/{THE_DRONE}/kill")).await;

    assert_eq!(status, StatusCode::OK);
    let job: JobSummary = ipc::decode("job summary", &body).expect("the Job, back");
    assert_eq!(
        job.status.as_wire(),
        "running",
        "killing a process does not end its Job"
    );
    assert!(
        job.assigned_drone.is_none(),
        "the Drone's own pid is the Drone"
    );
}

#[tokio::test]
async fn a_pid_outside_the_tree_is_a_conflict_naming_it() {
    let app = wired();

    let (status, body) = post(&app, "/jobs/01RUNNING/processes/1/kill").await;

    assert_eq!(status, StatusCode::CONFLICT);
    let refused: WireError = ipc::decode("a refusal", &body).expect("a wire error");
    assert_eq!(refused.code, "fleet.not_the_jobs_process");
    assert_eq!(refused.fields.get("pid"), Some(&WireValue::Int(1)));
}

#[tokio::test]
async fn a_segment_that_is_not_a_pid_is_undecodable_and_reaches_nothing() {
    let app = wired();

    let (status, body) = post(&app, "/jobs/01RUNNING/processes/-1/kill").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let refused: WireError = ipc::decode("a refusal", &body).expect("a wire error");
    assert!(refused.message.contains("a pid"), "{}", refused.message);
}

#[tokio::test]
async fn kill_all_answers_the_job_with_no_drone_on_it() {
    let app = wired();

    let (status, body) = post(&app, "/jobs/01RUNNING/processes/kill").await;

    assert_eq!(status, StatusCode::OK);
    let job: JobSummary = ipc::decode("job summary", &body).expect("the Job, back");
    assert_eq!(job.status.as_wire(), "running");
    assert!(job.assigned_drone.is_none());
}

#[tokio::test]
async fn both_kills_are_a_404_on_a_job_that_is_not_there() {
    let app = wired();

    for uri in [
        "/jobs/01NOTHING0000000000000000/processes/41233/kill",
        "/jobs/01NOTHING0000000000000000/processes/kill",
    ] {
        let (status, _) = post(&app, uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
}
