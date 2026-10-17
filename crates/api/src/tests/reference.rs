//! The `:job_id` segment, and the three things it may be.
//!
//! **The defect these are about is a person's, not a program's.** The id Bridge
//! shows beside a Job is its handle, and every route under `/jobs/:job_id` took
//! a ULID — so the one id somebody can read, say and paste was the one id
//! nothing accepted.
//!
//! What is proven here is the transport half: the segment reaches the daemon as
//! text, whatever form it is in, and what a handler is handed is the Job. Which
//! Job a handle names is `store`'s, and `store::tests::resolving` proves it.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

/// The router over a fake holding one Job — `shapes::job_at` gives it the
/// handle `1-a-job`, so all three forms have something to name.
async fn holding_a_job() -> (Router, String) {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    crate::tests::fake::running(&daemon, "01JOB");
    let app = router(Served::by(daemon, run_id(), events));
    (app, "01JOB".to_string())
}

async fn status(app: &Router, uri: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers")
        .status()
}

// **The whole of it.** An agent handed the handle out of Bridge can fetch the
// Job over the API, without being told the ULID.
#[tokio::test]
async fn a_route_takes_the_handle_the_number_and_the_id() {
    let (app, id) = holding_a_job().await;

    for named in ["1-a-job", "1", id.as_str()] {
        assert_eq!(
            status(&app, &format!("/jobs/{named}")).await,
            StatusCode::OK,
            "`{named}` names this Job"
        );
    }
}

// The same three on a route that is not `get_job`, because resolution is an
// extractor rather than a line in one handler.
#[tokio::test]
async fn the_other_routes_take_them_too() {
    let (app, id) = holding_a_job().await;

    for named in ["1-a-job", "1", id.as_str()] {
        for route in ["events"] {
            assert_eq!(
                status(&app, &format!("/jobs/{named}/{route}")).await,
                StatusCode::OK,
                "`{named}` on `{route}`"
            );
        }
    }
}

// A route carrying a second segment extracts both without either knowing the
// other is there.
#[tokio::test]
async fn a_second_segment_is_read_beside_the_job() {
    let (app, _) = holding_a_job().await;

    assert_eq!(
        status(
            &app,
            "/jobs/1-a-job/events"
        )
        .await,
        StatusCode::OK
    );
}

// **A 404 and not a 400.** Text that could be a reference and names no Job is
// the daemon saying no Job is that, which is what it has always said about a
// ULID it does not hold.
#[tokio::test]
async fn text_that_names_no_job_is_the_404_it_always_was() {
    let (app, _) = holding_a_job().await;

    for named in ["4098", "9-some-other-repositorys-job", "01NOSUCHJOB"] {
        assert_eq!(
            status(&app, &format!("/jobs/{named}")).await,
            StatusCode::NOT_FOUND,
            "`{named}`"
        );
    }
}
