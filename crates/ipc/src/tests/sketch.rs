//! `show_sketch`: a scene Fleet keeps is one `parseScene` draws, and one it refuses is refused with
//! words that name the part and what is allowed. Bridge draws a bad scene as an empty frame without
//! a word, so this is the only place the agent hears of it.

use serde_json::json;

use crate::sketch::{
    EDGE_FIELDS, NODE_FIELDS, MOST_EDGES, MOST_NODES, MOST_SCENE_BYTES, MOST_TITLE_CHARS, SCENE_ICONS, SCENE_KINDS, WIRE_PARTS,
};
use crate::ShowSketch;

const SCENE_TS: &str = include_str!("../../../../packages/components/src/compositions/SketchScene/scene.ts");
const OPERATION: &str = include_str!("../../operations/show_sketch.toml");

/// The strings of `export const <name> = [ ... ] as const;` in `scene.ts`.
fn listed_in_scene_ts(name: &str) -> Vec<String> {
    let start = SCENE_TS
        .find(&format!("export const {name} = ["))
        .unwrap_or_else(|| panic!("scene.ts no longer exports {name}"));
    let rest = &SCENE_TS[start..];
    let body = &rest[rest.find('[').unwrap() + 1..rest.find("] as const").unwrap()];
    body.split('"').skip(1).step_by(2).map(String::from).collect()
}

fn sent(title: &str, scene: serde_json::Value) -> ShowSketch {
    crate::decode("a sketch to show", json!({ "title": title, "scene": scene }).to_string().as_bytes()).unwrap()
}

fn refused(scene: serde_json::Value) -> String {
    sent("Flow", scene).checked().expect_err("the scene should be refused")
}

fn two_boxes() -> serde_json::Value {
    json!({
        "nodes": [
            { "id": "a", "kind": "box", "x": 0, "y": 0, "title": "Bridge", "icon": "globe" },
            { "id": "b", "kind": "box", "x": 300, "y": 0, "title": "Fleet", "body": "axum" },
        ],
        "edges": [{ "id": "e1", "from": "a", "to": "b", "label": "HTTP", "flow": true }],
    })
}

#[test]
fn the_lists_the_checker_allows_are_the_lists_scene_ts_parses() {
    assert_eq!(listed_in_scene_ts("SCENE_KINDS"), SCENE_KINDS);
    assert_eq!(listed_in_scene_ts("WIRE_PARTS"), WIRE_PARTS);
    assert_eq!(listed_in_scene_ts("SCENE_ICONS"), SCENE_ICONS);
}

/// The field names of `export type <name> = { ... };` in `scene.ts`.
fn fields_in_scene_ts(name: &str) -> Vec<String> {
    let start = SCENE_TS
        .find(&format!("export type {name} = {{"))
        .unwrap_or_else(|| panic!("scene.ts no longer exports type {name}"));
    let rest = &SCENE_TS[start..];
    let mut body = rest[rest.find('{').unwrap() + 1..[rest.find("\n};"), rest.find(" };")].into_iter().flatten().min().unwrap()].to_string();
    while let Some(open) = body.find("/*") {
        let close = body[open..].find("*/").expect("a comment closes") + open + 2;
        body.replace_range(open..close, "");
    }
    body.split(';')
        .filter_map(|part| part.split_once(':'))
        .map(|(field, _)| field.trim().trim_end_matches('?').to_string())
        .filter(|field| !field.is_empty())
        .collect()
}

/// Edge `step` is taken here before `scene.ts` has it; remove this when the type carries it.
const AHEAD_OF_SCENE_TS: [&str; 1] = ["step"];

#[test]
fn the_fields_the_checker_takes_are_the_fields_scene_ts_types() {
    let mut node = fields_in_scene_ts("SceneNode");
    node.sort();
    let mut ours: Vec<&str> = NODE_FIELDS.to_vec();
    ours.sort_unstable();
    assert_eq!(node, ours);

    let mut edge = fields_in_scene_ts("SceneEdge");
    edge.extend(AHEAD_OF_SCENE_TS.iter().map(|one| one.to_string()));
    edge.sort();
    edge.dedup();
    let mut ours: Vec<&str> = EDGE_FIELDS.to_vec();
    ours.sort_unstable();
    assert_eq!(edge, ours);
}

#[test]
fn a_step_is_a_number_on_a_node_and_on_an_edge() {
    let mut scene = two_boxes();
    scene["nodes"][0]["step"] = json!(1);
    scene["edges"][0]["step"] = json!(1.5);
    sent("Flow", scene.clone()).checked().unwrap();
    scene["edges"][0]["step"] = json!("second");
    let said = refused(scene.clone());
    assert!(said.contains("edges[0] `e1`: `step` is `second`. It must be a number"), "{said}");
    scene["edges"][0]["step"] = json!(null);
    assert!(refused(scene).contains("edges[0] `e1`: `step` is null"));
}

#[test]
fn the_description_says_how_steps_and_against_work() {
    for phrase in [
        "nodes and edges share one numbering",
        "plays together, in ascending order",
        "from `from` to `to`",
        "never pulses",
        "group nodes as lifelines",
        "label nodes as points",
        "against is the title of an earlier sketch",
    ] {
        assert!(OPERATION.contains(phrase), "show_sketch.toml does not say `{phrase}`");
    }
}

#[test]
fn against_is_trimmed_and_a_blank_one_is_refused_rather_than_taken_for_none() {
    let mut call = sent("Flow", two_boxes());
    assert_eq!(call.against_title(), Ok(None));
    call.against = Some("  Before ".into());
    assert_eq!(call.against_title(), Ok(Some("Before".into())));
    call.against = Some("  ".into());
    assert!(call.against_title().unwrap_err().contains("`against` is blank"));
}

#[test]
fn the_earlier_scene_is_kept_whole_beside_its_title_and_counted_in_the_cap() {
    let earlier = sent("Before", two_boxes()).checked().unwrap().scene;
    let kept = sent("Flow", two_boxes()).checked().unwrap().read_against("Before", &earlier).unwrap();
    let against: serde_json::Value = serde_json::from_str(&kept.against.expect("kept")).unwrap();
    assert_eq!(against["title"], "Before");
    assert_eq!(against["scene"], earlier.as_str());

    let heavy = "x".repeat(MOST_SCENE_BYTES);
    let said = sent("Flow", two_boxes()).checked().unwrap().read_against("Before", &heavy).unwrap_err();
    assert!(said.contains("the sketch `Before` it is drawn against weigh"), "{said}");
    assert!(said.contains("together and a sketch holds at most 32768"), "{said}");
}

#[test]
fn the_tool_description_names_every_kind_icon_and_wire_part() {
    for word in SCENE_KINDS.iter().chain(WIRE_PARTS.iter()).chain(SCENE_ICONS.iter()) {
        assert!(OPERATION.contains(word), "show_sketch.toml does not name `{word}`");
    }
    assert!(OPERATION.contains("ASCII"), "the description must forbid ASCII art");
}

#[test]
fn a_scene_that_parses_is_kept_whole_with_its_counts() {
    let kept = sent("  Flow  ", two_boxes()).checked().unwrap();
    assert_eq!((kept.title.as_str(), kept.nodes, kept.edges), ("Flow", 2, 1));
    let back: serde_json::Value = serde_json::from_str(&kept.scene).unwrap();
    assert_eq!(back, two_boxes());
}

#[test]
fn strokes_are_read_as_scene_ts_reads_them() {
    let mut scene = two_boxes();
    scene["strokes"] = json!([{ "id": "s1", "points": [{ "x": 1, "y": 2 }, { "x": 3.5, "y": 4 }] }]);
    sent("Flow", scene.clone()).checked().unwrap();
    scene["strokes"][0]["points"][1] = json!({ "x": 3 });
    assert!(refused(scene).contains("strokes[0].points[1]: needs `y`"));
}

#[test]
fn an_unknown_kind_names_the_node_and_lists_the_kinds() {
    let mut scene = two_boxes();
    scene["nodes"][1]["kind"] = json!("rectangle");
    let said = refused(scene);
    assert!(said.contains("nodes[1] `b`: `kind` is `rectangle`. Allowed: box, group, lane, label, code, wire"), "{said}");
}

#[test]
fn an_unknown_icon_and_wire_part_list_what_is_allowed() {
    let mut scene = two_boxes();
    scene["nodes"][0]["icon"] = json!("database");
    scene["nodes"][1]["kind"] = json!("wire");
    scene["nodes"][1]["wire"] = json!("table");
    let said = refused(scene);
    assert!(said.contains("nodes[0] `a`: `icon` is `database`. Allowed: activity, bot,"), "{said}");
    assert!(said.contains("nodes[1] `b`: `wire` is `table`. Allowed: button, input, list, card, nav, text"), "{said}");
}

#[test]
fn a_group_and_a_lane_need_their_size() {
    let mut scene = two_boxes();
    scene["nodes"][0]["kind"] = json!("group");
    scene["nodes"][1]["kind"] = json!("lane");
    scene["nodes"][1]["w"] = json!(400);
    let said = refused(scene);
    assert!(said.contains("nodes[0] `a`: a `group` needs `w`, its size"), "{said}");
    assert!(said.contains("nodes[0] `a`: a `group` needs `h`, its size"), "{said}");
    assert!(said.contains("nodes[1] `b`: a `lane` needs `h`, its size"), "{said}");
    assert!(!said.contains("nodes[1] `b`: a `lane` needs `w`"), "{said}");
}

#[test]
fn an_edge_to_a_node_the_scene_lacks_names_the_edge_and_the_ids_it_could_join() {
    let mut scene = two_boxes();
    scene["edges"][0]["to"] = json!("db");
    let said = refused(scene);
    assert!(said.contains("edges[0] `e1`: `to` is `db`, which no node has. Node ids: a, b"), "{said}");
}

#[test]
fn a_repeated_id_is_refused_in_nodes_and_in_edges() {
    let mut scene = two_boxes();
    scene["nodes"][1]["id"] = json!("a");
    scene["edges"] = json!([
        { "id": "e", "from": "a", "to": "a" },
        { "id": "e", "from": "a", "to": "a" },
    ]);
    let said = refused(scene);
    assert!(said.contains("nodes[1] `a`: repeats the id of nodes[0]"), "{said}");
    assert!(said.contains("edges[1] `e`: repeats the id of edges[0]"), "{said}");
}

#[test]
fn a_field_the_format_has_no_name_for_is_refused_instead_of_dropped() {
    let mut scene = two_boxes();
    scene["nodes"][0]["titel"] = json!("Bridge");
    scene["colour"] = json!("red");
    let said = refused(scene);
    assert!(said.contains("nodes[0] `a`: has `titel`, which it does not take. Fields: id, kind"), "{said}");
    assert!(said.contains("`scene`: has `colour`, which it does not take. Fields: nodes, edges, strokes"), "{said}");
}

#[test]
fn a_scene_sent_as_text_a_list_less_scene_and_an_empty_one_are_each_told_what_to_send() {
    assert!(refused(json!("{\"nodes\":[]}")).contains("`scene` is a string. Send the object itself"));
    assert!(refused(json!({ "nodes": [] })).contains("`edges` is missing"));
    assert!(refused(json!({ "nodes": [], "edges": [] })).contains("`nodes` is empty"));
    assert!(refused(json!({ "nodes": {}, "edges": [] })).contains("`nodes` is an object. It must be a list"));
}

#[test]
fn every_problem_is_said_at_once_up_to_a_dozen_and_the_rest_counted() {
    let nodes: Vec<_> = (0..20).map(|n| json!({ "id": format!("n{n}"), "kind": "oval", "x": 0, "y": 0 })).collect();
    let said = refused(json!({ "nodes": nodes, "edges": [] }));
    assert_eq!(said.matches("`kind` is `oval`").count(), 12, "{said}");
    assert!(said.contains("; and 8 more"), "{said}");
}

#[test]
fn a_blank_title_and_a_long_one_are_refused() {
    let blank = sent("   ", two_boxes()).checked().unwrap_err();
    assert!(blank.contains("`title` is blank"), "{blank}");
    let long = sent(&"t".repeat(MOST_TITLE_CHARS + 1), two_boxes()).checked().unwrap_err();
    assert!(long.contains("`title` is longer than 120 characters"), "{long}");
}

#[test]
fn a_scene_past_the_node_edge_or_byte_cap_is_refused_with_the_cap() {
    let nodes: Vec<_> = (0..=MOST_NODES).map(|n| json!({ "id": format!("n{n}"), "kind": "box", "x": 0, "y": 0 })).collect();
    let said = refused(json!({ "nodes": nodes, "edges": [] }));
    assert!(said.contains("`nodes` holds 81 and a sketch holds at most 80"), "{said}");

    let edges: Vec<_> = (0..=MOST_EDGES).map(|n| json!({ "id": format!("e{n}"), "from": "a", "to": "b" })).collect();
    let mut scene = two_boxes();
    scene["edges"] = json!(edges);
    let said = refused(scene);
    assert!(said.contains("`edges` holds 121 and a sketch holds at most 120"), "{said}");

    let mut heavy = two_boxes();
    heavy["nodes"][0]["body"] = json!("x".repeat(MOST_SCENE_BYTES));
    let said = refused(heavy);
    assert!(said.contains("a sketch holds at most 32768"), "{said}");
}

#[test]
fn the_call_reads_with_only_what_is_required_and_writes_nothing_absent() {
    let call = sent("Flow", two_boxes());
    assert!(call.session_id.is_none());
    let back = serde_json::to_value(&call).unwrap();
    assert_eq!(back, json!({ "title": "Flow", "scene": two_boxes() }));
}
