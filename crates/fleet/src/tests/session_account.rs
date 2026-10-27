//! The account a small model writes of a Session at work: what it reads, what it may say, and the
//! plan group it names. `docs/concepts/session/working-on.md`.
//!
//! **The model is the fake and the process is the stand-in**: what runs is Fleet's question, its
//! reading of the answer and the Session's row.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{
    ApprovePlan, PlanGroupInput, PlanTaskInput, PlanTaskState, RecordPlan, SessionAccount, SessionId, SessionRecord,
    SessionRow, SessionVoice,
};
use testkit::FakeJudge;

use super::session_host::{eventually, rig_fitted, Rig};
use super::tmp::TempDir;
use crate::session_account::{read, TurnSoFar};
use crate::session_host::account::Writing;

/// A fragment only the account's question holds, so the fake answers it and nothing else.
const ASKED: &str = "short account of the agent's progress";

/// What a model might write, with the file names and identifiers the prompt forbids in it.
const LEAKY: &str = "done: Edited gate.rs and src/lib.rs\n\
                     done: Fixed the sign-in page\n\
                     done: Wrote the sessionActivity type\n\
                     now: Running cargo_check\n\
                     next: Tell him what changed";

/// A rig whose model answers the account's question with `answer`, and whose account writer is set
/// to `another-model`.
fn rig_writing(answer: &str) -> (Rig, Arc<FakeJudge>, TempDir) {
    let judge = Arc::new(FakeJudge::answering(&[(ASKED, answer)]));
    let home = TempDir::new();
    let (held, file) = (Arc::clone(&judge), super::settings_json::settings_file_in(&home));
    let rig = rig_fitted(move |fittings| {
        fittings.judge = held;
        let supplied = super::settings_json::supplied().words(config::settings::MODELS_ACCOUNT, "another-model");
        fittings.settings = crate::settings::MachineSettings::at(file, supplied, config::settings::Env::none());
    });
    (rig, judge, home)
}

fn asked(judge: &FakeJudge) -> Vec<String> {
    judge.asked().into_iter().filter(|question| question.contains(ASKED)).collect()
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

async fn account(rig: &Rig, id: &SessionId) -> Option<SessionAccount> {
    record(rig, id).await.activity.and_then(|activity| activity.account)
}

/// A Session in a turn the person began with `words`.
async fn working(rig: &Rig, words: &str) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, words).await;
    rig.stand_in.init(0);
    // The process has said the turn began, which is what clears the last turn's account.
    let runtime = rig.fleet.hosts().of(id.as_str());
    eventually(|| async { runtime.state().turn_began.is_some() }).await;
    id
}

#[tokio::test]
async fn a_turns_tool_calls_give_an_account_in_his_words_with_no_file_names_in_it() {
    let (rig, judge, _home) = rig_writing(LEAKY);
    let id = working(&rig, "make the sign-in page remember me").await;

    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/login.tsx"}"#).await;
    rig.gate(&id, "Edit", r#"{"file_path":"/elsewhere/login.tsx"}"#).await;
    eventually(|| async { account(&rig, &id).await.is_some() }).await;

    let written = account(&rig, &id).await.expect("an account");
    let his = rig.rows(&id).await.into_iter().rev().find_map(|row| match row {
        SessionRow::Message { id, from: SessionVoice::You, .. } => Some(id),
        _ => None,
    });
    assert_eq!(Some(written.for_message.as_str()), his.as_deref(), "it answers his last message");
    assert_eq!(written.done, ["Fixed the sign-in page"], "the lines naming a file or a type are dropped");
    assert_eq!(written.now, None, "a line naming a command's identifier is dropped");
    assert_eq!(written.next.as_deref(), Some("Tell him what changed"));
    assert_eq!(written.group, None, "no plan, no group");

    let questions = asked(&judge);
    assert!(!questions.is_empty(), "the model was asked");
    let question = &questions[0];
    assert!(question.contains("make the sign-in page remember me"), "it reads his words: {question}");
    assert!(question.contains("Editing login.tsx"), "and the calls of the turn: {question}");
    assert!(question.contains("Never write a file name"), "and is told not to repeat them: {question}");
    let models: Vec<String> = judge.models();
    let at = judge.asked().iter().position(|one| one.contains(ASKED)).expect("asked");
    assert_eq!(models[at], "another-model", "the model `models.account` names");
}

#[test]
fn the_question_lays_the_turn_out_and_offers_groups_only_where_there_are_some() {
    let turn = TurnSoFar {
        message: "fix the sign-in page".into(),
        calls: vec!["Reading login.tsx".into(), "Running cargo test".into()],
        said: vec!["Looking at the form first.".into()],
        groups: Vec::new(),
    };
    let plain = turn.question();
    assert!(plain.contains("fix the sign-in page") && plain.contains("- Running cargo test"), "{plain}");
    assert!(plain.contains("Looking at the form first."), "{plain}");
    assert!(!plain.contains("group:"), "no choice to make, no line for it: {plain}");

    let planned = TurnSoFar { groups: vec![("g1".into(), "Store".into()), ("g2".into(), "Page".into())], ..turn };
    let question = planned.question();
    assert!(question.contains("g2 — Page") && question.contains("group: <the id"), "{question}");
}

#[test]
fn a_line_that_names_code_is_dropped_and_a_plain_one_with_a_product_name_is_kept() {
    let reply = "done: Rewrote it in JavaScript\n\
                 done: Changed `login.tsx`\n\
                 done: Ran the check on crates/fleet\n\
                 done: Renamed sessionActivity everywhere\n\
                 done: Called the TodoWrite tool\n\
                 done: Fixed it in version 5.5\n\
                 done: Used and/or for the labels\n\
                 now: none\n\
                 next: Check the page";
    let written = read(reply, &[]).expect("some lines survive");
    assert_eq!(written.done, ["Rewrote it in JavaScript", "Fixed it in version 5.5", "Used and/or for the labels"]);
    assert_eq!(written.now, None, "`none` is no line");
    assert_eq!(written.next.as_deref(), Some("Check the page"));
    assert_eq!(read("done: Edited gate.rs", &[]), None, "nothing left, no account");
}

// ---- How often it is written, and how long it is kept ----

#[tokio::test]
async fn a_burst_of_tool_calls_makes_one_rewrite() {
    let (rig, judge, _home) = rig_writing(PLANNED);
    rig.fleet.hosts().write_account_every(Duration::from_millis(300));
    let id = working(&rig, "record the plan").await;

    for name in ["a.rs", "b.rs", "c.rs", "d.rs"] {
        rig.gate(&id, "Read", &format!(r#"{{"file_path":"/elsewhere/{name}"}}"#)).await;
    }
    eventually(|| async { account(&rig, &id).await.is_some() }).await;
    tokio::time::sleep(Duration::from_millis(900)).await;
    assert_eq!(asked(&judge).len(), 1, "four calls, one rewrite, and it read all four");
    assert!(asked(&judge)[0].contains("Reading d.rs"), "{}", asked(&judge)[0]);

    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/e.rs"}"#).await;
    eventually(|| async { asked(&judge).len() == 2 }).await;
}

#[tokio::test]
async fn calls_that_keep_coming_are_rewritten_no_more_often_than_the_wait() {
    let (rig, judge, _home) = rig_writing(PLANNED);
    rig.fleet.hosts().write_account_every(Duration::from_millis(400));
    let id = working(&rig, "record the plan").await;

    let began = std::time::Instant::now();
    for step in 0..12 {
        rig.gate(&id, "Read", &format!(r#"{{"file_path":"/elsewhere/{step}.rs"}}"#)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    eventually(|| async { account(&rig, &id).await.is_some() }).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let rewrites = asked(&judge).len();
    let at_most = (began.elapsed().as_millis() / 400) as usize + 1;
    assert!((2..=at_most).contains(&rewrites), "{rewrites} rewrites in {:?}", began.elapsed());
}

#[tokio::test]
async fn the_account_is_kept_through_the_end_of_a_turn_and_a_notice_and_goes_with_his_next_message() {
    let (rig, _judge, _home) = rig_writing(PLANNED);
    rig.fleet.hosts().write_account_every(Duration::from_millis(100));
    let id = working(&rig, "record the plan").await;
    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/a.rs"}"#).await;
    eventually(|| async { account(&rig, &id).await.is_some() }).await;
    let runtime = rig.fleet.hosts().of(id.as_str());

    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    let kept = account(&rig, &id).await.expect("a turn ending clears nothing");
    assert_eq!(kept.now.as_deref(), Some("Writing the rest"));

    // A turn Fleet's own notice begins is not his message.
    assert!(rig.fleet.told_by_fleet(id.as_str(), "Commit and push slot 3.", ipc::Notice::SlotStranded).await.expect("told"));
    rig.stand_in.init(0);
    eventually(|| async { runtime.state().turn_began.is_some() }).await;
    assert!(account(&rig, &id).await.is_some(), "a notice's turn leaves it");

    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    rig.send(&id, "now the other thing").await;
    rig.stand_in.init(0);
    eventually(|| async { account(&rig, &id).await.is_none() }).await;
}

#[test]
fn an_answer_that_crossed_the_clear_of_his_next_message_is_dropped() {
    let account = |for_message: &str| SessionAccount {
        for_message: for_message.into(),
        done: Vec::new(),
        now: Some("Making the page".into()),
        next: None,
        group: None,
        at: ipc::Instant::carried("2026-10-22T09:00:00.000Z"),
    };
    let every = Duration::from_secs(5);
    let mut writing = Writing::default();
    writing.person_said("m1", "first");
    writing.turn_began();
    writing.person_said("m2", "second");
    writing.call_seen("Searching");
    // Asked after his second message and before its turn began, so it is read from the first turn.
    let reading = writing.begin(Vec::new()).expect("something to ask");
    writing.turn_began();

    let (changed, again) = writing.finished(reading.epoch, Some(account("m2")), every);
    assert!(!changed && again.is_none(), "an account read from the turn before is not the new turn's");
    assert_eq!(writing.account(), None);
}

// ---- The plan group an account names ----

const PLANNED: &str = "done: Made the tables\nnow: Writing the rest\nnext: Check it";

fn group(title: &str, tasks: &[&str]) -> PlanGroupInput {
    PlanGroupInput {
        id: None,
        title: title.to_string(),
        agent: "rust-engineer".to_string(),
        paths: vec![format!("crates/{}", title.to_lowercase())],
        issue: None,
        gives: None,
        tasks: tasks.iter().map(|title| PlanTaskInput { id: None, title: title.to_string() }).collect(),
    }
}

/// A plan of two groups, g1 Store (t1, t2) and g2 Fleet (t3), recorded and, where `approved`, approved.
async fn planned(rig: &Rig, id: &SessionId, approved: bool) {
    let groups = vec![group("Store", &["Add the tables", "Add the migration"]), group("Fleet", &["Record it"])];
    let plan = RecordPlan { summary: "You get a plan.".into(), groups, sketch: None, walk: None, session_id: Some(id.clone()) };
    rig.fleet.record_plan(None, plan, false).await.expect("recorded");
    if approved {
        Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 1 }).await.expect("approved");
    }
}

/// The agent's own list marks `text` in progress, and Fleet has read it into the plan.
async fn at_work_on(rig: &Rig, id: &SessionId, text: &str) {
    let list = format!(r#"{{"todos":[{{"content":"{text}","status":"in_progress","activeForm":"x"}}]}}"#);
    rig.gate(id, "TodoWrite", &list).await;
    eventually(|| async {
        let plan = record(rig, id).await.plan;
        plan.is_some_and(|plan| plan.groups.iter().flat_map(|one| &one.tasks).any(|task| task.state == PlanTaskState::Working))
    })
    .await;
}

#[tokio::test]
async fn a_working_task_in_group_two_sets_group_two() {
    let (rig, _judge, _home) = rig_writing(PLANNED);
    let id = working(&rig, "record the plan").await;
    planned(&rig, &id, true).await;
    at_work_on(&rig, &id, "Record it").await;

    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/a.rs"}"#).await;
    eventually(|| async { account(&rig, &id).await.is_some() }).await;
    assert_eq!(account(&rig, &id).await.expect("an account").group.as_deref(), Some("g2"));
}

#[tokio::test]
async fn no_approved_plan_sets_no_group_whatever_the_model_names() {
    let (rig, judge, _home) = rig_writing(&format!("{PLANNED}\ngroup: g1"));
    let id = working(&rig, "record the plan").await;
    planned(&rig, &id, false).await;
    at_work_on(&rig, &id, "Record it").await;

    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/a.rs"}"#).await;
    eventually(|| async { account(&rig, &id).await.is_some() }).await;
    assert_eq!(account(&rig, &id).await.expect("an account").group, None, "a plan he has not approved is not the work");
    assert!(asked(&judge).iter().all(|question| !question.contains("Name the one it is on")), "and none is offered");
}

#[tokio::test]
async fn with_no_task_at_work_the_model_picks_among_the_approved_groups() {
    let (rig, judge, _home) = rig_writing(&format!("{PLANNED}\ngroup: g1"));
    let id = working(&rig, "record the plan").await;
    planned(&rig, &id, true).await;

    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/a.rs"}"#).await;
    eventually(|| async { account(&rig, &id).await.is_some() }).await;
    assert_eq!(account(&rig, &id).await.expect("an account").group.as_deref(), Some("g1"), "its pick");
    assert!(asked(&judge)[0].contains("g2 — Fleet"), "from the groups it was offered");
}
