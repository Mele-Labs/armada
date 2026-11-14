//! The claim, written before the code: **a plan names the sketches its Job drew with show_sketch, for the
//! plan and beside a group; while that version is pending they wait with it and not on their own, and its
//! one Approve settles them approved and names them to the Job. A plan set aside, or a version that no
//! longer names one, lets it wait on its own again, and a title the Job holds no sketch for is refused.**
//! `crates/ipc/operations/record_plan.toml`, `docs/concepts/artifact.md`, *A plan's sketches*.

use std::sync::Arc;

use api::{HostedSessions, Refusal, Sessions};
use ipc::{ApprovePlan, ArtifactState, DismissWaiting, JobArtifact, PlanSketchInput, RecordPlan, SessionId, SessionRecord, ShowSketch, WaitingSource};

use super::session_host::{eventually, rig_placing, Rig};
use super::session_plan::{first, read, recorded, running, told};

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;

fn drawn(id: &SessionId, title: &str) -> ShowSketch {
    ipc::decode("a sketch to show", format!(r#"{{"title":"{title}","session_id":"{}","scene":{SCENE}}}"#, id.as_str()).as_bytes()).unwrap()
}

/// The two-group plan, naming `whole` for the plan and `beside` beside its second group.
fn naming(id: &SessionId, whole: &[&str], beside: &[&str]) -> RecordPlan {
    let mut plan = first(id);
    plan.sketches = whole.iter().map(|one| one.to_string()).collect();
    plan.groups[1].sketches = beside.iter().map(|one| one.to_string()).collect();
    plan
}

/// A running Job that drew `titles` with show_sketch, and whose turn then ended, so each waits on him.
async fn drew(rig: &Rig, titles: &[&str]) -> SessionId {
    let id = running(rig).await;
    for title in titles {
        rig.fleet.show_sketch(None, drawn(&id, title)).await.unwrap();
    }
    rig.stand_in.finishes(0, "Drawn.");
    eventually(|| async { waits(&read(rig, &id).await).len() == titles.len() }).await;
    id
}

/// The plan's and the sketches' items in his Needs you.
fn waits(record: &SessionRecord) -> Vec<String> {
    record.waiting_for.iter().filter(|one| matches!(one.source, WaitingSource::Plan | WaitingSource::Sketch)).map(|one| one.id.clone()).collect()
}

async fn sketches(rig: &Rig, id: &SessionId) -> Vec<JobArtifact> {
    let mut held: Vec<JobArtifact> = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    held.sort_by(|a, b| a.title.cmp(&b.title));
    held
}

fn standing(held: &[JobArtifact]) -> Vec<(&str, ArtifactState, Option<&str>, Option<u32>)> {
    held.iter().map(|one| (one.title.as_str(), one.state, one.item.as_deref(), one.in_plan)).collect()
}

#[tokio::test]
async fn two_sketches_a_pending_plan_names_wait_with_it_and_its_approve_settles_both_and_names_them() {
    let rig = rig_placing(None);
    let id = drew(&rig, &["Store rows", "Fleet flow"]).await;
    assert_eq!(waits(&read(&rig, &id).await), ["sketch:Fleet flow", "sketch:Store rows"], "each waits on its own before a plan names it");

    let record = recorded(&rig, naming(&id, &["Store rows"], &["Fleet flow"])).await;
    assert_eq!(waits(&record), ["plan:1"], "one item: the plan");
    let plan = record.plan.expect("a plan");
    assert_eq!(plan.sketches.iter().map(|one| (one.id.as_str(), one.title.as_str())).collect::<Vec<_>>(), [("sketch:Store rows", "Store rows")]);
    assert!(plan.groups[0].sketches.is_empty());
    assert_eq!(plan.groups[1].sketches.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["sketch:Fleet flow"]);
    assert_eq!(
        standing(&sketches(&rig, &id).await),
        [("Fleet flow", ArtifactState::Asking, Some("plan:1"), Some(1)), ("Store rows", ArtifactState::Asking, Some("plan:1"), Some(1))]
    );

    let approved = Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 1 }).await.unwrap();
    assert!(waits(&approved).is_empty(), "nothing waits after one Approve: {:?}", approved.waiting_for);
    eventually(|| async { told(&rig, r#"With it: \"Store rows\", \"Fleet flow\"."#) == 1 }).await;
    assert_eq!(
        standing(&sketches(&rig, &id).await),
        [("Fleet flow", ArtifactState::Approved, None, Some(1)), ("Store rows", ArtifactState::Approved, None, Some(1))]
    );
    let held = read(&rig, &id).await;
    for one in held.attachments.iter().filter(|one| one.target.starts_with("sketch:")) {
        assert_eq!(one.detail.get("settled").map(String::as_str), Some("approved"), "{:?}", one.detail);
        assert!(one.detail.contains_key("settled_at") && !one.detail.contains_key("words"), "{:?}", one.detail);
    }
}

#[tokio::test]
async fn a_sketch_title_the_job_holds_none_for_is_refused_naming_the_ones_it_holds() {
    let rig = rig_placing(None);
    let id = drew(&rig, &["Store rows"]).await;
    let Err(Refusal::Unacceptable(refused)) = rig.fleet.record_plan(None, naming(&id, &[], &["Fleet flow"]), false).await else {
        panic!("a plan naming a sketch never drawn is refused");
    };
    assert_eq!(refused.code, "fleet.plan_refused");
    assert!(refused.message.contains("`Fleet flow`") && refused.message.contains("Sketches it holds: Store rows"), "{}", refused.message);
    assert!(read(&rig, &id).await.plan.is_none(), "nothing is kept");
}

#[tokio::test]
async fn a_plan_set_aside_lets_its_asking_sketch_wait_on_its_own_again() {
    let rig = rig_placing(None);
    let id = drew(&rig, &["Store rows"]).await;
    assert_eq!(waits(&recorded(&rig, naming(&id, &["Store rows"], &[])).await), ["plan:1"]);

    let dismissal = DismissWaiting { session_id: id.clone(), item_id: "plan:1".into(), reason: None, misread: false };
    let after = Arc::clone(&rig.fleet).dismiss_waiting(dismissal).await.unwrap();
    assert_eq!(waits(&after), ["sketch:Store rows"]);
    assert_eq!(standing(&sketches(&rig, &id).await), [("Store rows", ArtifactState::Asking, Some("sketch:Store rows"), None)]);
}

#[tokio::test]
async fn a_version_that_no_longer_names_a_sketch_lets_it_wait_on_its_own_again() {
    let rig = rig_placing(None);
    let id = drew(&rig, &["Store rows"]).await;
    assert_eq!(waits(&recorded(&rig, naming(&id, &["Store rows"], &[])).await), ["plan:1"]);

    let again = recorded(&rig, naming(&id, &[], &[])).await;
    let mut ids = waits(&again);
    ids.sort();
    assert_eq!(ids, ["plan:2", "sketch:Store rows"]);
}

#[tokio::test]
async fn a_plans_inline_sketch_is_still_approved_with_it_and_names_nothing() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let mut plan = first(&id);
    plan.sketch = Some(ipc::decode::<PlanSketchInput>("a plan sketch", format!(r#"{{"title":"Plan flow","scene":{SCENE}}}"#).as_bytes()).unwrap());
    let record = recorded(&rig, plan).await;
    assert_eq!(waits(&record), ["plan:1"], "the inline sketch never waits on its own");
    assert_eq!(record.plan.expect("a plan").sketch.map(|one| one.id), Some("sketch:Plan flow".to_string()));

    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 1 }).await.unwrap();
    eventually(|| async { told(&rig, "approved plan v1") == 1 }).await;
    assert_eq!(told(&rig, "With it:"), 0);
    assert_eq!(standing(&sketches(&rig, &id).await), [("Plan flow", ArtifactState::Reading, None, None)]);
}
