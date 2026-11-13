//! The branches Jobs offer for the preview, over the router: `offer_preview` and `withdraw_preview` are a
//! Job's acts and answer what Fleet kept, `get_preview_offers` is the read Bridge draws the Refresh row
//! from, and skipping it is the person's, refused from inside a Job.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{PreviewOffered, PreviewOffers, PreviewRefreshSkipped, PreviewState, PreviewWithdrawn};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::{self, run_id};
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
async fn an_offer_is_answered_with_the_branch_and_its_tip_while_it_is_checked() {
    let app = wired();

    let (status, body) = call(&app, "POST", "/sessions/offer_preview", r#"{"branch":"armada/work","about":"Keeps two builds"}"#).await;

    assert_eq!(status, StatusCode::OK);
    let offered: PreviewOffered = ipc::decode("answer", &body).expect("decodes");
    assert_eq!((offered.branch.as_str(), offered.state), ("armada/work", PreviewState::Checking));
    assert_eq!(offered.tip.len(), 40);
}

#[tokio::test]
async fn an_offer_naming_no_branch_is_accepted_for_fleet_to_name_the_jobs_own() {
    let app = wired();

    let (status, _) = call(&app, "POST", "/sessions/offer_preview", "{}").await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_withdrawal_is_answered_with_the_branch_taken_off() {
    let app = wired();

    let (status, body) = call(&app, "POST", "/sessions/withdraw_preview", r#"{"branch":"armada/work"}"#).await;

    assert_eq!(status, StatusCode::OK);
    let withdrawn: PreviewWithdrawn = ipc::decode("answer", &body).expect("decodes");
    assert_eq!(withdrawn.branch, "armada/work");
}

#[tokio::test]
async fn the_read_is_the_offers_and_refresh_the_daemon_gave() {
    let app = wired();

    let (status, body) = call(&app, "GET", "/fleet/build/offers", "").await;

    assert_eq!(status, StatusCode::OK);
    let read: PreviewOffers = ipc::decode("offers", &body).expect("decodes");
    assert_eq!(read, shapes::preview_offers());
}

#[tokio::test]
async fn skipping_the_refresh_row_answers_the_id_it_skipped() {
    let app = wired();

    let (status, body) = call(&app, "POST", "/fleet/build/offers/skip", r#"{"id":"p-1"}"#).await;

    assert_eq!(status, StatusCode::OK);
    let skipped: PreviewRefreshSkipped = ipc::decode("answer", &body).expect("decodes");
    assert_eq!(skipped.id, "p-1");
}

#[tokio::test]
async fn skipping_the_refresh_row_without_an_id_is_a_request_that_does_not_decode() {
    let app = wired();

    let (status, _) = call(&app, "POST", "/fleet/build/offers/skip", "{}").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}
