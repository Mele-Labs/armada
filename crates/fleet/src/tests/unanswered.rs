//! A turn the person began that reaches a question, or its end, with no answer written to him is sent
//! back once. `crate::session_host::unanswered`.
//!
//! The record is Job 7f7f4ae4's of 24 Oct 2026. He asked why commands were recorded; the agent wrote
//! a "Status:" line, ran one command, and asked its question with the answer only in its reasoning,
//! which he never sees. He read the status line and the question, and said "you never answered me".
//!
//! Job 77d7b355 did the same on 2 Nov 2026 past that rule: its progress line was in plain words, not
//! "Status:", and its last question followed his answer to the one before with nothing written.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent, Speaker};
use api::HostedSessions;
use ipc::{AnswerSessionAsk, AskingToRun, HelmCallAnswer, QuestionAnswer, RunOrNot, SessionId, SessionTurn};

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::unanswered::{UNANSWERED_QUESTION, UNANSWERED_TURN};

const ASKED: &str = "What is the purpose in recording commands?";
const STATUS: &str = "Status: answering your question, and fixing two more docs that still say Jobs are told the remembered runs.";
const ANSWER: &str = "Commands were recorded so a folder with no armada.yml still told the next Job how to run its tests.";
/// What Job 77d7b355 wrote before its first question: a progress line in plain words, carried past.
const PROGRESS: &str = "Found the main causes. I'm writing them up now.";
/// An answer that fits on one line.
const ONE_LINE: &str = "Commands were recorded for the next Job's tests, and the Logbook covers that now.";
const QUESTION_ASKED: &str = "Now that the Logbook covers it, what should happen to the command recording itself?";

const QUESTION: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"t2","input":{"questions":[
  {"question":"Now that the Logbook covers it, what should happen to the command recording itself?",
   "header":"Run memory","multiSelect":false,
   "options":[{"label":"Remove it","description":""},{"label":"Leave it as it is","description":""}]}]}}"#;

fn question() -> AskingToRun {
    ipc::decode("an ask", QUESTION.as_bytes()).expect("an ask")
}

/// The same question, as the call `call`.
fn question_called(call: &str) -> AskingToRun {
    let asking = QUESTION.replace("\"t2\"", &format!("\"{call}\""));
    ipc::decode("an ask", asking.as_bytes()).expect("an ask")
}

fn said(text: &str) -> DroneEvent {
    DroneEvent::Said { text: text.into(), by: Speaker::Drone }
}

fn called(tool: &str, call: &str, detail: &str) -> DroneEvent {
    DroneEvent::Called { tool: tool.into(), call: call.into(), detail: CallDetail::of(detail) }
}

fn ended() -> DroneEvent {
    DroneEvent::Ended { turns: 1, cost_micros: 5_000, refusals: 0 }
}

/// How many lines the agent was sent that carry `words`.
fn sent_with(rig: &Rig, words: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(words)).count()
}

/// A Session whose turn on the person's question is running.
async fn asked(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, ASKED).await;
    rig.stand_in.init(0);
    eventually(|| async { rig.fleet.hosts().of(id.as_str()).state().turn_began.is_some() }).await;
    id
}

/// The turn's events up to its question: the status line and one command, as the record has them.
fn up_to_the_question(rig: &Rig, written: Option<&str>) {
    let mut events = vec![
        said(STATUS),
        called("Bash", "t1", "git diff --stat"),
        DroneEvent::Answered { call: "t1".into(), failed: false },
    ];
    events.extend(written.map(said));
    events.push(called("AskUserQuestion", "t2", ""));
    rig.stand_in.says(0, events);
}

/// The turn ended and whatever follows its end has had time to happen.
async fn settled(rig: &Rig, id: &SessionId) {
    eventually(|| async { rig.turn(id).await == SessionTurn::Idle }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
}

#[tokio::test]
async fn a_question_reached_with_only_a_status_line_written_is_sent_back_for_the_answer() {
    let rig = rig_placing(None);
    let id = asked(&rig).await;
    up_to_the_question(&rig, None);

    let answer = rig.fleet.session_permission(id.as_str(), question()).await.unwrap();
    let RunOrNot::Deny { message } = answer else {
        panic!("a question with no answer written before it was put to the person: {answer:?}");
    };
    assert_eq!(message, UNANSWERED_QUESTION);
    assert!(rig.fleet.hosts().asks().waiting().is_empty(), "nothing waits on him");
}

#[tokio::test]
async fn a_question_after_a_written_answer_is_put_to_the_person() {
    let rig = rig_placing(None);
    let id = asked(&rig).await;
    up_to_the_question(&rig, Some(ANSWER));

    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let held = tokio::spawn(async move { fleet.session_permission(&session, question()).await });
    eventually(|| async { !rig.fleet.hosts().asks().waiting().is_empty() }).await;
    held.abort();
}

#[tokio::test]
async fn the_same_question_asked_again_with_still_nothing_written_is_put_to_the_person() {
    let rig = rig_placing(None);
    let id = asked(&rig).await;
    up_to_the_question(&rig, None);
    let first = rig.fleet.session_permission(id.as_str(), question()).await.unwrap();
    assert!(matches!(first, RunOrNot::Deny { .. }), "{first:?}");

    // Sent back once a message: a second refusal would only loop.
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let held = tokio::spawn(async move { fleet.session_permission(&session, question()).await });
    eventually(|| async { !rig.fleet.hosts().asks().waiting().is_empty() }).await;
    held.abort();
}

#[tokio::test]
async fn a_question_reached_with_only_a_plain_progress_line_written_is_sent_back_for_the_answer() {
    let rig = rig_placing(None);
    let id = asked(&rig).await;
    rig.stand_in.says(
        0,
        vec![
            said(PROGRESS),
            called("Bash", "t1", "ps -Ao pid,pcpu,command"),
            DroneEvent::Answered { call: "t1".into(), failed: false },
            called("AskUserQuestion", "t2", ""),
        ],
    );

    let answer = rig.fleet.session_permission(id.as_str(), question()).await.unwrap();
    let RunOrNot::Deny { message } = answer else {
        panic!("a question after a progress line in plain words was put to the person: {answer:?}");
    };
    assert_eq!(message, UNANSWERED_QUESTION);
}

#[tokio::test]
async fn a_lone_line_written_right_before_the_question_is_its_answer() {
    let rig = rig_placing(None);
    let id = asked(&rig).await;
    rig.stand_in.says(
        0,
        vec![
            said(STATUS),
            called("Bash", "t1", "git diff --stat"),
            DroneEvent::Answered { call: "t1".into(), failed: false },
            said(ONE_LINE),
            called("AskUserQuestion", "t2", ""),
        ],
    );

    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let held = tokio::spawn(async move { fleet.session_permission(&session, question()).await });
    eventually(|| async { !rig.fleet.hosts().asks().waiting().is_empty() }).await;
    held.abort();
}

#[tokio::test]
async fn a_question_after_his_answer_to_one_with_nothing_written_since_is_sent_back() {
    let rig = rig_placing(None);
    let id = asked(&rig).await;
    up_to_the_question(&rig, Some(ANSWER));
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let held = tokio::spawn(async move { fleet.session_permission(&session, question()).await });
    eventually(|| async { !rig.fleet.hosts().asks().waiting().is_empty() }).await;

    // He chooses an answer on the card, which is his message in the thread.
    let call = rig.fleet.hosts().asks().waiting()[0].call.clone();
    Arc::clone(&rig.fleet)
        .answer_session_ask(AnswerSessionAsk {
            session_id: id.clone(),
            call,
            answer: HelmCallAnswer::AllowOnce,
            note: None,
            answers: vec![QuestionAnswer { question: QUESTION_ASKED.into(), chosen: vec!["Remove it".into()] }],
            folder: None,
            kind: None,
        })
        .await
        .expect("his answer is taken");
    assert!(matches!(held.await.unwrap().unwrap(), RunOrNot::Allow { .. }));

    // The agent runs a command and asks again, having written him nothing since.
    rig.stand_in.says(
        0,
        vec![
            called("Bash", "t3", "git rm -r runs"),
            DroneEvent::Answered { call: "t3".into(), failed: false },
            called("AskUserQuestion", "t4", ""),
        ],
    );
    let again = rig.fleet.session_permission(id.as_str(), question_called("t4")).await.unwrap();
    let RunOrNot::Deny { message } = again else {
        panic!("a question straight after his answer, with nothing written, was put to the person: {again:?}");
    };
    assert_eq!(message, UNANSWERED_QUESTION);
}

#[tokio::test]
async fn a_turn_the_person_began_that_ends_with_nothing_written_is_reminded_once() {
    let rig = rig_placing(None);
    let id = asked(&rig).await;
    rig.stand_in.says(
        0,
        vec![
            said(STATUS),
            called("Bash", "t1", "git diff --stat"),
            DroneEvent::Answered { call: "t1".into(), failed: false },
            ended(),
        ],
    );
    eventually(|| async { sent_with(&rig, UNANSWERED_TURN) == 1 }).await;
    settled(&rig, &id).await;
    assert_eq!(sent_with(&rig, UNANSWERED_TURN), 1, "one reminder for one message");
}

#[tokio::test]
async fn a_turn_ending_on_a_written_answer_is_not_reminded() {
    let rig = rig_placing(None);
    let id = asked(&rig).await;
    rig.stand_in.finishes(0, ANSWER);
    settled(&rig, &id).await;
    assert_eq!(sent_with(&rig, UNANSWERED_TURN), 0);
}

#[tokio::test]
async fn a_turn_fleet_began_that_ends_with_nothing_written_is_not_reminded() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let told = rig
        .fleet
        .told_by_fleet(id.as_str(), "slot-3 is 2 commits behind main.", ipc::Notice::BehindBase)
        .await
        .unwrap();
    assert!(told);
    rig.stand_in.init(0);
    rig.stand_in.says(0, vec![called("Bash", "t1", "git merge origin/main"), ended()]);
    settled(&rig, &id).await;
    assert_eq!(sent_with(&rig, UNANSWERED_TURN), 0);
}
