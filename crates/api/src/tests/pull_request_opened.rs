//! A pull request's page read by Bridge is him opening it; the same read by a Job is not, since a Job
//! reading the pull request it reviews says nothing of whether he has.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

/// The port a Job calls from, and one no Job holds, as Bridge's is.
const JOB: u16 = 53000;
const BRIDGE: u16 = 53001;

async fn read(app: &Router, number: u64, port: u16) -> StatusCode {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let request = Request::builder()
        .method("GET")
        .uri(format!("/pull_request_details/harbour/{number}"))
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::empty())
        .expect("a well-formed request");
    app.clone().oneshot(request).await.expect("an answer").status()
}

#[tokio::test]
async fn a_page_bridge_reads_is_him_opening_the_pull_request_and_a_jobs_read_is_not() {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    *daemon.session_on.lock().expect("not poisoned") = Some(JOB);
    let opened = Arc::clone(&daemon.opened);
    let app = router(Served::by(daemon, run_id(), events));

    assert_eq!(read(&app, 41, JOB).await, StatusCode::OK, "a Job still reads the page");
    assert_eq!(read(&app, 42, BRIDGE).await, StatusCode::OK);
    assert_eq!(*opened.lock().expect("not poisoned"), [("harbour".to_string(), 42)]);
}
