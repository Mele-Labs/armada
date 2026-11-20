//! The claim, written before the code: **an agent's `approves` item naming an artifact that already asks him is refused,
//! as the same Approve said twice, and one kept goes off his list when he approves that artifact; an item naming no
//! artifact, or one that asks nothing, stands.** `crates/ipc/operations/waiting_for.toml`.

use std::sync::Arc;

use api::{Refusal, Sessions};
use ipc::{AnswerSketch, SessionId, SetWaitingFor, ShowSketch, SketchHow, WaitingInput};

use super::session_host::{rig_placing, Rig};

const SCENE: &str = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"title":"Bay"}],"edges":[]}"#;

fn sketch(id: &SessionId, asks: bool) -> ShowSketch {
    let body = format!(r#"{{"title":"Bay flow","question":"Does this read right?","session_id":"{}","scene":{SCENE},"asks":{asks}}}"#, id.as_str());
    ipc::decode("a sketch to show", body.as_bytes()).unwrap()
}

fn approve(id: &str, artifact: Option<&str>) -> WaitingInput {
    WaitingInput { id: id.into(), text: "Approve the bay flow?".into(), act: None, options: Vec::new(), approves: true, steps: Vec::new(), artifact: artifact.map(String::from) }
}

async fn states(rig: &Rig, id: &SessionId, items: Vec<WaitingInput>) -> Result<(), Refusal> {
    rig.fleet.waiting_for(None, SetWaitingFor { items, session_id: Some(id.clone()) }).await.map(|_| ())
}

async fn his_list(rig: &Rig, id: &SessionId) -> Vec<String> {
    rig.fleet.published_hosted(id.as_str()).await.unwrap().waiting_for.into_iter().map(|one| one.id).collect()
}

#[tokio::test]
async fn an_approve_item_naming_a_sketch_that_asks_is_refused() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, sketch(&id, true)).await.unwrap();

    let Err(Refusal::Unacceptable(refused)) = states(&rig, &id, vec![approve("bay", Some("Bay flow"))]).await else { panic!("refused") };
    assert_eq!(refused.code, "fleet.session_waiting_artifact_asks");
    assert!(refused.message.contains("already asks him"), "{}", refused.message);
    assert!(!his_list(&rig, &id).await.contains(&"bay".to_string()));
}

#[tokio::test]
async fn an_approve_item_naming_no_artifact_or_one_that_asks_nothing_is_kept() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.show_sketch(None, sketch(&id, false)).await.unwrap();

    states(&rig, &id, vec![approve("plain", None), approve("bay", Some("Bay flow"))]).await.expect("kept");
    let list = his_list(&rig, &id).await;
    assert!(list.contains(&"plain".to_string()) && list.contains(&"bay".to_string()), "{list:?}");
}

#[tokio::test]
async fn his_approval_of_a_sketch_takes_off_the_item_naming_it_and_leaves_the_rest() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    // Kept while the sketch does not ask, then the Job draws it asking: the item stands from before.
    rig.fleet.show_sketch(None, sketch(&id, false)).await.unwrap();
    states(&rig, &id, vec![approve("bay", Some("Bay flow")), approve("plain", None)]).await.expect("kept");
    let shown = rig.fleet.show_sketch(None, sketch(&id, true)).await.unwrap();
    let drawn = shown.attachments.iter().find(|one| one.target == "sketch:Bay flow").and_then(|one| one.detail.get("drawn")).cloned().expect("drawn");
    assert!(his_list(&rig, &id).await.contains(&"bay".to_string()));

    let said = AnswerSketch { session_id: id.clone(), sketch: "sketch:Bay flow".into(), drawn, how: SketchHow::Approved, words: None };
    let record = Arc::clone(&rig.fleet).answer_sketch(said).await.expect("approved");
    let list: Vec<&str> = record.waiting_for.iter().map(|one| one.id.as_str()).collect();
    assert!(!list.contains(&"bay") && list.contains(&"plain"), "{list:?}");
    assert!(!his_list(&rig, &id).await.contains(&"bay".to_string()));
}
