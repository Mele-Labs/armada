//! Helm's reach over pausing a Job, through the agent door: `park_job` and
//! `resume_job` are `Helm only`, as `restart_step` is, so a Helm session is
//! offered them on a person's ask and no other agent or Drone is.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use ipc::door::Reachable;
use tower::ServiceExt;

use crate::tests::fake::{at, FakeDaemon};
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, DOOR_PATH, MCP_PATH};

const HELM: u16 = 52010;
const ANYONE: u16 = 52011;

const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;

fn acting(_: &Reachable) -> bool {
    true
}

/// A Job at a review gate, which a pause must leave reading `awaiting_review`.
fn helm_holding_a_gate_job() -> (Arc<FakeDaemon>, Router) {
    let daemon = FakeDaemon::new(Broadcaster::new());
    at(&daemon, "01JOB", "awaiting_review");
    *daemon.helm_on.lock().expect("not poisoned") = Some((HELM, acting));
    let daemon = Arc::new(daemon);
    let app = router(Served::sharing(
        Arc::clone(&daemon),
        run_id(),
        Broadcaster::new(),
    ));
    (daemon, app)
}

async fn posted(app: &Router, uri: &str, port: u16, body: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a readable body")
        .to_bytes();
    String::from_utf8_lossy(&body).to_string()
}

fn calling(tool: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{{"job_id":"01JOB"}}}}}}"#
    )
}

fn paused_marker(daemon: &FakeDaemon) -> bool {
    daemon.jobs.lock().expect("not poisoned")[0]
        .paused
        .is_some()
}

#[tokio::test]
async fn helm_is_offered_pause_and_resume_and_no_other_agent_or_drone_is() {
    let (_, app) = helm_holding_a_gate_job();
    let helm = posted(&app, DOOR_PATH, HELM, LISTING).await;
    let anyone = posted(&app, DOOR_PATH, ANYONE, LISTING).await;
    let drone = posted(&app, MCP_PATH, ANYONE, LISTING).await;
    for tool in ["park_job", "resume_job"] {
        let named = format!("\"name\":\"{tool}\"");
        assert!(helm.contains(&named), "Helm is offered `{tool}`: {helm}");
        assert!(
            !anyone.contains(&named),
            "another agent is offered `{tool}`"
        );
        assert!(!drone.contains(&named), "a Drone is offered `{tool}`");
    }
}

#[tokio::test]
async fn helms_pause_marks_the_gate_job_without_moving_its_status_and_anyone_elses_is_refused() {
    let (daemon, app) = helm_holding_a_gate_job();
    let refused = posted(&app, DOOR_PATH, ANYONE, &calling("park_job")).await;
    assert!(
        refused.contains("is not a tool this Fleet offers"),
        "{refused}"
    );
    assert!(
        !paused_marker(&daemon),
        "another agent's call reached nothing"
    );

    let body = posted(&app, DOOR_PATH, HELM, &calling("park_job")).await;
    assert!(body.contains("\"isError\":false"), "{body}");
    assert!(paused_marker(&daemon));
    assert_eq!(
        daemon.jobs.lock().expect("not poisoned")[0]
            .status
            .as_wire(),
        "awaiting_review",
        "the marker is beside the status"
    );

    let body = posted(&app, DOOR_PATH, HELM, &calling("resume_job")).await;
    assert!(body.contains("\"isError\":false"), "{body}");
    assert!(!paused_marker(&daemon));
}
