//! The claim, written before the code: **a sketch shown with `asks: false` explains and asks nothing. It never waits in
//! Needs you and has no Approve, is still among the Job's artifacts, takes his note as a message that reopens nothing, and
//! a redraw of its title does not start asking.** `crates/ipc/operations/show_sketch.toml`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{AnswerSketch, ArtifactState, SessionId, SessionRecord, SessionRow, SessionVoice, ShowSketch, SketchHow, ThreadItem};

use super::session_host::{rig_placing, Rig};
use super::thread_view::{show_sketch, with_ledger, SCENE};

const TITLE: &str = "Request flow";
const TARGET: &str = "sketch:Request flow";

fn shown_as(id: &SessionId, title: &str, asks: &str) -> ShowSketch {
    ipc::decode(
        "a sketch to show",
        format!(r#"{{"title":"{title}","question":"Does this read right?","session_id":"{}","scene":{SCENE}{asks}}}"#, id.as_str()).as_bytes(),
    )
    .unwrap()
}

fn explains(id: &SessionId, title: &str) -> ShowSketch {
    shown_as(id, title, r#","asks":false"#)
}

fn row<'r>(record: &'r SessionRecord, target: &str) -> &'r ipc::Attachment {
    record.attachments.iter().find(|one| one.target == target).unwrap_or_else(|| panic!("no sketch held: {:?}", record.attachments))
}

fn told(rows: &[SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

async fn held(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

#[tokio::test]
async fn a_sketch_shown_with_asks_false_waits_on_nobody_and_is_still_an_artifact() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let record = rig.fleet.show_sketch(None, explains(&id, TITLE)).await.unwrap();

    let detail = &row(&record, TARGET).detail;
    assert!(!detail.contains_key("asked"), "{detail:?}");
    assert!(detail.get("drawn").is_some_and(|one| !one.is_empty()), "it is still a drawing: {detail:?}");
    assert!(record.waiting_for.is_empty(), "no Needs you item: {:?}", record.waiting_for);

    let artifacts = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    assert_eq!(artifacts.iter().map(|one| (one.id.as_str(), one.state, one.item.as_deref())).collect::<Vec<_>>(), [(TARGET, ArtifactState::Reading, None)]);

    let cards = with_ledger(vec![show_sketch("s1", 1, TITLE, "")], record.attachments.clone());
    let ThreadItem::Sketch(card) = &cards[0] else { panic!("a card: {cards:?}") };
    assert!(!card.asked && card.settled.is_none(), "its card carries no ask");
}

#[tokio::test]
async fn a_sketch_shown_with_no_asks_field_still_asks() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let record = rig.fleet.show_sketch(None, shown_as(&id, TITLE, "")).await.unwrap();

    assert_eq!(row(&record, TARGET).detail.get("asked").map(String::as_str), Some("true"));
    assert_eq!(record.waiting_for.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [TARGET]);
}

#[tokio::test]
async fn his_note_on_a_sketch_that_asks_nothing_reaches_the_job_and_leaves_it_asking_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let record = rig.fleet.show_sketch(None, explains(&id, TITLE)).await.unwrap();
    let drawn = row(&record, TARGET).detail["drawn"].clone();

    // The note a viewer sends is a message of his, as for any sketch.
    let note = "Why is the store on the right?";
    rig.send(&id, note).await;
    assert_eq!(told(&rig.rows(&id).await).pop().as_deref(), Some(note));
    let record = held(&rig, &id).await;
    let detail = &row(&record, TARGET).detail;
    assert!(!detail.contains_key("asked") && !detail.contains_key("settled"), "a note reopens nothing: {detail:?}");
    assert!(record.waiting_for.is_empty(), "{:?}", record.waiting_for);

    // There is nothing to approve or send back, and nothing is told to the job.
    let before = told(&rig.rows(&id).await);
    for how in [SketchHow::Approved, SketchHow::SentBack] {
        let answer = AnswerSketch { session_id: id.clone(), sketch: TARGET.into(), drawn: drawn.clone(), how, words: Some(note.into()), extras: Default::default() };
        let refused = Arc::clone(&rig.fleet).answer_sketch(answer).await.expect_err("it asks nothing");
        assert!(format!("{refused:?}").contains("fleet.sketch_not_open"), "{refused:?}");
    }
    assert_eq!(told(&rig.rows(&id).await), before);
    assert!(!row(&held(&rig, &id).await, TARGET).detail.contains_key("settled"));
}

#[tokio::test]
async fn a_sketch_redrawn_under_its_title_with_asks_false_stops_asking_and_one_redrawn_with_asks_asks_again() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let asking = rig.fleet.show_sketch(None, shown_as(&id, TITLE, "")).await.unwrap();
    assert_eq!(asking.waiting_for.len(), 1);

    let quiet = rig.fleet.show_sketch(None, explains(&id, TITLE)).await.unwrap();
    assert!(!row(&quiet, TARGET).detail.contains_key("asked"), "{:?}", row(&quiet, TARGET).detail);
    assert!(quiet.waiting_for.is_empty(), "{:?}", quiet.waiting_for);

    let again = rig.fleet.show_sketch(None, shown_as(&id, TITLE, "")).await.unwrap();
    assert_eq!(again.waiting_for.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [TARGET]);
    let last = rig.fleet.show_sketch(None, explains(&id, TITLE)).await.unwrap();
    assert!(last.waiting_for.is_empty(), "{:?}", last.waiting_for);
}

#[tokio::test]
async fn a_sketch_that_asks_nothing_does_not_join_the_artifact_that_asks() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, shown_as(&id, TITLE, "")).await.unwrap();
    let record = rig.fleet.show_sketch(None, explains(&id, "Why it is slow")).await.unwrap();

    let other = row(&record, "sketch:Why it is slow");
    assert!(!other.detail.contains_key("in") && !other.detail.contains_key("asked"), "{:?}", other.detail);
    assert_eq!(record.waiting_for.iter().map(|one| (one.id.as_str(), one.text.as_str())).collect::<Vec<_>>(), [(TARGET, TITLE)], "the item names the one that asks");
    let artifacts = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    assert_eq!(artifacts.len(), 2, "each stands as its own artifact");
}
