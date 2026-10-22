//! Items on the Needs you list that are one ask fold to one row: the pure fold with hand vectors, and
//! Fleet's read through a fake model on the same seam, so no weights are fetched.
//! `docs/spikes/needs-you-dedup.md` has the real model measured. The items are written for the test.

use std::sync::Arc;

use adapter_traits::{NotEmbedded, TextModel, TextReading};
use api::{HostedSessions, Sessions};
use ipc::{Instant, SessionId, SessionRecord, SetWaitingFor, WaitingAct, WaitingActKind, WaitingInput, WaitingItem, WaitingSource};

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::dedup::{folded, FOLDS_AT, IN_FORCE};

fn item(id: &str, text: &str, since: &str, source: WaitingSource, act: bool) -> WaitingItem {
    WaitingItem {
        id: id.to_string(),
        text: text.to_string(),
        since: Instant::carried(since),
        source,
        act: act.then(|| WaitingAct { kind: WaitingActKind::Answer, target: String::new() }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
    }
}

const EARLY: &str = "2026-10-21T10:00:00Z";
const LATE: &str = "2026-10-21T10:05:00Z";

/// Hand vectors by text: the two wordings of the land ask share an axis, the key question has its own.
fn hand(text: &str) -> Option<Vec<f32>> {
    match text {
        "Land it?" | "Want me to land it?" | "Approve the plan" | "Approve the store plan" => Some(vec![1.0, 0.0]),
        "Which key opens the card?" => Some(vec![0.0, 1.0]),
        _ => None,
    }
}

fn ids(items: &[WaitingItem]) -> Vec<&str> {
    items.iter().map(|one| one.id.as_str()).collect()
}

#[test]
fn two_wordings_of_one_ask_fold_to_the_earlier() {
    let items = vec![
        item("later", "Want me to land it?", LATE, WaitingSource::Agent, false),
        item("earlier", "Land it?", EARLY, WaitingSource::Agent, false),
    ];
    let kept = folded(items, hand, FOLDS_AT);
    assert_eq!(ids(&kept), ["earlier"]);
    assert_eq!(kept[0].text, "Land it?", "the kept item stays as it was");
}

#[test]
fn fleets_own_item_stands_over_the_agents_restatement() {
    let items = vec![
        item("approve-plan", "Approve the store plan", EARLY, WaitingSource::Agent, true),
        item("plan:1", "Approve the plan", LATE, WaitingSource::Plan, true),
    ];
    assert_eq!(ids(&folded(items, hand, FOLDS_AT)), ["plan:1"]);
}

#[test]
fn the_agents_item_with_an_act_stands_over_the_replied_sentence() {
    let items = vec![
        item("replied:1", "Want me to land it?", EARLY, WaitingSource::Replied, false),
        item("land", "Land it?", LATE, WaitingSource::Agent, true),
    ];
    assert_eq!(ids(&folded(items, hand, FOLDS_AT)), ["land"]);
}

#[test]
fn an_item_with_an_act_stands_over_one_without_in_the_same_class() {
    let items = vec![
        item("bare", "Land it?", EARLY, WaitingSource::Agent, false),
        item("acted", "Want me to land it?", LATE, WaitingSource::Agent, true),
    ];
    assert_eq!(ids(&folded(items, hand, FOLDS_AT)), ["acted"]);
}

#[test]
fn two_different_asks_both_stay_in_order() {
    let items = vec![
        item("key", "Which key opens the card?", LATE, WaitingSource::Replied, false),
        item("land", "Land it?", EARLY, WaitingSource::Agent, false),
    ];
    assert_eq!(ids(&folded(items, hand, FOLDS_AT)), ["key", "land"]);
}

#[test]
fn an_item_with_no_vector_yet_is_kept_and_folds_nothing() {
    let items = vec![
        item("land", "Land it?", EARLY, WaitingSource::Agent, false),
        item("new", "Approve the plan, worded as the model has not seen", LATE, WaitingSource::Replied, false),
        item("again", "Want me to land it?", LATE, WaitingSource::Replied, false),
    ];
    assert_eq!(ids(&folded(items, hand, FOLDS_AT)), ["land", "new"]);
}

/// Puts a text that names the plan on one axis and every other on the other, so the cosine between
/// two plan texts is 1 and to anything else 0.
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

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
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

/// How many rows the list settles at: one where the fold is in force, else both. The constant is the
/// owner's pick, so the test holds either way and says which it read.
fn rows_expected() -> usize {
    if IN_FORCE {
        1
    } else {
        2
    }
}

/// Two agent items that are one ask in two wordings, with no plan pending so `plan_asks` lets them
/// through. The model embeds them on a blocking thread after the first read, which publishes the
/// Session again; the list then settles at one row, the earlier item kept.
#[tokio::test]
async fn two_agent_items_that_are_one_ask_are_one_row_while_the_fold_is_in_force() {
    let rig = rig_placing(None);
    rig.fleet.language_model().loaded(Arc::new(Fake));
    let id = rig.start().await;
    rig.fleet
        .waiting_for(None, say(&id, &[("ask", "Which key opens the card?"), ("go", "Approve the store plan"), ("again", "Approve this plan")]))
        .await
        .expect("kept");
    let want = rows_expected() + 1;
    eventually(|| async { record(&rig, &id).await.waiting_for.len() == want }).await;
    let listed = record(&rig, &id).await.waiting_for;
    let mut kept: Vec<&str> = listed.iter().map(|one| one.id.as_str()).collect();
    kept.sort_unstable();
    if IN_FORCE {
        assert_eq!(kept, ["ask", "go"], "{listed:?}");
    } else {
        assert_eq!(kept, ["again", "ask", "go"], "{listed:?}");
    }
}

/// The turn ends on a reply whose close asks him to approve the plan Fleet already lists as its own
/// item. Fleet's item stands; the `replied:` sentence folds into it.
#[tokio::test]
async fn a_reply_that_restates_fleets_plan_item_is_one_row_while_the_fold_is_in_force() {
    let rig = rig_placing(None);
    rig.fleet.language_model().loaded(Arc::new(Fake));
    let id = rig.start().await;
    rig.fleet.record_plan(None, super::plan_asks::plan(&id)).await.expect("recorded");
    rig.send(&id, "plan the store").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The plan is recorded.\n\nWant me to go ahead with the plan?");
    eventually(|| async {
        let row = record(&rig, &id).await;
        row.waiting_for.iter().any(|one| one.source == WaitingSource::Plan) && row.standing == Some(ipc::SessionStanding::Idle)
    })
    .await;
    let want = rows_expected();
    eventually(|| async { record(&rig, &id).await.waiting_for.len() == want }).await;
    let listed = record(&rig, &id).await.waiting_for;
    assert_eq!(listed.len(), want, "{listed:?}");
    assert!(listed.iter().any(|one| one.source == WaitingSource::Plan), "Fleet's own item stands: {listed:?}");
    assert_eq!(listed.iter().any(|one| one.source == WaitingSource::Replied), !IN_FORCE, "{listed:?}");
}
