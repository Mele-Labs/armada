//! The door a hosted session stands at in a root with no `armada.yml`: Setup's
//! session, whose `--permission-prompt-tool` is on this door. With no tools
//! here the agent CLI ended at the first call that needed a permission answer.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{door_without_a_manifest, router, Broadcaster, Served};

const SESSION: u16 = 52030;
const ANYONE: u16 = 52031;

const HANDSHAKE: &str =
    r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#;
const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;

fn hosting_a_session() -> (Arc<FakeDaemon>, Router) {
    let daemon = FakeDaemon::new(Broadcaster::new());
    *daemon.session_on.lock().expect("not poisoned") = Some(SESSION);
    let daemon = Arc::new(daemon);
    let app = router(Served::sharing(
        Arc::clone(&daemon),
        run_id(),
        Broadcaster::new(),
    ));
    (daemon, app)
}

async fn posted(app: &Router, port: u16, body: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let request = Request::builder()
        .method("POST")
        .uri(door_without_a_manifest())
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

fn named(tool: &str) -> String {
    format!("\"name\":\"{tool}\"")
}

/// **The permission tool and the session's own acts**, and nothing answered
/// inside a Manifest, which would be another repository's.
#[tokio::test]
async fn a_hosted_session_with_no_manifest_is_offered_its_permission_tool_and_its_own_acts() {
    let (_, app) = hosting_a_session();

    let listed = posted(&app, SESSION, LISTING).await;

    for tool in [
        "ask_the_person",
        "waiting_for",
        "all_done",
        "show_window",
        "show_sketch",
        "record_plan",
        "ask_in_window",
    ] {
        assert!(
            listed.contains(&named(tool)),
            "`{tool}` is offered: {listed}"
        );
    }
    for tool in [
        "list_jobs",
        "get_events_since",
        "list_sessions",
    ] {
        assert!(
            !listed.contains(&named(tool)),
            "`{tool}` needs a Manifest: {listed}"
        );
    }
    let said = posted(&app, SESSION, HANDSHAKE).await;
    assert!(said.contains("no armada.yml yet"), "{said}");
    assert!(!said.contains("inside Manifest"), "{said}");
}

/// What `--permission-prompt-tool` sends, answered by the daemon rather than
/// refused as a tool that is not there.
#[tokio::test]
async fn its_permission_prompt_is_answered() {
    let (daemon, app) = hosting_a_session();

    let answered = posted(
        &app,
        SESSION,
        &calling(
            "ask_the_person",
            r#"{"tool_name":"Bash","input":{"command":"ls"}}"#,
        ),
    )
    .await;

    assert!(answered.contains("allow"), "{answered}");
    assert!(!answered.contains("not a tool"), "{answered}");
    assert_eq!(daemon.asked_to_run.lock().expect("not poisoned").len(), 1);
}

#[tokio::test]
async fn a_tool_that_needs_a_manifest_is_refused_saying_why() {
    let (_, app) = hosting_a_session();

    let refused = posted(&app, SESSION, &calling("list_jobs", "{}")).await;

    assert!(refused.contains("\"isError\":true"), "{refused}");
    assert!(refused.contains("no armada.yml yet"), "{refused}");
}

/// **A connection that is no hosted session is offered nothing**: the acts
/// offered here are a session's own, and it has none.
#[tokio::test]
async fn a_caller_that_is_no_hosted_session_is_offered_nothing() {
    let (daemon, app) = hosting_a_session();

    let listed = posted(&app, ANYONE, LISTING).await;
    assert!(listed.contains("\"tools\":[]"), "{listed}");
    let said = posted(&app, ANYONE, HANDSHAKE).await;
    assert!(said.contains("not a job Armada is hosting"), "{said}");
    let refused = posted(
        &app,
        ANYONE,
        &calling(
            "ask_the_person",
            r#"{"tool_name":"Bash","input":{"command":"ls"}}"#,
        ),
    )
    .await;
    assert!(refused.contains("\"isError\":true"), "{refused}");
    assert!(daemon.asked_to_run.lock().expect("not poisoned").is_empty());
}
