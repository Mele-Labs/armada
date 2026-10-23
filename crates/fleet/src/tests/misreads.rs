//! A question Fleet read from a reply that he marks Not a question: its sentence is kept, and a reply
//! that says one near it raises no item. The pure fold with hand vectors, and Fleet's read through a
//! fake model, so no weights are fetched. The sentences are written for the test.

use std::sync::Arc;

use adapter_traits::{NotEmbedded, TextModel, TextReading};
use api::{HostedSessions, Sessions};
use ipc::{DismissWaiting, Instant, SessionId, SessionRecord, SetWaitingFor, WaitingInput, WaitingItem, WaitingSource};

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::dedup::{left_off_as_misread, FOLDS_AT};

fn item(id: &str, text: &str, source: WaitingSource) -> WaitingItem {
    WaitingItem {
        id: id.to_string(),
        text: text.to_string(),
        since: Instant::carried("2026-10-23T10:00:00Z"),
        source,
        act: None,
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
    }
}

/// Hand vectors by text: the two summary lines share an axis, the real question has its own.
fn hand(text: &str) -> Option<Vec<f32>> {
    match text {
        "What you get: the header keeps its icons." | "What you get: the panel keeps its name." => Some(vec![1.0, 0.0]),
        "Which key opens the card?" => Some(vec![0.0, 1.0]),
        _ => None,
    }
}

fn ids(items: &[WaitingItem]) -> Vec<&str> {
    items.iter().map(|one| one.id.as_str()).collect()
}

#[test]
fn a_sentence_read_from_a_reply_near_a_marked_one_is_left_off() {
    let items = vec![
        item("replied:t:0", "What you get: the panel keeps its name.", WaitingSource::Replied),
        item("replied:t:1", "Which key opens the card?", WaitingSource::Replied),
    ];
    let kept = left_off_as_misread(items, &["What you get: the header keeps its icons.".to_string()], hand, FOLDS_AT);
    assert_eq!(ids(&kept), ["replied:t:1"]);
}

#[test]
fn an_item_the_agent_stated_stands_however_near_a_marked_one() {
    let items = vec![item("summary", "What you get: the panel keeps its name.", WaitingSource::Agent)];
    let kept = left_off_as_misread(items, &["What you get: the header keeps its icons.".to_string()], hand, FOLDS_AT);
    assert_eq!(ids(&kept), ["summary"]);
}

#[test]
fn a_sentence_with_no_vector_yet_is_kept() {
    let items = vec![item("replied:t:0", "Not embedded yet", WaitingSource::Replied)];
    let kept = left_off_as_misread(items, &["What you get: the header keeps its icons.".to_string()], hand, FOLDS_AT);
    assert_eq!(ids(&kept), ["replied:t:0"]);
}

/// Puts a text that names the store on one axis and every other on the other.
struct Fake;

impl TextModel for Fake {
    fn model(&self) -> &str {
        "fake@0"
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NotEmbedded> {
        Ok(texts.iter().map(|text| if text.to_lowercase().contains("store") { vec![1.0, 0.0] } else { vec![0.0, 1.0] }).collect())
    }

    fn read(&self, _: TextReading, _: &[&str]) -> Result<Vec<f32>, NotEmbedded> {
        Err(NotEmbedded(String::from("not read here")))
    }
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

fn replied(row: &SessionRecord) -> Option<&WaitingItem> {
    row.waiting_for.iter().find(|one| one.source == WaitingSource::Replied)
}

fn misread(id: &SessionId, item: &str) -> DismissWaiting {
    DismissWaiting { session_id: id.clone(), item_id: item.to_string(), reason: None, misread: true }
}

/// He marks the question read from a reply as not one: it goes, its sentence is kept, and the next
/// reply that says one near it raises no item once the model has its vector.
#[tokio::test]
async fn a_question_marked_as_misread_is_kept_and_holds_a_near_one_off() {
    let rig = rig_placing(None);
    rig.fleet.language_model().loaded(Arc::new(Fake));
    let id = rig.start().await;
    rig.send(&id, "tidy up").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The first half is done.\n\nWant me to tidy the store next?");
    eventually(|| async { replied(&record(&rig, &id).await).is_some() }).await;
    let asked = replied(&record(&rig, &id).await).cloned().expect("raised");
    Arc::clone(&rig.fleet).dismiss_waiting(misread(&id, &asked.id)).await.expect("marked");
    assert!(replied(&record(&rig, &id).await).is_none(), "the item is gone");
    assert_eq!(rig.fleet.store().lock().await.misreads().expect("read"), [asked.text.clone()]);

    rig.send(&id, "and the rest").await;
    rig.stand_in.finishes(0, "The second half is done.\n\nWant me to tidy the store as well?");
    eventually(|| async { record(&rig, &id).await.standing == Some(ipc::SessionStanding::Idle) }).await;
    eventually(|| async { replied(&record(&rig, &id).await).is_none() }).await;
    assert!(replied(&record(&rig, &id).await).is_none(), "a near sentence raises no item");
}

/// Not a question is for what Fleet read from a reply; an item the agent stated is refused and stands.
#[tokio::test]
async fn an_agent_item_marked_as_misread_is_refused_and_stands() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let set = SetWaitingFor {
        session_id: Some(id.clone()),
        items: vec![WaitingInput { id: "a".into(), text: "Which key opens the card?".into(), act: None, options: Vec::new(), steps: Vec::new() }],
    };
    rig.fleet.waiting_for(None, set).await.expect("kept");
    assert!(Arc::clone(&rig.fleet).dismiss_waiting(misread(&id, "a")).await.is_err());
    assert!(record(&rig, &id).await.waiting_for.iter().any(|one| one.id == "a"), "it stands");
    assert!(rig.fleet.store().lock().await.misreads().expect("read").is_empty());
}
