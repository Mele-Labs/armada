//! A Job on your words passing words to a live Session, through the agent door: `tell_session` is offered to a
//! Job on your words alone, and a call reaches the route Fleet sends from.
//!
//! **What a send is, is Fleet's** (`fleet::tests::operator_telling`): the fake here keeps the words, so
//! this holds who is offered the tool and that a call arrives where it should.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use ipc::door::Reachable;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, DOOR_PATH};

/// The port a planted Job on your words calls from, and one nobody placed.
const WORDS: u16 = 52000;
const ANYONE: u16 = 52001;

const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
const TELLING: &str = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"tell_session","arguments":{"body":{"session_id":"01SESSION000000000000000001","text":"make the take names shorter"}}}}"#;

fn acting(_: &Reachable) -> bool {
    true
}

fn only_reading(row: &Reachable) -> bool {
    row.kind == "query"
}

fn words_on(may: fn(&Reachable) -> bool) -> (Arc<FakeDaemon>, Router) {
    let daemon = FakeDaemon::new(Broadcaster::new());
    *daemon.words_on.lock().expect("not poisoned") = Some((WORDS, may));
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
async fn a_job_on_your_words_is_offered_the_tool_and_no_other_agent_is() {
    let (_, app) = words_on(acting);
    let on_words = posted(&app, DOOR_PATH, WORDS, LISTING).await;
    let anyone = posted(&app, DOOR_PATH, ANYONE, LISTING).await;

    let named = "\"name\":\"tell_session\"";
    assert!(on_words.contains(named), "a Job on your words is offered `tell_session`: {on_words}");
    assert!(!anyone.contains(named), "another agent is offered it");
}

#[tokio::test]
async fn a_jobs_call_reaches_the_route_and_anyone_elses_is_refused() {
    let (daemon, app) = words_on(acting);

    let refused = posted(&app, DOOR_PATH, ANYONE, TELLING).await;
    assert!(refused.contains("is not a tool this Fleet offers"), "{refused}");
    assert!(
        daemon.words_sent.lock().expect("not poisoned").is_empty(),
        "another agent's call reached nothing"
    );

    let told = posted(&app, DOOR_PATH, WORDS, TELLING).await;
    assert!(told.contains("\"isError\":false"), "{told}");
    assert!(told.contains("01SESSION000000000000000001"), "it says where the words went: {told}");
    let sent = daemon.words_sent.lock().expect("not poisoned").clone();
    let [(text, target)] = sent.as_slice() else {
        panic!("the call reached the route once: {sent:?}");
    };
    assert_eq!(text, "make the take names shorter");
    assert_eq!(target.id, "01SESSION000000000000000001");
}

#[tokio::test]
async fn a_read_only_job_is_not_offered_it_and_is_refused_by_name() {
    let (_, app) = words_on(only_reading);

    let listed = posted(&app, DOOR_PATH, WORDS, LISTING).await;
    assert!(!listed.contains("\"name\":\"tell_session\""), "{listed}");

    let refused = posted(&app, DOOR_PATH, WORDS, TELLING).await;
    assert!(refused.contains("`tell_session` is not this Job's to call"), "{refused}");
}
