//! Sleep mode: what the night answers, what it holds, and what it leaves for the review.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{OverrideSleep, SessionRow, SessionVoice, SetSleep, ShowWindow, TuneSession, SessionMode};
use store::KeptWaiting;

use super::session_host::{eventually, rig_placing, Rig};

const OLD: &str = "2000-01-01T00:00:00.000Z";

fn item(id: &str, text: &str, options: &[&str], since: &str) -> KeptWaiting {
    KeptWaiting {
        item_id: id.into(),
        text: text.into(),
        since: since.into(),
        act: None,
        options: options.iter().map(|one| (*one).into()).collect(),
    }
}

async fn keep(rig: &Rig, id: &ipc::SessionId, items: &[KeptWaiting]) {
    rig.fleet.store().lock().await.replace_waiting_for(id.as_str(), items).unwrap();
}

async fn on(rig: &Rig) -> ipc::SleepState {
    Arc::clone(&rig.fleet).set_sleep(SetSleep { on: true }).await.unwrap()
}

async fn pass(rig: &Rig) {
    Arc::clone(&rig.fleet).sleep_pass().await.unwrap();
}

async fn night(rig: &Rig) -> ipc::SleepState {
    rig.fleet.get_sleep().await.unwrap()
}

async fn said(rig: &Rig, id: &ipc::SessionId) -> Vec<String> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn an_ask_older_than_the_grace_is_answered_with_best_and_recorded() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    on(&rig).await;
    keep(&rig, &id, &[item("a", "Which clock?", &["Fake", "Frozen"], OLD)]).await;
    pass(&rig).await;
    let state = night(&rig).await;
    assert_eq!(state.decided.len(), 1);
    assert_eq!(state.decided[0].asked, "Which clock?");
    assert_eq!(state.decided[0].chose, "", "best leaves the choice to the agent");
    let sent = said(&rig, &id).await;
    assert!(sent.last().unwrap().contains("Think it through and go with the best option"), "{sent:?}");
    pass(&rig).await;
    assert_eq!(night(&rig).await.decided.len(), 1, "an item is decided once");
}

#[tokio::test]
async fn a_recommended_option_is_picked_and_recorded_as_chosen() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    on(&rig).await;
    keep(&rig, &id, &[item("a", "Which clock?", &["Fake", "Frozen (Recommended)"], OLD)]).await;
    pass(&rig).await;
    let state = night(&rig).await;
    assert_eq!(state.decided[0].chose, "Frozen (Recommended)");
    assert!(said(&rig, &id).await.last().unwrap().starts_with("Re: Which clock?\nFrozen (Recommended)"));
}

#[tokio::test]
async fn a_destructive_question_is_held_and_never_answered() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    on(&rig).await;
    let before = said(&rig, &id).await.len();
    keep(
        &rig,
        &id,
        &[
            item("a", "Delete the old branch?", &["Yes (Recommended)", "No"], OLD),
            item("b", "Which one?", &["Keep", "Force push (Recommended)"], OLD),
            item("c", "Run rm -rf build?", &[], OLD),
        ],
    )
    .await;
    pass(&rig).await;
    let state = night(&rig).await;
    assert_eq!(state.blocked.len(), 3);
    assert!(state.decided.is_empty());
    assert_eq!(said(&rig, &id).await.len(), before, "nothing was said to the agent");
}

#[tokio::test]
async fn a_permission_is_blocked_and_left_open() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet
        .tune_session(TuneSession { session_id: id.clone(), model: None, effort: None, mode: SessionMode::Ask })
        .await
        .unwrap();
    on(&rig).await;
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let ask = r#"{"tool_name":"Bash","input":{"command":"cargo build"}}"#;
    let held = tokio::spawn(async move {
        fleet.session_permission(&session, ipc::decode("an ask", ask.as_bytes()).unwrap()).await
    });
    eventually(|| async {
        Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.hosted.unwrap().asked.is_some()
    })
    .await;
    pass(&rig).await;
    let state = night(&rig).await;
    assert_eq!(state.blocked.len(), 1);
    assert!(state.blocked[0].text.starts_with("Bash"), "{:?}", state.blocked);
    assert!(!held.is_finished(), "the permission is still held");
    held.abort();
}

#[tokio::test]
async fn within_the_grace_nothing_happens_and_a_walk_is_recorded() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    on(&rig).await;
    let fresh = rig.fleet.now().as_str().to_string();
    keep(&rig, &id, &[item("a", "Which clock?", &["Fake"], &fresh)]).await;
    pass(&rig).await;
    assert!(night(&rig).await.decided.is_empty(), "the agent has two minutes to carry on");
    rig.fleet
        .show_window(None, ShowWindow { url: "http://localhost:5173/".into(), title: Some("Findings".into()), session_id: Some(id.clone()) })
        .await
        .unwrap();
    pass(&rig).await;
    pass(&rig).await;
    let state = night(&rig).await;
    assert_eq!(state.walks.len(), 1, "once");
    assert_eq!(state.walks[0].title, "Findings");
}

#[tokio::test]
async fn off_means_no_more_decisions_and_on_clears_the_last_night() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    pass(&rig).await;
    keep(&rig, &id, &[item("a", "Which clock?", &[], OLD)]).await;
    pass(&rig).await;
    assert!(night(&rig).await.decided.is_empty(), "off by default");
    on(&rig).await;
    pass(&rig).await;
    assert_eq!(night(&rig).await.decided.len(), 1);
    let off = Arc::clone(&rig.fleet).set_sleep(SetSleep { on: false }).await.unwrap();
    assert!(!off.on);
    assert_eq!(off.decided.len(), 1, "the review reads the night after it ends");
    keep(&rig, &id, &[item("b", "Which port?", &[], OLD)]).await;
    pass(&rig).await;
    assert_eq!(night(&rig).await.decided.len(), 1, "no more decisions");
    let again = on(&rig).await;
    assert!(again.on && again.decided.is_empty(), "a new night starts clean");
}

#[tokio::test]
async fn an_override_sends_the_session_the_correction_and_marks_the_row() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    on(&rig).await;
    keep(&rig, &id, &[item("a", "Which clock?", &["Fake"], OLD)]).await;
    pass(&rig).await;
    let row = night(&rig).await.decided[0].id.clone();
    let blank = Arc::clone(&rig.fleet).override_sleep(OverrideSleep { id: row.clone(), text: "  ".into() }).await;
    assert!(matches!(blank, Err(api::Refusal::Unacceptable(_))));
    let ghost = Arc::clone(&rig.fleet).override_sleep(OverrideSleep { id: "nope".into(), text: "x".into() }).await;
    assert!(matches!(ghost, Err(api::Refusal::IllegalMove(_))));
    let state = Arc::clone(&rig.fleet)
        .override_sleep(OverrideSleep { id: row, text: "Use the frozen one".into() })
        .await
        .unwrap();
    assert_eq!(state.decided[0].corrected.as_deref(), Some("Use the frozen one"));
    assert_eq!(said(&rig, &id).await.last().unwrap(), "Re: Which clock?\nUse the frozen one");
}

#[test]
fn what_counts_as_destructive_reads_the_text_and_the_options() {
    let one = |text: &str, options: &[&str]| ipc::WaitingItem {
        id: "a".into(),
        text: text.into(),
        since: ipc::Instant::carried(OLD),
        source: ipc::WaitingSource::Agent,
        act: None,
        options: options.iter().map(|label| ipc::WaitingOption { label: (*label).into() }).collect(),
    };
    assert!(crate::sleeping::destructive(&one("Close PR 12?", &[])));
    assert!(crate::sleeping::destructive(&one("Which?", &["Overwrite it", "Keep"])));
    assert!(crate::sleeping::destructive(&one("rm the cache?", &[])));
    assert!(!crate::sleeping::destructive(&one("Which size?", &["Large", "Small"])));
    assert!(!crate::sleeping::destructive(&one("Name the form field", &[])));
}
