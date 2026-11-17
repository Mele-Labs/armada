//! The routes of the feature library, over the router. The paths and the methods are hand-written, so they are what is
//! held here; what Fleet answers is `fleet::feature_library`'s, against a real store.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn app() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let said = response.into_body().collect().await.expect("a body").to_bytes();
    (status, String::from_utf8_lossy(&said).into_owned())
}

#[tokio::test]
async fn the_library_is_listed_for_the_project_named_in_the_query() {
    let (status, said) = call(&app(), "GET", "/features?project=p9", "").await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(said.starts_with(r#"{"project":"p9","areas":[{"slug":"takes""#), "{said}");
}

#[tokio::test]
async fn a_page_and_its_brief_are_read_by_the_feature_slug() {
    let (status, said) = call(&app(), "GET", "/features/page?feature=voting-on-takes", "").await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(said.contains(r#""slug":"voting-on-takes""#), "{said}");
    let (status, said) = call(&app(), "GET", "/features/brief?feature=voting-on-takes", "").await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(said.starts_with(r#"{"feature":"voting-on-takes","text""#), "{said}");
    let (status, _) = call(&app(), "GET", "/features/page", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a read of one feature names it");
}

#[tokio::test]
async fn the_features_of_a_path_are_asked_by_the_path() {
    let (status, said) = call(&app(), "GET", "/features/of_path?path=crates/a.rs", "").await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"path":"crates/a.rs","features":[]}"#);
}

#[tokio::test]
async fn sections_are_written_and_features_upserted_over_post_and_a_bad_section_name_is_undecodable() {
    let sent = r#"{"feature":"voting-on-takes","sections":[{"section":"test","text":"armada check test page_votes"}]}"#;
    let (status, said) = call(&app(), "POST", "/features/write_sections", sent).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"feature":"voting-on-takes","written":["test"]}"#);
    let (status, said) = call(&app(), "POST", "/features/write_sections", r#"{"feature":"x","sections":[{"section":"tests","text":"t"}]}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");

    let upsert = r#"{"project":"p1","areas":[{"slug":"takes","title":"Takes"}],"features":[{"slug":"voting-on-takes","area":"takes","title":"Voting on takes","summary":"One line."}]}"#;
    let (status, said) = call(&app(), "POST", "/features/upsert", upsert).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"created":["voting-on-takes"],"updated":[],"areas":1}"#);
}

#[tokio::test]
async fn the_weekly_figure_is_read_with_or_without_the_number_of_weeks() {
    let (status, said) = call(&app(), "GET", "/usage/first_edit_searches", "").await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"weeks":[],"unmeasured":0}"#);
    let (status, _) = call(&app(), "GET", "/usage/first_edit_searches?weeks=4", "").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn the_mods_hook_asks_for_the_pages_a_job_is_owed_over_post() {
    let (status, said) = call(&app(), "POST", "/features/hand_over", r#"{"session_id":"s-1","paths":["crates/a.rs"]}"#).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"features":["crates/a.rs"],"text":""}"#);
    let (status, _) = call(&app(), "POST", "/features/hand_over", r#"{"paths":["a"]}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "the Job is named");
}
