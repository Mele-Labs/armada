//! A Job's routine question, once the person has said Always: offered as a go-ahead where the Operator
//! is sure of it and it is about nothing he would see, answered by the rule as it arrives, and never
//! queued. Anything unsure, about a look, the scope or work that could be lost, or destructive, is queued
//! for him as before. Autopilot leaves the questions a rule of the Operator's covers to it.

use std::sync::Arc;

use ipc::operator_page::{OperatorAnswer, OperatorRule, OperatorTurnKind, RuleScope};
use api::HostedSessions;
use ipc::AutopilotDecision;
use store::KeptOrder;

use super::autopilot_decisions::{item, keep, rig_calling, started, waiting as list_of, OLD, SURE_OF_THE_FIRST};
use super::operator_raises::{journal, page, pause, press, question, queued, seated, waiting};
use super::operator_sources::switched;
use super::session_host::Rig;
use crate::operator::keep_the_raises;

const FULL_SUITE: &str = "Tests pass on the three files I touched. Run the full suite before pushing, or push now?";
const OPTIONS: [&str; 2] = ["Run the full suite (Recommended)", "Push now"];

fn review(about: &str, sure: &str) -> String {
    format!("item: 1\nread: The Job's own files pass and the suite is the cheap safe step.\npick: Run the full suite (Recommended)\nsure: {sure}\nabout: {about}\n\njournal: Reviewed a question.")
}

/// A rule the person kept, for `manifest_id` or for every repository.
async fn rule_kept(rig: &Rig, manifest_id: Option<&str>) {
    let scope = manifest_id.map_or(RuleScope::Everywhere, |manifest_id| RuleScope::Repository { manifest_id: manifest_id.to_string() });
    let rule = OperatorRule::PickRule { scope, what: "question:answer".into(), pick: "do_it".into() };
    let one = KeptOrder { n: 0, form: rule.kept(), said: "Answer questions you are sure of".into(), until: None, at: "2026-11-10T09:00:00.000Z".into() };
    rig.fleet.store().lock().await.add_operator_order(&one).expect("kept");
}

#[tokio::test]
async fn a_question_it_is_sure_of_is_offered_with_always_here_and_the_rule_keeps_for_the_repository() {
    let rig = super::session_host::rig_placing(None);
    seated(&rig, &[Some(&review("none", "yes"))]).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;
    waiting(&rig, &job, vec![question("full", FULL_SUITE, &OPTIONS)]).await;

    let one = queued(&rig, &job, "full").await;

    assert_eq!((one.kind, one.keeps), (OperatorTurnKind::GoAhead, true));
    let answered = rig.fleet.operator_answered(press(&one, OperatorAnswer::AlwaysHere, None)).await.expect("answered");
    let learned = &answered.page.digest.learned;
    assert_eq!(learned.len(), 1, "{learned:#?}");
    assert_eq!((learned[0].said.as_str(), matches!(learned[0].scope, RuleScope::Repository { .. })), ("Answer questions you are sure of", true));
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session;
    assert!(record.waiting_for.iter().all(|item| item.id != "full"), "answered with the pick, as a press on the option is");
}

#[tokio::test]
async fn a_question_about_what_he_would_see_or_that_it_is_not_sure_of_is_queued_plain() {
    for (about, sure) in [("look", "yes"), ("scope", "yes"), ("lost", "yes"), ("none", "no"), ("", "yes")] {
        let rig = super::session_host::rig_placing(None);
        seated(&rig, &[Some(&review(about, sure))]).await;
        keep_the_raises(Arc::clone(&rig.fleet)).await;
        let job = rig.start_titled(Some("Build the pane")).await;
        waiting(&rig, &job, vec![question("full", FULL_SUITE, &OPTIONS)]).await;

        let one = queued(&rig, &job, "full").await;

        assert_eq!((one.kind, one.keeps), (OperatorTurnKind::Decision, false), "about {about:?}, sure {sure:?}");
        assert!(one.read.is_some_and(|read| read.pick == "Run the full suite (Recommended)"), "his read and pick still come with it");
    }
}

#[tokio::test]
async fn under_a_kept_rule_a_sure_question_is_answered_as_it_arrives_with_a_log_row_and_a_thread_line() {
    let rig = super::session_host::rig_placing(None);
    seated(&rig, &[Some(&review("none", "yes"))]).await;
    rule_kept(&rig, None).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("full", FULL_SUITE, &OPTIONS)]).await;

    super::session_host::eventually(|| async { !page(&rig).await.digest.handled.acts.is_empty() }).await;
    let after = page(&rig).await;
    assert!(after.turn.is_empty(), "never queued: {:#?}", after.turn);
    let act = &after.digest.handled.acts[0];
    assert_eq!(act.title, FULL_SUITE);
    assert_eq!(act.outcome, "By a rule: Answer questions you are sure of: Run the full suite (Recommended)");
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job");
    assert!(record.session.waiting_for.iter().all(|item| item.id != "full"));
    let said = record.rows.iter().map(|row| format!("{row:?}")).find(|row| row.contains("The Operator answered"));
    assert!(said.is_some_and(|row| row.contains("Run the full suite (Recommended)")), "the thread says who answered and with what");
    assert!(journal(&rig).await.contains("By a rule, Answer questions you are sure of: Build the pane"));
}

#[tokio::test]
async fn a_destructive_question_is_queued_for_him_even_under_a_rule() {
    let rig = super::session_host::rig_placing(None);
    let reply = "item: 1\nread: The old branch is merged.\npick: Delete it\nsure: yes\nabout: none\n\njournal: Reviewed.";
    seated(&rig, &[Some(reply)]).await;
    rule_kept(&rig, None).await;
    keep_the_raises(Arc::clone(&rig.fleet)).await;
    let job = rig.start_titled(Some("Build the pane")).await;

    waiting(&rig, &job, vec![question("old", "The old branch is merged. Delete it?", &["Delete it", "Keep it"])]).await;

    let one = queued(&rig, &job, "old").await;
    assert_eq!((one.kind, one.keeps), (OperatorTurnKind::Decision, false));
    pause().await;
    let record = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("the Job").session;
    assert!(record.waiting_for.iter().any(|item| item.id == "old"), "not answered");
    assert!(page(&rig).await.digest.handled.acts.is_empty());
}

#[tokio::test]
async fn autopilot_leaves_a_question_the_operators_rule_covers_and_still_answers_one_with_no_rule() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    let job = rig.start().await;
    started(&rig, &[AutopilotDecision::Recommended]).await;
    keep(&rig, &job, &[item("a", "Which clock?", &["Frozen (Recommended)", "Fake"])]).await;
    // The Operator is on, but no rule of its covers a question: Autopilot answers as it always has.
    switched(&rig, "features.operator", true).await;
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 1);
    assert!(list_of(&rig, &job).await.is_empty());

    let rig_with_rule = {
        let (rig, _) = rig_calling(SURE_OF_THE_FIRST);
        rig
    };
    let covered = rig_with_rule.start().await;
    started(&rig_with_rule, &[AutopilotDecision::Recommended]).await;
    switched(&rig_with_rule, "features.operator", true).await;
    rule_kept(&rig_with_rule, Some(rig_with_rule.manifest.as_str())).await;
    keep(&rig_with_rule, &covered, &[item("a", "Which clock?", &["Frozen (Recommended)", "Fake"])]).await;

    assert_eq!(rig_with_rule.fleet.decision_pass().await.expect("passed"), 0);
    assert_eq!(list_of(&rig_with_rule, &covered).await, ["a"], "the question is the Operator's");
    let _ = (judge, OLD);
}
