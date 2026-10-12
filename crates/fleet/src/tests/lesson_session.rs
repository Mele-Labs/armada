//! Starting a Session on a retro item, over the router: the item moves to
//! `agreed` with the Session on it, the first message is sent and not sent,
//! and the rules are `agree_lesson`'s. `docs/concepts/retro.md`, *Start Session*.

use api::{HostedSessions, Retros, Sessions};
use axum::http::StatusCode;
use axum::Router;
use ipc::{LessonSessionStarted, SessionList};
use testkit::FakeJudge;

use super::agreeing::{job_count, lesson, listed, retro_written};
use super::retro::{sent, From};
use super::session_retro::{rig, said};
use crate::tests::tmp::TempDir;

async fn started(app: &Router, id: &str) -> (StatusCode, Vec<u8>) {
    sent(app, "POST", &format!("/lessons/{id}/session"), "", From::Bridge).await
}

fn begun(body: &[u8]) -> LessonSessionStarted {
    ipc::decode("a started Session", body)
        .unwrap_or_else(|why| panic!("{why:?}: {}", String::from_utf8_lossy(body)))
}

async fn sessions(app: &Router) -> Vec<String> {
    let (status, body) = sent(app, "GET", "/sessions", "", From::Bridge).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    ipc::decode::<SessionList>("the sessions", &body)
        .expect("a SessionList")
        .sessions
        .into_iter()
        .map(|record| record.id.as_str().to_string())
        .collect()
}

fn refused(body: &[u8], code: &str) {
    assert!(
        String::from_utf8_lossy(body).contains(code),
        "{code}: {}",
        String::from_utf8_lossy(body)
    );
}

/// **A brief the Session refuses leaves the item agreed with the Session**,
/// named in the error so Bridge can open it. This Fleet runs no agent
/// process, so its first message is refused; a second press then answers 200
/// with that Session and sends nothing. The sent case is the Session-item test
/// below, which has a process to send to.
#[tokio::test]
async fn a_brief_the_session_refuses_leaves_the_item_agreed_with_the_session() {
    let home = TempDir::new();
    let (_fleet, app, judge, ids) = retro_written(&home).await;
    let jobs = job_count(&app).await;
    let before = sessions(&app).await;
    let asked = judge.asked().len();

    let (status, body) = started(&app, &ids[0]).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "fleet.lesson_session_brief_unsent");
    let now = sessions(&app).await;
    assert_eq!(now.len(), before.len() + 1, "the Session exists");
    let agreed = listed(&app, "?state=agreed").await;
    let [item] = agreed.as_slice() else { panic!("one agreed: {agreed:?}") };
    let session = item.session_proposed.clone().expect("the Session is on the item");
    assert!(now.contains(&session));
    assert!(String::from_utf8_lossy(&body).contains(&session), "the error names it");
    assert_eq!(item.job_proposed, None, "a Session and not a Job");
    assert_eq!(job_count(&app).await, jobs, "no Job");
    assert_eq!(judge.asked().len(), asked, "no model call");

    let (status, again) = started(&app, &ids[0]).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&again));
    let again = begun(&again);
    assert_eq!(again.session_id, session);
    for carried in [
        "Browser tests wait a fixed time",
        "desktop_test timed out after 15 seconds on three attempts.",
        "Raise the timeout in the desktop test config.",
        &format!("Job {}", again.lesson.handle),
    ] {
        assert!(again.brief.contains(carried), "{carried}: {}", again.brief);
    }
    assert!(!again.brief.contains('\u{2014}') && !again.brief.contains('\u{2013}'), "no dashes");
    assert_eq!(sessions(&app).await.len(), before.len() + 1, "one Session from two presses");
}

/// **A Kit item has nothing to work on in a repository**: refused, still open,
/// nothing started.
#[tokio::test]
async fn starting_on_a_kit_item_is_refused() {
    let home = TempDir::new();
    let (_fleet, app, _judge, ids) = retro_written(&home).await;
    let before = sessions(&app).await.len();

    let (status, body) = started(&app, &ids[1]).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "fleet.lesson_session_nothing_to_work_on");
    assert_eq!(sessions(&app).await.len(), before);
    let open: Vec<String> = listed(&app, "").await.into_iter().map(|l| l.id).collect();
    assert!(open.contains(&ids[1]), "still open");
}

/// **A fix in Armada is refused where Armada is not served**, with
/// `agree_lesson`'s code, and the item stays open.
#[tokio::test]
async fn starting_on_an_armada_item_is_refused_where_armada_is_not_served() {
    let home = TempDir::new();
    let (_fleet, app, _judge, ids) = retro_written(&home).await;
    let before = sessions(&app).await.len();

    let (status, body) = started(&app, &ids[2]).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "fleet.lesson_armada_not_served");
    assert_eq!(sessions(&app).await.len(), before);
    let open: Vec<String> = listed(&app, "").await.into_iter().map(|l| l.id).collect();
    assert!(open.contains(&ids[2]), "still open");
}

/// An id that names no item, and a malformed one, are 404s.
#[tokio::test]
async fn starting_on_an_id_that_names_no_item_is_not_found() {
    let home = TempDir::new();
    let (_fleet, app, _judge, ids) = retro_written(&home).await;
    let missing = format!("{}9", ids[0]);

    for id in [missing.as_str(), "nothing"] {
        let (status, body) = started(&app, id).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{id}");
        refused(&body, "fleet.no_such_lesson");
    }
}

/// **An item answered another way starts nothing** and says where it stands: a
/// 409 where no Session was started for it.
#[tokio::test]
async fn starting_on_an_item_already_answered_another_way_starts_nothing() {
    let home = TempDir::new();
    let (_fleet, app, _judge, ids) = retro_written(&home).await;
    sent(&app, "POST", &format!("/lessons/{}/agree", ids[0]), "", From::Bridge).await;
    sent(&app, "POST", &format!("/lessons/{}/disagree", ids[2]), "", From::Bridge).await;
    let before = sessions(&app).await.len();

    for id in [&ids[0], &ids[2]] {
        let (status, body) = started(&app, id).await;
        assert_eq!(status, StatusCode::CONFLICT, "{id}");
        refused(&body, "fleet.lesson_not_open");
    }
    assert_eq!(sessions(&app).await.len(), before);
    let (_, body) = sent(&app, "POST", &format!("/lessons/{}/agree", ids[0]), "", From::Bridge).await;
    assert_eq!(lesson(&body).session_proposed, None);
}

const ONE_MANIFEST_ITEM: &str = r#"{"items":[
  {"who":"owner","lands_in":"manifest","title":"Walks opened in the browser","what":"The agent ran open on a walk.","fix":"Use show_window for walks.","evidence":["correction:1"]}
]}"#;

/// **An item of a Session's retro starts a Session in the repository its own
/// Session was started in**, and a second press starts no more.
#[tokio::test]
async fn starting_on_a_session_item_starts_a_session_in_its_repository() {
    let (fleet, id, _home) = rig(FakeJudge::saying(ONE_MANIFEST_ITEM)).await;
    said(&fleet, &id, "no, show walks with show_window").await;
    let retro = std::sync::Arc::clone(&fleet)
        .write_session_retro(id.as_str().to_string())
        .await
        .expect("written");
    let item = retro.items[0].id.clone();

    let first = std::sync::Arc::clone(&fleet)
        .start_lesson_session(item.clone())
        .await
        .expect("started");
    let second = std::sync::Arc::clone(&fleet)
        .start_lesson_session(item)
        .await
        .expect("answered");

    assert_eq!(first, second);
    assert_ne!(first.session_id, id.as_str(), "a new Session");
    assert_eq!(first.lesson.state.as_wire(), "agreed");
    assert_eq!(first.lesson.session_proposed.as_deref(), Some(first.session_id.as_str()));
    assert!(first.brief.contains("Use show_window for walks."), "{}", first.brief);
    assert!(first.brief.contains(&format!("Session {}", first.lesson.handle)));
    let all = fleet.list_sessions(None, None, None).await.expect("listed").sessions;
    assert_eq!(all.len(), 2, "the first and the one started");
    let thread = std::sync::Arc::clone(&fleet)
        .get_session(ipc::SessionId::carried(first.session_id.clone()))
        .await
        .expect("the thread");
    let sent = thread
        .rows
        .iter()
        .filter(|row| format!("{row:?}").contains("Use show_window for walks."))
        .count();
    assert_eq!(sent, 1, "one message, from two presses");
}
