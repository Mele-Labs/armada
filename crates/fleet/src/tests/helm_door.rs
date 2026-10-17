//! The agent door in front of a real Fleet, with a Helm session placed by its
//! connection: what it is listed, and that no other caller is listed the same. `#941`.
//!
//! **The placement is planted, and the rule is not.** Which process holds a
//! socket is `crate::tests::peer`'s, against the kernel; here `fleet::helm::may`
//! and the door meet over the real router. The session is a hosted Session that is Helm's
//! (`helm_session`), running as a stand-in process.

use std::net::SocketAddr;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use http_body_util::BodyExt;
use ipc::RunId;
use testkit::FakeWorkProduct;
use tower::ServiceExt;

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::helm_session::with_helm_running;
use crate::tests::peer::Placing;
use crate::tests::tmp::TempDir;

/// The pid the Helm Session runs as, and the port its relay calls from.
const SESSION: u32 = 4242;
const RELAY: u16 = 52000;
/// A port a person's own agent session calls from.
const SOMEONE: u16 = 52001;

/// A Fleet hosting one Helm session whose relay holds [`RELAY`], behind the router.
async fn served(home: &TempDir) -> Router {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    let peers = Placing::nothing();
    peers.holding(SESSION + 1, RELAY, fittings.host.port);
    peers.started(SESSION, SESSION + 1);
    peers.holding(9999, SOMEONE, fittings.host.port);
    fittings.peers = peers;
    let fleet = with_helm_running(Fleet::assembled(fittings), SESSION).await;
    let events = fleet.events();
    api::router(api::Served::sharing(fleet, RunId::carried("01RUN"), events))
}

async fn from(app: &Router, port: u16, body: &str) -> String {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let request = Request::builder()
        .method("POST")
        .uri(api::DOOR_PATH)
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


const LISTING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;

#[tokio::test]
async fn a_helm_session_is_listed_its_acts() {
    let home = TempDir::new();
    let app = served(&home).await;

    let listed = from(&app, RELAY, LISTING).await;
    assert!(listed.contains("\"name\":\"tell_session\""), "{listed}");
}

/// **Only the session is narrowed.** A person's own agent session keeps what
/// it had, and is not offered what is Helm's alone by calling from beside a Helm session.
#[tokio::test]
async fn any_other_caller_is_answered_as_it_was() {
    let home = TempDir::new();
    let app = served(&home).await;
    let listed = from(&app, SOMEONE, LISTING).await;
    assert!(!listed.contains("\"name\":\"tell_session\""), "{listed}");
}
