//! The two retro reads over the router, and **which door a request came
//! through**, named before any handler runs. `docs/concepts/retro.md`.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::door::Reachable;
use tower::ServiceExt;

use crate::tests::fake::{running, FakeDaemon};
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, BRIDGE, CALLER_HEADER, DOOR_PATH};

const HELM: u16 = 52100;
const ANYONE: u16 = 52101;

fn reading(_: &Reachable) -> bool {
    true
}

fn a_fleet() -> (Arc<FakeDaemon>, Router) {
    let daemon = FakeDaemon::new(Broadcaster::new());
    *daemon.helm_on.lock().expect("not poisoned") = Some((HELM, reading));
    let daemon = Arc::new(daemon);
    let app = router(Served::sharing(
        Arc::clone(&daemon),
        run_id(),
        Broadcaster::new(),
    ));
    (daemon, app)
}

async fn sent(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    (status, String::from_utf8_lossy(&body).to_string())
}

fn get(uri: &str, header: Option<&str>) -> Request<Body> {
    let mut request = Request::builder().method("GET").uri(uri);
    if let Some(said) = header {
        request = request.header(CALLER_HEADER, said);
    }
    request.body(Body::empty()).expect("a well-formed request")
}

fn through_the_door(port: u16, tool: &str) -> Request<Body> {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    Request::builder()
        .method("POST")
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(format!(
            r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"{tool}","arguments":{{}}}}}}"#
        )))
        .expect("a well-formed request")
}

/// **Bridge names itself; anything else is not Bridge.** A request carrying
/// the header is Bridge's, one carrying nothing is a bare HTTP call, and the
/// door's two callers are told apart by who Fleet placed — never by bytes a
/// caller sent.
#[tokio::test]
async fn a_request_is_named_by_the_door_it_came_through() {
    let (daemon, app) = a_fleet();

    for request in [
        get("/lessons", Some(BRIDGE)),
        get("/lessons", None),
        get("/lessons", Some("something-else")),
        through_the_door(ANYONE, "list_lessons"),
        through_the_door(HELM, "list_lessons"),
    ] {
        let (status, body) = sent(&app, request).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    let named: Vec<Option<&str>> = daemon
        .read_via
        .lock()
        .expect("not poisoned")
        .iter()
        .map(|via| via.map(|via| via.as_wire()))
        .collect();
    assert_eq!(
        named,
        vec![
            Some("bridge"),
            Some("http"),
            Some("http"),
            Some("door"),
            Some("helm"),
        ]
    );
}

/// A Job's retro answers at its own route, and a Job nobody holds is a 404.
#[tokio::test]
async fn a_jobs_retro_is_read_at_its_own_route() {
    let (_, app) = a_fleet();
    let daemon_held = "01RETROJOB";
    let (status, _) = sent(&app, get(&format!("/jobs/{daemon_held}/retro"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (daemon, app) = a_fleet();
    running(&daemon, daemon_held);
    let (status, body) = sent(&app, get(&format!("/jobs/{daemon_held}/retro"), None)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let retro: ipc::JobRetro = ipc::decode("a retro", body.as_bytes()).expect("a retro");
    assert_eq!(retro.state, ipc::RetroState::Pending);
}
