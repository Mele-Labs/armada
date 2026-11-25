//! The claim, written before the code: **every sketch a job shows is put to the person on its card,
//! and waits in Needs you as `sketch:<title>` until he answers it there. Approve or a note sent back reaches the job as his message and settles the
//! sketch on the ledger, and only the drawing he read can be answered.**
//! `crates/ipc/operations/answer_sketch.toml`.

use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{AnswerSketch, Attachment, DismissWaiting, Instant, SessionFact, SessionId, SessionReport, SessionRow, SessionVoice, ShowSketch, SketchHow, WithdrawSketch};

use super::session_host::{rig_placing, Rig};
use crate::session_host::sketching::{sketch_approved, sketch_attachment};

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
        format!(r#"{{"title":"{TITLE}","question":"Does this read right?","session_id":"{}","scene":{}}}"#, id.as_str(), scene(second)).as_bytes(),
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
        extras: Default::default(),
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
    assert_eq!(waits, [(TARGET, "Does this read right?", ipc::WaitingSource::Sketch)], "one Needs you item, titled with its question");
    assert_eq!(record.waiting_for[0].artifact.as_deref(), Some(TARGET), "it names its artifact");
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

/// A sketch of two takes drawn as screens, the first the job's pick, held as a Fleet before the takes rule kept
/// one: `show_sketch` refuses it now, and he can still answer one already on his list. Answered with its `drawn`.
async fn takes_held(rig: &Rig, id: &SessionId) -> (ipc::SessionRecord, String) {
    let screen = |node: &str, x: u32, title: &str| {
        format!(r#"{{"id":"{node}","kind":"screen","x":{x},"y":0,"title":"{title}","screen":{{"type":"column","children":[]}}}}"#)
    };
    let scene = format!(r#"{{"nodes":[{},{}],"edges":[]}}"#, screen("a", 0, "A (my pick): one rail"), screen("b", 400, "B: two panes"));
    let shown: ShowSketch =
        ipc::decode("a sketch to show", format!(r#"{{"title":"{TITLE}","question":"Does this read right?","session_id":"{}","scene":{scene}}}"#, id.as_str()).as_bytes()).unwrap();
    let mut attachment = sketch_attachment(shown.checked().unwrap());
    let drawn = format!("{}-legacy-1", id.as_str());
    attachment.detail.insert("asked".into(), "true".into());
    attachment.detail.insert("drawn".into(), drawn.clone());
    let record = rig
        .fleet
        .report_session(SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact: SessionFact::Attached { attachment } })
        .await
        .expect("held");
    (record, drawn)
}

#[tokio::test]
async fn a_note_that_only_picks_a_take_settles_the_sketch_and_tells_the_job_which_take() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let (_, drawn) = takes_held(&rig, &id).await;
    let record = Arc::clone(&rig.fleet).answer_sketch(answer(&id, &drawn, SketchHow::SentBack, Some("B"))).await.unwrap();

    let rows = rig.rows(&id).await;
    let told = sent(&rows).pop().unwrap();
    assert_eq!(told, format!("Approved sketch: {TITLE}\nHe picked take B. Go on from take B as drawn, and drop the other takes."));
    assert!(!told.contains("Draw it again"), "a pick is not drawn again");
    let detail = &the_sketch(&record).detail;
    assert_eq!(detail.get("settled").map(String::as_str), Some("approved"), "{detail:?}");
    assert_eq!(detail.get("words").map(String::as_str), Some("B"));
    assert!(record.waiting_for.is_empty(), "the pick leaves his list: {:?}", record.waiting_for);
    assert!(sketch_approved(&rows).is_some(), "a pick is an approval to the rail");
}

#[tokio::test]
async fn a_note_that_picks_a_take_and_asks_for_more_is_still_sent_back() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let (_, drawn) = takes_held(&rig, &id).await;
    let note = "B, with the rail wider";
    let record = Arc::clone(&rig.fleet).answer_sketch(answer(&id, &drawn, SketchHow::SentBack, Some(note))).await.unwrap();

    assert_eq!(
        sent(&rig.rows(&id).await).pop().unwrap(),
        format!("Sent back sketch: {TITLE}\n{note}\nDraw it again with show_sketch under the same title.")
    );
    assert_eq!(the_sketch(&record).detail.get("settled").map(String::as_str), Some("sent_back"));
}

#[test]
fn a_take_is_named_by_the_lead_of_a_title_or_label_or_by_its_id() {
    use crate::session_host::sketching::take_picked;
    let node = |kind: &str, id: &str, title: &str| format!(r#"{{"id":"{id}","kind":"{kind}","x":0,"y":0,"title":"{title}"}}"#);
    let scene = |nodes: &[String]| format!(r#"{{"nodes":[{}],"edges":[]}}"#, nodes.join(","));
    let labelled = scene(&[
        node("label", "l1", "Take 1, Cards: each group a card"),
        node("mock", "t1", "Cards"),
        node("label", "l2", "Take 2, Spine: groups on a numbered spine"),
        node("mock", "t2", "Spine"),
    ]);
    for (note, take) in [("2", "2"), ("Take 2.", "2"), ("spine", "Spine"), ("t1", "T1"), ("\"Cards\"", "Cards")] {
        assert_eq!(take_picked(&labelled, note).as_deref(), Some(take), "{note}");
    }
    for note in ["3", "Spine, but bigger", "Looks good", ""] {
        assert_eq!(take_picked(&labelled, note), None, "{note}");
    }
    let lettered = scene(&[node("mock", "a", "A · My pick: held toast"), node("mock", "b", "B. Centred panel")]);
    assert_eq!(take_picked(&lettered, "b").as_deref(), Some("B"));
    assert_eq!(take_picked(&lettered, "go with A").as_deref(), Some("A"));
    let flow = scene(&[node("box", "a", "A"), node("box", "b", "B")]);
    assert_eq!(take_picked(&flow, "B"), None, "boxes and arrows draw no takes");
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

fn take_back(id: &SessionId, title: &str) -> WithdrawSketch {
    WithdrawSketch { title: title.into(), session_id: Some(id.clone()) }
}

#[tokio::test]
async fn a_sketch_the_job_withdraws_leaves_his_list_and_its_card_loses_its_buttons_until_it_is_drawn_again() {
    let rig = rig_placing(None);
    let (id, _) = shown(&rig).await;

    let record = rig.fleet.withdraw_sketch(None, take_back(&id, TITLE)).await.expect("withdrawn");
    assert!(record.waiting_for.is_empty(), "the sketch left his Needs you list: {:?}", record.waiting_for);
    assert_eq!(the_sketch(&record).detail.get("withdrawn").map(String::as_str), Some("true"), "the card is marked");

    let record = rig.fleet.show_sketch(None, drawn(&id, "Storage")).await.unwrap();
    assert_eq!(record.waiting_for.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [TARGET], "drawn again, it waits again");
    assert!(!the_sketch(&record).detail.contains_key("withdrawn"), "and the new drawing is not marked");
}

#[tokio::test]
async fn a_sketch_the_job_never_showed_is_not_withdrawn() {
    let rig = rig_placing(None);
    let (id, _) = shown(&rig).await;
    let Err(Refusal::Unacceptable(none)) = rig.fleet.withdraw_sketch(None, take_back(&id, "Storage")).await else {
        panic!("a title this job showed no sketch for is refused");
    };
    assert_eq!(none.code, "fleet.sketch_withdraw_not_shown");
}

#[tokio::test]
async fn a_sketch_drawn_under_a_new_title_is_its_own_item_and_each_is_answered_alone() {
    let rig = rig_placing(None);
    let (id, _) = shown(&rig).await;
    let retitled = "Request flow, settings too";
    let show: ShowSketch = ipc::decode(
        "a sketch to show",
        format!(r#"{{"title":"{retitled}","question":"Does this read right?","session_id":"{}","scene":{}}}"#, id.as_str(), scene("Daemon")).as_bytes(),
    )
    .unwrap();
    let record = rig.fleet.show_sketch(None, show).await.unwrap();
    let newer = format!("sketch:{retitled}");
    // One ask is one item: the new title waits on its own, newest first, each titled with its question.
    let items: Vec<(&str, &str)> = record.waiting_for.iter().map(|one| (one.id.as_str(), one.text.as_str())).collect();
    assert_eq!(items, [(newer.as_str(), "Does this read right?"), (TARGET, "Does this read right?")], "two items, newest first");
    assert_eq!(the_sketch(&record).detail.get("withdrawn"), None, "the old title is not taken back");

    let drawn = record.attachments.iter().find(|one| one.target == newer).unwrap().detail["drawn"].clone();
    let mut approve = answer(&id, &drawn, SketchHow::Approved, None);
    approve.sketch = newer.clone();
    let record = Arc::clone(&rig.fleet).answer_sketch(approve).await.unwrap();
    let items: Vec<(&str, &str)> = record.waiting_for.iter().map(|one| (one.id.as_str(), one.text.as_str())).collect();
    assert_eq!(items, [(TARGET, "Does this read right?")], "the older drawing still waits on its own");
}

#[tokio::test]
async fn the_owner_dismissing_a_sketch_marks_it_withdrawn_as_the_job_withdrawing_it_does() {
    let rig = rig_placing(None);
    let (id, _) = shown(&rig).await;
    let record = Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: id.clone(), item_id: TARGET.into(), reason: None, misread: false })
        .await
        .expect("dismissed");
    assert!(record.waiting_for.is_empty(), "{:?}", record.waiting_for);
    assert_eq!(the_sketch(&record).detail.get("withdrawn").map(String::as_str), Some("true"));
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
        edited_at: None,
        aside: None,
        sketches: Vec::new(),
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

fn titled(id: &SessionId, title: &str, against: Option<&str>) -> ShowSketch {
    let against = against.map(|earlier| format!(r#","against":"{earlier}""#)).unwrap_or_default();
    ipc::decode(
        "a sketch to show",
        format!(r#"{{"title":"{title}","question":"Does this read right?","session_id":"{}","scene":{}{against}}}"#, id.as_str(), scene(title)).as_bytes(),
    )
    .unwrap()
}

#[tokio::test]
async fn three_drawings_each_against_the_last_are_one_artifact_and_one_item_answered_once() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, titled(&id, "Plan", None)).await.unwrap();
    rig.fleet.show_sketch(None, titled(&id, "Plan (DSCR only)", Some("Plan"))).await.unwrap();
    let record = rig.fleet.show_sketch(None, titled(&id, "Plan (alerts)", Some("Plan (DSCR only)"))).await.unwrap();
    let waiting = |record: &ipc::SessionRecord| record.waiting_for.iter().map(|one| one.id.clone()).collect::<Vec<_>>();
    // Each is the next revision of the first, retitled, so his list holds one item for it.
    assert_eq!(waiting(&record), ["sketch:Plan"]);

    let drawn = record.attachments.iter().find(|one| one.target == "sketch:Plan").unwrap().detail["drawn"].clone();
    let mut approve = answer(&id, &drawn, SketchHow::Approved, None);
    approve.sketch = "sketch:Plan".into();
    let record = Arc::clone(&rig.fleet).answer_sketch(approve).await.unwrap();
    assert!(record.waiting_for.is_empty(), "{:?}", record.waiting_for);
    assert_eq!(sent(&rig.rows(&id).await).pop().unwrap(), "Approved sketch: Plan (alerts)\nGo on from it as drawn.");
}

#[test]
fn a_job_holding_two_sketches_left_open_lists_both_newest_first() {
    let open = |title: &str, at: &str| -> Attachment {
        ipc::decode(
            "an attachment",
            format!(
                r#"{{"kind":"artifact","target":"sketch:{title}","state":"standing","detail":{{"form":"sketch","title":"{title}","asked":"true"}},"since":"{at}","changed_at":"{at}"}}"#
            )
            .as_bytes(),
        )
        .unwrap()
    };
    let held = [open("Older", "2026-10-30T10:00:00Z"), open("Newer", "2026-11-02T10:00:00Z")];
    let listed: Vec<String> = crate::session_host::sketching::sketch_items(&held, "job", &[], &Default::default(), &[]).map(|one| one.id).collect();
    assert_eq!(listed, ["sketch:Newer", "sketch:Older"]);
}

/// A sketch put to him, drawn as `drawn` says, and answered or taken back as `settled` says.
fn asked(title: &str, drawn: &str, changed_at: &str, settled: Option<&str>) -> Attachment {
    let closing = settled.map(|key| format!(r#","{key}":"x""#)).unwrap_or_default();
    ipc::decode(
        "an attachment",
        format!(
            r#"{{"kind":"artifact","target":"sketch:{title}","state":"standing","detail":{{"form":"sketch","title":"{title}","asked":"true","drawn":"{drawn}"{closing}}},"since":"{changed_at}","changed_at":"{changed_at}"}}"#
        )
        .as_bytes(),
    )
    .unwrap()
}

fn listed(held: &[Attachment]) -> Vec<String> {
    crate::session_host::sketching::sketch_items(held, "job", &[], &Default::default(), &[]).map(|one| one.id).collect()
}

#[test]
fn an_answered_sketch_never_stays_first_so_the_pin_moves_to_the_newest_still_open() {
    let older = asked("Older", "abcd1234-20261104T120000000Z-1", "2026-11-04T17:00:00Z", None);
    let newest = asked("Newest", "abcd1234-20261104T151426339Z-2", "2026-11-04T16:00:00Z", Some("settled"));
    assert_eq!(listed(&[older.clone(), newest]), ["sketch:Older"]);
    let newest = asked("Newest", "abcd1234-20261104T151426339Z-2", "2026-11-04T15:14:26Z", None);
    assert_eq!(listed(&[older, newest]), ["sketch:Newest", "sketch:Older"], "newest drawn first, whatever changed later");
}

/// What he attached and tagged with an answer, as the row on the thread keeps it.
fn extras() -> ipc::AnswerExtras {
    ipc::AnswerExtras {
        attachments: vec![ipc::SessionUpload { name: "shot.png".into(), media_type: "image/png".into(), data: "aGVsbG8=".into() }],
        mentions: vec![ipc::SessionTag { kind: ipc::TagKind::Branch, id: "armada/other".into(), title: "armada/other".into(), url: None }],
    }
}

/// His row, the last he wrote: the picture and the tag it keeps.
fn his_row(rows: &[SessionRow]) -> (Vec<ipc::SentFile>, Vec<ipc::SessionTag>) {
    rows.iter()
        .rev()
        .find_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, files, tags, .. } => Some((files.clone(), tags.clone())),
            _ => None,
        })
        .expect("his message is on the thread")
}

#[tokio::test]
async fn a_picture_and_a_tag_sent_with_a_note_reach_the_job_and_stay_on_his_row() {
    let rig = rig_placing(None);
    let (id, drawn) = shown(&rig).await;
    let mut said = answer(&id, &drawn, SketchHow::SentBack, Some("Match this picture."));
    said.extras = extras();
    Arc::clone(&rig.fleet).answer_sketch(said).await.unwrap();

    let (files, tags) = his_row(&rig.rows(&id).await);
    assert_eq!(files.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["shot.png"], "the picture rides with the answer");
    assert_eq!(tags.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["armada/other"], "the tag rides with the answer");
}

#[tokio::test]
async fn an_answer_with_nothing_attached_keeps_a_plain_row() {
    let rig = rig_placing(None);
    let (id, drawn) = shown(&rig).await;
    Arc::clone(&rig.fleet).answer_sketch(answer(&id, &drawn, SketchHow::SentBack, Some("Plain note."))).await.unwrap();

    let (files, tags) = his_row(&rig.rows(&id).await);
    assert!(files.is_empty() && tags.is_empty(), "{files:?} {tags:?}");
}
