//! Requests over the router: the routes exist, a body reaches the daemon whole, and who sent a
//! request is the door it came through. Only Bridge and the phone are the owner; a request naming
//! nobody is an agent, so a script that leaves the header out is never answered as the owner.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{RequestEnvironmentsRead, RequestHosts, RequestSent};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, BRIDGE, CALLER_HEADER, PHONE};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str, caller: Option<&str>) -> (StatusCode, Vec<u8>) {
    let mut request = Request::builder().method(method).uri(uri).header("content-type", "application/json");
    if let Some(caller) = caller {
        request = request.header(CALLER_HEADER, caller);
    }
    let request = request.body(Body::from(body.to_string())).expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    (status, bytes.to_vec())
}

const SEND: &str = r#"{"method":"GET","url":"https://api.example.test/v1/me","headers":[]}"#;

#[tokio::test]
async fn only_bridge_and_the_phone_send_as_the_owner() {
    let app = wired();
    for (caller, by) in [(Some(BRIDGE), "\"owner\""), (Some(PHONE), "\"owner\""), (None, "\"agent\""), (Some("script"), "\"agent\"")] {
        let (status, body) = call(&app, "POST", "/requests/send", SEND, caller).await;
        assert_eq!(status, StatusCode::OK);
        let sent: RequestSent = ipc::decode("sent", &body).expect("decodes");
        assert_eq!((caller, sent.body.as_str()), (caller, by));
        assert_eq!(sent.url, "https://api.example.test/v1/me");
    }
}

#[tokio::test]
async fn the_cookie_and_environment_routes_answer() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/requests/cookies", r#"{"host":"api.example.test","value":"a=b"}"#, Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::OK);
    let hosts: RequestHosts = ipc::decode("hosts", &body).expect("decodes");
    let kept = |header: &str| vec![ipc::RequestHost { host: "api.example.test".to_string(), header: header.to_string() }];
    assert_eq!(hosts.hosts, kept("Cookie"), "a header left out is a Cookie");
    let named = r#"{"host":"api.example.test","header":"Authorization","value":"Bearer t0ken-0f-the-owner"}"#;
    let (status, body) = call(&app, "POST", "/requests/cookies", named, Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::OK);
    let hosts: RequestHosts = ipc::decode("hosts", &body).expect("decodes");
    assert_eq!(hosts.hosts, kept("Authorization"));
    let (status, _) = call(&app, "GET", "/requests/cookies", "", Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&app, "POST", "/requests/cookies/forget", r#"{"host":"api.example.test"}"#, Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&app, "GET", "/requests/environments?session_id=s1", "", Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::OK);
    let sent = r#"{"version":1,"environment":[{"name":"staging","base":"https://staging.example.test"}]}"#;
    let (status, body) = call(&app, "POST", "/requests/environments", sent, Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::OK);
    let RequestEnvironmentsRead::Read { file } = ipc::decode("saved", &body).expect("decodes") else {
        panic!("read")
    };
    assert_eq!(file.environment[0].name, "staging");
    let (status, _) = call(&app, "POST", "/requests/environments", r#"{"version":1,"environmnt":[]}"#, Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
