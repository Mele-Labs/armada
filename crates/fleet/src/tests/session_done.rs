//! An agent's word that it has nothing left to work on: kept on the row, and cleared by whatever
//! gives it work again. `docs/concepts/session.md`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    AllDone, Instant, SessionId, SessionRecord, SetWaitingFor, WaitingAct, WaitingActKind,
    WaitingInput,
};

use super::session_host::{eventually, rig_placing};

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
async fn an_ended_session_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    Arc::clone(&rig.fleet)
        .close_session(ipc::CloseSession { session_id: id.clone() })
        .await
        .unwrap();
    assert!(rig.fleet.all_done(None, done(&id)).await.is_err());
}
