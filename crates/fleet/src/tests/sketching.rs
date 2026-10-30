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

/// The paths a sketch's `revisions` detail names, oldest first.
fn revisions(one: &Attachment) -> Option<Vec<String>> {
    Some(ipc::decode("revisions", one.detail.get("revisions")?.as_bytes()).expect("a list of paths"))
}

#[tokio::test]
async fn a_redraw_keeps_each_changed_scene_as_a_revision_and_an_unchanged_one_adds_none() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow", &scene("Fleet"))).await.unwrap();
    let record = rig.fleet.show_sketch(None, drawn(&id, "Request flow", &scene("Fleet"))).await.unwrap();
    let held = revisions(sketches(&record)[0]).expect("revisions");
    assert_eq!(held.len(), 1, "the same scene again adds no file: {held:?}");

    let record = rig.fleet.show_sketch(None, drawn(&id, "Request flow", &scene("Daemon"))).await.unwrap();
    let kept = sketches(&record);
    assert_eq!(kept.len(), 1, "{:?}", record.attachments);
    let held = revisions(kept[0]).expect("revisions");
    assert_eq!(held.len(), 2, "{held:?}");
    for (path, name) in held.iter().zip(["r1.json", "r2.json"]) {
        assert!(path.ends_with(name), "oldest first: {held:?}");
    }
    same(&std::fs::read_to_string(&held[0]).unwrap(), &scene("Fleet"));
    same(&std::fs::read_to_string(&held[1]).unwrap(), &scene("Daemon"));
    same(&kept[0].detail["scene"], &scene("Daemon"));
}

#[tokio::test]
async fn titles_that_clean_up_alike_keep_separate_revisions() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "a b", &scene("Fleet"))).await.unwrap();
    let record = rig.fleet.show_sketch(None, drawn(&id, "a_b", &scene("Daemon"))).await.unwrap();

    let kept = sketches(&record);
    assert_eq!(kept.len(), 2, "{:?}", record.attachments);
    let [spaced, underscored] = ["sketch:a b", "sketch:a_b"].map(|target| {
        let one = kept.iter().find(|one| one.target == target).expect("held");
        revisions(one).expect("revisions")
    });
    assert_eq!((spaced.len(), underscored.len()), (1, 1));
    assert_ne!(std::path::Path::new(&spaced[0]).parent(), std::path::Path::new(&underscored[0]).parent());
    same(&std::fs::read_to_string(&spaced[0]).unwrap(), &scene("Fleet"));
    same(&std::fs::read_to_string(&underscored[0]).unwrap(), &scene("Daemon"));
}

#[tokio::test]
async fn a_plans_sketch_carries_no_revisions() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut recorded = super::session_plan::first(&id);
    recorded.sketch = Some(
        ipc::decode("a plan sketch", format!(r#"{{"title":"Plan flow","scene":{}}}"#, scene("Fleet")).as_bytes()).unwrap(),
    );
    let record = rig.fleet.record_plan(None, recorded, false).await.expect("recorded");

    let kept = sketches(&record);
    assert_eq!(kept.len(), 1, "{:?}", record.attachments);
    assert_eq!(kept[0].target, "sketch:Plan flow");
    assert!(!kept[0].detail.contains_key("revisions"), "{:?}", kept[0].detail);
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

fn drawn_against(id: &SessionId, title: &str, scene: &str, against: &str) -> ShowSketch {
    ipc::decode(
        "a sketch to show",
        format!(r#"{{"title":"{title}","session_id":"{}","scene":{scene},"against":"{against}"}}"#, id.as_str()).as_bytes(),
    )
    .unwrap()
}

#[derive(serde::Deserialize)]
struct Against {
    title: String,
    scene: String,
}

#[tokio::test]
async fn a_sketch_drawn_against_another_keeps_the_earlier_scene_whole_and_a_later_replacement_does_not_move_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Before", &scene("Fleet"))).await.unwrap();
    let record = rig.fleet.show_sketch(None, drawn_against(&id, "After", &scene("Daemon"), " Before ")).await.unwrap();

    let after = sketches(&record).into_iter().find(|one| one.detail["title"] == "After").unwrap();
    let kept: Against = ipc::decode("an against", after.detail["against"].as_bytes()).unwrap();
    assert_eq!(kept.title, "Before");
    same(&kept.scene, &scene("Fleet"));
    same(&after.detail["scene"], &scene("Daemon"));
    assert_eq!(revisions(after).map(|held| held.len()), Some(1), "an against sketch keeps its revision too");
    let before = sketches(&record).into_iter().find(|one| one.detail["title"] == "Before").unwrap();
    assert!(!before.detail.contains_key("against"), "a sketch with no against has none");

    // Replacing the earlier sketch afterwards leaves the diff where it was.
    let record = rig.fleet.show_sketch(None, drawn(&id, "Before", &scene("Store"))).await.unwrap();
    let after = sketches(&record).into_iter().find(|one| one.detail["title"] == "After").unwrap();
    let kept: Against = ipc::decode("an against", after.detail["against"].as_bytes()).unwrap();
    same(&kept.scene, &scene("Fleet"));
}

#[tokio::test]
async fn a_sketch_may_be_redrawn_against_the_one_it_replaces() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Flow", &scene("Fleet"))).await.unwrap();
    let record = rig.fleet.show_sketch(None, drawn_against(&id, "Flow", &scene("Daemon"), "Flow")).await.unwrap();

    let kept = sketches(&record);
    assert_eq!(kept.len(), 1);
    same(&kept[0].detail["scene"], &scene("Daemon"));
    let against: Against = ipc::decode("an against", kept[0].detail["against"].as_bytes()).unwrap();
    same(&against.scene, &scene("Fleet"));
    assert_eq!(revisions(kept[0]).map(|held| held.len()), Some(2), "the redraw is the second revision");
}

#[tokio::test]
async fn an_against_the_session_holds_no_sketch_of_is_refused_naming_the_titles_it_does_hold() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let Err(Refusal::Unacceptable(none)) =
        rig.fleet.show_sketch(None, drawn_against(&id, "After", &scene("Daemon"), "Before")).await
    else {
        panic!("an earlier sketch that does not exist is refused");
    };
    assert_eq!(none.code, "fleet.sketch_against_unknown");
    assert!(none.message.contains("holds no sketch titled `Before`") && none.message.contains("Sketches it holds: none"), "{}", none.message);

    rig.fleet.show_sketch(None, drawn(&id, "Storage", &scene("Store"))).await.unwrap();
    let Err(Refusal::Unacceptable(some)) =
        rig.fleet.show_sketch(None, drawn_against(&id, "After", &scene("Daemon"), "Before")).await
    else {
        panic!("still refused");
    };
    assert!(some.message.contains("Sketches it holds: Storage"), "{}", some.message);
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    assert_eq!(sketches(listed.iter().find(|one| one.id == id).unwrap()).len(), 1, "nothing was kept");
}

#[tokio::test]
async fn a_sketch_of_another_artifact_form_is_not_an_earlier_sketch() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet
        .show_window(None, ipc::ShowWindow { url: "http://localhost:5173/".into(), title: Some("Before".into()), session_id: Some(id.clone()), variations: Vec::new(), ..Default::default() })
        .await
        .unwrap();
    let Err(Refusal::Unacceptable(refused)) =
        rig.fleet.show_sketch(None, drawn_against(&id, "After", &scene("Daemon"), "Before")).await
    else {
        panic!("a window titled Before is not a sketch");
    };
    assert_eq!(refused.code, "fleet.sketch_against_unknown");
}
