//! A session that finds every worktree slot held is in the line, and the person is asked whether it
//! waits for the next slot or Fleet adds one. Wait changes nothing about the order; Add leases the
//! new slot to that session, ahead of whoever heads the line.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{AnswerWaiting, SessionId, SessionMode, SessionRecord, TuneSession, WaitingActKind, WaitingSource};

use super::session_host::{eventually, reason, rig_placing};
use super::session_queue::{close, queued_and_idle, resumes, the_keeper, with_slot_one_held, write_to_main};

const OTHERS: &str = "an agent's session";

async fn record(rig: &super::session_host::Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session
}

async fn calls(rig: &super::session_host::Rig, id: &SessionId) -> Vec<ipc::WaitingItem> {
    record(rig, id).await.waiting_for.into_iter().filter(|one| one.source == WaitingSource::SlotChoice).collect()
}

async fn choose(rig: &super::session_host::Rig, id: &SessionId, choice: u32) -> Result<SessionRecord, api::Refusal> {
    Arc::clone(&rig.fleet)
        .answer_waiting(AnswerWaiting {
            session_id: id.clone(),
            item_id: "slot-choice".into(),
            choice: Some(choice),
            text: None,
            mode: None,
        })
        .await
}

#[tokio::test]
async fn a_full_pool_asks_the_person_and_tells_the_agent_so() {
    let (rig, _holder) = with_slot_one_held().await;
    let (waiting, told) = queued_and_idle(&rig, "change a file").await;

    assert!(told.contains("Every worktree slot is in use"), "{told}");
    assert!(told.contains("The owner is being asked"), "{told}");
    assert!(told.contains("Stop here"), "{told}");
    let asked = calls(&rig, &waiting).await;
    assert_eq!(asked.len(), 1, "one call, however many writes were refused");
    let item = &asked[0];
    assert_eq!(item.id, "slot-choice");
    assert_eq!(item.text, "Every slot is in use");
    let labels: Vec<_> = item.options.iter().map(|one| (one.label.as_str(), one.description.is_some())).collect();
    assert_eq!(labels, [("Wait for a slot", false), ("Add a slot", false)]);
    let act = item.act.as_ref().expect("an act");
    assert_eq!(act.kind, WaitingActKind::Answer);

    // Another refused write raises nothing new and keeps when it began.
    rig.gate(&waiting, "Edit", &write_to_main(&rig)).await;
    let again = calls(&rig, &waiting).await;
    assert_eq!((again.len(), again[0].since.clone()), (1, item.since.clone()));
}

#[tokio::test]
async fn waiting_takes_the_call_off_and_the_next_freed_slot_is_the_sessions() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    let (waiting, _) = queued_and_idle(&rig, "change a file").await;
    let at = rig.stand_in.starts().len() - 1;
    let before = rig.stand_in.process(at).sent.lock().unwrap().len();

    let answered = choose(&rig, &waiting, 0).await.expect("answered");
    assert!(answered.waiting_for.iter().all(|one| one.source != WaitingSource::SlotChoice));
    assert!(calls(&rig, &waiting).await.is_empty());
    assert_eq!(rig.stand_in.process(at).sent.lock().unwrap().len(), before, "the agent is sent nothing");
    assert!(resumes(&rig, &waiting).is_empty(), "no slot is free yet");
    assert!(choose(&rig, &waiting, 0).await.is_err(), "answered once");

    close(&rig, &holder).await;
    eventually(|| async { resumes(&rig, &waiting).len() == 1 }).await;
    assert_eq!(resumes(&rig, &waiting)[0].directory, adapter_traits::slot_path(&rig.root, 1));
    keeper.abort();
}

#[tokio::test]
async fn adding_a_slot_gives_the_new_one_to_that_session_ahead_of_the_line() {
    let (rig, _holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    let (first, _) = queued_and_idle(&rig, "first").await;
    let (second, _) = queued_and_idle(&rig, "second").await;

    choose(&rig, &second, 1).await.expect("answered");

    eventually(|| async { resumes(&rig, &second).len() == 1 }).await;
    assert_eq!(resumes(&rig, &second)[0].directory, adapter_traits::slot_path(&rig.root, 9));
    assert!(resumes(&rig, &first).is_empty(), "the head of the line is not served ahead of its turn");
    assert!(calls(&rig, &second).await.is_empty());
    assert_eq!(calls(&rig, &first).await.len(), 1, "the other still waits on his answer");
    keeper.abort();
}

#[tokio::test]
async fn a_slot_given_back_while_the_call_stands_takes_it_off() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    let (waiting, _) = queued_and_idle(&rig, "change a file").await;
    assert_eq!(calls(&rig, &waiting).await.len(), 1);

    close(&rig, &holder).await;

    eventually(|| async { resumes(&rig, &waiting).len() == 1 && calls(&rig, &waiting).await.is_empty() }).await;
    keeper.abort();
}

#[tokio::test]
async fn a_session_that_ends_while_asked_leaves_the_line_and_its_call() {
    let (rig, _holder) = with_slot_one_held().await;
    let (gone, _) = queued_and_idle(&rig, "gone").await;
    let (next, _) = queued_and_idle(&rig, "next").await;
    let queue = rig.fleet.hosts().slot_queue();
    assert!(queue.pending(gone.as_str()).is_some());

    close(&rig, &gone).await;

    assert!(queue.pending(gone.as_str()).is_none());
    assert!(queue.pending(next.as_str()).is_some(), "the one behind it still waits on his answer");
    assert!(choose(&rig, &gone, 1).await.is_err(), "an ended session has no call to answer");
}

#[tokio::test]
async fn an_answer_that_is_not_one_of_the_two_choices_is_refused() {
    let (rig, _holder) = with_slot_one_held().await;
    let (waiting, _) = queued_and_idle(&rig, "change a file").await;

    assert!(choose(&rig, &waiting, 2).await.is_err());
    let words = Arc::clone(&rig.fleet)
        .answer_waiting(AnswerWaiting {
            session_id: waiting.clone(),
            item_id: "slot-choice".into(),
            choice: None,
            text: Some("wait".into()),
            mode: None,
        })
        .await;
    assert!(words.is_err(), "words are not a choice");
    let dismissed = Arc::clone(&rig.fleet)
        .dismiss_waiting(ipc::DismissWaiting {
            session_id: waiting.clone(),
            item_id: "slot-choice".into(),
            reason: None,
            misread: false,
        })
        .await;
    assert!(dismissed.is_err(), "it is answered, not dismissed");
    assert_eq!(calls(&rig, &waiting).await.len(), 1, "still asked");
}

/// A session resumed onto a full pool is asked about at once, before any write.
#[tokio::test]
async fn a_resume_onto_a_full_pool_raises_the_same_call() {
    let rig = rig_placing(None);
    for n in 1..=8 {
        rig.fleet.vcs().hold_slot(&rig.root, n, OTHERS);
    }
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "ready");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    assert!(calls(&rig, &id).await.is_empty(), "it has written nothing");
    rig.fleet
        .tune_session(TuneSession { session_id: id.clone(), model: None, effort: None, mode: SessionMode::Auto })
        .await
        .unwrap();

    rig.send(&id, "carry on").await;

    eventually(|| async { calls(&rig, &id).await.len() == 1 }).await;
    let answer = rig.gate(&id, "Edit", &write_to_main(&rig)).await;
    assert!(reason(&answer).contains("first in the queue"), "{}", reason(&answer));
}
