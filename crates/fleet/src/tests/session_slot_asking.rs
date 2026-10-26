//! `armada worktree lease` run by a hosted Session finds every slot held and tells Fleet. Fleet
//! raises the Session's one `slot_choice` call; Wait leaves the command looking, and Add makes a
//! slot and holds it for the command until it says it is done.

use std::sync::Arc;
use std::time::{Duration, Instant};

use api::{HostedSessions, Sessions};
use ipc::{AskForSlot, AskForSlotAct, SessionId, SlotAskStanding, WaitingSource};

use super::session_host::{eventually, reason, rig_placing, Rig};
use super::session_queue::{close, queued_and_idle, resumes, the_keeper, with_slot_one_held, write_to_main};

async fn asking(rig: &Rig, id: &SessionId, act: AskForSlotAct) -> SlotAskStanding {
    Arc::clone(&rig.fleet)
        .ask_for_slot(AskForSlot { act, session_id: id.clone() })
        .await
        .expect("answered")
        .standing
}

async fn calls(rig: &Rig, id: &SessionId) -> Vec<ipc::WaitingItem> {
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    record.waiting_for.into_iter().filter(|one| one.source == WaitingSource::SlotChoice).collect()
}

async fn choose(rig: &Rig, id: &SessionId, choice: u32) {
    Arc::clone(&rig.fleet)
        .answer_waiting(ipc::AnswerWaiting {
            session_id: id.clone(),
            item_id: "slot-choice".into(),
            choice: Some(choice),
            text: None,
            mode: None,
            fixed: None,
        })
        .await
        .expect("answered");
}

/// A session that has run a turn and written nothing: it holds no slot and is in no line.
async fn idle_without_a_slot(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    let at = rig.stand_in.starts().len() - 1;
    rig.stand_in.init(at);
    rig.stand_in.finishes(at, "ready");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    id
}

#[tokio::test]
async fn a_lease_that_finds_every_slot_held_raises_one_call_and_done_takes_it_off() {
    let rig = rig_placing(None);
    for n in 1..=8 {
        rig.fleet.vcs().hold_slot(&rig.root, n, "an agent's session");
    }
    let id = idle_without_a_slot(&rig).await;
    assert!(calls(&rig, &id).await.is_empty());

    assert_eq!(asking(&rig, &id, AskForSlotAct::Ask).await, SlotAskStanding::Asking);
    let first = calls(&rig, &id).await;
    assert_eq!((first.len(), first[0].text.as_str()), (1, "Every slot is in use"));
    asking(&rig, &id, AskForSlotAct::Ask).await;
    let again = calls(&rig, &id).await;
    assert_eq!((again.len(), again[0].since.clone()), (1, first[0].since.clone()), "heard again, not asked twice");

    asking(&rig, &id, AskForSlotAct::Done).await;
    assert!(calls(&rig, &id).await.is_empty());
}

#[tokio::test]
async fn a_session_fleet_does_not_host_or_that_has_ended_is_not_asked_about() {
    let rig = rig_placing(None);
    let stranger = SessionId::carried("00000000-0000-0000-0000-000000000000");
    assert_eq!(asking(&rig, &stranger, AskForSlotAct::Ask).await, SlotAskStanding::Unasked);

    let id = idle_without_a_slot(&rig).await;
    close(&rig, &id).await;
    assert_eq!(asking(&rig, &id, AskForSlotAct::Ask).await, SlotAskStanding::Unasked);
}

#[tokio::test]
async fn wait_leaves_the_command_looking_and_takes_the_call_off() {
    let rig = rig_placing(None);
    for n in 1..=8 {
        rig.fleet.vcs().hold_slot(&rig.root, n, "an agent's session");
    }
    let id = idle_without_a_slot(&rig).await;
    asking(&rig, &id, AskForSlotAct::Ask).await;

    choose(&rig, &id, 0).await;

    assert!(calls(&rig, &id).await.is_empty());
    assert_eq!(asking(&rig, &id, AskForSlotAct::Ask).await, SlotAskStanding::Waiting);
    assert!(calls(&rig, &id).await.is_empty(), "one answer, not asked again");
    assert_eq!(rig.fleet.vcs().slot_holders(&rig.root).len(), 8, "no slot was made");
}

#[tokio::test]
async fn add_holds_the_new_slot_for_the_command_ahead_of_the_line_until_it_is_done() {
    let (rig, _holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    let (head, _) = queued_and_idle(&rig, "the head of the line").await;
    choose(&rig, &head, 0).await;
    let id = idle_without_a_slot(&rig).await;
    asking(&rig, &id, AskForSlotAct::Ask).await;

    choose(&rig, &id, 1).await;

    assert_eq!(asking(&rig, &id, AskForSlotAct::Ask).await, SlotAskStanding::Added);
    assert_eq!(rig.fleet.vcs().slot_holders(&rig.root).len(), 9, "a ninth slot was made");
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(resumes(&rig, &head).is_empty(), "the line does not take the slot held for the command");
    // Every look the line got was under the reservation, so only the command's `done` can free it.

    asking(&rig, &id, AskForSlotAct::Done).await;
    eventually(|| async { resumes(&rig, &head).len() == 1 }).await;
    assert_eq!(resumes(&rig, &head)[0].directory, adapter_traits::slot_path(&rig.root, 9));
    keeper.abort();
}

#[tokio::test]
async fn a_session_already_in_the_line_has_the_one_call_and_its_slot_comes_through_the_line() {
    let (rig, _holder) = with_slot_one_held().await;
    let (lined, _) = queued_and_idle(&rig, "write first").await;

    asking(&rig, &lined, AskForSlotAct::Ask).await;
    assert_eq!(calls(&rig, &lined).await.len(), 1, "the line's call, and no second");

    choose(&rig, &lined, 1).await;

    eventually(|| async { resumes(&rig, &lined).len() == 1 }).await;
    assert_eq!(resumes(&rig, &lined)[0].directory, adapter_traits::slot_path(&rig.root, 9));
    assert_eq!(asking(&rig, &lined, AskForSlotAct::Ask).await, SlotAskStanding::Waiting, "the command keeps looking");
    assert!(calls(&rig, &lined).await.is_empty());
}

#[tokio::test]
async fn a_command_that_goes_quiet_loses_its_call_and_its_slot_after_the_expiry() {
    let rig = rig_placing(None);
    for n in 1..=8 {
        rig.fleet.vcs().hold_slot(&rig.root, n, "an agent's session");
    }
    let id = idle_without_a_slot(&rig).await;
    asking(&rig, &id, AskForSlotAct::Ask).await;
    choose(&rig, &id, 1).await;
    let queue = rig.fleet.hosts().slot_queue();
    let manifest = rig.fleet.repositories().first().expect("a served repository").manifest().id().as_str().to_string();
    let now = Instant::now();
    assert!(queue.reserved(&manifest, None, now));
    assert!(!queue.reserved(&manifest, Some(id.as_str()), now), "it is the command's own");
    assert!(!queue.reserved(&manifest, None, now + crate::session_host::slot_asking::RESERVED_FOR + Duration::from_secs(1)));

    let gone = queue.expire(now + crate::session_host::slot_asking::EXPIRES + Duration::from_secs(1));
    assert_eq!(gone, vec![id.as_str().to_string()]);
    assert!(!queue.reserved(&manifest, None, now));
    assert!(queue.expire(now + Duration::from_secs(60)).is_empty(), "forgotten once");
}

/// With no line at all, a write would lease the added slot straight off the pool.
#[tokio::test]
async fn a_write_does_not_lease_the_slot_added_for_a_command() {
    let rig = rig_placing(None);
    for n in 1..=8 {
        rig.fleet.vcs().hold_slot(&rig.root, n, "an agent's session");
    }
    let id = idle_without_a_slot(&rig).await;
    asking(&rig, &id, AskForSlotAct::Ask).await;
    choose(&rig, &id, 1).await;

    let newcomer = rig.start().await;
    rig.send(&newcomer, "write").await;
    let at = rig.stand_in.starts().len() - 1;
    rig.stand_in.init(at);
    let refused = rig.gate(&newcomer, "Edit", &write_to_main(&rig)).await;
    assert!(reason(&refused).contains("first in the queue"), "{}", reason(&refused));
    rig.stand_in.finishes(at, "waiting for a slot");
    eventually(|| async { rig.turn(&newcomer).await == ipc::SessionTurn::Idle }).await;

    asking(&rig, &id, AskForSlotAct::Done).await;
    let keeper = the_keeper(&rig);
    eventually(|| async { resumes(&rig, &newcomer).len() == 1 }).await;
    assert_eq!(resumes(&rig, &newcomer)[0].directory, adapter_traits::slot_path(&rig.root, 9));
    keeper.abort();
}
