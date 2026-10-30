//! An act whose course is to ask is one item in its Job's Needs you list, raised by Fleet and
//! kept in the store, so a restart keeps it. **Do it** carries the act out, **Skip** records it
//! skipped, and neither sends the agent a word.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{AnswerWaiting, DismissWaiting, SessionId, WaitingItem, WaitingSource};
use store::rule_act_standing as standing;

use super::rule_acting::{a_rig, acts, approved, failing, notices_in, save};
use super::session_host::Rig;

const STARTS_A_JOB: &str = "version: 1
rule_id: look_at_the_deploy
name: Look at the deploy
when: { enters: { lifecycle: pr, state: approved } }
do:
  - start_job: { prompt: Look at the deploy }
";

const DRAFTS_A_MESSAGE: &str = "version: 1
rule_id: ask_for_review
name: Ask for review
when: { enters: { lifecycle: pr, state: approved } }
do:
  - draft: { template: 'Ready for {pr}', on: { entered: true } }
";

pub(super) async fn items(rig: &Rig, job: &SessionId) -> Vec<WaitingItem> {
    let items = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("a session").session.waiting_for;
    items.into_iter().filter(|one| one.source == WaitingSource::RuleAct).collect()
}

fn choosing(job: &SessionId, item: &str, choice: u32) -> AnswerWaiting {
    AnswerWaiting { session_id: job.clone(), item_id: item.to_string(), choice: Some(choice), text: None, mode: None, fixed: None }
}

pub(super) async fn pressed(rig: &Rig, job: &SessionId, item: &str, choice: u32) -> Result<(), api::Refusal> {
    Arc::clone(&rig.fleet).answer_waiting(choosing(job, item, choice)).await.map(|_| ())
}

fn started_jobs(rows: &[ipc::SessionRow]) -> usize {
    rows.iter().filter(|row| matches!(row, ipc::SessionRow::Started { .. })).count()
}

async fn rows(rig: &Rig, job: &SessionId) -> Vec<ipc::SessionRow> {
    Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("a session").rows
}

#[tokio::test]
async fn an_act_to_ask_about_is_one_item_with_two_options_and_nothing_is_done_until_it_is_answered() {
    let rig = a_rig();
    save(&rig, "look.yml", STARTS_A_JOB);
    let job = rig.start().await;
    approved(&rig, &job).await;

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(acts(&rig, &job).await, [("Start a Job: Look at the deploy".to_string(), standing::ASKED.to_string(), 0)], "asked, and not tried");
    assert_eq!(started_jobs(&rows(&rig, &job).await), 0);

    let asked = items(&rig, &job).await;
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].text, "Start a Job: Look at the deploy");
    assert_eq!(asked[0].options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Do it", "Skip"]);
    assert_eq!(asked[0].act.as_ref().map(|act| act.target.clone()), Some(asked[0].id.clone()));
    assert!(asked[0].id.starts_with("rule_act:"), "{}", asked[0].id);
}

#[tokio::test]
async fn do_it_carries_the_act_out_once_and_the_item_goes() {
    let rig = a_rig();
    save(&rig, "look.yml", STARTS_A_JOB);
    let job = rig.start().await;
    approved(&rig, &job).await;
    let item = items(&rig, &job).await.remove(0);

    let before = notices_in(&rig, &job).await.len();
    pressed(&rig, &job, &item.id, 0).await.expect("answered");
    assert!(items(&rig, &job).await.is_empty(), "the item is gone once it is answered");
    assert!(pressed(&rig, &job, &item.id, 0).await.is_err(), "a second answer finds it settled");

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(acts(&rig, &job).await.iter().map(|one| (one.1.as_str(), one.2)).collect::<Vec<_>>(), [(standing::DONE, 1)]);
    assert_eq!(started_jobs(&rows(&rig, &job).await), 1, "the Job is started, with this one as its parent");
    assert_eq!(notices_in(&rig, &job).await.len(), before, "the agent was told nothing");
}

#[tokio::test]
async fn skip_and_dismiss_record_the_act_skipped_and_do_nothing() {
    let rig = a_rig();
    save(&rig, "look.yml", STARTS_A_JOB);
    let job = rig.start().await;
    approved(&rig, &job).await;
    let item = items(&rig, &job).await.remove(0);
    pressed(&rig, &job, &item.id, 1).await.expect("skipped");
    assert_eq!(acts(&rig, &job).await.iter().map(|one| one.1.as_str()).collect::<Vec<_>>(), [standing::SKIPPED]);

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(started_jobs(&rows(&rig, &job).await), 0);
    assert!(items(&rig, &job).await.is_empty());

    // The same through Dismiss, on another occurrence of it.
    let other = rig.start().await;
    approved(&rig, &other).await;
    let item = items(&rig, &other).await.remove(0);
    Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: other.clone(), item_id: item.id.clone(), reason: None, misread: false })
        .await
        .expect("dismissed");
    assert_eq!(acts(&rig, &other).await.iter().map(|one| one.1.as_str()).collect::<Vec<_>>(), [standing::SKIPPED]);
    assert!(items(&rig, &other).await.is_empty());
}

#[tokio::test]
async fn the_item_survives_a_restart_and_is_answered_there() {
    let rig = a_rig();
    save(&rig, "look.yml", STARTS_A_JOB);
    let job = rig.start().await;
    approved(&rig, &job).await;
    let before = items(&rig, &job).await;

    let rig = rig.restarted(|_| {});
    assert_eq!(items(&rig, &job).await, before, "the same item, read back off the kept act");
    pressed(&rig, &job, &before[0].id, 0).await.expect("answered");
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(acts(&rig, &job).await.iter().map(|one| one.1.as_str()).collect::<Vec<_>>(), [standing::DONE]);
}

#[tokio::test]
async fn an_act_that_failed_past_its_retries_is_an_item_saying_what_failed_and_try_again_starts_it_over() {
    let rig = a_rig();
    save(&rig, "watch.yml", &failing("ask_owner", 1));
    let job = rig.start().await;
    approved(&rig, &job).await;
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;

    let stuck = items(&rig, &job).await;
    assert_eq!(stuck.len(), 1);
    assert!(stuck[0].text.starts_with("Start the nope watch failed: "), "{}", stuck[0].text);
    assert_eq!(stuck[0].options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Try again", "Skip"]);

    pressed(&rig, &job, &stuck[0].id, 0).await.expect("answered");
    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    let kept = rig.fleet.store().lock().await.rule_acts_of(job.as_str()).expect("reads");
    assert_eq!((kept[0].state.as_str(), kept[0].attempts), (standing::STUCK, 2), "tried from the start again: the try and its one retry");
    assert_eq!(items(&rig, &job).await.len(), 1, "and it asks again");
}

#[tokio::test]
async fn a_draft_a_rule_plans_under_asks_is_proposed_at_once_and_the_drafts_own_wait_is_the_only_ask() {
    let rig = a_rig();
    save(&rig, "review.yml", DRAFTS_A_MESSAGE);
    let job = rig.start().await;
    approved(&rig, &job).await;
    let line = "Draft a message on PR #812: Ready for PR #812".to_string();
    assert_eq!(acts(&rig, &job).await, [(line.clone(), standing::PLANNED.to_string(), 0)], "planned for doing: Draft is Asks by default");
    assert!(items(&rig, &job).await.is_empty(), "no rule_act item to answer before the draft exists");

    Arc::clone(&rig.fleet).rule_acts_carried_out().await;
    assert_eq!(acts(&rig, &job).await, [(line, standing::DONE.to_string(), 1)], "done once proposed");
    assert!(items(&rig, &job).await.is_empty(), "still no rule_act item");
    let waiting = Arc::clone(&rig.fleet).get_session(job.clone()).await.expect("a session").session.waiting_for;
    let drafts: Vec<&WaitingItem> = waiting.iter().filter(|one| one.source == WaitingSource::Draft).collect();
    assert_eq!(drafts.len(), 1, "the draft waits for his press, and that is the ask: {waiting:?}");
}
