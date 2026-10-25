//! An adversarial review of a plan, over the routes: a start that parses and a start that is refused, and
//! the two acts on a finding. What a review reads and settles is Fleet's (`fleet::tests`); this holds the
//! route, the body and the refusal codes.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{
    AdversarialFinding, AdversarialReview, AdversarialReviewState, FindingAnchor, FindingHow, Instant, ReviewSubject,
    SessionFact, SessionId, SessionReport, WireError,
};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, Sessions};

const PLAN_VERSION: u32 = 2;

fn at() -> Instant {
    Instant::carried("2026-10-25T09:00:00.000Z")
}

fn subject(version: u32) -> ReviewSubject {
    ReviewSubject::Plan { session_id: SessionId::carried("session-1"), version }
}

fn finding(id: &str, about: FindingAnchor) -> AdversarialFinding {
    AdversarialFinding {
        id: id.into(),
        about,
        consequence: "A rotation can run twice".into(),
        expected: "One rotation a day".into(),
        found: "The task has no guard".into(),
        cites: vec!["crates/store/src/lib.rs:12".into()],
        settled: None,
    }
}

/// A Job whose pending plan, at version two, has a review kept: two findings, none settled.
async fn holding_a_review() -> Router {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    let mut row = daemon
        .report_session(SessionReport {
            harness: String::new(),
            session_id: SessionId::carried("session-1"),
            fact: SessionFact::TurnCompleted { said: None },
        })
        .await
        .expect("a row");
    row.plan = Some(ipc::SessionPlan {
        version: PLAN_VERSION,
        summary: String::new(),
        recorded_at: at(),
        approved: false,
        approved_at: None,
        set_aside_at: None,
        scope: Vec::new(),
        groups: Vec::new(),
        changes: None,
        sketch: None,
        held_by: Vec::new(),
        adversarial: Some(AdversarialReview {
            subject: subject(PLAN_VERSION),
            state: AdversarialReviewState::Done,
            started_at: at(),
            finished_at: Some(at()),
            failed: None,
            findings: vec![finding("f1", FindingAnchor::Plan), finding("f2", FindingAnchor::Task { id: "t1".into() })],
            cost_micros: Some(180_000),
        }),
    });
    *daemon.session_row.lock().expect("session row") = Some(row);
    router(Served::sharing(daemon, run_id(), events))
}

async fn post(app: &Router, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    let status = response.status();
    let body = response.into_body().collect().await.expect("a body").to_bytes().to_vec();
    (status, body)
}

fn start_body(version: u32, again: bool) -> String {
    format!(r#"{{"subject":{{"kind":"plan","session_id":"session-1","version":{version}}},"again":{again}}}"#)
}

fn finding_body(version: u32, id: &str) -> String {
    format!(r#"{{"subject":{{"kind":"plan","session_id":"session-1","version":{version}}},"finding_id":"{id}"}}"#)
}

fn review(body: &[u8]) -> AdversarialReview {
    ipc::decode("an adversarial review", body).unwrap_or_else(|why| panic!("{why}: {}", String::from_utf8_lossy(body)))
}

fn code(body: &[u8]) -> String {
    ipc::decode::<WireError>("a wire error", body).expect("an error body").code
}

#[tokio::test]
async fn a_start_that_parses_answers_the_kept_review_unless_again() {
    let app = holding_a_review().await;
    let (status, body) = post(&app, "/adversarial_review", &start_body(PLAN_VERSION, false)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let kept = review(&body);
    assert_eq!((kept.state, kept.findings.len()), (AdversarialReviewState::Done, 2), "the kept one, no call made");

    let (status, body) = post(&app, "/adversarial_review", &start_body(PLAN_VERSION, true)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let again = review(&body);
    assert_eq!((again.state, again.findings.len()), (AdversarialReviewState::Running, 0), "again starts a call");
    assert_eq!(again.subject, subject(PLAN_VERSION));
}

#[tokio::test]
async fn a_start_leaving_again_out_is_not_again() {
    let app = holding_a_review().await;
    let body = r#"{"subject":{"kind":"plan","session_id":"session-1","version":2}}"#;
    let (status, body) = post(&app, "/adversarial_review", body).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(review(&body).state, AdversarialReviewState::Done);
}

#[tokio::test]
async fn a_start_is_refused_for_a_job_nobody_has_and_for_a_plan_recorded_again() {
    let app = holding_a_review().await;
    let nobody = r#"{"subject":{"kind":"plan","session_id":"session-9","version":2},"again":false}"#;
    let (status, body) = post(&app, "/adversarial_review", nobody).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.review_subject_unknown"));

    let (status, body) = post(&app, "/adversarial_review", &start_body(PLAN_VERSION + 1, false)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::CONFLICT, "fleet.review_subject_stale"));
}

#[tokio::test]
async fn a_body_that_is_not_a_subject_is_refused_before_the_daemon_hears_it() {
    let app = holding_a_review().await;
    for route in ["/adversarial_review", "/adversarial_review/send", "/adversarial_review/dismiss"] {
        let (status, body) = post(&app, route, r#"{"subject":{"kind":"pull_request","number":7},"finding_id":"f1"}"#).await;
        assert_eq!((status, code(&body).as_str()), (StatusCode::BAD_REQUEST, "api.undecodable_request"), "{route}");
    }
}

#[tokio::test]
async fn a_finding_is_sent_or_dismissed_and_one_already_settled_is_answered_as_it_stands() {
    let app = holding_a_review().await;
    let (status, body) = post(&app, "/adversarial_review/send", &finding_body(PLAN_VERSION, "f1")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let sent = review(&body);
    assert_eq!(sent.findings[0].settled.as_ref().map(|settled| settled.how), Some(FindingHow::Sent));
    assert!(sent.findings[1].settled.is_none(), "the other waits");

    let (status, body) = post(&app, "/adversarial_review/dismiss", &finding_body(PLAN_VERSION, "f2")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(review(&body).findings[1].settled.as_ref().map(|settled| settled.how), Some(FindingHow::Dismissed));

    let (status, body) = post(&app, "/adversarial_review/dismiss", &finding_body(PLAN_VERSION, "f1")).await;
    assert_eq!(status, StatusCode::OK, "settled is answered, not refused: {}", String::from_utf8_lossy(&body));
    assert_eq!(review(&body).findings[0].settled.as_ref().map(|settled| settled.how), Some(FindingHow::Sent), "it stands as it was");
}

#[tokio::test]
async fn a_finding_act_is_refused_for_an_unknown_finding_job_or_plan_version() {
    let app = holding_a_review().await;
    for route in ["/adversarial_review/send", "/adversarial_review/dismiss"] {
        let (status, body) = post(&app, route, &finding_body(PLAN_VERSION, "f9")).await;
        assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.finding_unknown"), "{route}");

        let (status, body) = post(&app, route, &finding_body(PLAN_VERSION + 1, "f1")).await;
        assert_eq!((status, code(&body).as_str()), (StatusCode::CONFLICT, "fleet.review_subject_stale"), "{route}");

        let nobody = r#"{"subject":{"kind":"plan","session_id":"session-9","version":2},"finding_id":"f1"}"#;
        let (status, body) = post(&app, route, nobody).await;
        assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.review_subject_unknown"), "{route}");
    }
}
