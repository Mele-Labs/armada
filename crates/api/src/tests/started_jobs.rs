//! The two routes a Job reads and commands the Jobs it started by: the body must decode, the caller is
//! the connection and never the body, and a refused entry stands alone in an answer of 200.
//! Fleet decides what is a Job's own; the daemon here is a fake. `docs/concepts/session/a-job-that-started-jobs.md`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

/// `method` of `uri` with `body`, from a hosted Job's connection where `placed`, and from none where not.
async fn sent(method: &str, uri: &str, body: &str, placed: bool) -> (StatusCode, String) {
    let events = Broadcaster::new();
    let app = router(Served::by(FakeDaemon::new(events.clone()), run_id(), events));
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let caller = placed.then(|| crate::Caller::at("127.0.0.1:50999".parse().expect("an address")));
    let response = crate::acting::asked_by(caller, app.oneshot(request)).await.expect("an answer");
    let status = response.status();
    let said = response.into_body().collect().await.expect("a body").to_bytes();
    (status, String::from_utf8_lossy(&said).into_owned())
}

#[tokio::test]
async fn a_placed_job_reads_its_children_and_a_call_from_none_is_refused() {
    let (status, said) = sent("GET", "/sessions/started", "", true).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let started: ipc::StartedSessions = ipc::decode("started", said.as_bytes()).expect("StartedSessions");
    assert_eq!(started.jobs.len(), 1);
    assert_eq!(started.jobs[0].summary.id.as_str(), "01KID");

    let (status, said) = sent("GET", "/sessions/started", "", false).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{said}");
    let refused: ipc::WireError = ipc::decode("a refusal", said.as_bytes()).expect("a WireError");
    assert_eq!(refused.code, ipc::STARTED_UNPLACED);
}

#[tokio::test]
async fn commands_are_answered_in_order_and_a_refused_one_stands_alone() {
    let body = r#"{"commands":[{"session_id":"01KID","act":"withdraw"},
                              {"session_id":"01OTHERS","act":"end","why":"x"},
                              {"session_id":"01KID","act":"pause"}]}"#;
    let (status, said) = sent("POST", "/sessions/command_started", body, true).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    let answers: ipc::StartedAnswers = ipc::decode("answers", said.as_bytes()).expect("StartedAnswers");
    let [one, two, three] = &answers.commands[..] else { panic!("three answers: {answers:?}") };
    assert!(one.done.is_some() && one.refused.is_none());
    assert_eq!(two.refused.as_ref().map(|refused| refused.code.as_str()), Some("fleet.started_not_yours"));
    assert!(two.done.is_none());
    assert!(three.done.is_some(), "the one after a refusal is still done");
}

#[tokio::test]
async fn a_body_that_names_no_command_or_an_unknown_act_is_the_transports_400() {
    for body in [r#"{"commands":[]}"#, r#"{"commands":[{"session_id":"01KID","act":"destroy"}]}"#, r#"{}"#] {
        let (status, said) = sent("POST", "/sessions/command_started", body, true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {said}");
    }
}

#[tokio::test]
async fn commands_from_no_hosted_job_are_refused_whole() {
    let body = r#"{"commands":[{"session_id":"01KID","act":"pause"}]}"#;
    let (status, said) = sent("POST", "/sessions/command_started", body, false).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{said}");
    let refused: ipc::WireError = ipc::decode("a refusal", said.as_bytes()).expect("one WireError, not StartedAnswers");
    assert_eq!(refused.code, ipc::STARTED_UNPLACED);
}
