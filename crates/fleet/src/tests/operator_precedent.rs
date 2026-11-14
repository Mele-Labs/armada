//! A Job's routine question answered by following something the person already said: one of his own answers in
//! the same project, or a setting he saved. Fleet keeps his answers (never one given in his place), the review call
//! cites one, and Fleet checks the citation before the rule `question:precedent` carries the answer out. Anything
//! about a look, anything that departs from Classic, and anything unclear is queued for him as before.

use std::sync::Arc;

use api::HostedSessions;
use ipc::operator_page::{OperatorAnswer, OperatorFollowsKind, OperatorRule, OperatorTurnKind, OperatorUndoRuling, RuleScope};
use ipc::{AnswerWaiting, CloseSession, SavePreference, SessionId, WaitingActKind, WaitingSource};
use store::{KeptAnswer, KeptOrder};

use super::operator_raises::{page, pause, press, question, queued, seated, waiting};
use super::session_host::{eventually, Rig};
use crate::operator::keep_the_raises;

const SUITE: &str = "Tests pass on the three files I touched. Run the full suite before pushing, or push now?";
const OPTIONS: [&str; 2] = ["Run the full suite (Recommended)", "Push now"];
const BEFORE: &str = "The files pass. Run the whole suite first, or push?";
const OTHER_REPOSITORY: &str = "01JOTHERREPOSITORY00000000";
const SECONDS: &str = r#"{"hour12":false,"seconds":true,"date":false}"#;
const NO_SECONDS: &str = r#"{"hour12":false,"seconds":false,"date":false}"#;

fn review(about: &str, sure: &str, follows: &str) -> String {
    format!("item: 1\nread: His answer in this project covers it.\npick: Run the full suite (Recommended)\nsure: {sure}\nabout: {about}\nfollows: {follows}\n\njournal: Reviewed a question.")
}

/// His answer to a question of an earlier Job in `manifest_id`, kept as Fleet keeps one. Answers its number.
async fn answer_kept(rig: &Rig, manifest_id: &str) -> i64 {
    let one = KeptAnswer {
        session: "01EARLIERJOB".into(),
        manifest_id: manifest_id.into(),
        job: Some("Earlier pane".into()),
        item_id: "before".into(),
        question: BEFORE.into(),
        options: r#"["Run the full suite (Recommended)","Push now"]"#.into(),
        choice: Some("Run the full suite (Recommended)".into()),
        words: Some("always".into()),
        answered_at: "2026-11-12T14:03:00.000Z".into(),
        via: "job".into(),
        ..KeptAnswer::default()
    };
    rig.fleet.store().lock().await.add_operator_answer(&one).expect("kept")
}

/// A rule of `what` the person kept for every repository.
async fn rule_kept(rig: &Rig, what: &str) {
    let rule = OperatorRule::PickRule { scope: RuleScope::Everywhere, what: what.into(), pick: "do_it".into() };
    let one = KeptOrder { n: 0, form: rule.kept(), said: "a rule".into(), until: None, at: "2026-11-10T09:00:00.000Z".into() };
    rig.fleet.store().lock().await.add_operator_order(&one).expect("kept");
}

async fn time_format_saved(rig: &Rig, text: &str) {
    rig.fleet.save_preference_now(SavePreference { name: "time_format".into(), value: false, text: Some(text.into()) }).await.expect("saved");
}

/// A rig with the Operator seated on `reply`, the raises kept and a Job holding the question.
async fn asked(reply: &str, setup: impl std::future::Future<Output = ()>) -> (Rig, SessionId) {
    let rig = super::session_host::rig_placing(None);
    setup.await;
    seated(&rig, &[Some(reply)]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    (rig, job)
}

fn said_to(rig: &Rig, job: &SessionId, item: &str, choice: u32) -> AnswerWaiting {
    let _ = rig;
    AnswerWaiting { session_id: job.clone(), item_id: item.into(), choice: Some(choice), text: None, mode: None, fixed: None }
}

async fn answers_kept(rig: &Rig) -> Vec<KeptAnswer> {
    rig.fleet.store().lock().await.operator_answers_of(&[rig.manifest.as_str().to_string()], 50).expect("read")
}

#[tokio::test]
async fn his_answer_is_kept_when_he_answers_a_jobs_item_and_not_when_it_is_answered_in_his_place() {
    let rig = super::session_host::rig_placing(None);
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS), question("again", "Run it again?", &["Yes", "No"])]).await;

    Arc::clone(&rig.fleet).answer_waiting(said_to(&rig, &job, "full", 0)).await.expect("answered");
    Arc::clone(&rig.fleet).settle_waiting(said_to(&rig, &job, "again", 0)).await.expect("answered in his place");

    let kept = answers_kept(&rig).await;
    assert_eq!(kept.len(), 1, "only his own: {kept:#?}");
    assert_eq!((kept[0].question.as_str(), kept[0].choice.as_deref(), kept[0].via.as_str(), kept[0].job.as_deref()), (SUITE, Some("Run the full suite (Recommended)"), "job", Some("Build the pane")));
    assert_eq!(kept[0].manifest_id, rig.manifest.as_str());
    assert!(kept[0].options.contains("Push now"), "the options are kept: {}", kept[0].options);
}

#[tokio::test]
async fn a_press_on_the_operators_page_is_his_answer_too() {
    let (rig, job) = asked(&review("none", "yes", "none"), async {}).await;
    waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS)]).await;
    let one = queued(&rig, &job, "full").await;

    rig.fleet.operator_answered(press(&one, OperatorAnswer::Yes, None)).await.expect("answered");

    let kept = answers_kept(&rig).await;
    assert_eq!((kept.len(), kept[0].via.as_str(), kept[0].choice.as_deref()), (1, "operator_page", Some("Run the full suite (Recommended)")), "{kept:#?}");
}

#[tokio::test]
async fn a_sure_question_that_follows_his_answer_in_the_same_project_is_answered_and_the_job_is_told_which() {
    let rig = super::session_host::rig_placing(None);
    let n = answer_kept(&rig, rig.manifest.as_str()).await;
    rule_kept(&rig, "question:precedent").await;
    seated(&rig, &[Some(&review("none", "yes", &format!("a{n}")))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS)]).await;

    eventually(|| async { !page(&rig).await.digest.ruled.is_empty() }).await;
    let after = page(&rig).await;
    assert!(after.turn.is_empty(), "never queued: {:#?}", after.turn);
    let ruled = &after.digest.ruled[0];
    assert_eq!((ruled.question.as_str(), ruled.pick.as_str(), ruled.follows.kind), (SUITE, "Run the full suite (Recommended)", OperatorFollowsKind::Answer));
    assert_eq!((ruled.follows.job.as_deref(), ruled.follows.question.as_deref(), ruled.follows.said.as_str()), (Some("Earlier pane"), Some(BEFORE), "Run the full suite (Recommended): always"));
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job");
    assert!(record.session.waiting_for.iter().all(|item| item.id != "full"), "answered with the pick");
    let line = record.rows.iter().map(|row| format!("{row:?}")).find(|row| row.contains("The Operator answered")).expect("the thread says so");
    assert!(line.contains("with \\\"Run the full suite (Recommended)\\\", following the person's answer to \\\"The files pass. Run the whole suite first, or push?\\\" in Earlier pane on "), "{line}");
    assert!(line.contains("Run the full suite (Recommended): always"), "{line}");
    let turns = rig.fleet.store().lock().await.operator_turn_settled_since("2000-01-01T00:00:00.000Z").expect("read");
    let kept = turns.iter().find(|row| row.answer.as_deref() == Some("precedent")).expect("kept as a precedent");
    assert!(kept.follows.as_deref().is_some_and(|follows| follows.contains("\"kind\":\"answer\"") && follows.contains("Earlier pane")), "{:?}", kept.follows);
    assert!(answers_kept(&rig).await.len() == 1, "his answer, and never one given in his place");
}

/// A question whose pick cites `follows`, under a precedent rule: it must be queued, not answered.
async fn queued_not_answered(follows: &str, about: &str, sure: &str, kept_in: Option<&str>) -> (Rig, SessionId, ipc::operator_page::OperatorTurnItem) {
    let rig = super::session_host::rig_placing(None);
    let n = match kept_in {
        Some(manifest) => answer_kept(&rig, manifest).await,
        None => 999,
    };
    rule_kept(&rig, "question:precedent").await;
    let follows = follows.replace("{n}", &n.to_string());
    seated(&rig, &[Some(&review(about, sure, &follows))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS)]).await;
    let one = queued(&rig, &job, "full").await;
    pause().await;
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session;
    assert!(record.waiting_for.iter().any(|item| item.id == "full"), "not answered");
    assert!(page(&rig).await.digest.ruled.is_empty());
    (rig, job, one)
}

#[tokio::test]
async fn an_answer_from_another_project_is_not_followed() {
    let (_, _, one) = queued_not_answered("a{n}", "none", "yes", Some(OTHER_REPOSITORY)).await;
    assert!(one.follows.is_none(), "{:?}", one.follows);
}

#[tokio::test]
async fn an_answer_number_that_does_not_exist_is_not_followed() {
    let (_, _, one) = queued_not_answered("a999", "none", "yes", None).await;
    assert!(one.follows.is_none(), "{:?}", one.follows);
}

#[tokio::test]
async fn an_answer_that_departs_from_classic_is_queued_for_him() {
    let rig = super::session_host::rig_placing(None);
    let n = answer_kept(&rig, rig.manifest.as_str()).await;
    rule_kept(&rig, "question:precedent").await;
    seated(&rig, &[Some(&review("departs", "yes", &format!("a{n}")))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS)]).await;

    let one = queued(&rig, &job, "full").await;

    assert_eq!((one.kind, one.keeps, one.follows.is_none()), (OperatorTurnKind::Decision, false, true));
    pause().await;
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session;
    assert!(record.waiting_for.iter().any(|item| item.id == "full"), "not answered");
}

#[tokio::test]
async fn a_pick_about_how_a_time_reads_is_queued_while_his_saved_format_says_otherwise() {
    // About a look: held whatever it cites.
    let rig = super::session_host::rig_placing(None);
    time_format_saved(&rig, SECONDS).await;
    rule_kept(&rig, "question:precedent").await;
    seated(&rig, &[Some(&review("look", "yes", &format!("setting time_format = {NO_SECONDS}")))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS)]).await;
    let one = queued(&rig, &job, "full").await;
    assert_eq!((one.kind, one.follows.is_none()), (OperatorTurnKind::Decision, true));

    // About nothing, but citing a format other than the one he saved: no citation.
    let (_, _, one) = {
        let rig = super::session_host::rig_placing(None);
        time_format_saved(&rig, SECONDS).await;
        rule_kept(&rig, "question:precedent").await;
        seated(&rig, &[Some(&review("none", "yes", &format!("setting time_format = {NO_SECONDS}")))]).await;
        keep_the_raises(Arc::clone(&rig.fleet)).await;
        let job = rig.start_titled(Some("Build the pane")).await;
        waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS)]).await;
        let one = queued(&rig, &job, "full").await;
        pause().await;
        assert!(page(&rig).await.digest.ruled.is_empty());
        (rig, job, one)
    };
    assert!(one.follows.is_none(), "{:?}", one.follows);
}

#[tokio::test]
async fn a_setting_he_saved_with_that_value_is_followed() {
    let rig = super::session_host::rig_placing(None);
    time_format_saved(&rig, SECONDS).await;
    rule_kept(&rig, "question:precedent").await;
    seated(&rig, &[Some(&review("none", "yes", &format!("setting time_format = {SECONDS}")))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS)]).await;

    eventually(|| async { !page(&rig).await.digest.ruled.is_empty() }).await;
    let ruled = page(&rig).await.digest.ruled.remove(0);
    // The citation quotes the value as he saved it, whatever order the call wrote its keys in.
    let saved = rig.fleet.preferences_in_force().time_format;
    assert_eq!((ruled.follows.kind, ruled.follows.said.as_str()), (OperatorFollowsKind::Setting, format!("time_format = {saved}").as_str()));
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job");
    let line = record.rows.iter().map(|row| format!("{row:?}")).find(|row| row.contains("The Operator answered")).expect("the thread says so");
    assert!(line.contains("following his saved setting time_format = "), "{line}");
}

#[tokio::test]
async fn a_question_that_follows_nothing_is_queued() {
    let (_, _, one) = queued_not_answered("none", "none", "yes", None).await;
    assert!(one.follows.is_none(), "{:?}", one.follows);
}

#[tokio::test]
async fn with_no_precedent_rule_it_is_queued_as_a_go_ahead_carrying_what_it_follows_and_always_here_keeps_the_rule() {
    let rig = super::session_host::rig_placing(None);
    let n = answer_kept(&rig, rig.manifest.as_str()).await;
    seated(&rig, &[Some(&review("none", "yes", &format!("a{n}")))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS)]).await;

    let one = queued(&rig, &job, "full").await;

    assert_eq!((one.kind, one.keeps), (OperatorTurnKind::GoAhead, true));
    let follows = one.follows.clone().expect("it says what it follows");
    assert_eq!((follows.kind, follows.job.as_deref(), follows.said.as_str()), (OperatorFollowsKind::Answer, Some("Earlier pane"), "Run the full suite (Recommended): always"));
    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::AlwaysHere, None)).await.expect("answered");
    let rules = rig.fleet.store().lock().await.operator_orders().expect("read");
    let whats: Vec<String> = rules.iter().filter_map(|one| OperatorRule::read(&one.form)).map(|OperatorRule::PickRule { what, .. }| what).collect();
    assert_eq!(whats, ["question:precedent"], "{answered:#?}");
}

#[tokio::test]
async fn a_question_answer_rule_still_answers_first() {
    let rig = super::session_host::rig_placing(None);
    let n = answer_kept(&rig, rig.manifest.as_str()).await;
    rule_kept(&rig, "question:answer").await;
    rule_kept(&rig, "question:precedent").await;
    seated(&rig, &[Some(&review("none", "yes", &format!("a{n}")))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS)]).await;

    eventually(|| async { !page(&rig).await.digest.handled.acts.is_empty() }).await;
    let after = page(&rig).await;
    assert!(after.digest.ruled.is_empty(), "the rule answered it, not a precedent");
    let turns = rig.fleet.store().lock().await.operator_turn_settled_since("2000-01-01T00:00:00.000Z").expect("read");
    assert!(turns.iter().any(|row| row.answer.as_deref() == Some("rule") && row.follows.is_none()), "{turns:#?}");
}

/// A Job whose question was answered by following his answer: the rig, the Job and the ruling's `turn:<n>`.
async fn ruled_by_precedent() -> (Rig, SessionId, String) {
    let rig = super::session_host::rig_placing(None);
    let n = answer_kept(&rig, rig.manifest.as_str()).await;
    rule_kept(&rig, "question:precedent").await;
    seated(&rig, &[Some(&review("none", "yes", &format!("a{n}")))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("full", SUITE, &OPTIONS)]).await;
    eventually(|| async { !page(&rig).await.digest.ruled.is_empty() }).await;
    let ruled = page(&rig).await.digest.ruled.remove(0);
    assert_eq!((ruled.undone_at.is_none(), ruled.undo), (true, true), "it can be taken back while its Job is live");
    (rig, job, ruled.id)
}

fn undo(id: &str) -> OperatorUndoRuling {
    OperatorUndoRuling { id: id.into() }
}

async fn thread_of(rig: &Rig, job: &SessionId) -> Vec<String> {
    Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").rows.iter().map(|row| format!("{row:?}")).collect()
}

#[tokio::test]
async fn taking_a_ruling_back_tells_the_job_and_puts_its_question_on_the_jobs_list_with_the_same_options() {
    let (rig, job, id) = ruled_by_precedent().await;

    let after = rig.fleet.operator_ruling_undone(undo(&id)).await.expect("undone");

    let ruled = &after.digest.ruled[0];
    assert_eq!((ruled.id.as_str(), ruled.undone_at.is_some(), ruled.undo), (id.as_str(), true, false));
    let withdrawn = format!("The Operator's answer to \\\"{SUITE}\\\" is withdrawn. The person will answer it; wait for his answer.");
    assert!(thread_of(&rig, &job).await.iter().any(|row| row.contains(&withdrawn)), "the Job is told");
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session;
    let item = record.waiting_for.iter().find(|item| item.id == format!("undone:{}", id.trim_start_matches("turn:"))).expect("his question is on the Job's list");
    assert_eq!((item.source, item.text.as_str()), (WaitingSource::OperatorUndone, SUITE));
    assert_eq!(item.options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(), OPTIONS);
    assert_eq!(item.act.as_ref().map(|act| (act.kind, act.target.as_str())), Some((WaitingActKind::Answer, item.id.as_str())));
    let ruled_again = rig.fleet.store().lock().await.operator_turn_settled_since("2000-01-01T00:00:00.000Z").expect("read");
    assert!(ruled_again.iter().filter(|row| row.answer.as_deref() == Some("precedent")).all(|row| row.undone_at.is_some()));
}

#[tokio::test]
async fn his_answer_to_a_question_put_back_goes_to_the_job_and_the_item_leaves_the_list() {
    let (rig, job, id) = ruled_by_precedent().await;
    rig.fleet.operator_ruling_undone(undo(&id)).await.expect("undone");
    let item = format!("undone:{}", id.trim_start_matches("turn:"));

    Arc::clone(&rig.fleet).answer_waiting(said_to(&rig, &job, &item, 1)).await.expect("answered");

    assert_eq!(super::autopilot_decisions::told(&rig, &job).await.last().map(String::as_str), Some(format!("Re: {SUITE}\nPush now").as_str()));
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session;
    assert!(record.waiting_for.iter().all(|one| one.id != item), "answered, so it goes");
    let kept = answers_kept(&rig).await;
    assert!(kept.iter().any(|one| one.question == SUITE && one.choice.as_deref() == Some("Push now") && one.via == "job"), "his own answer is kept: {kept:#?}");
}

#[tokio::test]
async fn a_ruling_is_taken_back_once_and_never_for_a_job_that_has_ended() {
    let (rig, _, id) = ruled_by_precedent().await;
    rig.fleet.operator_ruling_undone(undo(&id)).await.expect("undone");
    let twice = rig.fleet.operator_ruling_undone(undo(&id)).await.expect_err("taken back already");
    assert_eq!((twice.status(), twice.error().code.as_str()), (422, "fleet.operator_ruling_undone"));

    let (rig, job, id) = ruled_by_precedent().await;
    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: job.clone() }).await.expect("ended");
    assert!(!page(&rig).await.digest.ruled[0].undo, "an ended Job has nothing to give back to");
    let ended = rig.fleet.operator_ruling_undone(undo(&id)).await.expect_err("the Job has ended");
    assert_eq!((ended.status(), ended.error().code.as_str()), (422, "fleet.operator_ruling_ended"));
    assert!(page(&rig).await.digest.ruled[0].undone_at.is_none(), "nothing was changed");

    let unknown = rig.fleet.operator_ruling_undone(undo("turn:99999")).await.expect_err("no such row");
    assert_eq!((unknown.status(), unknown.error().code.as_str()), (404, "fleet.operator_turn_unknown"));
}
