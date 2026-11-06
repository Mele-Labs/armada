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
            fixed: None,
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
            fixed: None,
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
        rig.fleet.fake_vcs().hold_slot(&rig.root, n, OTHERS);
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

// ---- Free a held slot, and Create one without keeping the person waiting --------------------

use std::time::Duration;

use ipc::{FreeSlot, SlotKeeps, SessionTurn};

const ITEM: &str = "slot-choice";

/// A pool whose first `jobs` slots are for Jobs this test starts and the rest are held by `OTHERS`,
/// which Fleet does not host.
fn pool_for_holders(jobs: u32) -> super::session_host::Rig {
    let rig = rig_placing(None);
    for n in jobs + 1..=8 {
        rig.fleet.fake_vcs().hold_slot(&rig.root, n, OTHERS);
    }
    rig
}

/// A Job that wrote and was leased the next free slot. Idle and quiet when `finished`, otherwise its
/// turn is still running.
async fn holder(rig: &super::session_host::Rig, text: &str, finished: bool) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, text).await;
    let at = rig.stand_in.starts().len() - 1;
    rig.stand_in.init(at);
    let held = rig.gate(&id, "Edit", &write_to_main(rig)).await;
    assert!(reason(&held).contains("Slot "), "{}", reason(&held));
    if finished {
        // Its turn ends, Fleet moves it into the slot, and the process resumed there ends its turn too.
        rig.stand_in.finishes(at, "done");
        eventually(|| async { resumes(rig, &id).len() == 1 }).await;
        let resumed = rig.stand_in.starts().len() - 1;
        rig.stand_in.init(resumed);
        rig.stand_in.finishes(resumed, "settled in");
        eventually(|| async { rig.turn(&id).await == SessionTurn::Idle }).await;
    }
    id
}

async fn quiet_since(rig: &super::session_host::Rig, id: &SessionId, at: &str) {
    let mut store = rig.fleet.store().lock().await;
    let mut row = store.session(id.as_str()).unwrap().expect("a row");
    row.last_turn_at = Some(at.to_string());
    store.keep_session(&row).unwrap();
}

async fn free(rig: &super::session_host::Rig, waiting: &SessionId, slot: u32) -> Result<SessionRecord, api::Refusal> {
    Arc::clone(&rig.fleet)
        .free_slot(FreeSlot { session_id: waiting.clone(), item_id: ITEM.into(), slot })
        .await
}

/// The Job waits, and Fleet has read who holds the slots.
async fn waiting_with_rows(rig: &super::session_host::Rig) -> (SessionId, ipc::WaitingItem) {
    let (waiting, _) = queued_and_idle(rig, "change a file").await;
    rig.fleet.refresh_slot_holders().await;
    let call = calls(rig, &waiting).await.remove(0);
    (waiting, call)
}

fn slot_of(rig: &super::session_host::Rig, slot: u32) -> Option<String> {
    rig.fleet.fake_vcs().slot_holders(&rig.root)[slot as usize - 1].clone()
}

async fn holds_a_slot(rig: &super::session_host::Rig, id: &SessionId) -> bool {
    let record = record(rig, id).await;
    record.attachments.iter().any(|one| one.kind == "slot" && one.state == ipc::AttachmentState::Standing)
}

#[tokio::test]
async fn the_call_lists_the_hosted_holders_longest_quiet_first_and_says_why_one_cannot_be_freed() {
    let rig = pool_for_holders(3);
    let idle = holder(&rig, "first", true).await;
    let running = holder(&rig, "second", false).await;
    let dirty = holder(&rig, "third", true).await;
    quiet_since(&rig, &idle, "2026-01-01T00:00:00Z").await;
    quiet_since(&rig, &dirty, "2026-03-01T00:00:00Z").await;
    rig.fleet.fake_vcs().dirty_slot(3, true);

    let (_, call) = waiting_with_rows(&rig).await;

    let rows: Vec<_> = call
        .holders
        .iter()
        .map(|one| (one.slot, one.job.id.as_str(), one.quiet_since.as_str(), one.keeps))
        .collect();
    assert_eq!(
        rows[..2],
        [
            (1, idle.as_str(), "2026-01-01T00:00:00Z", None),
            (3, dirty.as_str(), "2026-03-01T00:00:00Z", Some(SlotKeeps::Uncommitted)),
        ]
    );
    assert_eq!((rows[2].0, rows[2].1, rows[2].3), (2, running.as_str(), Some(SlotKeeps::Running)));
    assert_eq!(rows.len(), 3, "the five slots nobody hosted holds have no row: {rows:?}");
    assert_eq!(call.options.len(), 2, "both choices are still sent");
}

#[tokio::test]
async fn freeing_a_slot_moves_the_waiting_job_in_and_leaves_the_holder_live_with_none() {
    let rig = pool_for_holders(1);
    let idle = holder(&rig, "first", true).await;
    let (waiting, _) = waiting_with_rows(&rig).await;
    assert!(holds_a_slot(&rig, &idle).await);

    let answered = free(&rig, &waiting, 1).await.expect("freed");

    assert!(answered.waiting_for.iter().all(|one| one.source != WaitingSource::SlotChoice));
    eventually(|| async { resumes(&rig, &waiting).len() == 1 }).await;
    assert_eq!(resumes(&rig, &waiting)[0].directory, adapter_traits::slot_path(&rig.root, 1));
    assert_eq!(slot_of(&rig, 1).as_deref(), Some(waiting.as_str()));
    assert_eq!(rig.fleet.fake_vcs().parked_slots(), [(1, idle.as_str().to_string())]);
    let kept = record(&rig, &idle).await;
    assert_ne!(kept.state, ipc::SessionState::Ended, "the holder is not ended");
    assert!(!holds_a_slot(&rig, &idle).await, "it has no slot now; its next write leases one");
}

#[tokio::test]
async fn a_free_is_refused_while_the_holder_runs_holds_uncommitted_work_or_lost_the_slot_and_changes_nothing() {
    let rig = pool_for_holders(3);
    let idle = holder(&rig, "first", true).await;
    let running = holder(&rig, "second", false).await;
    let dirty = holder(&rig, "third", true).await;
    rig.fleet.fake_vcs().dirty_slot(3, true);
    let (waiting, _) = waiting_with_rows(&rig).await;

    let code = |refused: Result<SessionRecord, api::Refusal>| refused.expect_err("refused").error().code.clone();
    assert_eq!(code(free(&rig, &waiting, 2).await), "fleet.slot_session_working");
    assert_eq!(code(free(&rig, &waiting, 3).await), "fleet.slot_uncommitted");
    // Slot 1 is leased by somebody else after the rows were read.
    rig.fleet.fake_vcs().free_slot(&rig.root, 1);
    rig.fleet.fake_vcs().hold_slot(&rig.root, 1, OTHERS);
    assert_eq!(code(free(&rig, &waiting, 1).await), "fleet.slot_holder_changed");
    assert_eq!(code(free(&rig, &waiting, 8).await), "fleet.slot_holder_changed", "no row for a slot nobody hosted holds");

    assert!(rig.fleet.fake_vcs().parked_slots().is_empty(), "nothing was parked");
    assert_eq!(slot_of(&rig, 2).as_deref(), Some(running.as_str()));
    assert_eq!(slot_of(&rig, 3).as_deref(), Some(dirty.as_str()));
    assert!(holds_a_slot(&rig, &running).await && holds_a_slot(&rig, &dirty).await);
    assert!(holds_a_slot(&rig, &idle).await, "its record is as it was");
    assert_eq!(calls(&rig, &waiting).await.len(), 1, "the call stands");
    assert!(resumes(&rig, &waiting).is_empty());
}

#[tokio::test]
async fn a_free_for_a_call_that_is_gone_is_the_settled_refusal() {
    let rig = pool_for_holders(1);
    holder(&rig, "first", true).await;
    let (waiting, _) = waiting_with_rows(&rig).await;
    choose(&rig, &waiting, 0).await.expect("waits");

    let refused = free(&rig, &waiting, 1).await.expect_err("refused");

    assert_eq!(refused.error().code, "fleet.session_waiting_unheld");
    assert!(rig.fleet.fake_vcs().parked_slots().is_empty());
}

/// A Create answers before the slot exists: the checkout is held at a gate, and the answer comes back.
#[tokio::test]
async fn create_answers_before_the_slot_is_made_and_the_job_is_moved_in_when_it_is() {
    let (rig, _holder) = with_slot_one_held().await;
    let (waiting, _) = queued_and_idle(&rig, "change a file").await;
    let hold = rig.fleet.fake_vcs().hold_makes();

    let answered = tokio::time::timeout(Duration::from_secs(3), choose(&rig, &waiting, 1))
        .await
        .expect("it answered while the checkout was still under way")
        .expect("answered");

    let call = answered.waiting_for.iter().find(|one| one.source == WaitingSource::SlotChoice).expect("the call stays");
    assert!(call.creating_since.is_some());
    assert!(call.create_refused.is_none());
    eventually(|| async { hold.arrived() == 1 }).await;
    assert!(rig.fleet.fake_vcs().made_slots().is_empty(), "the slot is not there yet");
    assert!(resumes(&rig, &waiting).is_empty());
    // A second press while one is out changes nothing.
    choose(&rig, &waiting, 1).await.expect("answered");
    assert_eq!(hold.arrived(), 1);

    hold.open();

    eventually(|| async { resumes(&rig, &waiting).len() == 1 && calls(&rig, &waiting).await.is_empty() }).await;
    assert_eq!(resumes(&rig, &waiting)[0].directory, adapter_traits::slot_path(&rig.root, 9));
    assert_eq!(rig.fleet.fake_vcs().made_slots(), [9], "one slot was made, however often it was asked");
}

#[tokio::test]
async fn a_slot_that_cannot_be_made_puts_the_call_back_with_the_reason_and_create_works_again() {
    let (rig, _holder) = with_slot_one_held().await;
    let (waiting, _) = queued_and_idle(&rig, "change a file").await;
    rig.fleet.fake_vcs().refuse_next_make("no space left on the disk");

    choose(&rig, &waiting, 1).await.expect("answered");

    eventually(|| async {
        let asked = calls(&rig, &waiting).await;
        asked.len() == 1 && asked[0].create_refused.as_deref() == Some("no space left on the disk")
    })
    .await;
    let call = calls(&rig, &waiting).await.remove(0);
    assert!(call.creating_since.is_none());
    assert!(resumes(&rig, &waiting).is_empty());

    choose(&rig, &waiting, 1).await.expect("pressed again");

    eventually(|| async { resumes(&rig, &waiting).len() == 1 }).await;
    assert_eq!(resumes(&rig, &waiting)[0].directory, adapter_traits::slot_path(&rig.root, 9));
}

#[tokio::test]
async fn a_wait_while_a_slot_is_made_takes_the_call_off_and_the_slot_still_goes_to_that_job() {
    let (rig, _holder) = with_slot_one_held().await;
    let (waiting, _) = queued_and_idle(&rig, "change a file").await;
    let hold = rig.fleet.fake_vcs().hold_makes();
    choose(&rig, &waiting, 1).await.expect("answered");
    eventually(|| async { hold.arrived() == 1 }).await;

    choose(&rig, &waiting, 0).await.expect("waits");
    assert!(calls(&rig, &waiting).await.is_empty());
    hold.open();

    eventually(|| async { resumes(&rig, &waiting).len() == 1 }).await;
    assert_eq!(resumes(&rig, &waiting)[0].directory, adapter_traits::slot_path(&rig.root, 9));
}

#[tokio::test]
async fn a_free_while_a_slot_is_made_serves_the_job_and_the_new_slot_stays_free_for_the_next() {
    let rig = pool_for_holders(1);
    let idle = holder(&rig, "first", true).await;
    let keeper = the_keeper(&rig);
    let (waiting, _) = waiting_with_rows(&rig).await;
    let (next, _) = queued_and_idle(&rig, "next in line").await;
    let hold = rig.fleet.fake_vcs().hold_makes();
    choose(&rig, &waiting, 1).await.expect("answered");
    eventually(|| async { hold.arrived() == 1 }).await;

    // Free is usable meanwhile: nothing here waits for the checkout.
    tokio::time::timeout(Duration::from_secs(3), free(&rig, &waiting, 1))
        .await
        .expect("a free is not kept waiting on the checkout")
        .expect("freed");
    eventually(|| async { resumes(&rig, &waiting).len() == 1 }).await;
    assert_eq!(resumes(&rig, &waiting)[0].directory, adapter_traits::slot_path(&rig.root, 1));
    assert!(!holds_a_slot(&rig, &idle).await);

    hold.open();

    eventually(|| async { resumes(&rig, &next).len() == 1 }).await;
    assert_eq!(resumes(&rig, &next)[0].directory, adapter_traits::slot_path(&rig.root, 9), "the new slot went to the next Job");
    assert_eq!(resumes(&rig, &waiting).len(), 1, "the Job that asked keeps the slot a Free gave it");
    keeper.abort();
}

/// Fleet frees a slot for a person's press and for nothing else.
#[tokio::test]
async fn fleet_never_frees_a_slot_alone() {
    let rig = pool_for_holders(1);
    let idle = holder(&rig, "first", true).await;
    quiet_since(&rig, &idle, "2020-01-01T00:00:00Z").await;
    let keeper = the_keeper(&rig);
    let (waiting, _) = waiting_with_rows(&rig).await;

    rig.fleet.grant_queued_slots().await;
    rig.fleet.refresh_slot_holders().await;
    choose(&rig, &waiting, 0).await.expect("waits");
    rig.fleet.grant_queued_slots().await;
    rig.fleet.refresh_slot_holders().await;
    let (other, _) = queued_and_idle(&rig, "another").await;
    choose(&rig, &other, 1).await.expect("asks for a slot");
    eventually(|| async { resumes(&rig, &other).len() == 1 }).await;

    assert_eq!(slot_of(&rig, 1).as_deref(), Some(idle.as_str()), "a Job quiet for years keeps its slot");
    assert!(rig.fleet.fake_vcs().parked_slots().is_empty());
    assert!(rig.fleet.fake_vcs().released_slots().is_empty());
    assert!(resumes(&rig, &waiting).is_empty());
    keeper.abort();
}
