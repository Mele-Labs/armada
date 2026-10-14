//! The claim, written before the code: **a sketch an agent draws is kept on its Session's ledger as
//! an artifact it drew, a second one with the same title replaces the first, and a scene that
//! `parseScene` would draw as an empty frame is refused with the part at fault.**
//! `docs/concepts/session.md`, *Drawing a sketch*.

use api::{Refusal, Sessions};
use ipc::{Attachment, AttachmentState, SessionId, ShowSketch};

use super::session_host::rig_placing;

fn scene(second: &str) -> String {
    format!(
        r#"{{"nodes":[{{"id":"a","kind":"box","x":0,"y":0,"title":"Bridge","icon":"globe"}},{{"id":"b","kind":"box","x":300,"y":0,"title":"{second}"}}],"edges":[{{"id":"e1","from":"a","to":"b","label":"HTTP","flow":true}}]}}"#
    )
}

fn drawn(id: &SessionId, title: &str, scene: &str) -> ShowSketch {
    ipc::decode(
        "a sketch to show",
        format!(r#"{{"title":"{title}","session_id":"{}","scene":{scene}}}"#, id.as_str()).as_bytes(),
    )
    .unwrap()
}

/// The same scene whatever order the keys were written in.
fn same(kept: &str, sent: &str) {
    let read = |text: &str| ipc::decode::<ipc::RawScene>("a scene", text.as_bytes()).unwrap();
    assert_eq!(read(kept), read(sent));
}

fn sketches(record: &ipc::SessionRecord) -> Vec<&Attachment> {
    record
        .attachments
        .iter()
        .filter(|one| one.detail.get("form").map(String::as_str) == Some("sketch"))
        .collect()
}

#[tokio::test]
async fn a_sketch_is_kept_on_the_ledger_as_an_artifact_the_agent_drew() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let record = rig.fleet.show_sketch(None, drawn(&id, "Request flow", &scene("Fleet"))).await.unwrap();

    let kept = sketches(&record);
    assert_eq!(kept.len(), 1, "{:?}", record.attachments);
    assert_eq!((kept[0].kind.as_str(), kept[0].target.as_str()), ("artifact", "sketch:Request flow"));
    assert_eq!(kept[0].state, AttachmentState::Standing);
    assert_eq!(kept[0].detail["by"], "agent");
    assert_eq!(kept[0].detail["title"], "Request flow");
    same(&kept[0].detail["scene"], &scene("Fleet"));
}

#[tokio::test]
async fn the_sketch_is_read_back_from_the_store_and_not_only_answered() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow", &scene("Fleet"))).await.unwrap();

    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    let record = listed.iter().find(|one| one.id == id).expect("the session is listed");
    assert_eq!(sketches(record).len(), 1);
}

#[tokio::test]
async fn a_second_sketch_with_the_same_title_replaces_the_first_and_another_title_adds_one() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow", &scene("Fleet"))).await.unwrap();
    let record = rig.fleet.show_sketch(None, drawn(&id, " Request flow ", &scene("Daemon"))).await.unwrap();

    let kept = sketches(&record);
    assert_eq!(kept.len(), 1, "{:?}", record.attachments);
    same(&kept[0].detail["scene"], &scene("Daemon"));

    let record = rig.fleet.show_sketch(None, drawn(&id, "Storage", &scene("Store"))).await.unwrap();
    assert_eq!(sketches(&record).len(), 2);
}

#[tokio::test]
async fn a_scene_that_breaks_a_rule_is_refused_naming_the_part_and_nothing_is_kept() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let bad = scene("Fleet").replace(r#""kind":"box","x":300"#, r#""kind":"oval","x":300"#);
    let Err(Refusal::Unacceptable(refused)) = rig.fleet.show_sketch(None, drawn(&id, "Flow", &bad)).await else {
        panic!("a scene with an unknown kind is refused");
    };
    assert_eq!(refused.code, "fleet.sketch_refused");
    assert!(
        refused.message.contains("nodes[1] `b`: `kind` is `oval`. Allowed: box, group, lane, label, code, wire"),
        "{}",
        refused.message
    );
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    assert!(sketches(listed.iter().find(|one| one.id == id).unwrap()).is_empty());
}

#[tokio::test]
async fn a_call_that_places_no_session_is_refused() {
    let rig = rig_placing(None);
    let mut show = drawn(&SessionId::carried("nobody"), "Flow", &scene("Fleet"));
    show.session_id = None;
    let Err(Refusal::Unacceptable(refused)) = rig.fleet.show_sketch(None, show).await else {
        panic!("a sketch placed nowhere is refused");
    };
    assert_eq!(refused.code, "fleet.sketch_unnamed");
    let Err(Refusal::Unacceptable(_)) =
        rig.fleet.show_sketch(None, drawn(&SessionId::carried("nobody"), "Flow", &scene("Fleet"))).await
    else {
        panic!("a session Fleet does not know is refused");
    };
}
