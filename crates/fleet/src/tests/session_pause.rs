//! Pausing a hosted Session and resuming it. Since 23.92. **A stand-in process and a fake pool**:
//! what is read is the ledger, the row Fleet publishes and the pool's lease.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent, Vcs};
use api::HostedSessions;
use ipc::{
    AttachmentState, PauseSession, ResumeSession, SessionId, SessionRecord, SessionRow, SessionState,
    SessionTurn, SessionVoice,
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
async fn a_resume_takes_a_slot_again_and_clears_the_marker() {
    let rig = rig_placing(None);
    let id = holding_slot_one(&rig).await;
    rig.fleet.pause_session(pause(&id)).await.expect("paused");

    let resumed = Arc::clone(&rig.fleet).resume_session(resume(&id)).await.expect("resumed");

    assert!(resumed.hosted.as_ref().unwrap().paused.is_none());
    assert!(holds(&resumed, "slot") && holds(&resumed, "branch"));
}

#[tokio::test]
async fn a_resume_with_no_turn_kept_tells_the_job_so_it_carries_on() {
    let rig = rig_placing(None);
    let id = paused_job(&rig).await;
    let starts = rig.stand_in.starts().len();

    Arc::clone(&rig.fleet).resume_session(resume(&id)).await.expect("resumed");

    eventually(|| async { rig.stand_in.starts().len() == starts + 1 }).await;
    let start = &rig.stand_in.starts()[starts];
    assert!(start.resuming, "the same conversation");
    assert_eq!(start.directory, adapter_traits::slot_path(&rig.root, 1));
    let said = sent_to(&rig, starts);
    assert!(said.contains("You were paused and have been resumed."), "{said}");
    assert!(rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Message { from: SessionVoice::Session { .. }, text, .. } if text.contains("have been resumed"))), "on the thread as Fleet's");
}

#[tokio::test]
async fn a_resume_sends_the_turns_kept_while_paused_in_order_and_no_resume_line() {
    let rig = rig_placing(None);
    let id = paused_job(&rig).await;
    let starts = rig.stand_in.starts().len();
    rig.send(&id, "first thought").await;
    rig.send(&id, "second thought").await;
    assert_eq!(kept_turns(&rig, &id).await, 2);

    Arc::clone(&rig.fleet).resume_session(resume(&id)).await.expect("resumed");

    eventually(|| async { rig.stand_in.starts().len() == starts + 1 }).await;
    eventually(|| async { sent_to(&rig, starts).contains("second thought") }).await;
    let said = sent_to(&rig, starts);
    assert!(said.find("first thought") < said.find("second thought"), "{said}");
    assert!(!said.contains("have been resumed"), "{said}");
    assert_eq!(kept_turns(&rig, &id).await, 0, "taken");
}

#[tokio::test]
async fn a_message_while_the_resume_waits_for_a_slot_is_kept_and_goes_when_the_line_grants_one() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    rig.fleet.pause_session(pause(&holder)).await.expect("paused");
    rig.fleet.vcs().hold_slot(&rig.root, 1, OTHERS);
    Arc::clone(&rig.fleet).resume_session(resume(&holder)).await.expect("waits");
    let starts = rig.stand_in.starts().len();

    rig.send(&holder, "while you wait").await;

    stays_paused(&rig, &holder, starts).await;
    assert_eq!(kept_turns(&rig, &holder).await, 1);
    rig.fleet.vcs().free_slot(&rig.root, 1);
    rig.fleet.hosts().slot_freed();
    eventually(|| async { rig.stand_in.starts().len() == starts + 1 }).await;
    eventually(|| async { sent_to(&rig, starts).contains("while you wait") }).await;
    keeper.abort();
}

/// A Job that held slot one and was paused.
async fn paused_job(rig: &Rig) -> SessionId {
    let id = holding_slot_one(rig).await;
    rig.fleet.pause_session(pause(&id)).await.expect("paused");
    id
}

/// Nothing woke the Job: it is paused, holds no slot, and no process was started after `starts`.
async fn stays_paused(rig: &Rig, id: &SessionId, starts: usize) {
    let row = row_of(rig, id).await;
    assert!(row.hosted.as_ref().unwrap().paused.is_some(), "still paused");
    assert!(!holds(&row, "slot"), "no slot was taken");
    assert_eq!(rig.stand_in.starts().len(), starts, "no process was started");
}

/// Everything written to the `index`th process the stand-in started.
fn sent_to(rig: &Rig, index: usize) -> String {
    rig.stand_in.process(index).sent.lock().unwrap().join("\n")
}

async fn kept_turns(rig: &Rig, id: &SessionId) -> usize {
    rig.fleet.store().lock().await.held_later_turns(id.as_str()).unwrap().len()
}

#[tokio::test]
async fn a_message_from_a_person_leaves_a_paused_job_paused() {
    let rig = rig_placing(None);
    let id = paused_job(&rig).await;
    let starts = rig.stand_in.starts().len();

    rig.send(&id, "carry on").await;

    stays_paused(&rig, &id, starts).await;
    assert!(rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text == "carry on")), "on the thread");
    assert_eq!(kept_turns(&rig, &id).await, 1, "kept for the resume");
}

#[tokio::test]
async fn a_fleet_notice_leaves_a_paused_job_paused_and_is_asked_again_after_the_resume() {
    let rig = rig_placing(None);
    let id = paused_job(&rig).await;
    let starts = rig.stand_in.starts().len();
    let notice = "slot-3 is 2 commits behind main.";

    let told = Arc::clone(&rig.fleet).told_by_fleet(id.as_str(), notice, ipc::Notice::BehindBase).await.expect("answered");

    assert!(!told, "not told, so the caller asks again later");
    stays_paused(&rig, &id, starts).await;
    assert_eq!(kept_turns(&rig, &id).await, 0, "a notice is retried, not kept");
    assert!(!rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text == notice)), "not on the thread");

    Arc::clone(&rig.fleet).resume_session(resume(&id)).await.expect("resumed");
    let told = Arc::clone(&rig.fleet).told_by_fleet(id.as_str(), notice, ipc::Notice::BehindBase).await.expect("answered");
    assert!(told, "told once it is resumed");
}

#[tokio::test]
async fn a_note_from_helm_leaves_a_paused_job_paused() {
    let rig = rig_placing(None);
    let id = paused_job(&rig).await;
    let starts = rig.stand_in.starts().len();

    Arc::clone(&rig.fleet)
        .session_told(ipc::TellSession { session_id: id.clone(), text: "look at the failing test".into() }, None)
        .await
        .expect("told");

    stays_paused(&rig, &id, starts).await;
    assert_eq!(kept_turns(&rig, &id).await, 1);
}

#[tokio::test]
async fn another_jobs_note_leaves_a_paused_job_paused() {
    let rig = rig_placing(None);
    let sender = rig.start().await;
    rig.send(&sender, "hello").await;
    rig.stand_in.init(0);
    let id = rig.start().await;
    rig.send(&id, "change a file").await;
    rig.stand_in.init(1);
    let path = format!("{}/src/lib.rs", rig.root);
    assert!(denied(&rig.gate(&id, "Edit", &format!(r#"{{"file_path":"{path}"}}"#)).await));
    rig.fleet.pause_session(pause(&id)).await.expect("paused");
    let starts = rig.stand_in.starts().len();

    rig.stand_in.says(
        0,
        vec![DroneEvent::Called {
            tool: "SendMessage".into(),
            call: "toolu_1".into(),
            detail: CallDetail::of(&format!("to {}: please review", crate::session_host::address_of(id.as_str()))),
        }],
    );
    eventually(|| async { kept_turns(&rig, &id).await == 1 }).await;

    stays_paused(&rig, &id, starts).await;
    assert!(rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text == "please review")), "on the thread");
}

#[tokio::test]
async fn a_resume_with_every_slot_held_waits_and_the_first_free_slot_is_its() {
    let (rig, holder) = with_slot_one_held().await;
    let keeper = the_keeper(&rig);
    rig.fleet.pause_session(pause(&holder)).await.expect("paused");
    rig.fleet.vcs().hold_slot(&rig.root, 1, OTHERS);

    let waiting = Arc::clone(&rig.fleet).resume_session(resume(&holder)).await.expect("waits");
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
    Arc::clone(&rig.fleet).resume_session(resume(&holder)).await.expect("waits");

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
    let refused = Arc::clone(&rig.fleet).resume_session(resume(&bare)).await.expect_err("not paused");
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
    let refused = Arc::clone(&rig.fleet).resume_session(resume(&bare)).await.expect_err("ended");
    assert_eq!(code(refused), "fleet.session_not_pausable");
}

async fn row_of(rig: &Rig, id: &SessionId) -> SessionRecord {
    std::sync::Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session
}
