//! Pausing a hosted Session and resuming it. Since 23.92. **A stand-in process and a fake pool**:
//! what is read is the ledger, the row Fleet publishes and the pool's lease.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use adapter_traits::Vcs;
use api::HostedSessions;
use ipc::{
    AttachmentState, PauseSession, ResumeSession, SessionId, SessionRecord, SessionState,
    SessionTurn,
};

use super::session_host::{denied, eventually, rig_placing, Rig};
use super::session_queue::{the_keeper, with_slot_one_held};

const OTHERS: &str = "an agent's session";

/// A session with a turn running and slot 1 leased by its first write.
async fn holding_slot_one(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "change a file").await;
    rig.stand_in.init(0);
    let path = format!("{}/src/lib.rs", rig.root);
    let held = rig.gate(&id, "Edit", &format!(r#"{{"file_path":"{path}"}}"#)).await;
    assert!(denied(&held));
    id
}

fn pause(id: &SessionId) -> PauseSession {
    PauseSession { session_id: id.clone() }
}

fn resume(id: &SessionId) -> ResumeSession {
    ResumeSession { session_id: id.clone() }
}

fn holds(record: &SessionRecord, kind: &str) -> bool {
    record.attachments.iter().any(|one| one.kind == kind && one.state == AttachmentState::Standing)
}

fn code(refusal: api::Refusal) -> String {
    match refusal {
        api::Refusal::IllegalMove(why) | api::Refusal::Unacceptable(why) => why.code,
        other => panic!("not a refusal with a code: {other:?}"),
    }
}

#[tokio::test]
async fn a_pause_ends_the_turn_parks_the_work_and_gives_the_slot_back() {
    let rig = rig_placing(None);
    let id = holding_slot_one(&rig).await;

    let paused = rig.fleet.pause_session(pause(&id)).await.expect("paused");

    let marker = paused.hosted.as_ref().expect("hosted").paused.as_ref().expect("a marker");
    assert!(!marker.resuming);
    assert_eq!(paused.state, SessionState::Live);
    assert_eq!(rig.turn(&id).await, SessionTurn::Idle);
    assert!(rig.stand_in.process(0).ended.load(Ordering::SeqCst));
    assert!(!holds(&paused, "slot"), "the slot is given back");
    assert!(holds(&paused, "branch"), "the branch stays the session's");
    let pool = crate::leasing::pool_of(&rig.fleet.repositories().first().unwrap());
    assert!(rig.fleet.vcs().slot_standing(&pool, 1, id.as_str()) != adapter_traits::SlotStanding::Held);
}

#[tokio::test]
async fn a_resume_takes_a_slot_again_and_clears_the_marker_without_starting_a_turn() {
    let rig = rig_placing(None);
    let id = holding_slot_one(&rig).await;
    rig.fleet.pause_session(pause(&id)).await.expect("paused");
    let starts = rig.stand_in.starts().len();

    let resumed = rig.fleet.resume_session(resume(&id)).await.expect("resumed");

    assert!(resumed.hosted.as_ref().unwrap().paused.is_none());
    assert!(holds(&resumed, "slot") && holds(&resumed, "branch"));
    assert_eq!(rig.stand_in.starts().len(), starts, "a resume starts no process");
    assert_eq!(rig.turn(&id).await, SessionTurn::Idle);
}

#[tokio::test]
async fn a_message_to_a_paused_session_resumes_it() {
    let rig = rig_placing(None);
    let id = holding_slot_one(&rig).await;
    rig.fleet.pause_session(pause(&id)).await.expect("paused");

    rig.send(&id, "carry on").await;

    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    let start = &rig.stand_in.starts()[1];
    assert!(start.resuming, "the same conversation");
    assert_eq!(start.directory, adapter_traits::slot_path(&rig.root, 1));
    let row = row_of(&rig, &id).await;
    assert!(row.hosted.as_ref().unwrap().paused.is_none());
    assert!(holds(&row, "slot"));
}

#[tokio::test]
async fn a_resume_with_every_slot_held_waits_and_the_first_free_slot_is_its() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    rig.fleet.pause_session(pause(&holder)).await.expect("paused");
    rig.fleet.vcs().hold_slot(&rig.root, 1, OTHERS);

    let waiting = rig.fleet.resume_session(resume(&holder)).await.expect("waits");
    assert!(waiting.hosted.as_ref().unwrap().paused.as_ref().unwrap().resuming);
    assert!(!holds(&waiting, "slot"), "nobody was moved out for it");

    rig.fleet.vcs().free_slot(&rig.root, 1);
    rig.fleet.hosts().slot_freed();
    eventually(|| async {
        let row = row_of(&rig, &holder).await;
        row.hosted.as_ref().unwrap().paused.is_none() && holds(&row, "slot")
    })
    .await;
    keeper.abort();
}

#[tokio::test]
async fn a_pause_of_a_session_waiting_to_resume_calls_the_wait_off() {
    let (rig, holder) = with_slot_one_held().await;
    rig.fleet.pause_session(pause(&holder)).await.expect("paused");
    rig.fleet.vcs().hold_slot(&rig.root, 1, OTHERS);
    rig.fleet.resume_session(resume(&holder)).await.expect("waits");

    let again = rig.fleet.pause_session(pause(&holder)).await.expect("wait called off");

    assert!(!again.hosted.unwrap().paused.unwrap().resuming);
}

#[tokio::test]
async fn pause_and_resume_refuse_what_they_cannot_do() {
    let rig = rig_placing(None);
    // A session with no slot yet holds nothing to park.
    let bare = rig.start().await;
    let refused = rig.fleet.pause_session(pause(&bare)).await.expect_err("no slot");
    assert_eq!(code(refused), "fleet.session_not_pausable");
    let refused = rig.fleet.resume_session(resume(&bare)).await.expect_err("not paused");
    assert_eq!(code(refused), "fleet.session_not_paused");

    let id = holding_slot_one(&rig).await;
    rig.fleet.pause_session(pause(&id)).await.expect("paused");
    let refused = rig.fleet.pause_session(pause(&id)).await.expect_err("already");
    assert_eq!(code(refused), "fleet.session_already_paused");

    // The pool gave the slot to another between the pause and a second session's pause.
    let other = rig.start().await;
    rig.send(&other, "also").await;
    let at = rig.stand_in.starts().len() - 1;
    rig.stand_in.init(at);
    let path = format!("{}/src/lib.rs", rig.root);
    rig.gate(&other, "Edit", &format!(r#"{{"file_path":"{path}"}}"#)).await;
    rig.fleet.vcs().hold_slot(&rig.root, 1, OTHERS);
    let refused = rig.fleet.pause_session(pause(&other)).await.expect_err("slot is another's");
    assert_eq!(code(refused), "fleet.session_pause_refused");

    // Ended.
    Arc::clone(&rig.fleet)
        .close_session(ipc::CloseSession { session_id: bare.clone() })
        .await
        .expect("closed");
    let refused = rig.fleet.pause_session(pause(&bare)).await.expect_err("ended");
    assert_eq!(code(refused), "fleet.session_not_pausable");
    let refused = rig.fleet.resume_session(resume(&bare)).await.expect_err("ended");
    assert_eq!(code(refused), "fleet.session_not_pausable");
}

async fn row_of(rig: &Rig, id: &SessionId) -> SessionRecord {
    std::sync::Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session
}
