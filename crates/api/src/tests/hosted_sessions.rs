//! `send_session_message` over the router: a message carrying a screenshot is
//! past axum's 2 MB default, and reaches the daemon rather than a 413.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

#[tokio::test]
async fn a_message_past_two_megabytes_reaches_the_daemon() {
    let events = Broadcaster::new();
    let app = router(Served::by(FakeDaemon::new(events.clone()), run_id(), events));
    let body = format!(
        r#"{{"session_id":"c279e883-d316-4202-8134-76ea8b531835","text":"{}"}}"#,
        "a".repeat(5 * 1024 * 1024)
    );
    let request = Request::builder()
        .method("POST")
        .uri("/sessions/message")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .expect("a well-formed request");
    let response = app.oneshot(request).await.expect("an answer");
    assert_ne!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let said = response.into_body().collect().await.expect("a body").to_bytes();
    assert!(
        String::from_utf8_lossy(&said).contains("fake.no_hosted_sessions"),
        "the fake daemon is what answered"
    );
}

/// The route is hand-written, so a typo in it is a runtime 500: the query must reach the daemon
/// whole, a path with a space and a `&` in it among the rest.
#[tokio::test]
async fn a_clash_file_is_asked_over_the_router_and_answers_both_patches() {
    let events = Broadcaster::new();
    let app = router(Served::by(FakeDaemon::new(events.clone()), run_id(), events));
    let ask = |query: &str| {
        Request::builder().method("GET").uri(format!("/clashes/file?{query}")).body(Body::empty()).expect("a well-formed request")
    };
    let response = app
        .clone()
        .oneshot(ask("session_id=s-1&other_kind=main&other_id=main&path=src%2Fa%20%26%20b.rs"))
        .await
        .expect("an answer");
    assert_eq!(response.status(), StatusCode::OK);
    let said = response.into_body().collect().await.expect("a body").to_bytes();
    let said = String::from_utf8_lossy(&said);
    assert!(said.contains("\"ours\"") && said.contains("\"theirs\""), "{said}");

    let unknown_kind = app.oneshot(ask("session_id=s-1&other_kind=job&other_id=j&path=a")).await.expect("an answer");
    assert_eq!(unknown_kind.status(), StatusCode::BAD_REQUEST, "a kind that is not a ClashSide");
}

/// The route is hand-written: the query, `seen` optional, must reach the daemon.
#[tokio::test]
async fn a_jobs_file_change_is_asked_over_the_router() {
    let events = Broadcaster::new();
    let app = router(Served::by(FakeDaemon::new(events.clone()), run_id(), events));
    let ask = |query: &str| {
        Request::builder().method("GET").uri(format!("/sessions/file_change?{query}")).body(Body::empty()).expect("a well-formed request")
    };
    for query in ["session_id=s-1&path=src%2Fa%20%26%20b.rs", "session_id=s-1&path=a.rs&seen=0123abcd"] {
        let response = app.clone().oneshot(ask(query)).await.expect("an answer");
        assert_eq!(response.status(), StatusCode::OK, "{query}");
        let said = response.into_body().collect().await.expect("a body").to_bytes();
        let said = String::from_utf8_lossy(&said);
        assert!(said.contains("\"stamp\"") && said.contains("\"diff\""), "{said}");
    }
    let no_path = app.oneshot(ask("session_id=s-1")).await.expect("an answer");
    assert_eq!(no_path.status(), StatusCode::BAD_REQUEST, "path is asked for");
}

/// `SendSessionMessageBody` is flat on the wire: `aim` sits beside the message's own fields, and the
/// route hands both to the daemon. Held here, at the route, because the decode and the hand-over are
/// what Fleet's own tests do not reach.
async fn post_message(body: &str) -> (StatusCode, Vec<(ipc::SendSessionMessage, Option<ipc::ModAim>)>) {
    let events = Broadcaster::new();
    let daemon = std::sync::Arc::new(FakeDaemon::new(events.clone()));
    let app = router(Served::sharing(daemon.clone(), run_id(), events));
    let request = Request::builder()
        .method("POST")
        .uri("/sessions/message")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.oneshot(request).await.expect("an answer");
    let seen = daemon.messages_aimed.lock().expect("messages aimed").clone();
    (response.status(), seen)
}

const SESSION: &str = "c279e883-d316-4202-8134-76ea8b531835";

#[tokio::test]
async fn a_flat_message_with_an_aim_reaches_the_daemon_with_both() {
    let (status, seen) = post_message(&format!(r#"{{"session_id":"{SESSION}","text":"a darker theme","aim":"theme"}}"#)).await;
    assert_ne!(status, StatusCode::BAD_REQUEST, "a flat body with an aim decodes");
    assert_eq!(seen.len(), 1, "the daemon was handed one message");
    assert_eq!(seen[0].0.text, "a darker theme");
    assert_eq!(seen[0].0.session_id.as_str(), SESSION);
    assert_eq!(seen[0].1, Some(ipc::ModAim::Theme), "the aim travels beside the message");
}

#[tokio::test]
async fn a_flat_message_without_an_aim_reaches_the_daemon_with_none() {
    let (status, seen) = post_message(&format!(r#"{{"session_id":"{SESSION}","text":"hello"}}"#)).await;
    assert_ne!(status, StatusCode::BAD_REQUEST, "aim is optional");
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0.text, "hello");
    assert_eq!(seen[0].1, None);
}

#[tokio::test]
async fn an_aim_that_is_not_a_mod_kind_is_refused_before_the_daemon() {
    let (status, seen) = post_message(&format!(r#"{{"session_id":"{SESSION}","text":"hello","aim":"sparkle"}}"#)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "an unknown aim is undecodable");
    assert!(seen.is_empty(), "nothing reached the daemon");
}

/// The route is hand-written: the body must decode and reach the daemon, whose answer is the route's.
#[tokio::test]
async fn naming_a_job_reaches_the_daemon_and_a_body_without_a_job_does_not() {
    let events = Broadcaster::new();
    let app = router(Served::by(FakeDaemon::new(events.clone()), run_id(), events));
    let post = |body: &str| {
        Request::builder()
            .method("POST")
            .uri("/sessions/name")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("a well-formed request")
    };
    let response = app.clone().oneshot(post(r#"{"session_id":"s-1"}"#)).await.expect("an answer");
    let status = response.status();
    let said = response.into_body().collect().await.expect("a body").to_bytes();
    let said = String::from_utf8_lossy(&said);
    // The fake asks no proposer and refuses as Fleet does without one, which only a routed request can.
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{said}");
    assert!(said.contains("fleet.session_not_named"), "the daemon's refusal: {said}");

    let response = app.oneshot(post(r#"{}"#)).await.expect("an answer");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST, "a body with no job is the transport's 400");
}

/// `body` sent to the route, from a hosted Job's connection where `placed`, and from none where not.
async fn start_another(body: &str, placed: bool) -> (StatusCode, String) {
    let events = Broadcaster::new();
    let app = router(Served::by(FakeDaemon::new(events.clone()), run_id(), events));
    let request = Request::builder()
        .method("POST")
        .uri("/sessions/start_another")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let caller = placed.then(|| crate::Caller::at("127.0.0.1:50999".parse().expect("an address")));
    let response = crate::acting::asked_by(caller, app.oneshot(request)).await.expect("an answer");
    let status = response.status();
    let said = response.into_body().collect().await.expect("a body").to_bytes();
    (status, String::from_utf8_lossy(&said).into_owned())
}

/// One call with `jobs` answers each Job in the order given, and a refused one stands alone.
#[tokio::test]
async fn several_jobs_started_in_one_call_answer_in_order_and_a_refused_one_alone() {
    let (status, said) = start_another(
        r#"{"jobs":[{"manifest_id":"armada","title":"one","message":"m"},
                    {"manifest_id":"armada","title":" ","message":"m"},
                    {"manifest_id":"armada","title":"three","message":"m"}]}"#,
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let held: ipc::SessionsHeld = ipc::decode("several held", said.as_bytes()).expect("SessionsHeld");
    let ids: Vec<Option<String>> =
        held.jobs.iter().map(|job| job.held.as_ref().map(|held| held.session_id.as_str().to_string())).collect();
    assert_eq!(ids, vec![Some("session-one".into()), None, Some("session-three".into())]);
    assert_eq!(held.jobs[1].refused.as_ref().map(|refused| refused.code.as_str()), Some("fleet.agent_start_empty"));
}

/// The single form answers `SessionHeld` as it always did, and the two forms together are refused.
#[tokio::test]
async fn one_job_still_answers_held_and_one_job_beside_jobs_is_refused() {
    let (status, said) = start_another(r#"{"manifest_id":"armada","title":"one","message":"m"}"#, true).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let held: ipc::SessionHeld = ipc::decode("one held", said.as_bytes()).expect("SessionHeld");
    assert_eq!(held.title, "one");

    let (status, said) = start_another(r#"{"manifest_id":"armada","title":"one"}"#, true).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(said.contains("missing field `message`") && said.contains("jobs?: ["), "{said}");

    let (status, said) =
        start_another(r#"{"title":"one","jobs":[{"manifest_id":"armada","title":"two","message":"m"}]}"#, true).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");
}

/// A batch from no hosted Job is refused whole, with the one 422 a one-Job call from it gets.
#[tokio::test]
async fn a_batch_from_no_hosted_job_is_refused_as_one_job_is() {
    let (one_status, one) = start_another(r#"{"manifest_id":"armada","title":"one","message":"m"}"#, false).await;
    let (status, said) = start_another(
        r#"{"jobs":[{"manifest_id":"armada","title":"one","message":"m"},
                    {"manifest_id":"armada","title":"two","message":"m"}]}"#,
        false,
    )
    .await;
    assert_eq!((one_status, status), (StatusCode::UNPROCESSABLE_ENTITY, StatusCode::UNPROCESSABLE_ENTITY), "{said}");
    let refused: ipc::WireError = ipc::decode("a refusal", said.as_bytes()).expect("one WireError, not SessionsHeld");
    let alone: ipc::WireError = ipc::decode("a refusal", one.as_bytes()).expect("a WireError");
    assert_eq!((refused.code.as_str(), alone.code.as_str()), (ipc::AGENT_START_UNPLACED, ipc::AGENT_START_UNPLACED));
}
