//! The claim, written before the code: **a Job never starts its successor on its own; the owner
//! agrees on the Job's own question card, and then a fresh Job, not a copy of the conversation,
//! takes the old one's place.** The new Job is started first and told the handoff and the brief the
//! old Job wrote in the card; the old Job's ledger is handed to it with `release_session_holds` and
//! its target, and only then is the old Job ended. A hand that fails at any step leaves the old Job
//! live as it was and exactly one Job live, records `handoff_failed` with the reason, and the owner's
//! Retry runs it again. `docs/concepts/session/a-handed-off-job.md`.
//!
//! **The rig's call rides on the old Job's process, as a real one does**: when Fleet ends that
//! process, the call is dropped where it stands. A hand run on the call died with it (19 Oct 2026).

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{
    AnswerSessionAsk, AskingToRun, AttachmentState, HelmCallAnswer, QuestionAnswer, ReleaseSessionHolds, RetryHandoff,
    RunOrNot, SessionId, SessionState, StartSession, HANDOFF_STAY, HANDOFF_YES,
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
async fn asked_and_answered(rig: &Rig, id: &SessionId, label: &str) -> Option<RunOrNot> {
    asked_by(rig, id, label, ASK).await
}

/// What the Job's call was answered, or `None` where its process ended first and took the call with
/// it. A yes waits for the hand to settle, and fails rather than hangs where it never does.
async fn asked_by(rig: &Rig, id: &SessionId, label: &str, ask: &'static str) -> Option<RunOrNot> {
    let fleet = Arc::clone(&rig.fleet);
    let session = id.as_str().to_string();
    let process = rig.stand_in.process_of(id.as_str()).expect("the job's process");
    assert!(!process.ended.load(Ordering::SeqCst), "the call comes from a live process");
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
    let decided = loop {
        if held.is_finished() {
            break Some(held.await.unwrap().expect("decided"));
        }
        // The process the call came from is gone, and the connection with it: the call is dropped.
        if process.ended.load(Ordering::SeqCst) {
            held.abort();
            break None;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    };
    if label == HANDOFF_YES {
        eventually(|| async { standing(&rig.record(id).await, "handoff_starting").is_empty() }).await;
    }
    decided
}

/// The old Job's agent was told the work moved, and to stop.
fn told_it_moved(decided: &Option<RunOrNot>) -> bool {
    matches!(decided, Some(RunOrNot::Deny { message }) if message.contains("moved to a new job") && message.contains("Stop here"))
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

/// The first process the new Job `id` starts, with nobody writing to it.
async fn start_of(rig: &Rig, id: &SessionId) -> Start {
    eventually(|| async { rig.stand_in.starts().iter().any(|one| one.session == id.as_str()) }).await;
    rig.stand_in.starts().into_iter().find(|one| one.session == id.as_str()).unwrap()
}

#[tokio::test]
async fn the_owners_yes_starts_a_fresh_job_with_the_brief_hands_it_the_ledger_and_ends_the_old_one() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;

    let decided = asked_and_answered(&rig, &old, HANDOFF_YES).await;

    assert!(told_it_moved(&decided), "the call is answered, not dropped with the process: {decided:?}");
    assert_eq!(live(&all_of(&rig).await), 1, "exactly one live job");
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

    // Nobody writes: it starts on its own, told the handoff and the brief, and copies nothing.
    let start = start_of(&rig, &new.id).await;
    assert!(start.forking.is_none() && start.fork_cut.is_none(), "no fork of the old thread");
    let told = start.note.expect("a note");
    assert!(told.contains("You are a new job"), "{told}");
    assert!(told.contains("The fix is on branch fix/reader. The lint run is open."), "the brief: {told}");
    assert!(told.contains("Handoff from The agent") && told.contains("Pull request #2430"), "the composed context: {told}");
    assert!(told.contains("Agent: Fixed."), "the last of the thread: {told}");
    let first = rig.stand_in.process_of(new.id.as_str()).unwrap().sent.lock().unwrap().join("\n");
    assert!(first.contains("Carry on with the work of the job you took the place of"), "its first turn: {first}");
}

#[tokio::test]
async fn a_card_with_no_brief_still_hands_on_with_the_composed_context() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;

    let decided = asked_by(&rig, &old, HANDOFF_YES, ASK_NO_BRIEF).await;

    assert!(told_it_moved(&decided), "{decided:?}");
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

    assert!(matches!(decided, Some(RunOrNot::Allow { .. })));
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

    assert!(told_it_moved(&decided), "answered before the hand ran: {decided:?}");
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

    assert!(told_it_moved(&decided), "{decided:?}");
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

/// A hand a Fleet left partway, as the one that stopped left it: the new Job made and handed the
/// ledger, the old one live, and `handoff_starting` standing. `named` writes the new Job's id on the
/// mark, as this Fleet does; without it is an older Fleet's mark.
async fn left_partway(rig: &Rig, named: bool) -> (SessionId, SessionId) {
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(rig, &old).await;
    let since = rig.fleet.now().as_str().to_string();
    let new = rig
        .fleet
        .start_session(StartSession {
            manifest_id: rig.manifest.clone(),
            title: Some("The agent".into()),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
        })
        .await
        .expect("started")
        .id;
    rig.fleet
        .release_session_holds(ReleaseSessionHolds { session_id: old.clone(), to_session_id: Some(new.clone()), rows: None })
        .await
        .expect("handed");
    let detail = match named {
        true => [(String::from("new"), new.as_str().to_string())].into(),
        false => Default::default(),
    };
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(old.as_str()),
                kind: String::from("handoff_starting"),
                manifest_id: rig.manifest.as_str().to_string(),
                target: old.as_str().to_string(),
                state: Kept::Standing,
                detail,
                since: since.clone(),
                changed_at: since,
            },
            false,
        )
        .unwrap();
    assert_eq!(live(&all_of(rig).await), 2, "broken as the owner found it");
    (old, new)
}

async fn rolled_back_at_boot(named: bool) {
    let rig = rig(None);
    let (old, new) = left_partway(&rig, named).await;

    assert_eq!(rig.fleet.handoffs_healed().await, 1);

    assert_eq!(live(&all_of(&rig).await), 1, "exactly one live job");
    let old_record = rig.record(&old).await;
    assert_eq!(old_record.state, SessionState::Live, "the old job stands");
    assert_eq!(rig.record(&new).await.state, SessionState::Ended, "the new one is closed");
    assert_eq!(standing(&old_record, "pr"), vec!["2430"], "the ledger came back");
    assert!(standing(&old_record, "handoff_starting").is_empty());
    let reason = kind(&old_record, "handoff_failed").and_then(|one| one.detail.get("reason").cloned());
    assert_eq!(reason.as_deref(), Some("fleet stopped during the hand off"));
    assert_eq!(standing(&old_record, "handoff_failed").len(), 1, "for its Retry");
}

#[tokio::test]
async fn a_hand_left_standing_at_boot_is_rolled_back_to_one_live_job_with_handoff_failed() {
    rolled_back_at_boot(true).await;
}

#[tokio::test]
async fn a_hand_an_older_fleet_left_standing_at_boot_finds_its_new_job_and_is_rolled_back() {
    rolled_back_at_boot(false).await;
}

// ---- The four answers, and the Large context setting ----

const ASK_FOUR: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_h","input":{"questions":[
  {"question":"Carry on in a new Job?","header":"Context","multiSelect":false,
   "options":[{"label":"Carry on in a new Job","description":"The fix is on branch fix/reader."},{"label":"Stay here","description":""},
              {"label":"Stay, don't ask again this Job","description":""},{"label":"Remind me at a higher limit","description":""}]}]}}"#;

/// The card standing on `id` now, once one is raised whose question is `question`.
async fn card_of(rig: &Rig, id: &SessionId, question: &str) -> ipc::HelmCallInFlight {
    let found = std::sync::Mutex::new(None);
    eventually(|| async {
        let asked = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.hosted.unwrap().asked;
        let here = asked.filter(|one| one.questions.iter().any(|asked| asked.question == question));
        let held = here.is_some();
        *found.lock().unwrap() = here;
        held
    })
    .await;
    found.into_inner().unwrap().unwrap()
}

/// The owner presses `label` under `question` on the card standing on `id`.
async fn pressed(rig: &Rig, id: &SessionId, question: &str, label: &str) {
    let card = card_of(rig, id, question).await;
    Arc::clone(&rig.fleet)
        .answer_session_ask(AnswerSessionAsk {
            session_id: id.clone(),
            call: card.call,
            answer: HelmCallAnswer::AllowOnce,
            note: None,
            answers: vec![QuestionAnswer { question: question.into(), chosen: vec![label.into()] }],
            folder: None,
        })
        .await
        .expect("answered");
}

/// The Job's call, held on a task of its own while the owner answers.
fn asking_now(rig: &Rig, id: &SessionId) -> tokio::task::JoinHandle<Result<RunOrNot, api::Refusal>> {
    let (fleet, session) = (Arc::clone(&rig.fleet), id.as_str().to_string());
    tokio::spawn(async move { fleet.session_permission(&session, asking(ASK_FOUR)).await })
}

/// One main-thread turn of the Job holding `context` tokens, at `minute` past eight, read by the rules.
async fn turn_heard(rig: &Rig, id: &SessionId, minute: u32, context: u64) {
    let turn = store::KeptTurnUsage {
        message_id: format!("m{minute}"),
        at: format!("2026-10-18T{:02}:{:02}:00.000Z", 8 + minute / 60, minute % 60),
        model: "model-a".into(),
        input_tokens: 0,
        cache_creation_tokens: 0,
        cache_read_tokens: context,
        output_tokens: 1_000,
        tools: vec!["Edit".into()],
        subagent: false,
        bash_use: None,
    };
    rig.fleet.store().lock().await.keep_turn_usage(id.as_str(), &turn).unwrap();
    rig.fleet.usage_rules_read(&[id.as_str().to_string()]).await;
}

/// What Fleet said in the Job's thread that starts with `opening`.
async fn fleet_said(rig: &Rig, id: &SessionId, opening: &str) -> Vec<String> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            ipc::SessionRow::Message { text, from: ipc::SessionVoice::Session { .. }, .. } if text.starts_with(opening) => Some(text),
            _ => None,
        })
        .collect()
}

async fn handoff_set(rig: &Rig, value: &str) {
    let changes = [(String::from("plan.contextHandoff"), Some(ipc::SettingValue::Text(value.into())))].into();
    api::Settings::save_settings(rig.fleet.as_ref(), ipc::SaveSettings { changes }).await.expect("saved");
}

#[tokio::test]
async fn the_context_tell_names_the_four_answers_and_the_card_carries_them() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;

    turn_heard(&rig, &old, 0, 312_000).await;

    let told = fleet_said(&rig, &old, "Context is").await;
    assert_eq!(told.len(), 1, "told once: {told:?}");
    let labels = [HANDOFF_YES, HANDOFF_STAY, ipc::HANDOFF_QUIET, ipc::HANDOFF_REMIND];
    for label in labels {
        assert!(told[0].contains(&format!("\"{label}\"")), "{label} in {}", told[0]);
    }
    assert!(told[0].contains(&format!("brief (where the work is, what is open, what you hold) in the description of \"{HANDOFF_YES}\"")));

    let held = asking_now(&rig, &old);
    let card = card_of(&rig, &old, "Carry on in a new Job?").await;
    let drawn: Vec<&str> = card.questions[0].options.iter().map(|one| one.label.as_str()).collect();
    assert_eq!(drawn, labels);
    pressed(&rig, &old, "Carry on in a new Job?", HANDOFF_STAY).await;
    assert!(matches!(held.await.unwrap(), Ok(RunOrNot::Allow { .. })));
}

#[tokio::test]
async fn stay_and_do_not_ask_again_is_kept_on_the_ledger_and_the_job_is_never_told_again() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    turn_heard(&rig, &old, 0, 312_000).await;

    let held = asking_now(&rig, &old);
    pressed(&rig, &old, "Carry on in a new Job?", ipc::HANDOFF_QUIET).await;
    assert!(matches!(held.await.unwrap(), Ok(RunOrNot::Allow { .. })), "an ordinary answer");

    let record = rig.record(&old).await;
    assert!(kind(&record, "handoff_quiet").is_some(), "the owner's answer is a fact on the Job's ledger");
    assert!(others(&rig, &old).await.is_empty(), "nothing started");
    // Under the line, then over it again past the cooldown: the rule would re-arm, and stays quiet.
    turn_heard(&rig, &old, 1, 80_000).await;
    turn_heard(&rig, &old, 45, 340_000).await;
    turn_heard(&rig, &old, 46, 700_000).await;
    assert_eq!(fleet_said(&rig, &old, "Context is").await.len(), 1, "never told again");

    // Another Job is told as before: the answer was this Job's alone.
    let other = rig
        .fleet
        .start_session(StartSession {
            manifest_id: rig.manifest.clone(),
            title: Some("Another".into()),
            model: None,
            effort: None,
            mode: None,
            root: None,
            fork: None,
            request: None,
            after: None,
        })
        .await
        .expect("started")
        .id;
    turn_heard(&rig, &other, 0, 312_000).await;
    assert_eq!(fleet_said(&rig, &other, "Context is").await.len(), 1);
}

#[tokio::test]
async fn remind_me_asks_again_at_on_the_same_card_and_the_job_is_told_only_past_the_figure() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    turn_heard(&rig, &old, 0, 312_000).await;

    let held = asking_now(&rig, &old);
    pressed(&rig, &old, "Carry on in a new Job?", ipc::HANDOFF_REMIND).await;

    // Fleet's follow-on, put to the owner as the next card on the same Job, numbered like the answers.
    let follow = card_of(&rig, &old, ipc::HANDOFF_REMIND_QUESTION).await;
    let figures: Vec<&str> = follow.questions[0].options.iter().map(|one| one.label.as_str()).collect();
    assert_eq!(figures, ["450k", "500k", "600k"]);
    assert!(!held.is_finished(), "the Job's call waits on the owner's figure");
    pressed(&rig, &old, ipc::HANDOFF_REMIND_QUESTION, "500k").await;

    let Ok(RunOrNot::Allow { updated_input }) = held.await.unwrap() else { panic!("an ordinary answer") };
    let answers = &updated_input["answers"];
    assert_eq!(answers["Carry on in a new Job?"], ipc::HANDOFF_REMIND);
    assert_eq!(answers[ipc::HANDOFF_REMIND_QUESTION], "500k", "the agent reads the figure too");
    let record = rig.record(&old).await;
    assert_eq!(standing(&record, "handoff_remind_at"), vec!["500000"], "the figure is kept on the ledger");
    assert!(record.hosted.as_ref().unwrap().asked.is_none(), "the card is gone");

    turn_heard(&rig, &old, 1, 480_000).await;
    assert_eq!(fleet_said(&rig, &old, "Context is").await.len(), 1, "under the figure: not told");
    turn_heard(&rig, &old, 2, 512_000).await;
    let told = fleet_said(&rig, &old, "Context is").await;
    assert_eq!(told.len(), 2, "past it: told again {told:?}");
    assert!(told[1].starts_with("Context is 512k tokens."));
    assert!(standing(&rig.record(&old).await, "handoff_remind_at").is_empty(), "the reminder is spent");
    turn_heard(&rig, &old, 3, 530_000).await;
    assert_eq!(fleet_said(&rig, &old, "Context is").await.len(), 2, "once");
}

#[tokio::test]
async fn set_to_never_no_job_is_told() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    handoff_set(&rig, "never").await;

    turn_heard(&rig, &old, 0, 312_000).await;

    assert!(fleet_said(&rig, &old, "Context is").await.is_empty());
    assert!(fleet_said(&rig, &old, "Reply with your brief").await.is_empty());
    assert!(others(&rig, &old).await.is_empty());
}

#[tokio::test]
async fn set_to_carry_on_automatically_the_job_is_asked_its_brief_and_handed_on_once_with_no_card() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    handoff_set(&rig, "carry on automatically").await;

    turn_heard(&rig, &old, 0, 312_000).await;

    assert!(fleet_said(&rig, &old, "Context is").await.is_empty(), "the Job is not told to ask");
    eventually(|| async { !fleet_said(&rig, &old, "Reply with your brief").await.is_empty() }).await;
    assert!(rig.record(&old).await.hosted.unwrap().asked.is_none(), "no card");
    rig.stand_in.finishes(0, "The fix is on branch fix/reader. The lint run is open.");

    eventually(|| async { rig.record(&old).await.state == SessionState::Ended }).await;
    eventually(|| async { standing(&rig.record(&old).await, "handoff_starting").is_empty() }).await;
    let made = others(&rig, &old).await;
    assert_eq!(made.len(), 1, "one successor");
    assert_eq!(live(&all_of(&rig).await), 1, "exactly one live job");
    assert_eq!(standing(&made[0], "pr"), vec!["2430"], "the ledger moved");
    let told = start_of(&rig, &made[0].id).await.note.expect("a note");
    assert!(told.contains("The fix is on branch fix/reader. The lint run is open."), "the brief it replied: {told}");

    // The old Job's later turns start nothing more.
    turn_heard(&rig, &old, 45, 340_000).await;
    turn_heard(&rig, &old, 46, 360_000).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(others(&rig, &old).await.len(), 1, "handed on once");
    assert_eq!(live(&all_of(&rig).await), 1);
}

/// Bridge's answer to a question kept for a walk: the person's message, a line a question
/// (`lapsedAnswerText`, `packages/screens/src/sessions-wire.ts`).
fn answered_later(question: &str, label: &str) -> String {
    format!("Answering your earlier question \"{question}\": {label}")
}

const KEPT_ASK: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_k","input":{"questions":[
  {"question":"Carry on in a new Job?","header":"Context","multiSelect":false,
   "options":[{"label":"Carry on in a new Job","description":"The fix is on branch fix/reader. The lint run is open."},
              {"label":"Stay here","description":""},
              {"label":"Stay, don't ask again this Job","description":""}]}]}}"#;

/// The Job asks while the owner walks a page: the question is refused and kept for Needs you.
async fn kept_for_a_walk(rig: &Rig, id: &SessionId) {
    let shown = ipc::ShowWindow {
        url: "http://localhost:5173/one".into(),
        title: None,
        session_id: Some(id.clone()),
        variations: Vec::new(),
        ..Default::default()
    };
    rig.fleet.show_window(None, shown).await.expect("shown");
    let decided = rig.fleet.session_permission(id.as_str(), asking(KEPT_ASK)).await.expect("decided");
    assert!(matches!(decided, RunOrNot::Deny { .. }), "kept, not held: {decided:?}");
}

#[tokio::test]
async fn carry_on_answered_from_needs_you_after_a_walk_starts_the_successor_with_the_brief() {
    let rig = rig(None);
    let (old, _) = rig.an_agent_that_has_spoken().await;
    holding_a_pull_request(&rig, &old).await;
    kept_for_a_walk(&rig, &old).await;

    rig.send(&old, &answered_later("Carry on in a new Job?", HANDOFF_YES)).await;

    eventually(|| async { rig.record(&old).await.state == SessionState::Ended }).await;
    eventually(|| async { standing(&rig.record(&old).await, "handoff_starting").is_empty() }).await;
    let made = others(&rig, &old).await;
    assert_eq!(made.len(), 1, "one successor");
    assert_eq!(live(&all_of(&rig).await), 1, "exactly one live job");
    assert_eq!(standing(&made[0], "pr"), vec!["2430"], "the ledger moved");
    let told = start_of(&rig, &made[0].id).await.note.expect("a note");
    assert!(told.contains("The fix is on branch fix/reader. The lint run is open."), "the kept card's brief: {told}");
}

#[tokio::test]
async fn do_not_ask_again_answered_from_needs_you_after_a_walk_is_kept() {
    let rig = rig(None);
    let (quiet, _) = rig.an_agent_that_has_spoken().await;
    kept_for_a_walk(&rig, &quiet).await;
    rig.send(&quiet, &answered_later("Carry on in a new Job?", ipc::HANDOFF_QUIET)).await;
    let record = rig.record(&quiet).await;
    assert!(kind(&record, "handoff_quiet").is_some(), "the owner's answer is a fact on the Job's ledger");
    assert_eq!(record.state, SessionState::Live);
}

#[tokio::test]
async fn stay_answered_from_needs_you_after_a_walk_starts_nothing_and_is_not_answered_twice() {
    let rig = rig(None);
    let (stay, _) = rig.an_agent_that_has_spoken().await;
    kept_for_a_walk(&rig, &stay).await;
    rig.send(&stay, &answered_later("Carry on in a new Job?", HANDOFF_STAY)).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let record = rig.record(&stay).await;
    assert_eq!(record.state, SessionState::Live, "staying ends nothing");
    assert!(kind(&record, "handoff_quiet").is_none() && kind(&record, "replaced_by").is_none());
    assert_eq!(live(&all_of(&rig).await), 1, "nothing started");

    // The words alone, with no kept question behind them, start nothing.
    rig.send(&stay, &answered_later("Carry on in a new Job?", HANDOFF_YES)).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(rig.record(&stay).await.state, SessionState::Live, "a question answered once is not answered again");
}
