//! The claim, written before the code: **a Job never starts its successor on its own; the owner
//! agrees on the Job's own question card, and then the new Job takes the old one's place.**
//! The new Job is started first, the old Job's ledger is handed to it with `release_session_holds`
//! and its target, and only then is the old Job ended. A new Job that does not start leaves the
//! old one as it was, and records the failure. `docs/concepts/session/a-handed-off-job.md`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    AnswerSessionAsk, AskingToRun, AttachmentState, HelmCallAnswer, QuestionAnswer, RunOrNot, SessionId,
    SessionState, HANDOFF_STAY, HANDOFF_YES,
};
use store::{AttachmentState as Kept, Holder, KeptAttachment};

use super::agent_starts::{kind, rig, Rig};
use super::session_host::eventually;

const ASK: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_h","input":{"questions":[
  {"question":"Carry on in a new Job?","header":"Context","multiSelect":false,
   "options":[{"label":"Carry on in a new Job","description":""},{"label":"Stay here","description":""}]}]}}"#;

fn asking() -> AskingToRun {
    ipc::decode("an ask", ASK.as_bytes()).expect("an ask")
}

/// The Job asks, and the owner presses `label` on the card.
async fn asked_and_answered(rig: &Rig, id: &SessionId, label: &str) -> RunOrNot {
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let held = tokio::spawn(async move { fleet.session_permission(&session, asking()).await });
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

#[tokio::test]
async fn the_owners_yes_starts_the_new_job_hands_it_the_ledger_and_ends_the_old_one() {
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
    assert!(kind(new, "forked_from").is_some(), "the conversation was carried by the fork");
    assert_eq!(old_record.state, SessionState::Ended, "the old job is ended last");
    assert!(standing(&old_record, "handoff_starting").is_empty());
    assert!(kind(&old_record, "handoff_failed").is_none());
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
    // Nothing of its own written to the transcript yet, so there is no point to carry on from.
    let old = rig.an_agent().await;
    holding_a_pull_request(&rig, &old).await;

    let decided = asked_and_answered(&rig, &old, HANDOFF_YES).await;

    assert!(matches!(decided, RunOrNot::Deny { .. }), "the agent is told it is still here");
    assert!(others(&rig, &old).await.is_empty(), "no successor");
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Live);
    assert_eq!(standing(&old_record, "pr"), vec!["2430"], "nothing moved");
    assert!(kind(&old_record, "replaced_by").is_none());
    assert_eq!(standing(&old_record, "handoff_failed").len(), 1, "the failure is a fact on the ledger");
    assert!(standing(&old_record, "handoff_starting").is_empty());
    assert!(old_record.hosted.as_ref().unwrap().asked.is_none(), "no second call is raised");
}
