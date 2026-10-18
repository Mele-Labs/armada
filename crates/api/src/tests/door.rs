//! The agent's door, over the router, with no agent and no socket.
//!
//! What these prove is the one thing `ipc`'s own cases cannot: that a tool
//! call reaches the same handler a Bridge request reaches, and comes back with
//! what that handler answered.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use std::net::SocketAddr;
use std::sync::Arc;

use ipc::door::Reachable;

use crate::tests::fake::{FakeDaemon, THE_STUDIO};
use crate::tests::shapes::{
    run_id, THE_CALL, THE_DRONE, THE_FRAME, THE_MANIFEST, THE_OUTPUT,
};
use crate::{offerable, offered, router, Broadcaster, Served, DOOR_PATH, SERVED};

fn wired(daemon: FakeDaemon) -> Router {
    let events = Broadcaster::new();
    router(Served::by(daemon, run_id(), events))
}

/// A Fleet holding one Studio, which every case below reaches by its id.
fn holding_one() -> FakeDaemon {
    FakeDaemon::new(Broadcaster::new())
}

async fn call(app: &Router, body: &str) -> (StatusCode, String) {
    method(app, "POST", body).await
}

async fn method(app: &Router, method: &str, body: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a readable body")
        .to_bytes();
    (status, String::from_utf8_lossy(&body).to_string())
}

/// **The tool set is the inventory's, not this crate's.** A hand-kept list is
/// how a row gains `agent_access = "Yes"` and stays unreachable.
#[test]
fn every_tool_offered_is_an_operation_the_route_table_serves() {
    let unserved: Vec<&str> = offered()
        .iter()
        .filter(|shape| shape.path.is_empty())
        .map(|shape| shape.operation)
        .collect();
    assert!(
        unserved.is_empty(),
        "these operations are reachable by an agent and served at no route: {unserved:?}. \
         The gate rule in `xtask` names the same set"
    );
    assert!(
        offered()
            .iter()
            .all(|shape| SERVED.iter().any(|route| route.path == shape.path)),
        "a tool names a path the route table does not"
    );
}

#[tokio::test]
async fn the_tool_list_is_every_reachable_operation_and_names_its_route() {
    let app = wired(holding_one());
    let (status, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for expected in ["get_studio", "get_events_since", "list_jobs"] {
        assert!(
            body.contains(&format!("\"name\":\"{expected}\"")),
            "the list is missing {expected}"
        );
    }
    assert!(
        !body.contains("\"name\":\"observe_land_check\""),
        "a Bridge-only operation reached the agent's tool set"
    );
    assert!(
        body.contains("GET /studios/:studio_id"),
        "a tool must name its route"
    );
}

/// Every tool, called. **The door's half of `every_operation_the_table_names_is_routed`**:
/// that one proves a path is in the router, and this proves the door builds a
/// request that reaches it. A tool that answers "no such tool" or a route that
/// is not there is what fails here.
#[tokio::test]
async fn every_tool_the_door_offers_answers_something_other_than_a_missing_route() {
    // The Studio every path below names is real, so a route that is there and
    // answers "no such Studio" cannot be mistaken for a route that is not.
    let app = wired(holding_one());
    for shape in offered() {
        let arguments = format!(
            r#"{{"studio_id":"{THE_STUDIO}","drone_id":"{THE_DRONE}","manifest_id":"{THE_MANIFEST}",
               "call_id":"{THE_CALL}","kept":"{THE_OUTPUT}","run":"{}","name":"{}",
               "since":"0","q":"src","body":{{}}}}"#,
            THE_FRAME.split('/').next().expect("a run directory"),
            THE_FRAME.split('/').nth(1).expect("a frame name"),
        );
        let (status, body) = call(
            &app,
            &format!(
                r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{}",
                   "arguments":{arguments}}}}}}}"#,
                shape.operation
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{}", shape.operation);
        assert!(
            !body.contains("is not a tool this Fleet offers"),
            "{} is offered and not callable: {body}",
            shape.operation
        );
        assert!(
            !body.contains("served at no route"),
            "{} is offered and routed nowhere: {body}",
            shape.operation
        );
        assert!(
            !body.contains("did not make a request"),
            "{} built no request: {body}",
            shape.operation
        );
        // **An empty text block is the shape a missing route takes here.** A
        // route axum does not hold answers 404 with no body, which reaches the
        // agent as a tool error saying nothing at all.
        assert!(
            !body.contains(r#""text":"""#),
            "{} answered with nothing, which is what an unrouted path does: {body}",
            shape.operation
        );
    }
}

/// The point of the whole door: a tool call and a Bridge request reach one
/// handler and come back with one answer.
#[tokio::test]
async fn a_tool_call_is_answered_by_the_route_that_serves_the_operation() {
    let app = wired(holding_one());
    let (status, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"get_studio","arguments":{"studio_id":"01STUDIO"}}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("01STUDIO"), "{body}");
    assert!(body.contains("\"isError\":false"), "{body}");
}

/// A 404 from the surface is a tool error the model reads, never a transport
/// failure it can only retry.
#[tokio::test]
async fn a_refusal_from_the_surface_reaches_the_agent_as_something_to_read() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let (status, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"get_studio","arguments":{"studio_id":"99"}}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"isError\":true"), "{body}");
    assert!(body.contains("no_such_studio"), "{body}");
}

/// A command goes through as a POST and its body decodes.
///
/// **The answer is the proof.** The fake echoes the note it was handed, which
/// it could only do with a decoded body — an undecodable one is a different
/// refusal one layer out.
#[tokio::test]
async fn a_command_reaches_the_post_route_with_its_body() {
    let app = wired(holding_one());
    let (status, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call",
            "params":{"name":"mark_annotation_done","arguments":
            {"body":{"note_id":"note-7"}}}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("annotations/note-7.json"), "{body}");
    assert!(
        !body.contains("undecodable"),
        "the body did not decode: {body}"
    );
}

/// The handshake says which Manifest the session is inside and what stands in
/// for the stream it does not get.
#[tokio::test]
async fn the_handshake_states_the_scope_and_the_polling_obligation() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    let (_, body) = call(
        &app,
        r#"{"jsonrpc":"2.0","id":6,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
    )
    .await;
    assert!(body.contains("armada-fleet"), "{body}");
    assert!(body.contains("01MF"), "the scope is named: {body}");
    assert!(body.contains("get_events_since"), "{body}");
}

/// No server-initiated stream and no session to end, which is what keeps this
/// door out of the unbounded-sink risk on the event socket.
#[tokio::test]
async fn a_get_or_a_delete_is_refused() {
    let app = wired(FakeDaemon::new(Broadcaster::new()));
    for verb in ["GET", "DELETE"] {
        let (status, _) = method(&app, verb, "").await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{verb}");
    }
}

/// The port a planted Helm session calls from, and one nobody placed.
const HELM: u16 = 52000;
const ANYONE: u16 = 52001;

/// An acting Helm, as a test states it. **Not `fleet::helm::may`**, which this
/// crate cannot name and which is tested beside itself; here the door is under
/// test, and any predicate proves it enforces what it was handed.
fn acting(row: &Reachable) -> bool {
    row.kind == "query" || row.operation != "all_done"
}

fn read_only(row: &Reachable) -> bool {
    row.kind == "query"
}

fn helm_holding(may: fn(&Reachable) -> bool) -> Arc<FakeDaemon> {
    let daemon = holding_one();
    *daemon.helm_on.lock().expect("not poisoned") = Some((HELM, may));
    Arc::new(daemon)
}

fn shared(daemon: &Arc<FakeDaemon>) -> Router {
    router(Served::sharing(
        Arc::clone(daemon),
        run_id(),
        Broadcaster::new(),
    ))
}

async fn from(app: &Router, port: u16, body: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let request = Request::builder()
        .method("POST")
        .uri(DOOR_PATH)
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

const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;

/// **Only a Helm session is narrowed.** Every other caller is listed exactly
/// what the door listed before.
#[tokio::test]
async fn a_helm_session_is_listed_what_it_may_call_and_anyone_else_what_they_were() {
    let app = shared(&helm_holding(acting));
    let helm = from(&app, HELM, LISTING).await;
    assert!(helm.contains("\"name\":\"get_studio\""), "{helm}");
    assert!(!helm.contains("\"name\":\"all_done\""), "{helm}");

    let anyone = from(&app, ANYONE, LISTING).await;
    assert!(anyone.contains("\"name\":\"all_done\""), "{anyone}");
    let unplaced = call(&app, LISTING).await.1;
    assert_eq!(unplaced, anyone, "a call with no peer is any agent");
}

#[tokio::test]
async fn read_only_refuses_every_command_and_still_reads() {
    let app = shared(&helm_holding(read_only));
    for row in offerable().filter(|row| row.kind == "command") {
        let body = from(
            &app,
            HELM,
            &calling(
                row.operation,
                r#"{"studio_id":"01STUDIO","pid":"1","task_id":"T1","drone_id":"01DRONE","lesson_id":"01LESSON-0","repository":"01REPO","number":"1","body":{}}"#,
            ),
        )
        .await;
        // An operation nothing serves (`xtask`'s `NOT_BUILT`) 404s before the
        // reach is ever consulted — for any caller, acting or not — so its
        // refusal names no route rather than naming Helm's authority.
        let expected = match SERVED.iter().any(|route| route.operation == row.operation) {
            true => format!("`{}` is not Helm's to call", row.operation),
            false => format!(
                "`{}` is named in this Fleet's operation inventory and is served at no route",
                row.operation
            ),
        };
        assert!(body.contains(&expected), "{}: {body}", row.operation);
    }
    let read = from(&app, HELM, &calling("get_studio", r#"{"studio_id":"01STUDIO"}"#)).await;
    assert!(
        read.contains("01STUDIO") && read.contains("\"isError\":false"),
        "{read}"
    );
}

