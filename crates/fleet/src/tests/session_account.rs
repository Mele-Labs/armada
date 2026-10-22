//! The account a small model writes of a Session at work: what it reads, what it may say, and the
//! plan group it names. `docs/concepts/session/working-on.md`.
//!
//! **The model is the fake and the process is the stand-in**: what runs is Fleet's question, its
//! reading of the answer and the Session's row.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    ApprovePlan, PlanGroupInput, PlanTaskInput, PlanTaskState, RecordPlan, SessionAccount, SessionId, SessionRecord,
    SessionRow, SessionVoice,
};
use testkit::FakeJudge;

use super::session_host::{eventually, rig_fitted, Rig};
use super::tmp::TempDir;
use crate::session_account::{read, TurnSoFar};

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
    let reply = "done: Opened the GitHub issue\n\
                 done: Changed `login.tsx`\n\
                 done: Ran the check on crates/fleet\n\
                 done: Renamed sessionActivity everywhere\n\
                 done: Called the TodoWrite tool\n\
                 done: Fixed it in version 5.5\n\
                 done: Used and/or for the labels\n\
                 now: none\n\
                 next: Check the page";
    let written = read(reply, &[]).expect("some lines survive");
    assert_eq!(written.done, ["Opened the GitHub issue", "Fixed it in version 5.5", "Used and/or for the labels"]);
    assert_eq!(written.now, None, "`none` is no line");
    assert_eq!(written.next.as_deref(), Some("Check the page"));
    assert_eq!(read("done: Edited gate.rs", &[]), None, "nothing left, no account");
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
    rig.fleet.record_plan(None, plan).await.expect("recorded");
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
