//! Requests over the router: the routes exist, a body reaches the daemon whole, and who sent a
//! request is the door it came through. Only Bridge and the phone are the owner; a request naming
//! nobody is an agent, so a script that leaves the header out is never answered as the owner.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{RequestSaved, RequestSent, Requests};
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
async fn the_collection_and_environment_routes_answer() {
    let app = wired();
    let (status, body) = call(&app, "GET", "/requests?repository=%2Frepo&session_id=s1", "", Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::OK);
    let read: Requests = ipc::decode("read", &body).expect("decodes");
    assert_eq!(read.repository, "/repo");

    let collection = r#"{"repository":"/repo","collection":{"id":"users","name":"Users","headers":[],"requests":[]}}"#;
    let (status, body) = call(&app, "POST", "/requests/collections", collection, Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::OK);
    let read: Requests = ipc::decode("saved", &body).expect("decodes");
    assert_eq!(read.collections[0].item.id, "users");

    let environment = r#"{"repository":"/repo","environment":{"name":"staging","variables":[{"name":"token","secret":true,"value":"t0ken-0f-the-0wner"}]}}"#;
    let (status, _) = call(&app, "POST", "/requests/environments", environment, Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::OK);
    let plain_without_value = r#"{"repository":"/repo","environment":{"name":"staging","variables":[{"name":"host","secret":false}]}}"#;
    let (status, body) = call(&app, "POST", "/requests/environments", plain_without_value, Some(BRIDGE)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!String::from_utf8_lossy(&body).contains("t0ken"), "an error never quotes a value");

    let save = r#"{"collection":"Users","request":{"name":"me","method":"GET","url":"{{base}}/v1/me","headers":[]}}"#;
    let (status, body) = call(&app, "POST", "/requests/collections/request", save, None).await;
    assert_eq!(status, StatusCode::OK);
    let saved: RequestSaved = ipc::decode("saved", &body).expect("decodes");
    assert_eq!((saved.collection_id.as_str(), saved.request_id.as_str()), ("Users", "me"));

    for retired in ["/requests/cookies", "/requests/cookies/forget"] {
        let (status, _) = call(&app, "POST", retired, "{}", Some(BRIDGE)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{retired} is retired");
    }
}
