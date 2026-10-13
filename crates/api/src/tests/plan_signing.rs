//! Who a plan act is signed as is the connection's word. `#2168`.
//!
//! The five acts that change a plan, each from the three callers the daemon
//! can place: Bridge's own request, a Helm session at the door, and a
//! Commander at the door. **A body that names a signer is ignored**, so the
//! same bytes from a connection nobody placed are a person's.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::{running, FakeDaemon};
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, Steerer, DOOR_PATH};

const HELM: u16 = 52000;
const COMMANDER: u16 = 52010;
const ANYONE: u16 = 52011;

fn a_helm_and_a_commander_on_one_job() -> Arc<FakeDaemon> {
    let daemon = FakeDaemon::new(Broadcaster::new());
    running(&daemon, "01JOB");
    *daemon.helm_on.lock().expect("not poisoned") = Some((HELM, |_| true));
    *daemon.commander_on.lock().expect("not poisoned") = Some(COMMANDER);
    Arc::new(daemon)
}

fn shared(daemon: &Arc<FakeDaemon>) -> Router {
    router(Served::sharing(
        Arc::clone(daemon),
        run_id(),
        Broadcaster::new(),
    ))
}

/// The five acts, as the door's tool arguments name them.
const THE_ACTS: [(&str, &str); 5] = [
    (
        "add_task",
        r#"{"job_id":"1","body":{"title":"t","note":"","scope":[],"expects":"","after":"","by":"commander"}}"#,
    ),
    (
        "drop_task",
        r#"{"job_id":"1","body":{"task":"T1","reason":"r","by":"commander"}}"#,
    ),
    (
        "edit_task",
        r#"{"job_id":"1","task_id":"T1","body":{"title":"x","by":"commander"}}"#,
    ),
    (
        "move_plan",
        r#"{"job_id":"1","body":{"group":"G1","task":"T1","by":"commander"}}"#,
    ),
    ("restart_task", r#"{"job_id":"1","task_id":"T1"}"#),
];

async fn at_the_door(app: &Router, port: u16, tool: &str, arguments: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let call = format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    );
    let request = Request::builder()
        .method("POST")
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(call))
        .expect("a request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

async fn bridge(app: &Router, method: &str, uri: &str, body: &str) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a request");
    app.clone().oneshot(request).await.expect("an answer");
}

fn signed(daemon: &FakeDaemon) -> Vec<Steerer> {
    daemon.plan_changed_by.lock().expect("not poisoned").clone()
}

#[tokio::test]
async fn a_commander_signs_all_five_plan_acts_as_the_commander() {
    let daemon = a_helm_and_a_commander_on_one_job();
    let app = shared(&daemon);
    for (tool, arguments) in THE_ACTS {
        let body = at_the_door(&app, COMMANDER, tool, arguments).await;
        assert!(body.contains("\"isError\":false"), "{tool}: {body}");
    }
    assert_eq!(signed(&daemon), vec![Steerer::Commander; 5]);
}

#[tokio::test]
async fn a_helm_session_signs_all_five_plan_acts_as_helm() {
    let daemon = a_helm_and_a_commander_on_one_job();
    let app = shared(&daemon);
    for (tool, arguments) in THE_ACTS {
        let body = at_the_door(&app, HELM, tool, arguments).await;
        assert!(body.contains("\"isError\":false"), "{tool}: {body}");
    }
    assert_eq!(signed(&daemon), vec![Steerer::Helm; 5]);
}

/// Bridge's request carries no mark, and a body claiming the Commander's
/// signature changes nothing: there is no field for it to land in.
#[tokio::test]
async fn bridge_signs_all_five_plan_acts_as_a_person_whatever_the_body_claims() {
    let daemon = a_helm_and_a_commander_on_one_job();
    let app = shared(&daemon);
    let [add, drop, edit, moving, _] = THE_ACTS.map(|(_, arguments)| arguments);
    let body_of = |arguments: &str| {
        let start = arguments.find("\"body\":").map(|at| at + 7);
        start.map_or("{}".to_string(), |at| {
            arguments[at..arguments.len() - 1].to_string()
        })
    };
    bridge(&app, "POST", "/jobs/01JOB/add_task", &body_of(add)).await;
    bridge(&app, "POST", "/jobs/01JOB/drop_task", &body_of(drop)).await;
    bridge(&app, "POST", "/jobs/01JOB/tasks/T1/edit", &body_of(edit)).await;
    bridge(&app, "POST", "/jobs/01JOB/plan/move", &body_of(moving)).await;
    bridge(&app, "POST", "/jobs/01JOB/tasks/T1/restart", "").await;
    assert_eq!(signed(&daemon), vec![Steerer::Person; 5]);
}

/// A connection nobody placed, calling at the door with the Commander's name
/// in a header and in every body, is a person's agent session.
#[tokio::test]
async fn an_unplaced_caller_at_the_door_is_not_a_commander_for_saying_so() {
    let daemon = a_helm_and_a_commander_on_one_job();
    let app = shared(&daemon);
    // `ANYONE` is offered neither Helm's rows nor a Commander's, so the call
    // is refused and nothing is signed at all.
    for (tool, arguments) in THE_ACTS {
        let body = at_the_door(&app, ANYONE, tool, arguments).await;
        assert!(body.contains("\"isError\":true"), "{tool}: {body}");
    }
    assert!(signed(&daemon).is_empty());
}
