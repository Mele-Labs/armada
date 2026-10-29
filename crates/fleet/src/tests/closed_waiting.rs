//! What the person closed from a Job's Needs you list is kept whole, listed on the record and to the
//! agent, and put back by `reopen_waiting` where what it stood for still stands.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{DismissWaiting, ReopenWaiting, SetWaitingFor, ShowWindow, WaitingAct, WaitingActKind, WaitingInput, WaitingOption, WaitingSource};

use super::session_host::{rig_placing, Rig};

fn steps_item(id: &ipc::SessionId) -> SetWaitingFor {
    SetWaitingFor {
        session_id: Some(id.clone()),
        items: vec![
            WaitingInput {
                id: "a".into(),
                text: "Run the tests".into(),
                act: Some(WaitingAct { kind: WaitingActKind::Run, target: "cargo test".into() }),
                options: vec![WaitingOption { label: "Yes".into(), description: None }],
                steps: vec!["first".into(), "second".into()],
            },
            WaitingInput {
                id: "b".into(),
                text: "Look".into(),
                act: Some(WaitingAct { kind: WaitingActKind::Run, target: "cargo test".into() }),
                options: Vec::new(),
                steps: Vec::new(),
            },
        ],
    }
}

fn close(id: &ipc::SessionId, item: &str) -> DismissWaiting {
    DismissWaiting { session_id: id.clone(), item_id: item.into(), reason: None, misread: false }
}

fn reopen(id: &ipc::SessionId, item: &str) -> ReopenWaiting {
    ReopenWaiting { session_id: id.clone(), item_id: item.into() }
}

fn waiting_ids(record: &ipc::SessionRecord) -> Vec<&str> {
    record.waiting_for.iter().map(|one| one.id.as_str()).collect()
}

fn closed_ids(record: &ipc::SessionRecord) -> Vec<&str> {
    record.closed.iter().map(|one| one.item.id.as_str()).collect()
}

async fn record_of(rig: &Rig, id: &ipc::SessionId) -> ipc::SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session
}

async fn show(rig: &Rig, id: &ipc::SessionId, url: &str) {
    rig.fleet
        .show_window(None, ShowWindow { url: url.into(), title: Some("Findings".into()), session_id: Some(id.clone()), variations: Vec::new(), ..Default::default() })
        .await
        .unwrap();
}

#[tokio::test]
async fn closing_an_agent_item_keeps_it_whole_and_reopening_puts_it_back_as_stated() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.waiting_for(None, steps_item(&id)).await.unwrap();
    let closed = Arc::clone(&rig.fleet).dismiss_waiting(close(&id, "a")).await.unwrap();
    assert_eq!(waiting_ids(&closed), ["b"]);
    assert_eq!(closed_ids(&closed), ["a"]);
    let kept = &closed.closed[0];
    assert_eq!((kept.item.text.as_str(), kept.item.source, kept.ended), ("Run the tests", WaitingSource::Agent, false));
    assert!(!kept.at.as_str().is_empty());
    assert_eq!(kept.item.act.as_ref().map(|act| act.target.as_str()), Some("cargo test"));
    assert_eq!(kept.item.steps, ["first", "second"]);

    let back = Arc::clone(&rig.fleet).reopen_waiting(reopen(&id, "a")).await.unwrap();
    assert_eq!(waiting_ids(&back), ["b", "a"], "it goes to the end of the agent's list");
    assert!(back.closed.is_empty(), "a reopened item is no longer closed");
    let item = back.waiting_for.iter().find(|one| one.id == "a").unwrap();
    assert_eq!(item.text, "Run the tests");
    assert_eq!(item.act.as_ref().map(|act| (act.kind, act.target.as_str())), Some((WaitingActKind::Run, "cargo test")));
    assert_eq!(item.options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Yes"]);
    assert_eq!(item.steps, ["first", "second"]);
}

#[tokio::test]
async fn closing_a_walk_keeps_it_and_reopening_makes_it_wait_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let url = "http://localhost:5173/one";
    show(&rig, &id, url).await;
    let closed = Arc::clone(&rig.fleet).dismiss_waiting(close(&id, &format!("walk:{url}"))).await.unwrap();
    assert!(closed.waiting_for.is_empty());
    let kept = &closed.closed[0];
    assert_eq!((kept.item.id.as_str(), kept.item.text.as_str(), kept.item.source), (format!("walk:{url}").as_str(), "Findings", WaitingSource::Walk));
    assert!(!kept.ended, "the window is still shown");

    let back = Arc::clone(&rig.fleet).reopen_waiting(reopen(&id, &format!("walk:{url}"))).await.unwrap();
    assert_eq!(waiting_ids(&back), [format!("walk:{url}")]);
    assert!(back.closed.is_empty());
}

#[tokio::test]
async fn the_newest_closed_item_comes_first() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.waiting_for(None, steps_item(&id)).await.unwrap();
    Arc::clone(&rig.fleet).dismiss_waiting(close(&id, "a")).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let record = Arc::clone(&rig.fleet).dismiss_waiting(close(&id, "b")).await.unwrap();
    assert_eq!(closed_ids(&record), ["b", "a"]);
}

#[tokio::test]
async fn an_item_whose_subject_has_ended_is_listed_and_refused_a_reopening() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let since = rig.fleet.all_done(None, ipc::AllDone { session_id: Some(id.clone()) }).await.unwrap().done_since.expect("when");
    let item = format!("done:{}", since.as_str());
    let closed = Arc::clone(&rig.fleet).dismiss_waiting(close(&id, &item)).await.unwrap();
    assert!(!closed.closed[0].ended, "while the Job still says it is done");

    rig.send(&id, "One more thing").await;
    let after = record_of(&rig, &id).await;
    assert_eq!(closed_ids(&after), [item.as_str()]);
    assert!(after.closed[0].ended, "the person's message cleared the done");
    let refused = Arc::clone(&rig.fleet).reopen_waiting(reopen(&id, &item)).await;
    assert!(matches!(refused, Err(api::Refusal::Unacceptable(_))), "{refused:?}");
    assert!(record_of(&rig, &id).await.waiting_for.iter().all(|one| one.id != item), "nothing came back");

    let unknown = Arc::clone(&rig.fleet).reopen_waiting(reopen(&id, "never-closed")).await;
    assert!(matches!(unknown, Err(api::Refusal::IllegalMove(_))), "{unknown:?}");
}

#[tokio::test]
async fn the_agent_lists_what_was_closed_newest_first_and_a_call_placing_no_job_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.waiting_for(None, steps_item(&id)).await.unwrap();
    Arc::clone(&rig.fleet).dismiss_waiting(close(&id, "a")).await.unwrap();
    let list = rig.fleet.list_closed_waiting(None, Some(id.clone())).await.unwrap();
    assert_eq!(list.session_id, id);
    assert_eq!(list.closed.len(), 1);
    assert_eq!((list.closed[0].item.id.as_str(), list.closed[0].item.text.as_str(), list.closed[0].ended), ("a", "Run the tests", false));

    let unplaced = rig.fleet.list_closed_waiting(None, None).await;
    assert!(matches!(unplaced, Err(api::Refusal::Unacceptable(_))), "{unplaced:?}");
}
