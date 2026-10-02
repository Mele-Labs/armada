//! Helm's reach over a Job's processes, through the agent door: the owner's
//! decision of 2 Oct 2026 that Helm can kill one process of a Job, or every
//! one, as it can kill the Drone.
//!
//! **Offered to a Helm session alone**, so another agent's door session and a
//! Drone's endpoint list neither tool, and a Helm call reaches the same route
//! Bridge's press does — Fleet's tree guard included.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use ipc::door::Reachable;
use tower::ServiceExt;

use crate::tests::fake::{running, FakeDaemon};
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, DOOR_PATH, MCP_PATH};

/// The port a planted Helm session calls from, and one nobody placed.
const HELM: u16 = 52000;
const ANYONE: u16 = 52001;

/// The Drone's pid in the fake's reading, `shapes::resources`.
const THE_DRONE: u32 = 41233;

const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;

fn acting(_: &Reachable) -> bool {
    true
}

fn helm_holding_one() -> (Arc<FakeDaemon>, Router) {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB");
    // A Drone on it, so a kill that reached the act is one that took it away.
    daemon.jobs.lock().expect("not poisoned")[0].assigned_drone =
        Some(ipc::DroneId::carried("01DRONE"));
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

fn calling(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

#[tokio::test]
async fn helm_is_offered_both_kills_and_no_other_agent_or_drone_is() {
    let (_, app) = helm_holding_one();
    let helm = posted(&app, DOOR_PATH, HELM, LISTING).await;
    let anyone = posted(&app, DOOR_PATH, ANYONE, LISTING).await;
    let drone = posted(&app, MCP_PATH, ANYONE, LISTING).await;
    for tool in ["kill_process", "kill_processes"] {
        let named = format!("\"name\":\"{tool}\"");
        assert!(helm.contains(&named), "Helm is offered `{tool}`: {helm}");
        assert!(
            !anyone.contains(&named),
            "another agent is offered `{tool}`"
        );
        assert!(!drone.contains(&named), "a Drone is offered `{tool}`");
    }
    assert!(
        helm.contains("killing one leaves the Drone") && helm.contains("is `kill_drone`"),
        "the description says a child is not the Drone: {helm}"
    );
}

#[tokio::test]
async fn helms_kill_process_reaches_the_act_and_the_job_survives() {
    let (daemon, app) = helm_holding_one();
    let body = posted(
        &app,
        DOOR_PATH,
        HELM,
        &calling(
            "kill_process",
            &format!(r#"{{"job_id":"01JOB","pid":"{THE_DRONE}"}}"#),
        ),
    )
    .await;
    assert!(body.contains("\"isError\":false"), "{body}");
    let jobs = daemon.jobs.lock().expect("not poisoned");
    let job = jobs
        .iter()
        .find(|job| job.id.as_str() == "01JOB")
        .expect("the Job");
    assert_eq!(job.status.as_wire(), "running", "the Job survives");
    assert!(
        job.assigned_drone.is_none(),
        "the Drone's own pid is the Drone"
    );
}

#[tokio::test]
async fn helms_kill_process_outside_the_tree_meets_fleets_refusal() {
    let (_, app) = helm_holding_one();
    let body = posted(
        &app,
        DOOR_PATH,
        HELM,
        &calling("kill_process", r#"{"job_id":"01JOB","pid":"1"}"#),
    )
    .await;
    assert!(body.contains("\"isError\":true"), "{body}");
    assert!(body.contains("fleet.not_the_jobs_process"), "{body}");
}

#[tokio::test]
async fn helms_kill_processes_reaches_the_act_and_anyone_elses_is_refused() {
    let (daemon, app) = helm_holding_one();
    let kill_all = calling("kill_processes", r#"{"job_id":"01JOB"}"#);
    let refused = posted(&app, DOOR_PATH, ANYONE, &kill_all).await;
    assert!(
        refused.contains("is not a tool this Fleet offers"),
        "{refused}"
    );
    assert!(
        daemon.jobs.lock().expect("not poisoned")[0]
            .assigned_drone
            .is_some(),
        "another agent's call reached nothing"
    );

    let body = posted(&app, DOOR_PATH, HELM, &kill_all).await;
    assert!(body.contains("\"isError\":false"), "{body}");
    assert!(daemon.jobs.lock().expect("not poisoned")[0]
        .assigned_drone
        .is_none());
}
