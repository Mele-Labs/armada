//! Helm passing words to a live Session, through the agent door: `tell_session` is offered to a
//! Helm session alone, and a call reaches the route Fleet keeps the thread's row behind.
//!
//! **What a send is, is Fleet's** (`fleet::tests::helm_telling`): the fake here keeps the row, so
//! this holds who is offered the tool and that a call arrives where it should.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use ipc::door::Reachable;
use ipc::HelmMessage;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, HelmSeen, Served, DOOR_PATH, MCP_PATH};

/// The port a planted Helm session calls from, and one nobody placed.
const HELM: u16 = 52000;
const ANYONE: u16 = 52001;

const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
const TELLING: &str = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"tell_session","arguments":{"body":{"session_id":"01SESSION000000000000000001","text":"make the take names shorter"}}}}"#;

fn acting(_: &Reachable) -> bool {
    true
}

fn only_reading(row: &Reachable) -> bool {
    row.kind == "query"
}

fn helm_on(may: fn(&Reachable) -> bool) -> (Arc<FakeDaemon>, Router) {
    let daemon = FakeDaemon::new(Broadcaster::new());
    *daemon.helm_on.lock().expect("not poisoned") = Some((HELM, may));
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

#[tokio::test]
async fn helm_is_offered_the_tool_and_no_other_agent_or_drone_is() {
    let (_, app) = helm_on(acting);
    let helm = posted(&app, DOOR_PATH, HELM, LISTING).await;
    let anyone = posted(&app, DOOR_PATH, ANYONE, LISTING).await;
    let drone = posted(&app, MCP_PATH, ANYONE, LISTING).await;

    let named = "\"name\":\"tell_session\"";
    assert!(helm.contains(named), "Helm is offered `tell_session`: {helm}");
    assert!(!anyone.contains(named), "another agent is offered it");
    assert!(!drone.contains(named), "a Drone is offered it");
}

#[tokio::test]
async fn helms_call_reaches_the_route_and_leaves_a_row_and_anyone_elses_is_refused() {
    let (daemon, app) = helm_on(acting);
    let mut watching = daemon.helm.watch();

    let refused = posted(&app, DOOR_PATH, ANYONE, TELLING).await;
    assert!(refused.contains("is not a tool this Fleet offers"), "{refused}");
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), watching.next())
            .await
            .is_err(),
        "another agent's call reached nothing"
    );

    let told = posted(&app, DOOR_PATH, HELM, TELLING).await;
    assert!(told.contains("\"isError\":false"), "{told}");
    assert!(told.contains("01SESSION000000000000000001"), "it says where the words went: {told}");
    let Some(HelmSeen::Message(HelmMessage::Sent(row))) = watching.next().await else {
        panic!("the thread kept a sent row");
    };
    assert_eq!(row.text, "make the take names shorter");
    assert_eq!(row.target.id, "01SESSION000000000000000001");
}

#[tokio::test]
async fn a_read_only_helm_is_not_offered_it_and_is_refused_by_name() {
    let (_, app) = helm_on(only_reading);

    let listed = posted(&app, DOOR_PATH, HELM, LISTING).await;
    assert!(!listed.contains("\"name\":\"tell_session\""), "{listed}");

    let refused = posted(&app, DOOR_PATH, HELM, TELLING).await;
    assert!(refused.contains("`tell_session` is not Helm's to call"), "{refused}");
}
