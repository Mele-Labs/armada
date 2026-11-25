//! The claim, written before the code: **takes drawn side by side in one sketch, for Approve to stand in for a
//! pick, are refused, and the refusal says to put them in a `takes` part he answers with Send pick.** A plan's
//! sketch is held to the same rule. Two pictures that are not takes, and boxes titled like one, are drawn.
//! `crates/ipc/operations/show_sketch.toml`, `docs/concepts/artifact.md`, *Parts*.

use api::{HostedSessions, Refusal, Sessions};
use ipc::{SessionId, ShowSketch};

use super::session_host::rig_placing;

const SAID: &str = "the sketch was not drawn: `A · Bar (my pick)`, `B · Tabs` are takes, so put them to him with add_part, `kind` `takes`, each take a scene, your pick first, and he picks one with Send pick; Approve never stands in for a pick";

fn mock(id: &str, x: u32, title: &str) -> String {
    format!(r#"{{"id":"{id}","kind":"mock","x":{x},"y":0,"w":300,"title":"{title}","html":"<b class=\"a\">x</b>","css":".a{{color:red}}"}}"#)
}

fn scene(nodes: &[String]) -> String {
    format!(r#"{{"nodes":[{}],"edges":[]}}"#, nodes.join(","))
}

fn drawn(id: &SessionId, scene: &str) -> ShowSketch {
    ipc::decode("a sketch to show", format!(r#"{{"title":"Toolbar","question":"Does this read right?","session_id":"{}","scene":{scene}}}"#, id.as_str()).as_bytes()).unwrap()
}

fn refused<T: std::fmt::Debug>(said: Result<T, Refusal>) -> ipc::WireError {
    match said {
        Err(Refusal::Unacceptable(why)) => why,
        other => panic!("not refused: {other:?}"),
    }
}

#[tokio::test]
async fn takes_drawn_as_one_sketch_are_refused_saying_to_put_them_as_a_takes_part() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let takes = scene(&[mock("a", 0, "A · Bar (my pick)"), mock("b", 400, "B · Tabs")]);
    let why = refused(rig.fleet.show_sketch(None, drawn(&id, &takes)).await);
    assert_eq!(why.code, "fleet.sketch_takes");
    assert_eq!(why.message, SAID);
    let held = rig.fleet.get_job_artifacts(id.clone()).await.unwrap();
    assert!(held.is_empty(), "nothing is drawn: {held:?}");
}

#[tokio::test]
async fn takes_named_by_the_labels_just_above_them_are_refused_too() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let label = |id: &str, x: u32, title: &str| format!(r#"{{"id":"{id}","kind":"label","x":{x},"y":-60,"title":"{title}"}}"#);
    let takes = scene(&[label("la", 0, "Take A: a bar"), mock("a", 0, "Bar"), label("lb", 400, "Take B: tabs"), mock("b", 400, "Tabs")]);
    let why = refused(rig.fleet.show_sketch(None, drawn(&id, &takes)).await);
    assert_eq!(why.code, "fleet.sketch_takes");
    assert!(why.message.starts_with("the sketch was not drawn: `Take A: a bar`, `Take B: tabs` are takes"), "{}", why.message);
}

#[tokio::test]
async fn two_pictures_that_are_not_takes_and_boxes_titled_like_one_are_drawn() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let before_after = scene(&[mock("a", 0, "Before"), mock("b", 400, "After")]);
    rig.fleet.show_sketch(None, drawn(&id, &before_after)).await.expect("before and after are drawn");
    let box_ = |id: &str, x: u32, title: &str| format!(r#"{{"id":"{id}","kind":"box","x":{x},"y":0,"title":"{title}"}}"#);
    let flow = scene(&[box_("a", 0, "A Job takes it"), box_("b", 300, "B · Fleet checks it")]);
    rig.fleet.show_sketch(None, drawn(&id, &flow)).await.expect("boxes are not takes");
}

#[tokio::test]
async fn a_plan_whose_sketch_draws_takes_is_refused_the_same_way() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let mut plan = super::session_rail::plan(&id);
    let takes = scene(&[mock("a", 0, "A · Bar (my pick)"), mock("b", 400, "B · Tabs")]);
    plan.sketch = Some(ipc::PlanSketchInput { title: "Toolbar".into(), scene: ipc::decode("a scene", takes.as_bytes()).unwrap() });
    let why = refused(rig.fleet.record_plan(None, plan, false).await);
    assert_eq!(why.code, "fleet.sketch_takes");
    assert_eq!(why.message, SAID.replace("the sketch was not drawn", "the plan was not recorded"));
}
