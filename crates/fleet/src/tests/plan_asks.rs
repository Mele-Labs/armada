//! Whether an agent's item that repeats the pending plan is refused, through a fake model on the same
//! seam, so no weights are fetched. `docs/spikes/walk-item-classifier.md` has the real model measured.
//! The items are written for the test, in the shapes agents send.

use std::sync::Arc;

use adapter_traits::{NotEmbedded, TextModel, TextReading};
use api::{HostedSessions, Sessions};
use ipc::{ApprovePlan, PlanGroupInput, PlanTaskInput, RecordPlan, SessionId, SetWaitingFor, WaitingInput};

use super::session_host::{rig_placing, Rig};
use crate::session_host::plan_asks::{item_text, scores, REPEATS_AT};

/// Puts a text that names a plan on one axis and every other on the other, so the cosine to Fleet's
/// own "Plan to approve" is 1 or 0.
struct Fake;

impl TextModel for Fake {
    fn model(&self) -> &str {
        "fake@0"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        Ok(texts.iter().map(|text| if text.to_lowercase().contains("plan") { vec![1.0, 0.0] } else { vec![0.0, 1.0] }).collect())
    }

    fn read(&self, _: TextReading, _: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
        Err(NotEmbedded(String::from("not read here")))
    }
}

fn items(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(id, text)| (id.to_string(), text.to_string())).collect()
}

#[test]
fn an_item_is_worded_as_its_id_in_words_then_its_text_on_one_line() {
    assert_eq!(item_text("approve-plan_v2", "Approve  the\nplan"), "approve plan v2: Approve the plan");
    assert_eq!(item_text("", "Approve the plan"), "Approve the plan");
}

#[test]
fn each_item_is_scored_against_fleets_plan_item_in_order_from_one_batch() {
    let read = scores(&Fake, &items(&[("a", "Approve the plan"), ("b", "Which key opens the card?")])).unwrap();
    assert!(read[0] >= REPEATS_AT, "{read:?}");
    assert!(read[1] < REPEATS_AT, "{read:?}");
}

fn say(id: &SessionId, pairs: &[(&str, &str)]) -> SetWaitingFor {
    SetWaitingFor {
        session_id: Some(id.clone()),
        items: pairs
            .iter()
            .map(|(item, text)| WaitingInput { id: (*item).into(), text: (*text).into(), act: None, options: Vec::new(), steps: Vec::new() })
            .collect(),
    }
}

fn plan(id: &SessionId) -> RecordPlan {
    RecordPlan {
        groups: vec![PlanGroupInput {
            id: None,
            title: "Store".into(),
            agent: "rust-engineer".into(),
            paths: vec!["crates/store".into()],
            issue: None,
            tasks: vec![PlanTaskInput { id: None, title: "Add the table".into() }],
        }],
        sketch: None,
        walk: None,
        session_id: Some(id.clone()),
    }
}

async fn loaded() -> (Rig, SessionId) {
    let rig = rig_placing(None);
    rig.fleet.language_model().loaded(Arc::new(Fake));
    let id = rig.start().await;
    (rig, id)
}

fn refused_for(answer: Result<ipc::SessionRecord, api::Refusal>) -> String {
    match answer {
        Err(api::Refusal::Unacceptable(error)) => error.message,
        other => panic!("expected the item refused, got {other:?}"),
    }
}

#[tokio::test]
async fn an_item_that_repeats_the_pending_plan_is_refused_and_names_it() {
    let (rig, id) = loaded().await;
    rig.fleet.record_plan(None, plan(&id)).await.expect("recorded");
    let why = refused_for(rig.fleet.waiting_for(None, say(&id, &[("ask", "Which key opens the card?"), ("go", "Approve the store plan")])).await);
    assert!(why.contains("\"go\"") && why.contains("version 1") && why.contains("Plan to approve"), "{why}");
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert!(record.waiting_for.iter().all(|one| one.id != "ask" && one.id != "go"), "a refused list keeps nothing");
}

#[tokio::test]
async fn an_item_about_something_else_is_kept_while_a_plan_is_pending() {
    let (rig, id) = loaded().await;
    rig.fleet.record_plan(None, plan(&id)).await.expect("recorded");
    rig.fleet.waiting_for(None, say(&id, &[("ask", "Which key opens the card?")])).await.expect("kept");
}

#[tokio::test]
async fn the_same_words_are_kept_once_no_plan_waits() {
    let (rig, id) = loaded().await;
    rig.fleet.waiting_for(None, say(&id, &[("go", "Approve the store plan")])).await.expect("no plan is recorded");
    rig.fleet.record_plan(None, plan(&id)).await.expect("recorded");
    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 1 }).await.expect("approved");
    rig.fleet.waiting_for(None, say(&id, &[("go", "Approve the store plan")])).await.expect("an approved plan waits on nothing");
}

#[tokio::test]
async fn until_the_model_is_loaded_the_list_is_accepted_as_it_was() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.record_plan(None, plan(&id)).await.expect("recorded");
    rig.fleet.waiting_for(None, say(&id, &[("go", "Approve the store plan")])).await.expect("the rules have no phrase for it");
}

#[test]
fn the_model_is_fetched_while_the_plan_reader_is_in_force() {
    assert!(crate::language_model::wanted());
}
