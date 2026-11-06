//! An error Bridge showed the person, as it reaches the Operator: once for the same fault in a local
//! day, first in the order, and by the role's position as a bug report goes. Nothing arrives while the
//! Operator is off.

use ipc::operator_board::OperatorBoardKind;
use ipc::operator_errors::{BridgeErrorField, ReportBridgeError};

use super::autopilot_decisions_repository::jobs_titled;
use super::operator_board::waiting;
use super::operator_notes::filing;
use super::operator_sources::{rulings, switched, Answers};
use super::rule_acting::notices_in;
use super::session_host::{eventually, Rig};
use crate::operator::fingerprint;

const SUBJECT: &str = "Build failed: restart exited 1";

fn card(code: &str, message: &str, job: Option<&str>) -> ReportBridgeError {
    ReportBridgeError {
        code: code.into(),
        message: message.into(),
        fields: vec![BridgeErrorField { key: "exit".into(), value: "1".into() }],
        chain: vec!["restart exited 1".into(), "no such file".into()],
        logs: vec!["/tmp/restart.log".into()],
        commit: Some("e1e0644c02".into()),
        job_id: job.map(str::to_string),
        at: "2026-11-06T09:00:00.000Z".into(),
    }
}

async fn positioned(rig: &Rig, key: &str, position: &str) {
    let save = ipc::SaveSettings { changes: std::collections::BTreeMap::from([(key.to_string(), Some(ipc::SettingValue::Text(position.to_string())))]) };
    api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
}

/// A Job ruled to start, which needs no near Job: ways are 1. file, 2. new job, 3. raise.
const START: &str = "item: 1\nway: 2\nbecause: no Job is near it";
const RAISE: &str = "item: 1\nway: 3\nbecause: it is for him to read";

#[test]
fn what_differs_between_two_of_the_same_fault_is_not_in_its_fingerprint() {
    let same = |one: &str, other: &str| assert_eq!(fingerprint("c", one), fingerprint("c", other), "{one} / {other}");
    same("Build failed: restart exited 1", "Build failed: restart exited 137");
    same("Cannot read /Users/nick/a/run.log", "Cannot read /tmp/b.log");
    same("Worktree 01JABC123XYZ is gone", "Worktree 01JQQQ999ZZZ is gone");
    same("Commit deadbeef12 is missing", "Commit cafebabe99 is missing");
    same("Retry 3 of 5 failed", "Retry 4 of 5 failed");

    assert_ne!(fingerprint("c", "Build failed: restart exited 1"), fingerprint("c", "Build failed: restart was killed"));
    assert_ne!(fingerprint("c", "Build failed"), fingerprint("d", "Build failed"));
    let print = fingerprint("c", "Build failed");
    assert!(print.len() == 16 && print.chars().all(|one| one.is_ascii_hexdigit()), "{print}");
}

#[tokio::test]
async fn a_reported_error_arrives_once_with_the_cards_facts_and_the_same_error_again_does_not_but_another_does() {
    let rig = filing().await;
    let call = Answers::with(&[RAISE, RAISE]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    let taken = rig.fleet.bridge_error_taken(card("bridge.fleet.build_failed", SUBJECT, None)).await;

    assert!(!taken.repeated);
    let parts: Vec<&str> = taken.id.split(':').collect();
    assert!(parts.len() == 3 && parts[0] == "error" && parts[1].len() == 16 && parts[2].len() == 10, "{}", taken.id);
    eventually(|| async { !rulings(&rig).await.is_empty() }).await;
    let asked = call.asked();
    assert_eq!(asked.len(), 1, "{asked:#?}");
    for part in [
        "Bridge showed the person an error, from Bridge: \"Build failed: restart exited 1\"",
        "Code: bridge.fleet.build_failed",
        "Message: Build failed: restart exited 1",
        "Fields: exit: 1",
        "1. restart exited 1 2. no such file",
        "Logs: /tmp/restart.log",
        "Commit: e1e0644c02",
        "Time: 2026-11-06T09:00:00.000Z",
        "1. file a new issue; 2. new job; 3. raise\n",
    ] {
        assert!(asked[0].contains(part), "{part}:\n{}", asked[0]);
    }
    let log = rulings(&rig).await;
    assert_eq!((log[0].arrival.as_str(), log[0].arrived.as_str(), log[0].outcome.as_str()), (taken.id.as_str(), "error", "raise"));

    // The same fault, its number differing, is in the Log that day, so it is not handed again.
    let again = rig.fleet.bridge_error_taken(card("bridge.fleet.build_failed", "Build failed: restart exited 137", None)).await;
    assert_eq!((again.id.as_str(), again.repeated), (taken.id.as_str(), true));
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(call.asked().len(), 1);
    assert_eq!(rulings(&rig).await.len(), 1);

    let other = rig.fleet.bridge_error_taken(card("bridge.not_sent", "No handler registered", None)).await;
    assert!(!other.repeated && other.id != taken.id);
    eventually(|| async { rulings(&rig).await.len() == 2 }).await;
    assert_eq!(call.asked().len(), 2);
}

#[tokio::test]
async fn an_error_goes_ahead_of_what_waits_and_shows_first_in_the_boards_next_and_two_keep_their_order() {
    let waiting = waiting(&[], &[]).await;
    assert_eq!(waiting.next().await, ["question:second", "question:third"]);

    let first = waiting.rig.fleet.bridge_error_taken(card("bridge.fleet.build_failed", SUBJECT, None)).await;
    waiting.waits_for(3).await;
    let second = waiting.rig.fleet.bridge_error_taken(card("bridge.not_sent", "No handler registered", None)).await;
    waiting.waits_for(4).await;

    assert_eq!(waiting.next().await, [first.id.as_str(), second.id.as_str(), "question:second", "question:third"]);
    let board = waiting.rig.fleet.operator_board_wire().await.expect("read");
    assert_eq!((board.next[0].kind, board.next[0].title.as_str()), (OperatorBoardKind::Report, SUBJECT));

    // Waiting already, the same error is not put in twice.
    let repeated = waiting.rig.fleet.bridge_error_taken(card("bridge.fleet.build_failed", "Build failed: restart exited 2", None)).await;
    assert_eq!((repeated.id.as_str(), repeated.repeated), (first.id.as_str(), true));
    assert_eq!(waiting.next().await.len(), 4);
}

#[tokio::test]
async fn with_dispatch_at_suggest_an_error_is_kept_as_a_suggestion_and_a_job_starts_only_on_do_it() {
    let rig = filing().await;
    positioned(&rig, "features.operatorDispatch", "suggest").await;
    assert!(rig.fleet.operator_seated_with(Answers::with(&[START])));

    rig.fleet.bridge_error_taken(card("bridge.fleet.build_failed", SUBJECT, None)).await;

    eventually(|| async { rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").len() == 1 }).await;
    let kept = rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").remove(0);
    assert_eq!(kept.text, format!("The Operator would start a Job on the error \"{SUBJECT}\": no Job is near it"));
    assert_eq!(jobs_titled(&rig, SUBJECT).await, 0, "nothing is started until he presses");

    let _ = std::sync::Arc::clone(&rig.fleet).suggestion_chosen("", &format!("suggestion:{}", kept.n), Some(0)).await;

    eventually(|| async { jobs_titled(&rig, SUBJECT).await == 1 }).await;
}

#[tokio::test]
async fn with_dispatch_on_an_error_ruled_a_new_job_starts_it_at_once_with_every_word_of_the_card_as_its_brief() {
    let rig = filing().await;
    assert!(rig.fleet.operator_seated_with(Answers::with(&[START])));

    rig.fleet.bridge_error_taken(card("bridge.fleet.build_failed", SUBJECT, None)).await;

    eventually(|| async { jobs_titled(&rig, SUBJECT).await == 1 }).await;
    assert!(rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").is_empty());
    let job = {
        let store = rig.fleet.store().lock().await;
        let found = store.find_sessions(&store::SessionSearch { manifest_id: None, state: None, text: None }).expect("read");
        found.into_iter().find(|one| one.title.as_deref() == Some(SUBJECT)).expect("a Job").id
    };
    let brief = eventually_told(&rig, &job).await;
    for part in ["Code: bridge.fleet.build_failed", "exit: 1", "no such file", "/tmp/restart.log", "Commit: e1e0644c02"] {
        assert!(brief.contains(part), "{part}:\n{brief}");
    }
}

async fn eventually_told(rig: &Rig, job: &str) -> String {
    let id = ipc::SessionId::carried(job);
    eventually(|| async { notices_in(rig, &id).await.iter().any(|(_, text)| text.contains("Code: bridge.fleet.build_failed")) }).await;
    notices_in(rig, &id).await.into_iter().map(|(_, text)| text).collect::<Vec<_>>().join("\n")
}

#[tokio::test]
async fn an_error_raised_in_a_live_job_is_weighed_against_it_and_handed_to_it_at_on() {
    let rig = filing().await;
    let near = rig.start_titled(Some("Terminal scrollback loss")).await;
    let ruling = format!("item: 1\nway: 2\njob: {}\nshared: terminal, scrollback\nbecause: its work caused it", near.as_str());
    let call = Answers::with(&[ruling.as_str()]);
    assert!(rig.fleet.operator_seated_with(call.clone()));

    rig.fleet.bridge_error_taken(card("bridge.panes.crashed", "The pane exited early", Some(near.as_str()))).await;

    eventually(|| async { notices_in(&rig, &near).await.iter().any(|(_, text)| text.contains("The Operator handed you an error") && text.contains("Code: bridge.panes.crashed")) }).await;
    let asked = call.asked();
    assert!(asked[0].contains("Raised in Job") && asked[0].contains("Terminal scrollback loss"), "{}", asked[0]);
    assert!(asked[0].contains("2. existing job"), "{}", asked[0]);
}

#[tokio::test]
async fn while_the_operator_is_off_an_error_is_accepted_and_nothing_arrives() {
    let rig = filing().await;
    let call = Answers::with(&[RAISE]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    switched(&rig, "features.operator", false).await;

    let taken = rig.fleet.bridge_error_taken(card("bridge.fleet.build_failed", SUBJECT, None)).await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    assert!(!taken.repeated);
    assert!(call.asked().is_empty());
    assert!(rulings(&rig).await.is_empty());
}
