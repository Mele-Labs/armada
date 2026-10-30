//! The claim, written before the code: **every sketch a job shows is put to the person on its card,
//! and waits in Needs you as `sketch:<title>` until he answers it there. Approve or a note sent back reaches the job as his message and settles the
//! sketch on the ledger, and only the drawing he read can be answered.**
//! `crates/ipc/operations/answer_sketch.toml`.

use std::sync::Arc;

use api::{Refusal, Sessions};
use ipc::{AnswerSketch, Attachment, Instant, SessionId, SessionRow, SessionVoice, ShowSketch, SketchHow};

use super::session_host::{rig_placing, Rig};
use crate::session_host::sketching::sketch_approved;

const TITLE: &str = "Request flow";
const TARGET: &str = "sketch:Request flow";

fn scene(second: &str) -> String {
    format!(
        r#"{{"nodes":[{{"id":"a","kind":"box","x":0,"y":0,"title":"Bridge"}},{{"id":"b","kind":"box","x":300,"y":0,"title":"{second}"}}],"edges":[{{"id":"e1","from":"a","to":"b"}}]}}"#
    )
}

fn drawn(id: &SessionId, second: &str) -> ShowSketch {
    ipc::decode(
        "a sketch to show",
        format!(r#"{{"title":"{TITLE}","session_id":"{}","scene":{}}}"#, id.as_str(), scene(second)).as_bytes(),
    )
    .unwrap()
}

fn the_sketch(record: &ipc::SessionRecord) -> &Attachment {
    record
        .attachments
        .iter()
        .find(|one| one.target == TARGET)
        .unwrap_or_else(|| panic!("no sketch held: {:?}", record.attachments))
}

fn answer(id: &SessionId, drawn: &str, how: SketchHow, words: Option<&str>) -> AnswerSketch {
    AnswerSketch {
        session_id: id.clone(),
        sketch: TARGET.into(),
        drawn: drawn.into(),
        how,
        words: words.map(str::to_string),
    }
}

fn sent(rows: &[SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// A job holding one sketch, and the `drawn` it was shown with.
async fn shown(rig: &Rig) -> (SessionId, String) {
    let id = rig.start().await;
    let record = rig.fleet.show_sketch(None, drawn(&id, "Fleet")).await.unwrap();
    let drawn = the_sketch(&record).detail.get("drawn").cloned().expect("a sketch shown says which drawing it is");
    (id, drawn)
}

#[tokio::test]
async fn a_sketch_shown_is_asked_on_its_card_and_waits_as_one_item() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let record = rig.fleet.show_sketch(None, drawn(&id, "Fleet")).await.unwrap();

    let sketch = the_sketch(&record);
    assert_eq!(sketch.detail.get("asked").map(String::as_str), Some("true"), "{:?}", sketch.detail);
    assert!(sketch.detail.get("drawn").is_some_and(|one| !one.is_empty()), "{:?}", sketch.detail);
    assert!(!sketch.detail.contains_key("settled"), "open until he answers it");
    let waits: Vec<_> = record.waiting_for.iter().map(|one| (one.id.as_str(), one.text.as_str(), one.source)).collect();
    assert_eq!(waits, [(TARGET, TITLE, ipc::WaitingSource::Sketch)], "one Needs you item, its title");
}

#[tokio::test]
async fn approving_a_sketch_tells_the_job_and_settles_it_where_bridge_reads_it() {
    let rig = rig_placing(None);
    let (id, drawn) = shown(&rig).await;
    let record = Arc::clone(&rig.fleet).answer_sketch(answer(&id, &drawn, SketchHow::Approved, None)).await.unwrap();

    let rows = rig.rows(&id).await;
    assert_eq!(sent(&rows).pop().unwrap(), format!("Approved sketch: {TITLE}\nGo on from it as drawn."));
    let detail = &the_sketch(&record).detail;
    assert_eq!(detail.get("settled").map(String::as_str), Some("approved"), "{detail:?}");
    assert!(detail.get("settled_at").is_some_and(|at| !at.is_empty()), "{detail:?}");
    assert!(!detail.contains_key("words"), "{detail:?}");
    assert_eq!(detail.get("drawn"), Some(&drawn), "the drawing he answered is the one kept");
    assert!(sketch_approved(&rows).is_some(), "the rail reads the approval off the thread");
}

#[tokio::test]
async fn sending_a_sketch_back_tells_the_job_his_note_and_keeps_it() {
    let rig = rig_placing(None);
    let (id, drawn) = shown(&rig).await;
    let note = "Put the store on the left.";
    let record = Arc::clone(&rig.fleet)
        .answer_sketch(answer(&id, &drawn, SketchHow::SentBack, Some(&format!("  {note}\n"))))
        .await
        .unwrap();

    let rows = rig.rows(&id).await;
    assert_eq!(
        sent(&rows).pop().unwrap(),
        format!("Sent back sketch: {TITLE}\n{note}\nDraw it again with show_sketch under the same title.")
    );
    let detail = &the_sketch(&record).detail;
    assert_eq!(detail.get("settled").map(String::as_str), Some("sent_back"), "{detail:?}");
    assert_eq!(detail.get("words").map(String::as_str), Some(note));
    assert_eq!(sketch_approved(&rows), None, "a send-back is not an approval");
}

#[tokio::test]
async fn a_send_back_with_no_note_is_refused_and_nothing_is_sent() {
    let rig = rig_placing(None);
    let (id, drawn) = shown(&rig).await;
    let Err(Refusal::Unacceptable(why)) =
        Arc::clone(&rig.fleet).answer_sketch(answer(&id, &drawn, SketchHow::SentBack, Some(" \n"))).await
    else {
        panic!("a send-back needs a note");
    };
    assert_eq!(why.code, "fleet.sketch_answer_empty");
    assert!(sent(&rig.rows(&id).await).is_empty());
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    let record = listed.iter().find(|one| one.id == id).unwrap();
    assert!(!the_sketch(record).detail.contains_key("settled"), "still open");
}

#[tokio::test]
async fn a_sketch_drawn_again_since_he_read_it_is_refused_and_the_new_drawing_stays_open() {
    let rig = rig_placing(None);
    let (id, older) = shown(&rig).await;
    let record = rig.fleet.show_sketch(None, drawn(&id, "Daemon")).await.unwrap();
    let newer = the_sketch(&record).detail["drawn"].clone();
    assert_ne!(older, newer, "each drawing is its own");

    let Err(Refusal::IllegalMove(why)) =
        Arc::clone(&rig.fleet).answer_sketch(answer(&id, &older, SketchHow::Approved, None)).await
    else {
        panic!("an answer to the replaced drawing is refused");
    };
    assert_eq!(why.code, "fleet.sketch_redrawn");
    assert!(why.message.contains(TITLE), "{}", why.message);
    assert!(sent(&rig.rows(&id).await).is_empty(), "the job is told nothing");

    let record = Arc::clone(&rig.fleet).answer_sketch(answer(&id, &newer, SketchHow::Approved, None)).await.unwrap();
    assert_eq!(the_sketch(&record).detail.get("settled").map(String::as_str), Some("approved"));
}

#[tokio::test]
async fn a_settled_sketch_or_one_the_job_does_not_hold_is_not_open() {
    let rig = rig_placing(None);
    let (id, drawn) = shown(&rig).await;
    Arc::clone(&rig.fleet).answer_sketch(answer(&id, &drawn, SketchHow::Approved, None)).await.unwrap();

    let Err(Refusal::Unacceptable(twice)) =
        Arc::clone(&rig.fleet).answer_sketch(answer(&id, &drawn, SketchHow::SentBack, Some("No."))).await
    else {
        panic!("a settled sketch takes no second answer");
    };
    assert_eq!(twice.code, "fleet.sketch_not_open");
    let mut unknown = answer(&id, &drawn, SketchHow::Approved, None);
    unknown.sketch = "sketch:Storage".into();
    let Err(Refusal::Unacceptable(none)) = Arc::clone(&rig.fleet).answer_sketch(unknown).await else {
        panic!("a sketch the job does not hold is refused");
    };
    assert_eq!(none.code, "fleet.sketch_not_open");
    assert_eq!(sent(&rig.rows(&id).await).len(), 1, "only the first answer was sent");
}

fn said(at: &str, from: SessionVoice, text: &str) -> SessionRow {
    SessionRow::Message {
        id: format!("m-{at}"),
        at: Instant::carried(at),
        from,
        text: text.into(),
        files: Vec::new(),
        tags: Vec::new(),
        notice: None,
        queued: false,
    }
}

#[test]
fn the_rail_reads_the_newest_approval_of_any_sketch_off_the_thread() {
    assert_eq!(sketch_approved(&[]), None);
    let rows = [
        said("2026-10-28T09:00:00.000Z", SessionVoice::You, "Approved sketch: Flow\nGo on from it as drawn."),
        said("2026-10-28T09:05:00.000Z", SessionVoice::Agent, "Approved sketch: Store\nGo on from it as drawn."),
        said("2026-10-28T09:06:00.000Z", SessionVoice::You, "Approved sketch: Store\nGo on from it as drawn.\nKeep it small."),
        said("2026-10-28T09:07:00.000Z", SessionVoice::You, "Sent back sketch: Flow\nNo.\nDraw it again with show_sketch under the same title."),
        said("2026-10-28T09:08:00.000Z", SessionVoice::You, "I approved sketch: Flow earlier"),
    ];
    assert_eq!(sketch_approved(&rows), Some("2026-10-28T09:06:00.000Z"), "his newest approval, never the agent's words");
    assert_eq!(sketch_approved(&rows[3..]), None);
}
