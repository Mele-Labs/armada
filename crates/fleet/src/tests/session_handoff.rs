//! The claim, written before the code: **a Job never starts its successor on its own; the owner
//! agrees on the Job's own question card, and then a fresh Job, not a copy of the conversation,
//! takes the old one's place.** The new Job is started first and told the handoff and the brief the
//! old Job wrote in the card; the old Job's ledger is handed to it with `release_session_holds` and
//! its target, and only then is the old Job ended. A hand that fails at any step leaves the old Job
//! live as it was and exactly one Job live, records `handoff_failed` with the reason, and the owner's
//! Retry runs it again. `docs/concepts/session/a-handed-off-job.md`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    AnswerSessionAsk, AskingToRun, AttachmentState, HelmCallAnswer, QuestionAnswer, RetryHandoff, RunOrNot,
    SessionId, SessionState, HANDOFF_STAY, HANDOFF_YES,
};
use store::{AttachmentState as Kept, Holder, KeptAttachment};

use super::agent_starts::{kind, rig, Rig};
use super::session_host::eventually;
use crate::session_host::replacing::FAULT;
use crate::session_host::Start;

const ASK_NO_BRIEF: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_h","input":{"questions":[
  {"question":"Carry on in a new Job?","header":"Context","multiSelect":false,
   "options":[{"label":"Carry on in a new Job","description":""},{"label":"Stay here","description":""}]}]}}"#;

const ASK: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_h","input":{"questions":[
  {"question":"Carry on in a new Job?","header":"Context","multiSelect":false,
   "options":[{"label":"Carry on in a new Job","description":"The fix is on branch fix/reader. The lint run is open."},{"label":"Stay here","description":""}]}]}}"#;

fn asking(ask: &str) -> AskingToRun {
    ipc::decode("an ask", ask.as_bytes()).expect("an ask")
}

/// The Job asks, and the owner presses `label` on the card.
async fn asked_and_answered(rig: &Rig, id: &SessionId, label: &str) -> RunOrNot {
    asked_by(rig, id, label, ASK).await
}

async fn asked_by(rig: &Rig, id: &SessionId, label: &str, ask: &'static str) -> RunOrNot {
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let held = tokio::spawn(async move { fleet.session_permission(&session, asking(ask)).await });
    let call = std::sync::Mutex::new(String::new());
    eventually(|| async {
        let asked = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.hosted.unwrap().asked;
        if let Some(asked) = asked {
            *call.lock().unwrap() = asked.call;
            return true;
        }
        false
    })
    .await;
    let call = call.lock().unwrap().clone();
    Arc::clone(&rig.fleet)
        .answer_session_ask(AnswerSessionAsk {
            session_id: id.clone(),
            call,
            answer: HelmCallAnswer::AllowOnce,
            note: None,
            answers: vec![QuestionAnswer { question: "Carry on in a new Job?".into(), chosen: vec![label.into()] }],
            folder: None,
        })
        .await
        .expect("answered");
    held.await.unwrap().expect("decided")
}

async fn holding_a_pull_request(rig: &Rig, id: &SessionId) {
    let now = String::from("2026-10-18T09:00:00Z");
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(id.as_str()),
                kind: String::from("pr"),
                manifest_id: rig.manifest.as_str().to_string(),
                target: String::from("2430"),
                state: Kept::Standing,
                detail: Default::default(),
                since: now.clone(),
                changed_at: now,
            },
            false,
        )
        .unwrap();
}

async fn others(rig: &Rig, old: &SessionId) -> Vec<ipc::SessionRecord> {
    Arc::clone(&rig.fleet)
        .list_sessions(None, None, None)
        .await
        .expect("listed")
        .sessions
        .into_iter()
        .filter(|one| one.id != *old)
        .collect()
}

fn standing(record: &ipc::SessionRecord, kind: &str) -> Vec<String> {
    record
        .attachments
        .iter()
        .filter(|one| one.kind == kind && one.state == AttachmentState::Standing)
        .map(|one| one.target.clone())
        .collect()
}

fn live(all: &[ipc::SessionRecord]) -> usize {
    all.iter().filter(|one| one.state == SessionState::Live).count()
}

async fn all_of(rig: &Rig) -> Vec<ipc::SessionRecord> {
    Arc::clone(&rig.fleet).list_sessions(None, None, None).await.expect("listed").sessions
}

/// Break one step of the next hand, on this test's thread.
fn breaking(at: Option<&'static str>) {
    FAULT.with(|one| one.set(at));
}

/// The first process the new Job `id` starts, once the owner writes to it.
async fn start_of(rig: &Rig, id: &SessionId) -> Start {
    rig.send_unsettled(id, "go on").await;
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id.as_str()) }).await;
    rig.stand_in.starts().into_iter().find(|one| one.session == id.as_str()).unwrap()
}

#[tokio::test]
async fn the_owners_yes_starts_a_fresh_job_with_the_brief_hands_it_the_ledger_and_ends_the_old_one() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;

    let decided = asked_and_answered(&rig, &old, HANDOFF_YES).await;

    assert!(matches!(decided, RunOrNot::Allow { .. }));
    let made = others(&rig, &old).await;
    assert_eq!(made.len(), 1, "one successor");
    let new = &made[0];
    assert_eq!(new.state, SessionState::Live);
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.title, new.title, "the title carries");
    assert_eq!(kind(new, "replaces").map(|one| one.target.as_str()), Some(old.as_str()));
    assert_eq!(kind(&old_record, "replaced_by").map(|one| one.target.as_str()), Some(new.id.as_str()));
    assert_eq!(standing(new, "pr"), vec!["2430"], "the ledger moved to the new job");
    assert!(standing(&old_record, "pr").is_empty(), "and the old job keeps none");
    assert!(kind(new, "forked_from").is_none(), "not a fork: the conversation is not carried");
    assert_eq!(old_record.state, SessionState::Ended, "the old job is ended last");
    assert!(standing(&old_record, "handoff_starting").is_empty());
    assert!(kind(&old_record, "handoff_failed").is_none());

    // The owner writes first; the process it starts is told the handoff and the brief, and copies nothing.
    let start = start_of(&rig, &new.id).await;
    assert!(start.forking.is_none() && start.fork_cut.is_none(), "no fork of the old thread");
    let told = start.note.expect("a note");
    assert!(told.contains("You are a new job"), "{told}");
    assert!(told.contains("The fix is on branch fix/reader. The lint run is open."), "the brief: {told}");
    assert!(told.contains("Handoff from The agent") && told.contains("Pull request #2430"), "the composed context: {told}");
    assert!(told.contains("Agent: Fixed."), "the last of the thread: {told}");
}

#[tokio::test]
async fn a_card_with_no_brief_still_hands_on_with_the_composed_context() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;

    let decided = asked_by(&rig, &old, HANDOFF_YES, ASK_NO_BRIEF).await;

    assert!(matches!(decided, RunOrNot::Allow { .. }));
    let new = others(&rig, &old).await.remove(0);
    assert_eq!(standing(&new, "pr"), vec!["2430"]);
    let told = start_of(&rig, &new.id).await.note.expect("a note");
    assert!(told.contains("Handoff from The agent") && told.contains("Pull request #2430"), "{told}");
    assert!(!told.contains("Brief from"), "{told}");
}

#[tokio::test]
async fn staying_starts_nothing_and_changes_nothing() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;

    let decided = asked_and_answered(&rig, &old, HANDOFF_STAY).await;

    assert!(matches!(decided, RunOrNot::Allow { .. }));
    assert!(others(&rig, &old).await.is_empty());
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Live);
    assert_eq!(standing(&old_record, "pr"), vec!["2430"]);
    assert!(kind(&old_record, "replaced_by").is_none());
}

#[tokio::test]
async fn a_new_job_that_does_not_start_leaves_the_old_one_as_it_was_and_records_the_failure() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    breaking(Some("start"));

    let decided = asked_and_answered(&rig, &old, HANDOFF_YES).await;
    breaking(None);

    assert!(matches!(decided, RunOrNot::Deny { .. }), "the agent is told it is still here");
    assert!(others(&rig, &old).await.is_empty(), "no successor");
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Live);
    assert_eq!(standing(&old_record, "pr"), vec!["2430"], "nothing moved");
    assert!(kind(&old_record, "replaced_by").is_none());
    assert_eq!(standing(&old_record, "handoff_failed").len(), 1, "the failure is a fact on the ledger");
    let reason = kind(&old_record, "handoff_failed").unwrap().detail.get("reason").cloned().unwrap();
    assert!(reason.starts_with("new job did not start"), "a bare fact, lowercase: {reason}");
    assert!(standing(&old_record, "handoff_starting").is_empty());
    assert!(old_record.hosted.as_ref().unwrap().asked.is_none(), "no second call is raised");
}

#[tokio::test]
async fn a_failure_after_the_new_job_is_up_leaves_exactly_one_live_job_and_the_ledger_where_it_was() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    breaking(Some("close"));

    let decided = asked_and_answered(&rig, &old, HANDOFF_YES).await;
    breaking(None);

    assert!(matches!(decided, RunOrNot::Deny { .. }));
    assert_eq!(live(&all_of(&rig).await), 1, "one live job: the old one");
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Live);
    assert_eq!(standing(&old_record, "pr"), vec!["2430"], "the ledger came back");
    let made = others(&rig, &old).await;
    assert_eq!(made.len(), 1, "the new job was started");
    assert_eq!(made[0].state, SessionState::Ended, "and closed");
    assert!(standing(&made[0], "pr").is_empty());
    let spent = |record: &ipc::SessionRecord, link: &str| {
        record.attachments.iter().any(|one| one.kind == link && one.state == AttachmentState::Spent)
    };
    assert!(!spent(&old_record, "replaced_by") && !spent(&made[0], "replaces"), "no link stands");
    assert_eq!(standing(&old_record, "handoff_failed").len(), 1);
}

#[tokio::test]
async fn retry_after_a_failure_succeeds_and_gives_back_the_failed_row() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    breaking(Some("close"));
    asked_and_answered(&rig, &old, HANDOFF_YES).await;

    // Retry while it still fails: a fresh row replaces the old one, and one job is live.
    let again = Arc::clone(&rig.fleet).retry_handoff(RetryHandoff { session_id: old.clone() }).await;
    assert!(again.is_err(), "still failing");
    assert_eq!(standing(&rig.record(&old).await, "handoff_failed").len(), 1, "a fresh row replaces it");
    assert_eq!(live(&all_of(&rig).await), 1);

    breaking(None);
    let new = Arc::clone(&rig.fleet).retry_handoff(RetryHandoff { session_id: old.clone() }).await.expect("retried");
    assert_eq!(new.state, SessionState::Live);
    assert_ne!(new.id, old);
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Ended);
    assert!(standing(&old_record, "handoff_failed").is_empty(), "the failed row is given back");
    assert_eq!(standing(&rig.record(&new.id).await, "pr"), vec!["2430"]);
    assert_eq!(kind(&old_record, "replaced_by").map(|one| one.target.as_str()), Some(new.id.as_str()));
    assert_eq!(live(&all_of(&rig).await), 1, "one live job: the new one");

    // The brief the card carried survived the failures and reached the retried job.
    let told = start_of(&rig, &new.id).await.note.expect("a note");
    assert!(told.contains("The fix is on branch fix/reader."), "{told}");
}

#[tokio::test]
async fn retry_where_no_hand_off_failed_is_refused() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;

    let refused = Arc::clone(&rig.fleet).retry_handoff(RetryHandoff { session_id: old.clone() }).await;

    assert!(matches!(refused, Err(api::Refusal::IllegalMove(_))), "{refused:?}");
    assert!(others(&rig, &old).await.is_empty());
}
