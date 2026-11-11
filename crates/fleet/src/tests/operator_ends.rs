//! A hosted Job ending and a pull request merging, as the Operator hears them: once each, for every Job
//! whoever started it, **with no call, no card and no suggestion**. Each stays in what Discussion reads,
//! the Jobs ended today and the pull requests merged today.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{AttachmentReport, CloseSession, SessionFact, SessionId, SessionRecord, SessionReport};

use super::operator_notes::filing;
use super::operator_sources::{rulings, switched, Answers};
use super::session_host::Rig;
use crate::operator::keep_the_ends_told;

const NOW: &str = "2026-11-06T09:00:00.000Z";
/// What a call that could rule on an end would say, which nothing may ask for.
const NOTED: &str = "item: 1\nway: 3\njob: none\nshared: none\nbecause: nothing follows";

async fn end(rig: &Rig, id: &SessionId) {
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.expect("closed");
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

/// What the mod tells Fleet of Job `id`.
async fn report(rig: &Rig, id: &SessionId, fact: SessionFact) {
    let harness = record(rig, id).await.harness;
    rig.fleet.report_session(SessionReport { harness, session_id: id.clone(), fact }).await.expect("reported");
}

/// A pull request of Job `id` as the forge describes it, then settled spent, as the mod tells one.
pub(super) async fn merge(rig: &Rig, id: &SessionId, number: &str, title: &str) {
    let detail = [("state".to_string(), "merged".to_string()), ("title".to_string(), title.to_string())].into();
    report(rig, id, SessionFact::Attached { attachment: AttachmentReport { kind: "pr".into(), target: number.into(), detail } }).await;
    report(rig, id, SessionFact::Settled { attachment: ipc::AttachmentNamed { kind: "pr".into(), target: number.into() }, state: ipc::AttachmentState::Spent }).await;
}

async fn pause() {
    tokio::time::sleep(Duration::from_millis(200)).await;
}

/// Whether the tap heard `job`'s end (`key` empty) or the merge of pull request `key`.
async fn heard(rig: &Rig, job: &SessionId, event: &str, key: &str) -> bool {
    rig.fleet.store().lock().await.linked_was_told(job.as_str(), event, key).expect("read")
}

async fn quiet(rig: &Rig, call: &Answers) {
    pause().await;
    assert!(call.asked().is_empty(), "{:#?}", call.asked());
    assert!(rulings(rig).await.is_empty(), "no Log row");
    assert!(rig.fleet.store().lock().await.operator_suggestions_of("").expect("read").is_empty(), "no suggestion");
    assert!(rig.fleet.operator_board_wire().await.expect("read").now.is_none());
}

#[tokio::test]
async fn an_end_after_the_tap_started_is_heard_once_and_makes_no_call() {
    let rig = filing().await;
    let before = rig.start_titled(Some("Old work")).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    end(&rig, &before).await;
    let call = Answers::with(&[NOTED, NOTED]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;
    rig.fleet.published_hosted(before.as_str()).await.expect("published");

    rig.fleet.store().lock().await.keep_done(job.as_str(), "2026-11-06T09:00:00.000Z", Some("Moved the pane to the store.")).expect("kept");
    end(&rig, &job).await;

    quiet(&rig, &call).await;
    assert!(heard(&rig, &job, "operator_ended", "").await, "the end was heard");
    // Published again, it is heard no more: nothing new is kept and nothing asked.
    rig.fleet.published_hosted(job.as_str()).await.expect("published");
    quiet(&rig, &call).await;
}

#[tokio::test]
async fn a_pull_request_merging_after_the_tap_started_is_heard_once_and_makes_no_call() {
    let rig = filing().await;
    let job = rig.start_titled(Some("Build the pane")).await;
    merge(&rig, &job, "100", "An older change").await;
    let call = Answers::with(&[NOTED, NOTED]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;
    rig.fleet.published_hosted(job.as_str()).await.expect("published");

    merge(&rig, &job, "3042", "Make the pane keep scrollback").await;

    quiet(&rig, &call).await;
    assert!(heard(&rig, &job, "operator_merged", "3042").await, "the merge was heard");
    merge(&rig, &job, "3042", "Make the pane keep scrollback").await;
    quiet(&rig, &call).await;
}

#[tokio::test]
async fn an_end_and_a_merge_are_heard_while_the_operator_is_off_and_make_no_call_when_it_comes_on() {
    let rig = filing().await;
    let job = rig.start_titled(Some("Build the pane")).await;
    let call = Answers::with(&[NOTED]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;
    switched(&rig, "features.operator", false).await;

    merge(&rig, &job, "3042", "Make the pane keep scrollback").await;
    end(&rig, &job).await;
    pause().await;
    assert!(heard(&rig, &job, "operator_merged", "3042").await && heard(&rig, &job, "operator_ended", "").await);

    switched(&rig, "features.operator", true).await;
    rig.fleet.published_hosted(job.as_str()).await.expect("published");
    quiet(&rig, &call).await;
}

#[tokio::test]
async fn a_job_handed_on_for_context_is_no_end_and_is_not_heard() {
    let rig = filing().await;
    let job = rig.start_titled(Some("Build the pane")).await;
    let call = Answers::with(&[NOTED]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;

    // Its record as it ends with a successor, which `replaced_by` names.
    let mut ended = record(&rig, &job).await;
    ended.state = ipc::SessionState::Ended;
    let successor = ipc::Attachment {
        kind: "replaced_by".to_string(),
        manifest_id: None,
        target: "01NEXT".to_string(),
        state: ipc::AttachmentState::Standing,
        detail: BTreeMap::new(),
        since: ipc::Instant::carried(NOW),
        changed_at: ipc::Instant::carried(NOW),
    };
    let mut with = ended.clone();
    with.attachments.push(successor);
    rig.fleet.events().publish(ipc::Event::SessionChanged(with));
    pause().await;
    assert!(!heard(&rig, &job, "operator_ended", "").await, "its work goes on in its successor");

    // The same record with nothing handed on is an end, so the quiet above was the rule and not the test.
    rig.fleet.events().publish(ipc::Event::SessionChanged(ended));
    pause().await;
    assert!(heard(&rig, &job, "operator_ended", "").await);
    assert!(call.asked().is_empty());
}

#[tokio::test]
async fn an_ended_job_and_a_merged_pull_request_still_show_in_a_talk_question() {
    let rig = filing().await;
    let job = rig.start_titled(Some("Build the pane")).await;
    let call = Answers::with(&["cites: none\njob: none\nstart: none\norder: none\nfirst: none\nsaid: The pane landed."]);
    assert!(rig.fleet.operator_seated_with(call.clone()));
    keep_the_ends_told(Arc::clone(&rig.fleet)).await;
    merge(&rig, &job, "3042", "Make the pane keep scrollback").await;
    rig.fleet.store().lock().await.keep_done(job.as_str(), NOW, Some("Moved the pane to the store.")).expect("kept");
    end(&rig, &job).await;
    pause().await;
    assert!(rulings(&rig).await.is_empty(), "an end and a merge are no Log row");

    rig.fleet.operator_talked("what finished today?", None).await.expect("answered");

    let asked = call.asked();
    assert_eq!(asked.len(), 1, "{asked:#?}");
    assert!(asked[0].contains(&format!("Ended today:\n  Job {} \"Build the pane\": Moved the pane to the store.", job.as_str())), "{}", asked[0]);
    assert!(asked[0].contains(&format!("Merged today:\n  #3042 \"Make the pane keep scrollback\", held by Job {} \"Build the pane\"", job.as_str())), "{}", asked[0]);
}
