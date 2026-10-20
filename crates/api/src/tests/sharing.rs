//! Share's five routes over the router: each exists, a body reaches the daemon whole, one with a
//! key nobody knows is a 400, and an unserved repository is a 404. `crate::sharing`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{Landed, PackageAdded, PackageRead, PieceAdded, SharedPackage, UnboundPieces};
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
async fn a_package_is_made_from_the_pieces_chosen() {
    let app = wired();
    let sent = r#"{"pieces":[{"repository":"/work/shop","piece":{"kind":"flight_controls","job":["specs"]}},{"repository":"/work/shop","piece":{"kind":"review_tool","id":"security"}}]}"#;
    let (status, body) = call(&app, "POST", "/share/package", sent).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let made: SharedPackage = ipc::decode("a package", &body).expect("decodes");
    assert_eq!(made.file_name, "specs.armada.toml");
    assert!(made.text.contains("2 pieces"), "both pieces reached the daemon: {}", made.text);

    let (status, _) = call(&app, "POST", "/share/package", r#"{"pieces":[],"path":"/tmp"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a key nobody knows");
    let elsewhere = r#"{"pieces":[{"repository":"/not/served","piece":{"kind":"review_tool","id":"x"}}]}"#;
    let (status, body) = call(&app, "POST", "/share/package", elsewhere).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("fleet.run_memory_repository_not_served"));
}

#[tokio::test]
async fn a_package_is_read_and_added_with_its_text_whole() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/share/read", r#"{"text":"version = 1\n"}"#).await;
    assert_eq!(status, StatusCode::OK);
    let read: PackageRead = ipc::decode("a package read", &body).expect("decodes");
    assert_eq!(read.pieces[0].id, "version = 1\n");
    let wire = String::from_utf8_lossy(&body);
    assert!(wire.contains(r#""kind":"tile""#) && wire.contains(r#""list":"job""#) && !wire.contains("null"), "optional fields are left out: {wire}");

    let (status, body) = call(&app, "POST", "/share/add", r#"{"text":"the text","replace":true}"#).await;
    assert_eq!(status, StatusCode::OK);
    let added: PackageAdded = ipc::decode("added", &body).expect("decodes");
    assert_eq!((added.pieces[0].name.as_str(), added.pieces[0].id.as_str(), added.pieces[0].landed), ("the text", "true", Landed::Unbound));

    let (status, _) = call(&app, "POST", "/share/add", r#"{"text":"x","replce":true}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unbound_pieces_are_listed_and_bound_to_a_repository_fleet_serves() {
    let app = wired();
    let (status, body) = call(&app, "GET", "/share/unbound", "").await;
    assert_eq!(status, StatusCode::OK);
    let listed: UnboundPieces = ipc::decode("unbound", &body).expect("decodes");
    assert!(listed.pieces.is_empty());

    let (status, body) = call(&app, "POST", "/share/bind", r#"{"unbound_id":"0123456789abcdef","repository":"/work/shop"}"#).await;
    assert_eq!(status, StatusCode::OK);
    let bound: PieceAdded = ipc::decode("bound", &body).expect("decodes");
    assert_eq!((bound.landed, bound.repository.as_deref()), (Landed::Added, Some("/work/shop")));

    let (status, body) = call(&app, "POST", "/share/bind", r#"{"unbound_id":"0123456789abcdef","repository":"/not/served"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("fleet.run_memory_repository_not_served"));
}
