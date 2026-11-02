//! The account a small model writes of a Session at work: what it reads, what it may say, and the
//! plan group it names. `docs/concepts/session/working-on.md`.
//!
//! **The model is the fake and the process is the stand-in**: what runs is Fleet's question, its
//! reading of the answer and the Session's row.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{DroneEvent, Speaker};
use api::{HostedSessions, Sessions};
use ipc::{
    ApprovePlan, AskingToRun, PlanGroupInput, PlanTaskInput, PlanTaskState, RecordPlan, SessionAccount, SessionId,
    SessionRecord, SessionRow, SessionTurn, SessionVoice,
};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};

use super::session_host::{eventually, rig_fitted, Rig};
use super::tmp::TempDir;
use crate::session_account::{read, read_so_far, JobSoFar, TurnSoFar};
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
        lands_in: None,
        own_job: false,
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

// ---- The Job's running summary, `so_far` ----

/// A fragment only the summary's question holds. The account's question does not carry it.
const SO_FAR: &str = "what it has done so far";

const SUMMARY: &str = "Made the sign-in page remember him.";

/// A question the agent puts to the person, as the permission door gets it.
const ASK: &str = r#"{"tool_name":"AskUserQuestion","tool_use_id":"toolu_1","input":{"questions":[
  {"question":"Which size?","header":"Size","multiSelect":false,
   "options":[{"label":"S","description":"Small"},{"label":"L","description":"Large"}]}]}}"#;

/// The fittings that put `judge` on the account's model, the one the summary is asked of too.
fn judged_by(
    judge: Arc<FakeJudge>,
    file: std::path::PathBuf,
) -> impl FnOnce(&mut crate::daemon::Fittings<FakeHarness, FakeVcs, FakeWorkProduct>) {
    move |fittings| {
        fittings.judge = judge;
        let supplied = super::settings_json::supplied().words(config::settings::MODELS_ACCOUNT, "another-model");
        fittings.settings = crate::settings::MachineSettings::at(file, supplied, config::settings::Env::none());
    }
}

/// A rig whose model answers the account's question with a plan and the summary's with `summary`.
fn rig_summarising(summary: &str) -> (Rig, Arc<FakeJudge>, TempDir) {
    let judge = Arc::new(FakeJudge::answering(&[(ASKED, PLANNED), (SO_FAR, summary)]));
    let home = TempDir::new();
    let rig = rig_fitted(judged_by(Arc::clone(&judge), super::settings_json::settings_file_in(&home)));
    (rig, judge, home)
}

fn summaries(judge: &FakeJudge) -> Vec<String> {
    judge.asked().into_iter().filter(|question| question.contains(SO_FAR)).collect()
}

async fn so_far(rig: &Rig, id: &SessionId) -> Option<String> {
    record(rig, id).await.so_far
}

/// His message to a Job whose process is not running, and the turn it begins.
async fn begun_by(rig: &Rig, id: &SessionId, words: &str) {
    rig.send(id, words).await;
    rig.stand_in.init(0);
    let runtime = rig.fleet.hosts().of(id.as_str());
    eventually(|| async { runtime.state().turn_began.is_some() }).await;
}

/// A Job with a summary kept, Fleet restarted over `later` as its model, and two turns of his that end.
/// The summary after, and each question `later` was asked for one. The second question is asked only
/// once the first call is over, so what it carries as the last summary is what that call left.
async fn after_a_restart_over(later: FakeJudge) -> (Option<String>, Vec<String>) {
    let (rig, _judge, home) = rig_summarising(SUMMARY);
    let id = working(&rig, "make the sign-in page remember me").await;
    rig.stand_in.finishes(0, "Made the page remember him.");
    eventually(|| async { so_far(&rig, &id).await.is_some() }).await;

    let later = Arc::new(later);
    let rig = rig.restarted(judged_by(Arc::clone(&later), super::settings_json::settings_file_in(&home)));
    begun_by(&rig, &id, "and now the other thing").await;
    rig.stand_in.finishes(0, "Moved on to the other thing.");
    eventually(|| async { !summaries(&later).is_empty() }).await;
    begun_by(&rig, &id, "and one more thing").await;
    rig.stand_in.finishes(0, "Did one more thing.");
    eventually(|| async { summaries(&later).len() >= 2 }).await;
    (so_far(&rig, &id).await, summaries(&later))
}

#[tokio::test]
async fn a_turn_ending_keeps_a_summary_on_the_job_and_his_next_message_leaves_it_standing_while_the_account_clears() {
    let (rig, _judge, _home) = rig_summarising(SUMMARY);
    let id = working(&rig, "make the sign-in page remember me").await;
    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/login.tsx"}"#).await;
    eventually(|| async { account(&rig, &id).await.is_some() }).await;
    assert_eq!(so_far(&rig, &id).await, None, "nothing is written while the turn runs and nobody was asked");

    rig.stand_in.finishes(0, "Made the page remember him.");
    eventually(|| async { so_far(&rig, &id).await.is_some() }).await;
    assert_eq!(so_far(&rig, &id).await.as_deref(), Some(SUMMARY));
    assert!(account(&rig, &id).await.is_some(), "the account stands through the end of the turn");

    rig.send(&id, "now the other thing").await;
    rig.stand_in.init(0);
    eventually(|| async { account(&rig, &id).await.is_none() }).await;
    assert_eq!(so_far(&rig, &id).await.as_deref(), Some(SUMMARY), "it covers the Job, not one message");
}

#[tokio::test]
async fn a_question_put_to_him_mid_turn_writes_the_summary_while_the_turn_is_still_running() {
    let (rig, judge, _home) = rig_summarising(SUMMARY);
    let id = working(&rig, "make the sign-in page remember me").await;
    // A question before any answer to his message is sent back, so the agent has written him one.
    let runtime = rig.fleet.hosts().of(id.as_str());
    rig.stand_in.says(0, vec![DroneEvent::Said { text: "I will start with the form.\nThen the cookie.".into(), by: Speaker::Drone }]);
    eventually(|| async { runtime.state().said.is_some() }).await;
    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/login.tsx"}"#).await;
    assert!(summaries(&judge).is_empty(), "something read is not a reason to ask");

    let asking: AskingToRun = ipc::decode("an ask", ASK.as_bytes()).expect("an ask");
    let (fleet, session) = (Arc::clone(&rig.fleet), id.as_str().to_string());
    let held = tokio::spawn(async move { fleet.session_permission(&session, asking).await });

    eventually(|| async { so_far(&rig, &id).await.is_some() }).await;
    assert_eq!(so_far(&rig, &id).await.as_deref(), Some(SUMMARY));
    assert!(matches!(rig.turn(&id).await, SessionTurn::Working { .. }), "the turn has not ended");
    assert!(!held.is_finished(), "and the question still waits for him");
    held.abort();
}

#[tokio::test]
async fn the_next_summary_is_asked_from_the_last_one_and_his_latest_message() {
    let (rig, judge, _home) = rig_summarising(SUMMARY);
    let id = working(&rig, "make the sign-in page remember me").await;
    rig.stand_in.finishes(0, "Made the page remember him.");
    eventually(|| async { so_far(&rig, &id).await.is_some() }).await;
    let first = summaries(&judge);
    assert_eq!(first.len(), 1);
    assert!(first[0].contains("(nothing yet)"), "there is no summary to carry yet: {}", first[0]);
    assert!(first[0].contains("make the sign-in page remember me"), "{}", first[0]);

    begun_by(&rig, &id, "now the other thing").await;
    rig.stand_in.finishes(0, "Moved on to the other thing.");
    eventually(|| async { summaries(&judge).len() == 2 }).await;
    let second = &summaries(&judge)[1];
    assert!(second.contains(SUMMARY), "it carries the first summary's words: {second}");
    assert!(second.contains("now the other thing"), "and his latest message: {second}");
    assert!(!second.contains("make the sign-in page remember me"), "not the one before: {second}");
    assert!(second.contains("Moved on to the other thing."), "and what the agent said this turn: {second}");
}

#[tokio::test]
async fn an_answer_that_names_code_leaves_the_last_summary_standing() {
    let (kept, asked) = after_a_restart_over(FakeJudge::answering(&[(ASKED, PLANNED), (SO_FAR, "Edited gate.rs")])).await;
    assert_eq!(kept.as_deref(), Some(SUMMARY));
    assert!(asked[1].contains(SUMMARY) && !asked[1].contains("Edited gate.rs"), "the next question carries the old one: {}", asked[1]);
}

#[tokio::test]
async fn a_model_call_that_fails_leaves_the_last_summary_standing() {
    let (kept, asked) = after_a_restart_over(FakeJudge::that_fails("the model is down")).await;
    assert_eq!(kept.as_deref(), Some(SUMMARY));
    assert!(asked[1].contains(SUMMARY), "the next question carries the old one: {}", asked[1]);
}

#[tokio::test]
async fn a_turn_that_ended_with_no_call_and_nothing_said_asks_for_no_summary() {
    let (rig, judge, _home) = rig_summarising(SUMMARY);
    let id = working(&rig, "make the sign-in page remember me").await;
    rig.stand_in.says(0, vec![DroneEvent::Ended { turns: 1, cost_micros: 5_000, refusals: 0 }]);
    eventually(|| async { rig.turn(&id).await == SessionTurn::Idle }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(summaries(&judge).is_empty(), "nothing was read, so the model is not asked: {:?}", judge.asked());
    assert_eq!(so_far(&rig, &id).await, None);
}

#[test]
fn a_summary_wanted_while_a_call_is_out_waits_and_is_owed_again_only_if_more_was_read_meanwhile() {
    let mut writing = Writing::default();
    writing.person_said("m1", "fix the sign-in page");
    writing.said_seen("Looking at the form.");
    let reading = writing.so_far_begin().expect("something was read");
    assert_eq!(reading.message.as_deref(), Some("fix the sign-in page"));
    assert_eq!(reading.said, ["Looking at the form."]);

    writing.said_seen("Found it.");
    assert_eq!(writing.so_far_begin(), None, "a call is out");
    assert!(writing.so_far_finished(), "another was asked for and more was read meanwhile, so another is owed");
    assert!(writing.so_far_begin().is_some(), "and it reads what came in");

    assert_eq!(writing.so_far_begin(), None, "a call is out again");
    assert!(!writing.so_far_finished(), "another was asked for but nothing was read meanwhile");
    assert!(!writing.so_far_finished(), "and a call that nobody asked again over owes nothing");
}

#[test]
fn his_message_starting_a_turn_leaves_what_the_summary_has_yet_to_read() {
    let mut writing = Writing::default();
    writing.person_said("m1", "first");
    writing.call_seen("Searching");
    writing.said_seen("Looking.");
    writing.person_said("m2", "second");
    writing.turn_began();

    let reading = writing.so_far_begin().expect("his message did not clear what the summary has not read");
    assert_eq!(reading.message.as_deref(), Some("second"));
    assert!(reading.said.is_empty(), "the turn's words went with the account");
}

#[test]
fn nothing_read_asks_for_no_summary() {
    let mut writing = Writing::default();
    assert_eq!(writing.so_far_begin(), None);
    writing.person_said("m1", "first");
    writing.turn_began();
    assert_eq!(writing.so_far_begin(), None, "a message of his is not something the agent did");
}

#[test]
fn a_call_seen_and_something_said_each_leave_a_summary_to_write() {
    let mut writing = Writing::default();
    writing.call_seen("Searching");
    assert!(writing.so_far_begin().is_some(), "a call");
    assert!(!writing.so_far_finished());
    assert_eq!(writing.so_far_begin(), None, "it was read");

    writing.said_seen("Looking.");
    assert!(writing.so_far_begin().is_some(), "a word said");
}

#[test]
fn a_summary_is_read_into_one_plain_line_and_dropped_where_it_says_nothing_or_names_code() {
    assert_eq!(
        read_so_far("  Found the reads happen once.\n\n  Added a cache, not keyed yet.\n").as_deref(),
        Some("Found the reads happen once. Added a cache, not keyed yet."),
        "lines join and blanks go"
    );
    assert_eq!(read_so_far("\"Fixed the sign-in page.\"").as_deref(), Some("Fixed the sign-in page."), "quotes are trimmed");
    assert_eq!(read_so_far("\u{201c}Fixed the sign-in page.\u{201d}").as_deref(), Some("Fixed the sign-in page."), "so are curly ones");
    assert_eq!(read_so_far("`Fixed the sign-in page.`").as_deref(), Some("Fixed the sign-in page."), "and backticks round the whole");

    for nothing in ["", "  \n ", "none", "None.", "nothing", "N/A"] {
        assert_eq!(read_so_far(nothing), None, "{nothing:?} is no summary");
    }
    assert_eq!(read_so_far("Edited gate.rs"), None, "a file name drops the whole line");
    assert_eq!(read_so_far("Fixed the sign-in page.\nRenamed sessionActivity everywhere."), None, "and so does one line of two");

    let long = read_so_far(&"word ".repeat(300)).expect("a long one is cut, not dropped");
    assert!(long.chars().count() <= 280 && long.ends_with('\u{2026}'), "{} chars", long.chars().count());
    assert!(long.starts_with("word word"));
}

#[test]
fn the_summarys_question_shows_nothing_yet_for_what_is_empty_and_the_words_for_what_is_not() {
    let empty = JobSoFar::default().question();
    assert_eq!(empty.matches("(nothing yet)").count(), 4, "the last summary, his message, what was done and what was said: {empty}");
    assert!(empty.contains(SO_FAR) && !empty.contains('{'), "every blank is filled: {empty}");

    let full = JobSoFar {
        last: Some("Made the page remember him.".into()),
        message: Some("now the other thing".into()),
        done: vec!["Fixed the sign-in page".into()],
        said: vec!["Looking at the form.".into()],
    }
    .question();
    assert!(!full.contains("(nothing yet)"), "{full}");
    for words in ["Made the page remember him.", "now the other thing", "- Fixed the sign-in page", "- Looking at the form."] {
        assert!(full.contains(words), "{words}: {full}");
    }
}
