//! An agent's word that it has nothing left to work on: kept on the row, and cleared by whatever
//! gives it work again. `docs/concepts/session.md`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    AllDone, Instant, SessionId, SessionRecord, SessionRow, SessionVoice, SetWaitingFor,
    WaitingAct, WaitingActKind, WaitingInput,
};

use super::session_host::{eventually, rig_placing};

/// The end of the line `sending.rs` adds for the agent; the whole line is not reachable from here.
const TAKES_DONE_BACK: &str = "call all_done again";

fn done(id: &SessionId) -> AllDone {
    AllDone { session_id: Some(id.clone()) }
}

fn owed(id: &SessionId, items: usize) -> SetWaitingFor {
    SetWaitingFor {
        session_id: Some(id.clone()),
        items: (0..items)
            .map(|one| WaitingInput {
                id: format!("i{one}"),
                text: "Look".into(),
                act: Some(WaitingAct { kind: WaitingActKind::Run, target: "cargo test".into() }),
                options: Vec::new(),
                steps: Vec::new(),
            })
            .collect(),
    }
}

async fn read(rig: &super::session_host::Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session
}

#[tokio::test]
async fn a_done_session_reads_back_and_a_second_call_keeps_the_first_instant() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert_eq!(read(&rig, &id).await.done_since, None);

    let said = rig.fleet.all_done(None, done(&id)).await.unwrap();
    let since: Instant = said.done_since.clone().expect("the answer carries when");
    assert_eq!(read(&rig, &id).await.done_since, Some(since.clone()), "read back");

    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let again = rig.fleet.all_done(None, done(&id)).await.unwrap();
    assert_eq!(again.done_since, Some(since), "the callout does not move");

    let nameless = rig.fleet.all_done(None, AllDone { session_id: None }).await;
    assert!(nameless.is_err(), "a call that places no session says nothing");
}

#[tokio::test]
async fn the_persons_message_clears_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.all_done(None, done(&id)).await.unwrap();
    rig.send(&id, "one more thing").await;
    assert_eq!(read(&rig, &id).await.done_since, None);
}

#[tokio::test]
async fn the_persons_message_tells_the_agent_it_took_the_done_back() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    let sent = || rig.stand_in.process(0).sent.lock().unwrap().clone();
    eventually(|| async { !sent().is_empty() }).await;
    assert!(
        sent().iter().all(|line| !line.contains(TAKES_DONE_BACK)),
        "no done stood, so there is nothing to take back"
    );

    rig.fleet.all_done(None, done(&id)).await.unwrap();
    rig.send(&id, "one more thing").await;
    eventually(|| async { sent().len() == 2 }).await;
    let last = sent().pop().unwrap();
    assert!(last.contains("one more thing") && last.contains(TAKES_DONE_BACK), "{last}");

    // The line is for the agent: the person's row keeps their words.
    let said: Vec<String> = rig
        .rows(&id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(said, ["do the thing", "one more thing"]);
}

#[tokio::test]
async fn a_non_empty_waiting_list_clears_it_and_an_empty_one_does_not() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.all_done(None, done(&id)).await.unwrap();
    let empty = rig.fleet.waiting_for(None, owed(&id, 0)).await.unwrap();
    assert!(empty.done_since.is_some(), "clearing the list is what comes before saying done");
    let stated = rig.fleet.waiting_for(None, owed(&id, 1)).await.unwrap();
    assert_eq!(stated.done_since, None);
    assert_eq!(read(&rig, &id).await.done_since, None);
}

#[tokio::test]
async fn a_hosted_turn_starting_again_clears_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;

    rig.fleet.all_done(None, done(&id)).await.unwrap();
    assert!(read(&rig, &id).await.done_since.is_some());
    // A background task woke it: a turn begins with nobody having spoken.
    rig.stand_in.init(0);
    eventually(|| async { read(&rig, &id).await.done_since.is_none() }).await;
}

#[tokio::test]
async fn a_turn_fleets_news_starts_leaves_it_standing_and_a_request_clears_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;

    rig.fleet.all_done(None, done(&id)).await.unwrap();
    Arc::clone(&rig.fleet).told_news_by_fleet(id.as_str(), "#1 merged into main.").await.unwrap();
    rig.stand_in.init(0);
    eventually(|| async { rig.fleet.hosts().of(id.as_str()).state().fleet_told == 0 }).await;
    assert!(read(&rig, &id).await.done_since.is_some(), "the notice gave the agent nothing to do");

    // A word that asks for work is a turn like any other.
    Arc::clone(&rig.fleet).told_by_fleet(id.as_str(), "Commit and push slot 3.").await.unwrap();
    rig.stand_in.init(0);
    eventually(|| async { read(&rig, &id).await.done_since.is_none() }).await;
}

#[tokio::test]
async fn a_session_goes_idle_after_a_turn_fleets_news_started() {
    for news in [true, false] {
        let rig = rig_placing(None);
        let id = rig.start().await;
        rig.send(&id, "do the thing").await;
        rig.stand_in.init(0);
        rig.stand_in.finishes(0, "done");
        eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;

        let told = if news {
            Arc::clone(&rig.fleet).told_news_by_fleet(id.as_str(), "#1 merged into main.").await
        } else {
            Arc::clone(&rig.fleet).told_by_fleet(id.as_str(), "#1 merged into main.").await
        };
        assert!(told.unwrap());
        rig.stand_in.init(0);
        rig.stand_in.finishes(0, "noted");
        eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    }
}

/// Two lines reach the process together and it answers them in one turn.
#[tokio::test]
async fn a_session_whose_two_lines_ran_as_one_turn_does_not_stay_working() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "first").await;
    rig.send(&id, "second").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "both");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
}

#[tokio::test]
async fn an_ended_session_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    Arc::clone(&rig.fleet)
        .close_session(ipc::CloseSession { session_id: id.clone() })
        .await
        .unwrap();
    assert!(rig.fleet.all_done(None, done(&id)).await.is_err());
}
