//! The claim, written before the code: **a step that ends on the person waits on him in Needs you
//! from what the Job showed or said, never from the agent's `waiting_for`. A sketch shown and not yet
//! answered is one item until he approves it, sends it back or dismisses it, and it stays through the
//! Job's move into a slot. A step that ends on his reply waits after every reply, one that only
//! reports included.** The owner's request of 29 Oct 2026, "A step that waits on me shows as needing me".

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{DismissWaiting, SessionId, SessionRecord, ShowSketch, SketchHow, WaitingSource};
use testkit::FakeJudge;

use super::session_host::{denied, eventually, rig_placing, Rig};
use super::session_rail::{on_the_rail, record, ruled_on};
use crate::rail_ends::{End, Waiter};

const TITLE: &str = "Request flow";
const TARGET: &str = "sketch:Request flow";

// ---- The rule ----

#[test]
fn every_end_the_person_gives_names_the_item_that_waits_for_it() {
    let person = [
        (End::PlanApproved, WaitingSource::Plan),
        (End::WalkApproved, WaitingSource::Walk),
        (End::AnyWalkApproved, WaitingSource::Walk),
        (End::Replied, WaitingSource::Replied),
        (End::SketchApproved, WaitingSource::Sketch),
        (End::DraftApproved, WaitingSource::Draft),
        (End::IdeaPicked, WaitingSource::AskCard),
        (End::DocumentApproved, WaitingSource::Document),
    ];
    for (end, source) in person {
        assert_eq!(end.waits_on(), Waiter::ThePerson(source), "{end:?}");
        assert!(!end.by_the_job(), "{end:?}");
    }
    assert_eq!(End::Merged.waits_on(), Waiter::TheMerge, "the merge is nobody's to press");
    assert!(!End::Merged.by_the_job());
    for end in [End::PlanRecorded, End::Pushed, End::WalkShown, End::Edited, End::Checked, End::Committed, End::PrOpened, End::Undelivered] {
        assert_eq!(end.waits_on(), Waiter::TheJob, "{end:?}");
        assert!(end.by_the_job(), "{end:?}");
    }
}

// ---- Through a Fleet ----

/// A workflow whose first step ends on `ends`, then Discuss, a plan and a hand-off.
fn ending_on(ends: &str) -> config::ResolvedWorkflow {
    let text = format!(
        "version: 1\nworkflow_id: rail\nname: Rail\nsteps:\n  \
        - id: propose\n    label: Propose\n    ends_on: [{ends}]\n    delivers: false\n    advance_gate: auto\n  \
        - id: discuss\n    label: Discuss\n    delivers: false\n    advance_gate: auto\n  \
        - id: plan\n    label: \"Plan the change\"\n    evidence: {{submitted: {{type: plan}}}}\n    \
        mechanical_checks:\n      - {{ type: plan_recorded, min_tasks: 1 }}\n    delivers: false\n    advance_gate: auto\n  \
        - id: handoff\n    label: \"Hand off\"\n    delivers: true\n    advance_gate: auto\n"
    );
    let def = config::WorkflowDef::parse(std::path::Path::new("rail.yml"), &text, &config::Roster::offering_nothing())
        .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &super::daemon::manifest())
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

async fn on(ends: &str) -> (Rig, SessionId) {
    let rig = ruled_on(Arc::new(FakeJudge::with_no_objection()), ending_on(ends));
    let id = on_the_rail(&rig).await;
    (rig, id)
}

fn drawn(id: &SessionId, second: &str) -> ShowSketch {
    let scene = format!(
        r#"{{"nodes":[{{"id":"a","kind":"box","x":0,"y":0,"title":"Bridge"}},{{"id":"b","kind":"box","x":300,"y":0,"title":"{second}"}}],"edges":[{{"id":"e1","from":"a","to":"b"}}]}}"#
    );
    ipc::decode("a sketch to show", format!(r#"{{"title":"{TITLE}","session_id":"{}","scene":{scene}}}"#, id.as_str()).as_bytes())
        .unwrap()
}

/// Shows the sketch and gives back the drawing it was shown as.
async fn show(rig: &Rig, id: &SessionId, second: &str) -> String {
    let record = rig.fleet.show_sketch(None, drawn(id, second)).await.expect("shown");
    let sketch = record.attachments.iter().find(|one| one.target == TARGET).expect("the sketch is held");
    sketch.detail["drawn"].clone()
}

/// What Needs you holds for the Job, as id and text.
fn needs(record: &SessionRecord) -> Vec<(&str, &str)> {
    record.waiting_for.iter().map(|one| (one.id.as_str(), one.text.as_str())).collect()
}

fn dismissed(id: &SessionId, item: &str) -> DismissWaiting {
    DismissWaiting { session_id: id.clone(), item_id: item.into(), reason: None, misread: false }
}

#[tokio::test]
async fn a_job_on_a_step_ending_on_a_sketch_waits_in_needs_you_from_the_sketch_until_he_approves_it() {
    let (rig, id) = on("sketch_approved").await;
    assert!(needs(&record(&rig, &id).await).is_empty(), "nothing shown, nothing waits");
    let drawing = show(&rig, &id, "Fleet").await;

    let asked = record(&rig, &id).await;
    assert_eq!(needs(&asked), [(TARGET, TITLE)], "the one sketch, while the turn still runs");
    let item = &asked.waiting_for[0];
    assert_eq!(item.act.as_ref().map(|act| (act.kind, act.target.as_str())), Some((ipc::WaitingActKind::Message, id.as_str())), "it opens the Job");
    assert_eq!(asked.workflow_step, Some(0));

    let answer = ipc::AnswerSketch { session_id: id.clone(), sketch: TARGET.into(), drawn: drawing, how: SketchHow::Approved, words: None };
    Arc::clone(&rig.fleet).answer_sketch(answer).await.expect("approved");
    let after = record(&rig, &id).await;
    assert!(needs(&after).iter().all(|(item, _)| *item != TARGET), "approved, it is off: {:?}", needs(&after));
    eventually(|| async { record(&rig, &id).await.workflow_step == Some(1) }).await;
}

#[tokio::test]
async fn a_step_ending_on_a_document_ends_only_when_he_approves_its_newest_revision() {
    let (rig, id) = on("document_approved").await;
    let work = super::tmp::TempDir::new();
    let file = work.path().join(".armada/documents/plan.html");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let path = file.to_string_lossy().into_owned();
    // Each write waits for its revision, so the file holds the bytes its Write call meant.
    for (n, body) in ["first", "second"].into_iter().enumerate() {
        std::fs::write(&file, format!("<title>Plan</title><p>{body}</p>")).unwrap();
        let call = format!("toolu_{n}");
        rig.stand_in.says(
            0,
            vec![
                adapter_traits::DroneEvent::Called {
                    tool: "Write".into(),
                    call: call.clone(),
                    detail: adapter_traits::CallDetail::of(&format!("{path} +1")),
                },
                adapter_traits::DroneEvent::Answered { call, failed: false },
            ],
        );
        let kept = (n + 1).to_string();
        eventually(|| async {
            let record = record(&rig, &id).await;
            record.attachments.iter().any(|one| one.target == path && one.detail.get("revision") == Some(&kept))
        })
        .await;
    }
    let item = format!("document:{path}");
    assert!(needs(&record(&rig, &id).await).contains(&(item.as_str(), "Plan")), "the newest revision waits on him");

    let read = |revision: &str| ipc::AnswerDocument {
        session_id: id.clone(),
        document: item.clone(),
        revision: revision.into(),
        how: ipc::DocumentHow::Approved,
        words: None,
    };
    assert!(Arc::clone(&rig.fleet).answer_document(read("1")).await.is_err(), "the first revision is not the newest");
    assert_eq!(record(&rig, &id).await.workflow_step, Some(0), "an older revision approved ends nothing");
    Arc::clone(&rig.fleet).answer_document(read("2")).await.expect("approved");
    eventually(|| async { record(&rig, &id).await.workflow_step == Some(1) }).await;
}

#[tokio::test]
async fn a_sketch_dismissed_is_off_until_it_is_drawn_again_and_is_never_answered_from_the_list() {
    let (rig, id) = on("sketch_approved").await;
    show(&rig, &id, "Fleet").await;
    let answered = ipc::decode::<ipc::AnswerWaiting>(
        "an answer",
        format!(r#"{{"session_id":"{}","item_id":"{TARGET}","text":"Looks right"}}"#, id.as_str()).as_bytes(),
    )
    .unwrap();
    assert!(Arc::clone(&rig.fleet).answer_waiting(answered).await.is_err(), "a sketch is answered on its card");
    assert_eq!(needs(&record(&rig, &id).await), [(TARGET, TITLE)], "still waiting");

    let after = Arc::clone(&rig.fleet).dismiss_waiting(dismissed(&id, TARGET)).await.expect("dismissed");
    assert!(needs(&after).iter().all(|(item, _)| *item != TARGET), "{:?}", needs(&after));
    show(&rig, &id, "Daemon").await;
    assert_eq!(needs(&record(&rig, &id).await), [(TARGET, TITLE)], "drawn again, it waits again");
}

#[tokio::test]
async fn a_job_moved_from_the_checkout_into_a_slot_keeps_its_sketch_waiting() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "change a file").await;
    rig.stand_in.init(0);
    show(&rig, &id, "Fleet").await;

    let path = format!("{}/src/lib.rs", rig.root);
    assert!(denied(&rig.gate(&id, "Edit", &format!(r#"{{"file_path":"{path}"}}"#)).await), "the first write is held for a slot");
    rig.stand_in.finishes(0, "I will stop here");
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    assert_eq!(rig.stand_in.starts()[1].directory, adapter_traits::slot_path(&rig.root, 1), "moved into the slot");

    let moved = record(&rig, &id).await;
    assert_eq!(needs(&moved).iter().filter(|(item, _)| *item == TARGET).count(), 1, "{:?}", needs(&moved));
    let sketch = moved.attachments.iter().find(|one| one.target == TARGET).expect("the sketch went with it");
    assert!(!sketch.detail.contains_key("settled"), "still open");
}

#[tokio::test]
async fn a_step_ending_on_his_reply_waits_after_a_reply_that_only_reports_until_he_dismisses_it() {
    let (rig, id) = on("replied").await;
    rig.stand_in.finishes(0, "The reader keeps the last line now.");
    eventually(|| async { !record(&rig, &id).await.waiting_for.is_empty() }).await;
    let ended = record(&rig, &id).await;
    let [item] = ended.waiting_for.as_slice() else { panic!("one item: {:?}", needs(&ended)) };
    assert!(item.id.starts_with("replied:"), "{}", item.id);
    assert_eq!((item.text.as_str(), item.source), ("Propose: you reply", WaitingSource::Replied));

    let after = Arc::clone(&rig.fleet).dismiss_waiting(dismissed(&id, &item.id)).await.expect("dismissed");
    assert!(after.waiting_for.is_empty(), "{:?}", needs(&after));
    assert!(record(&rig, &id).await.waiting_for.is_empty(), "and it stays off");
}

#[tokio::test]
async fn a_reply_that_only_reports_on_a_step_his_reply_does_not_end_waits_on_nothing() {
    let (rig, id) = on("sketch_approved").await;
    rig.stand_in.finishes(0, "The reader keeps the last line now.");
    eventually(|| async { record(&rig, &id).await.standing == Some(ipc::SessionStanding::Replied) }).await;
    assert!(record(&rig, &id).await.waiting_for.is_empty(), "the step waits on a sketch, and none was shown");
}
