//! A Judge over the routes: an ask that parses and one that is refused, and the three acts on a reading. What
//! a reading reads and settles is Fleet's (`fleet::tests::judge`); this holds the routes, the bodies and the
//! refusal codes.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{
    Instant, JudgeArtifact, JudgeKind, JudgeNote, JudgeNoteHow, JudgeReading, JudgeState, SessionFact, SessionId,
    SessionReport, SessionState, WireError,
};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, Sessions};

fn at() -> Instant {
    Instant::carried("2026-10-31T09:00:00.000Z")
}

fn artifact(kind: JudgeKind, id: &str) -> JudgeArtifact {
    JudgeArtifact { kind, id: id.into(), title: "Rotate the keys".into() }
}

fn note(id: &str, about: Option<&str>) -> JudgeNote {
    JudgeNote {
        id: id.into(),
        about: about.map(str::to_string),
        claim: "A rotation can run twice".into(),
        r#where: None,
        expected: Some("One rotation a day".into()),
        found: Some("The task has no guard".into()),
        impact: None,
        cites: vec!["crates/store/src/lib.rs:12".into()],
        settled: None,
    }
}

fn reading(id: &str, state: JudgeState, artifact: JudgeArtifact, notes: Vec<JudgeNote>) -> JudgeReading {
    JudgeReading {
        id: id.into(),
        artifact,
        prompt: String::new(),
        state,
        asked_at: at(),
        finished_at: (state != JudgeState::Reading).then(at),
        failed: None,
        read: false,
        notes,
        cost_micros: Some(180_000),
    }
}

/// A Job with two readings kept: the plan's, done with two notes, and a pull request's, still reading.
async fn holding_readings(state: SessionState) -> Router {
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
    row.state = state;
    row.judge_readings = vec![
        reading("r1", JudgeState::Done, artifact(JudgeKind::Plan, "2"), vec![note("n1", Some("t1")), note("n2", None)]),
        reading("r2", JudgeState::Reading, artifact(JudgeKind::PullRequest, "812"), Vec::new()),
    ];
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

fn ask_body(session: &str, kind: &str, id: &str, prompt: Option<&str>) -> String {
    let prompt = prompt.map(|line| format!(r#","prompt":"{line}""#)).unwrap_or_default();
    format!(r#"{{"session_id":"{session}","artifact":{{"kind":"{kind}","id":"{id}","title":"Rotate the keys"}}{prompt}}}"#)
}

fn note_body(session: &str, reading: &str, note: &str) -> String {
    format!(r#"{{"session_id":"{session}","reading":"{reading}","note":"{note}"}}"#)
}

fn read(body: &[u8]) -> JudgeReading {
    ipc::decode("a Judge reading", body).unwrap_or_else(|why| panic!("{why}: {}", String::from_utf8_lossy(body)))
}

fn code(body: &[u8]) -> String {
    ipc::decode::<WireError>("a wire error", body).expect("an error body").code
}

#[tokio::test]
async fn an_ask_answers_a_reading_at_once_and_a_prompt_is_optional() {
    let app = holding_readings(SessionState::Live).await;
    let (status, body) = post(&app, "/sessions/judge", &ask_body("session-1", "file", "crates/store/src/lib.rs", Some("Verify that it locks"))).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let asked = read(&body);
    assert_eq!((asked.state, asked.prompt.as_str()), (JudgeState::Reading, "Verify that it locks"));
    assert_eq!(asked.artifact.kind, JudgeKind::File);

    let (status, body) = post(&app, "/sessions/judge", &ask_body("session-1", "doc", "notes.html", None)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(read(&body).prompt, "", "no line asks what was missed");
}

#[tokio::test]
async fn an_ask_for_an_artifact_already_being_read_answers_that_reading() {
    let app = holding_readings(SessionState::Live).await;
    let (status, body) = post(&app, "/sessions/judge", &ask_body("session-1", "pull_request", "812", None)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(read(&body).id, "r2", "the reading going, not a second");

    let (status, body) = post(&app, "/sessions/judge", &ask_body("session-1", "plan", "2", None)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_ne!(read(&body).id, "r1", "a finished reading asked again is a fresh one");
}

#[tokio::test]
async fn an_ask_is_refused_for_a_job_nobody_has_and_for_one_that_ended() {
    let app = holding_readings(SessionState::Live).await;
    let (status, body) = post(&app, "/sessions/judge", &ask_body("session-9", "plan", "2", None)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.no_such_session"));

    let ended = holding_readings(SessionState::Ended).await;
    let (status, body) = post(&ended, "/sessions/judge", &ask_body("session-1", "plan", "2", None)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::CONFLICT, "fleet.session_closed"));
}

#[tokio::test]
async fn a_body_that_names_no_artifact_kind_is_refused_before_the_daemon_hears_it() {
    let app = holding_readings(SessionState::Live).await;
    let (status, body) = post(&app, "/sessions/judge", &ask_body("session-1", "pull-request", "812", None)).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::BAD_REQUEST, "api.undecodable_request"));
    for route in ["/sessions/judge/send", "/sessions/judge/dismiss", "/sessions/judge/read"] {
        let (status, body) = post(&app, route, r#"{"session_id":"session-1"}"#).await;
        assert_eq!((status, code(&body).as_str()), (StatusCode::BAD_REQUEST, "api.undecodable_request"), "{route}");
    }
}

#[tokio::test]
async fn a_note_is_sent_or_dismissed_and_one_already_settled_is_answered_as_it_stands() {
    let app = holding_readings(SessionState::Live).await;
    let (status, body) = post(&app, "/sessions/judge/send", &note_body("session-1", "r1", "n1")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let sent = read(&body);
    assert_eq!(sent.notes[0].settled.as_ref().map(|settled| settled.how), Some(JudgeNoteHow::Sent));
    assert!(sent.notes[1].settled.is_none(), "the other waits");

    let (status, body) = post(&app, "/sessions/judge/dismiss", &note_body("session-1", "r1", "n2")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(read(&body).notes[1].settled.as_ref().map(|settled| settled.how), Some(JudgeNoteHow::Dismissed));

    let (status, body) = post(&app, "/sessions/judge/dismiss", &note_body("session-1", "r1", "n1")).await;
    assert_eq!(status, StatusCode::OK, "settled is answered, not refused: {}", String::from_utf8_lossy(&body));
    assert_eq!(read(&body).notes[0].settled.as_ref().map(|settled| settled.how), Some(JudgeNoteHow::Sent), "it stands as it was");
}

#[tokio::test]
async fn a_note_act_is_refused_for_an_unknown_note_reading_or_job() {
    let app = holding_readings(SessionState::Live).await;
    for route in ["/sessions/judge/send", "/sessions/judge/dismiss"] {
        let (status, body) = post(&app, route, &note_body("session-1", "r1", "n9")).await;
        assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.no_such_judge_note"), "{route}");

        let (status, body) = post(&app, route, &note_body("session-1", "r9", "n1")).await;
        assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.no_such_judge_reading"), "{route}");

        let (status, body) = post(&app, route, &note_body("session-9", "r1", "n1")).await;
        assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.no_such_session"), "{route}");
    }
}

#[tokio::test]
async fn a_reading_is_marked_read() {
    let app = holding_readings(SessionState::Live).await;
    let (status, body) = post(&app, "/sessions/judge/read", r#"{"session_id":"session-1","reading":"r1"}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(read(&body).read);

    let (status, body) = post(&app, "/sessions/judge/read", r#"{"session_id":"session-1","reading":"r9"}"#).await;
    assert_eq!((status, code(&body).as_str()), (StatusCode::NOT_FOUND, "fleet.no_such_judge_reading"));
}
