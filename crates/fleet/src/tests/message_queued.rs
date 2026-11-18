//! The claim: **a message to a live Job whose agent is between turns is kept and delivered, and the
//! sender is told it was queued**, never "not reachable". The CLI's own `SendMessage` reaches only a
//! live process; Fleet carries the rest. The mod's ask (`message_reach`) and Fleet's reading of the
//! sender's call settle one carrier between them, in whichever order they come, so the message is
//! carried once. 17 Nov 2026: three sends to a Job between turns, two told "not reachable" and
//! delivered anyway.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use api::HostedSessions;
use ipc::{MessageReach, QuietWhy, SessionId, SessionMode, SessionRow, SessionTurn, SessionVoice, TuneSession};

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::address_of;

/// The connection of the first agent the stand-in starts, pid 1000: the sender's.
fn sender() -> api::Caller {
    api::Caller::at("127.0.0.1:50999".parse().unwrap())
}

/// A rig where only the sender's connection is placed, so no ask is read as another Job's.
fn placed() -> Rig {
    let peers = crate::tests::peer::Placing::nothing();
    peers.holding(1000, 50999, 12345);
    rig_placing(Some(peers))
}

/// `Sender` with a live process (the stand-in's first) and `Reader`, whose process Fleet let go after
/// its turn, as a quiet spell does.
async fn reader_between_turns(rig: &Rig) -> (SessionId, SessionId) {
    let from = rig.start_titled(Some("Sender")).await;
    let to = rig.start_titled(Some("Reader")).await;
    rig.send(&from, "go").await;
    rig.send(&to, "go").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "ready");
    rig.stand_in.init(1);
    rig.stand_in.finishes(1, "ready");
    eventually(|| async { rig.turn(&to).await == SessionTurn::Idle }).await;
    let tune = TuneSession { session_id: to.clone(), model: None, effort: None, mode: SessionMode::Auto };
    rig.fleet.tune_session(tune).await.unwrap();
    assert!(rig.stand_in.process(1).ended.load(std::sync::atomic::Ordering::SeqCst));
    (from, to)
}

async fn asked(rig: &Rig, to: &SessionId, words: &str) -> MessageReach {
    let to = ipc::MessageTo { to: address_of(to.as_str()), message: Some(words.to_string()) };
    Arc::clone(&rig.fleet).message_reach(sender(), to).await.expect("answered")
}

/// The sender's `SendMessage` call, as Fleet reads it off the sender's stream.
fn call_read(rig: &Rig, to: &SessionId, words: &str) {
    let detail = CallDetail::of(&format!("to {}: {words}", address_of(to.as_str())));
    rig.stand_in.says(0, vec![DroneEvent::Called { tool: "SendMessage".into(), call: "toolu_q".into(), detail }]);
}

/// The lines Fleet wrote to the reader's processes that carry `words`.
fn lines_carrying(rig: &Rig, words: &str) -> usize {
    (2..rig.stand_in.starts().len())
        .map(|index| rig.stand_in.process(index).sent.lock().unwrap().iter().filter(|line| line.contains(words)).count())
        .sum()
}

async fn rows_carrying(rig: &Rig, id: &SessionId, words: &str) -> usize {
    rig.rows(id)
        .await
        .iter()
        .filter(|row| matches!(row, SessionRow::Message { from: SessionVoice::Session { .. }, text, .. } if text == words))
        .count()
}

/// Until the sender's call is on its thread, and a little past, so a second delivery would have come.
async fn call_heard(rig: &Rig, from: &SessionId) {
    eventually(|| async { rig.rows(from).await.iter().any(|row| matches!(row, SessionRow::Tool { text, .. } if text.starts_with("SendMessage"))) }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
}

#[tokio::test]
async fn a_message_asked_before_its_call_is_read_is_queued_and_delivered_once() {
    let rig = placed();
    let (from, to) = reader_between_turns(&rig).await;

    let reach = asked(&rig, &to, "are you there").await;
    let address = address_of(to.as_str());
    assert_eq!(
        reach,
        MessageReach::Queued {
            message: format!("Queued for job \"Reader\" ({address}). Its agent is between turns, and Fleet starts it with your message.")
        },
        "the sender is told it was queued for that Job, and why"
    );
    eventually(|| async { lines_carrying(&rig, "are you there") == 1 }).await;
    let resumed = &rig.stand_in.starts()[2];
    assert_eq!(resumed.session, to.as_str());
    assert!(resumed.resuming, "the reader is resumed with the message");

    call_read(&rig, &to, "are you there");
    call_heard(&rig, &from).await;
    assert_eq!(lines_carrying(&rig, "are you there"), 1, "reading the call carries nothing more");
    assert_eq!(rows_carrying(&rig, &to, "are you there").await, 1, "the reader's thread draws it once");
    assert_eq!(rig.stand_in.starts().len(), 3);
}

#[tokio::test]
async fn a_message_whose_call_is_read_first_is_still_told_as_queued() {
    let rig = placed();
    let (from, to) = reader_between_turns(&rig).await;

    call_read(&rig, &to, "are you there");
    eventually(|| async { lines_carrying(&rig, "are you there") == 1 }).await;
    let reach = asked(&rig, &to, "are you there").await;

    assert!(
        matches!(&reach, MessageReach::Queued { message } if message.contains("between turns")),
        "the ask follows what reading the call settled, though the reader now runs: {reach:?}"
    );
    call_heard(&rig, &from).await;
    assert_eq!(lines_carrying(&rig, "are you there"), 1);
    assert_eq!(rows_carrying(&rig, &to, "are you there").await, 1);
}

#[tokio::test]
async fn a_message_to_a_job_whose_process_runs_is_left_to_the_cli() {
    let rig = placed();
    let from = rig.start_titled(Some("Sender")).await;
    let to = rig.start_titled(Some("Reader")).await;
    rig.send(&from, "go").await;
    rig.send(&to, "go").await;
    rig.stand_in.init(0);
    rig.stand_in.init(1);
    rig.stand_in.finishes(1, "ready");
    eventually(|| async { rig.turn(&to).await == SessionTurn::Idle }).await;
    let before = rig.stand_in.process(1).sent.lock().unwrap().len();

    assert_eq!(asked(&rig, &to, "please review").await, MessageReach::Allowed {});
    call_read(&rig, &to, "please review");
    call_heard(&rig, &from).await;

    assert_eq!(rig.stand_in.process(1).sent.lock().unwrap().len(), before, "the CLI's own delivery wakes it, not Fleet's");
    assert_eq!(rows_carrying(&rig, &to, "please review").await, 1, "the reader's thread still draws it");
}

#[tokio::test]
async fn an_ask_without_the_words_is_only_told_it_may_send() {
    let rig = placed();
    let (_, to) = reader_between_turns(&rig).await;

    let to = ipc::MessageTo { to: address_of(to.as_str()), message: None };
    let reach = Arc::clone(&rig.fleet).message_reach(sender(), to).await.expect("answered");

    assert_eq!(reach, MessageReach::Allowed {}, "an older mod sends no words, and Fleet carries the call it reads as before");
    assert_eq!(rig.stand_in.starts().len(), 2);
}

#[tokio::test]
async fn list_agents_is_told_the_jobs_between_turns_and_not_the_asker() {
    let rig = placed();
    let (_, to) = reader_between_turns(&rig).await;

    let quiet = Arc::clone(&rig.fleet).quiet_jobs(sender()).await.expect("answered");

    assert_eq!(quiet.jobs.len(), 1, "the sender's own live Job is not listed: {quiet:?}");
    assert_eq!(quiet.jobs[0].address, address_of(to.as_str()));
    assert_eq!(quiet.jobs[0].title.as_deref(), Some("Reader"));
    assert_eq!(quiet.jobs[0].why, QuietWhy::BetweenTurns);
    let unplaced = api::Caller::at("127.0.0.1:59999".parse().unwrap());
    let quiet = Arc::clone(&rig.fleet).quiet_jobs(unplaced).await.expect("answered");
    assert!(quiet.jobs.is_empty(), "a caller that is no hosted Job, whose messages Fleet does not carry, is told of none");
}
