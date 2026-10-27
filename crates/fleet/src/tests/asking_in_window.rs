//! The claim, written before the code: **a question about the walk opens in the walk.** It waits on
//! the person with the place it points at, answering it reaches the session as the person's message,
//! and a question about no page the session showed is refused and sent to `AskUserQuestion`.
//! `docs/concepts/session.md`, *Asking in the window*.

use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{
    AnswerWaiting, AskInWindow, DismissWaiting, SessionRow, SessionVoice, ShowWindow, WaitingActKind,
    WaitingOption, WaitingSource,
};

use super::session_host::{rig_placing, Rig};

const WALK: &str = "http://localhost:5173/";

fn asked(id: &ipc::SessionId, url: &str) -> AskInWindow {
    AskInWindow {
        question: "Should the count sit beside the title or under it?".into(),
        options: vec![
            WaitingOption { label: "Beside".into(), description: Some("One line, as on the Board".into()) },
            WaitingOption { label: "Under".into(), description: None },
        ],
        url: url.into(),
        selector: "main > h1".into(),
        text: Some("Findings".into()),
        session_id: Some(id.clone()),
    }
}

async fn walking(rig: &Rig) -> ipc::SessionId {
    let id = rig.start().await;
    rig.fleet
        .show_window(None, ShowWindow { url: WALK.into(), title: Some("Findings".into()), session_id: Some(id.clone()), variations: Vec::new(), ..Default::default() })
        .await
        .unwrap();
    id
}

fn the_ask(record: &ipc::SessionRecord) -> &ipc::WaitingItem {
    record
        .waiting_for
        .iter()
        .find(|one| one.source == WaitingSource::WindowAsk)
        .unwrap_or_else(|| panic!("no question waits: {:?}", record.waiting_for))
}

fn sent(rows: &[SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_question_about_the_walk_waits_with_its_place_and_its_answer_reaches_the_session() {
    let rig = rig_placing(None);
    let id = walking(&rig).await;
    let record = rig.fleet.ask_in_window(None, asked(&id, "http://localhost:5173/findings?open=1")).await.unwrap();

    let item = the_ask(&record);
    assert!(item.id.starts_with("window_ask:"), "{}", item.id);
    assert_eq!(item.text, "Should the count sit beside the title or under it?");
    assert_eq!(item.options[0].description.as_deref(), Some("One line, as on the Board"));
    let place = item.place.as_ref().expect("the place travels with the item");
    assert_eq!(place.url, "http://localhost:5173/findings?open=1");
    assert_eq!(place.selector, "main > h1");
    assert_eq!(place.text.as_deref(), Some("Findings"));
    let act = item.act.as_ref().unwrap();
    assert_eq!((act.kind, act.target.as_str()), (WaitingActKind::Walk, WALK), "a press opens the walk it was asked in");

    let rows = rig.rows(&id).await;
    assert!(
        rows.iter().any(|row| matches!(row, SessionRow::WindowAsk { answer: None, window, .. } if window == WALK)),
        "the thread keeps the question: {rows:?}"
    );

    let mut said = AnswerWaiting { session_id: id.clone(), item_id: item.id.clone(), choice: None, text: None, mode: None, fixed: None };
    assert!(Arc::clone(&rig.fleet).answer_waiting(said.clone()).await.is_err(), "an empty answer is refused");
    said.choice = Some(1);
    said.text = Some("and smaller".into());
    let after = Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();
    assert!(after.waiting_for.iter().all(|one| one.source != WaitingSource::WindowAsk), "answered, so it waits no more");

    let rows = rig.rows(&id).await;
    let last = sent(&rows).pop().unwrap();
    assert_eq!(last, "Re: Should the count sit beside the title or under it?\nUnder\nand smaller");
    assert!(
        rows.iter().any(|row| matches!(row, SessionRow::WindowAsk { answer: Some(answer), .. } if answer == "Under\nand smaller")),
        "the row says how it was answered"
    );
}

#[tokio::test]
async fn a_question_about_no_page_the_session_showed_is_refused_and_pointed_at_ask_user_question() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let unshown = rig.fleet.ask_in_window(None, asked(&id, WALK)).await;
    let Err(Refusal::Unacceptable(why)) = unshown else { panic!("a session that walks nothing cannot ask in a walk") };
    assert!(why.message.contains("AskUserQuestion"), "{}", why.message);

    let id = walking(&rig).await;
    let elsewhere = rig.fleet.ask_in_window(None, asked(&id, "http://localhost:6006/")).await;
    assert!(matches!(elsewhere, Err(Refusal::Unacceptable(_))), "another origin is not the walk");
    assert!(rig.fleet.ask_in_window(None, asked(&id, "file:///etc/hosts")).await.is_err());
    let mut blank = asked(&id, WALK);
    blank.question = "  ".into();
    assert!(rig.fleet.ask_in_window(None, blank).await.is_err(), "a question with no words");
    let mut nowhere = asked(&id, WALK);
    nowhere.selector = String::new();
    assert!(rig.fleet.ask_in_window(None, nowhere).await.is_err(), "a question that points at nothing");
}

/// A Fleet a build behind the one that wrote the thread meets a row kind it does not know. That one
/// row cost the answer its place on the question's, so the question stood after he answered it.
#[tokio::test]
async fn a_question_answered_on_a_thread_with_a_row_this_fleet_cannot_read_waits_no_more() {
    let rig = rig_placing(None);
    let id = walking(&rig).await;
    rig.fleet
        .store()
        .lock()
        .await
        .keep_session_row(id.as_str(), "unknown-1", r#"{"kind":"from_a_later_build","id":"unknown-1","at":"2026-10-21T13:15:23.936Z"}"#)
        .expect("kept");
    let record = rig.fleet.ask_in_window(None, asked(&id, WALK)).await.unwrap();
    let item = the_ask(&record).id.clone();

    let said = AnswerWaiting { session_id: id.clone(), item_id: item, choice: Some(0), text: None, mode: None, fixed: None };
    let after = Arc::clone(&rig.fleet).answer_waiting(said).await.unwrap();
    assert!(after.waiting_for.iter().all(|one| one.source != WaitingSource::WindowAsk), "answered, so it waits no more: {:?}", after.waiting_for);
    let rows = rig.rows(&id).await;
    assert!(
        rows.iter().any(|row| matches!(row, SessionRow::WindowAsk { answer: Some(answer), .. } if answer == "Beside")),
        "the row says how it was answered: {rows:?}"
    );
}

#[tokio::test]
async fn a_dismissed_question_waits_no_more_and_nothing_is_sent() {
    let rig = rig_placing(None);
    let id = walking(&rig).await;
    let record = rig.fleet.ask_in_window(None, asked(&id, WALK)).await.unwrap();
    let item = the_ask(&record).id.clone();
    let before = sent(&rig.rows(&id).await).len();
    let gone = Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: id.clone(), item_id: item, reason: None, misread: false })
        .await
        .unwrap();
    assert!(gone.waiting_for.iter().all(|one| one.source != WaitingSource::WindowAsk));
    assert_eq!(sent(&rig.rows(&id).await).len(), before);
}
