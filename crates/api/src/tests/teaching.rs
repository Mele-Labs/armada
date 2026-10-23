//! Teaching's three routes over the router: each is served at its path and answers its wire type, and
//! a body or reach the wire does not hold is a 400 the daemon never sees.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{SavedTeaching, TeachingProposal, WordRules};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn each_route_is_served_and_answers_its_wire_type() {
    let app = wired();

    let read = r#"{"words":"STORE-1234","message":"See STORE-1234.","said":"A Jira key","reach":"everywhere"}"#;
    let (status, body) = call(&app, "POST", "/teachings/read", read).await;
    assert_eq!(status, StatusCode::OK);
    let proposal: TeachingProposal = ipc::decode("a proposal", &body).expect("decodes");
    assert_eq!(proposal.words.expect("words").example, "STORE-1234");

    let pressed = r#"{"said":"Warmer","reach":"everywhere","source":{"kind":"title_bar","text":"The title bar"}}"#;
    assert_eq!(call(&app, "POST", "/teachings/read", pressed).await.0, StatusCode::OK, "words may be absent for Bridge's own interface");

    let save = r#"{"words":{"name":"Key","pattern":"STORE-\\d+","example":"STORE-1234"},"note":{"section":"repo","kind":"convention","text":"Use it."},"reach":"repository","repository":"/work/shop","session_id":"s1","cite":"m3","source":{"kind":"words","text":"STORE-1234"}}"#;
    let (status, body) = call(&app, "POST", "/teachings", save).await;
    assert_eq!(status, StatusCode::OK);
    let saved: SavedTeaching = ipc::decode("a saved teaching", &body).expect("decodes");
    assert_eq!((saved.entry.as_deref(), saved.mods.as_slice()), (Some("7"), ["key".to_string()].as_slice()));

    let (status, body) = call(&app, "GET", "/teachings/words?repository=/work/shop&session_id=s1", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(ipc::decode::<WordRules>("the rules", &body).expect("decodes").rules.is_empty());
}

#[tokio::test]
async fn a_bad_reach_or_body_is_a_400() {
    let app = wired();
    let reach = r#"{"words":"a","message":"b","said":"c","reach":"nowhere"}"#;
    assert_eq!(call(&app, "POST", "/teachings/read", reach).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app, "POST", "/teachings/read", "{}").await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app, "POST", "/teachings", r#"{"words":{"pattern":"a"},"reach":"everywhere"}"#).await.0, StatusCode::BAD_REQUEST);
    let kind = r#"{"said":"c","reach":"everywhere","source":{"kind":"sidebar","text":"x"}}"#;
    assert_eq!(call(&app, "POST", "/teachings/read", kind).await.0, StatusCode::BAD_REQUEST);
}
