//! The claim, written before the code: **an item a Job raises about one of its artifacts names it, by its id or its
//! title, and Fleet keeps and serves the id so Bridge can open it; an artifact the Job does not hold is refused.**
//! `crates/ipc/operations/waiting_for.toml`.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{DismissWaiting, ReopenWaiting, SessionId, SetWaitingFor, ShowSketch, WaitingInput};

use super::session_host::rig_placing;

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;

fn drawn(id: &SessionId, title: &str) -> ShowSketch {
    ipc::decode("a sketch to show", format!(r#"{{"title":"{title}","question":"Does this read right?","session_id":"{}","scene":{SCENE}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn about(id: &SessionId, item: &str, artifact: Option<&str>) -> SetWaitingFor {
    let item = WaitingInput {
        approves: false,
        id: item.into(),
        text: "Pick the layout".into(),
        act: None,
        options: Vec::new(),
        steps: Vec::new(),
        artifact: artifact.map(str::to_string),
    };
    SetWaitingFor { session_id: Some(id.clone()), items: vec![item] }
}

fn linked(record: &ipc::SessionRecord, item: &str) -> Option<String> {
    record.waiting_for.iter().find(|one| one.id == item)?.artifact.clone()
}

#[tokio::test]
async fn an_item_names_an_artifact_by_its_id_or_its_title_and_the_wire_carries_the_id() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();

    let by_id = rig.fleet.waiting_for(None, about(&id, "pick", Some("sketch:Request flow"))).await.unwrap();
    assert_eq!(linked(&by_id, "pick").as_deref(), Some("sketch:Request flow"));
    let by_title = rig.fleet.waiting_for(None, about(&id, "pick", Some(" Request flow "))).await.unwrap();
    assert_eq!(linked(&by_title, "pick").as_deref(), Some("sketch:Request flow"));
    let none = rig.fleet.waiting_for(None, about(&id, "pick", None)).await.unwrap();
    assert_eq!(linked(&none, "pick"), None);
}

#[tokio::test]
async fn an_artifact_the_job_does_not_hold_is_refused_naming_the_ones_it_does_and_nothing_is_kept() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();
    rig.fleet.waiting_for(None, about(&id, "kept", None)).await.unwrap();

    let refused = rig.fleet.waiting_for(None, about(&id, "pick", Some("Storage"))).await.expect_err("no such artifact");
    let said = format!("{refused:?}");
    assert!(said.contains("fleet.artifact_not_held") && said.contains("sketch:Request flow"), "{said}");
    let record = Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session;
    assert!(record.waiting_for.iter().any(|one| one.id == "kept"), "{:?}", record.waiting_for);
    assert!(record.waiting_for.iter().all(|one| one.id != "pick"), "{:?}", record.waiting_for);
}

#[tokio::test]
async fn the_link_survives_closing_and_reopening_the_item() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, drawn(&id, "Request flow")).await.unwrap();
    rig.fleet.waiting_for(None, about(&id, "pick", Some("Request flow"))).await.unwrap();

    let closed = Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: id.clone(), item_id: "pick".into(), reason: None, misread: false })
        .await
        .unwrap();
    assert_eq!(closed.closed.iter().find(|one| one.item.id == "pick").and_then(|one| one.item.artifact.clone()).as_deref(), Some("sketch:Request flow"));
    let back = Arc::clone(&rig.fleet).reopen_waiting(ReopenWaiting { session_id: id.clone(), item_id: "pick".into() }).await.unwrap();
    assert_eq!(linked(&back, "pick").as_deref(), Some("sketch:Request flow"));
}
