//! The claim, written before the code: **the sketch a Job leads its result with rides on the done card, and
//! never waits in Needs you on its own.** `all_done` takes the lead sketch off his list with no verdict, so his
//! End is the one press, and a sketch drawn in the turn still running waits only once that turn ends, so none
//! shows and goes. A sketch a gate or the Job's own question waits on still waits.
//! `docs/practices/protocol/how-he-works-with-jobs-and-a-question-offered-as-pictures.md`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{AllDone, AnswerItems, ItemAnswer, ItemHow, ItemSettled, LeadPicture, PlanGate, PlanGroupInput, SessionId, SessionRecord, SessionTurn, SetWaitingFor, ShowSketch, WaitingInput, WaitingOption, WaitingSource};

use super::session_host::{eventually, rig_placing, Rig};
use super::session_plan::{group, plan, read, recorded, running};

const TITLE: &str = "What changed";

fn drawn(id: &SessionId) -> ShowSketch {
    let scene = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Before"},{"id":"b","kind":"box","x":300,"y":0,"title":"After"}],"edges":[{"id":"e1","from":"a","to":"b"}]}"#;
    ipc::decode("a sketch to show", format!(r#"{{"title":"{TITLE}","session_id":"{}","scene":{scene}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn sketch_items(record: &SessionRecord) -> Vec<&str> {
    record.waiting_for.iter().filter(|one| one.source == WaitingSource::Sketch).map(|one| one.id.as_str()).collect()
}

async fn done(rig: &Rig, id: &SessionId) -> SessionRecord {
    rig.fleet.all_done(None, AllDone { session_id: Some(id.clone()), summary: Some("It shipped.".into()) }).await.expect("done or held")
}

/// A Job whose turn is running.
async fn working(rig: &Rig) -> SessionId {
    let id = running(rig).await;
    eventually(|| async { matches!(rig.turn(&id).await, SessionTurn::Working { .. }) }).await;
    // The turn's start is heard on a task of its own, and a turn begun clears a done: let it land.
    eventually(|| async { rig.fleet.hosts().of(id.as_str()).state().resolved_model.is_some() }).await;
    id
}

async fn turn_ended(rig: &Rig, id: &SessionId) {
    rig.stand_in.finishes(0, "Here it is.");
    eventually(|| async { !matches!(rig.turn(id).await, SessionTurn::Working { .. }) }).await;
}

async fn lead(rig: &Rig, id: &SessionId) -> LeadPicture {
    rig.fleet.get_lead_picture(id.clone()).await.expect("a lead picture")
}

#[tokio::test]
async fn a_result_sketch_rides_on_the_done_card_and_never_waits_on_its_own() {
    let rig = rig_placing(None);
    let id = working(&rig).await;

    let shown = rig.fleet.show_sketch(None, drawn(&id)).await.unwrap();
    assert_eq!(sketch_items(&shown), Vec::<&str>::new(), "drawn in the turn still running: nothing shows to go again");
    let finished = done(&rig, &id).await;
    assert!(finished.done_since.is_some(), "the Job is done");
    assert_eq!(sketch_items(&finished), Vec::<&str>::new());

    turn_ended(&rig, &id).await;
    let after = read(&rig, &id).await;
    assert_eq!(sketch_items(&after), Vec::<&str>::new(), "the done card is the one place it shows: {:?}", after.waiting_for);
    assert!(after.closed.is_empty(), "Fleet settled it, he closed nothing: {:?}", after.closed);
    assert_eq!(lead(&rig, &id).await, LeadPicture::Sketch { title: TITLE.into() }, "the done card leads with it");
}

#[tokio::test]
async fn a_sketch_drawn_in_a_turn_that_ends_without_all_done_waits_once_the_turn_ends() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    let shown = rig.fleet.show_sketch(None, drawn(&id)).await.unwrap();
    assert_eq!(sketch_items(&shown), Vec::<&str>::new());

    turn_ended(&rig, &id).await;
    assert_eq!(sketch_items(&read(&rig, &id).await), ["sketch:What changed"], "a sketch the turn ended on is put to him");
}

#[tokio::test]
async fn a_sketch_drawn_in_the_turn_still_running_takes_his_answer_on_its_card() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    rig.fleet.show_sketch(None, drawn(&id)).await.unwrap();

    let answer = ItemAnswer { session_id: id.clone(), item: "sketch:What changed".into(), how: ItemHow::Approve, notes: Vec::new() };
    let settled = Arc::clone(&rig.fleet).answer_items(AnswerItems { items: vec![answer] }, Vec::new()).await;
    assert_eq!(settled.outcomes[0].outcome, ItemSettled::Settled, "not raised yet, and answered all the same");
    turn_ended(&rig, &id).await;
    assert_eq!(sketch_items(&read(&rig, &id).await), Vec::<&str>::new(), "answered, so it never waits");
}

#[tokio::test]
async fn a_sketch_left_waiting_from_an_earlier_turn_leaves_his_list_at_all_done() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let shown = rig.fleet.show_sketch(None, drawn(&id)).await.unwrap();
    assert_eq!(sketch_items(&shown), ["sketch:What changed"]);

    let finished = done(&rig, &id).await;
    assert!(finished.done_since.is_some());
    assert_eq!(sketch_items(&finished), Vec::<&str>::new(), "{:?}", finished.waiting_for);
    let sketch = finished.attachments.iter().find(|one| one.target == "sketch:What changed").expect("still held");
    assert!(!sketch.detail.contains_key("settled") && !sketch.detail.contains_key("withdrawn"), "no verdict: {:?}", sketch.detail);
    assert_eq!(lead(&rig, &id).await, LeadPicture::Sketch { title: TITLE.into() });
}

#[tokio::test]
async fn a_sketch_an_open_gate_waits_on_still_waits() {
    let rig = rig_placing(None);
    let id = working(&rig).await;
    let gated = PlanGroupInput { gate: PlanGate::SketchApproved, ..group(None, "Shape", "rust-engineer", &["crates/fleet"], &[(None, "Draw it")]) };
    recorded(&rig, plan(&id, vec![gated])).await;
    let shown = rig.fleet.show_sketch(None, drawn(&id)).await.unwrap();
    assert_eq!(sketch_items(&shown), ["sketch:What changed"], "a gate on a sketch raises it while the turn still runs");

    let held = done(&rig, &id).await;
    assert!(held.done_since.is_none(), "the gate holds the call");
    turn_ended(&rig, &id).await;
    assert_eq!(sketch_items(&read(&rig, &id).await), ["sketch:What changed"], "his Approve of it closes the gate");
}

#[tokio::test]
async fn a_sketch_beside_the_jobs_own_open_question_still_waits() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id)).await.unwrap();
    let option = |label: &str| WaitingOption { label: label.into(), description: None, picture: None };
    let question = WaitingInput {
        id: "which".into(),
        text: "Pick a take from the sketch".into(),
        act: None,
        options: vec![option("Left"), option("Right")],
        approves: false,
        steps: Vec::new(),
    };
    rig.fleet.set_waiting(None, SetWaitingFor { items: vec![question], session_id: Some(id.clone()) }).await.expect("asked");

    let finished = done(&rig, &id).await;
    assert_eq!(sketch_items(&finished), ["sketch:What changed"], "a question still waits on it: {:?}", finished.waiting_for);
}

#[tokio::test]
async fn the_lead_sketch_alone_is_settled_and_an_older_one_still_waits() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id)).await.unwrap();
    let mut newer = drawn(&id);
    newer.title = "The result".into();
    rig.fleet.show_sketch(None, newer).await.unwrap();

    let finished = done(&rig, &id).await;
    assert_eq!(sketch_items(&finished), ["sketch:What changed"], "{:?}", finished.waiting_for);
    assert_eq!(lead(&rig, &id).await, LeadPicture::Sketch { title: "The result".into() });
}
