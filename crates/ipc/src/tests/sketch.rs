//! `show_sketch`: a scene Fleet keeps is one `parseScene` draws, and one it refuses is refused with
//! words that name the part and what is allowed. Bridge draws a bad scene as an empty frame without
//! a word, so this is the only place the agent hears of it.

use serde_json::json;

use crate::sketch::screen::{
    fields_of, Read, BASE, LAYOUT, MESSAGE_FIELDS, MOST_SCREEN_DEPTH, ROW_FIELDS, SCREEN_ALIGNS, SCREEN_CHROMES, SCREEN_FIELDS,
    SCREEN_GAPS, SCREEN_ICONS, SCREEN_TEXTS, SCREEN_VARIANTS, SPECS,
};
use crate::sketch::{
    EDGE_FIELDS, NODE_FIELDS, MOST_EDGES, MOST_NODES, MOST_SCENE_BYTES, MOST_TITLE_CHARS, SCENE_COLORS, SCENE_ICONS, SCENE_KINDS,
    SCENE_SHAPES, WIRE_PARTS,
};
use crate::ShowSketch;

const SCENE_TS: &str = include_str!("../../../../packages/components/src/compositions/SketchScene/scene.ts");
const SCREEN_TS: &str = include_str!("../../../../packages/components/src/compositions/SketchScene/screen.ts");
const OPERATION: &str = include_str!("../../operations/show_sketch.toml");

/// The strings of `export const <name> = [ ... ] as const;` in a source file.
fn listed_in(source: &str, name: &str) -> Vec<String> {
    let start = source
        .find(&format!("export const {name} = ["))
        .unwrap_or_else(|| panic!("the source no longer exports {name}"));
    let rest = &source[start..];
    let body = &rest[rest.find('[').unwrap() + 1..rest.find("] as const").unwrap()];
    body.split('"').skip(1).step_by(2).map(String::from).collect()
}

fn listed_in_scene_ts(name: &str) -> Vec<String> {
    listed_in(SCENE_TS, name)
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
    assert_eq!(listed_in_scene_ts("SCENE_SHAPES"), SCENE_SHAPES);
    assert_eq!(listed_in_scene_ts("SCENE_COLORS"), SCENE_COLORS);
}

#[test]
fn a_shape_and_a_colour_are_taken_by_name_and_refused_with_the_allowed_list() {
    let mut scene = two_boxes();
    scene["nodes"][0]["shape"] = json!("diamond");
    scene["nodes"][0]["color"] = json!("amber");
    scene["edges"][0]["color"] = json!("green");
    sent("Flow", scene.clone()).checked().unwrap();

    scene["nodes"][1]["shape"] = json!("star");
    scene["edges"][0]["color"] = json!("purple");
    let said = refused(scene);
    assert!(said.contains("nodes[1] `b`: `shape` is `star`. Allowed: box, pill, circle, diamond, cylinder, hexagon"), "{said}");
    assert!(said.contains("edges[0] `e1`: `color` is `purple`. Allowed: blue, teal, green, amber, orange, red, grey"), "{said}");
}

#[test]
fn the_lists_the_screen_checker_allows_are_the_lists_screen_ts_parses() {
    let lists: [(&str, &[&str]); 7] = [
        ("SCREEN_ICONS", SCREEN_ICONS),
        ("SCREEN_GAPS", &SCREEN_GAPS),
        ("SCREEN_ALIGNS", &SCREEN_ALIGNS),
        ("SCREEN_CHROMES", &SCREEN_CHROMES),
        ("SCREEN_VARIANTS", &SCREEN_VARIANTS),
        ("SCREEN_FIELDS", &SCREEN_FIELDS),
        ("SCREEN_TEXTS", &SCREEN_TEXTS),
    ];
    for (name, ours) in lists {
        assert_eq!(listed_in(SCREEN_TS, name), ours, "{name} differs from screen.ts");
    }
}

/// The text from `open` to its matching `close`, with the index just past it.
fn balanced(text: &str, open: char, close: char) -> (&str, usize) {
    let start = text.find(open).expect("an opening bracket");
    let mut depth = 0;
    for (at, one) in text[start..].char_indices() {
        depth += i32::from(one == open) - i32::from(one == close);
        if depth == 0 {
            return (&text[start + 1..start + at], start + at + 1);
        }
    }
    panic!("a bracket never closes");
}

/// The top-level `key: reader` pieces of an object literal's body, a bare name being its own reader and `...NAME` a spread.
fn pieces(body: &str) -> Vec<(String, String)> {
    let (mut found, mut depth, mut from) = (Vec::new(), 0, 0);
    let mut cut = |end: usize, found: &mut Vec<(String, String)>| {
        let piece = body[from..end].trim();
        from = end + 1;
        if let Some(name) = piece.strip_prefix("...") {
            found.push((String::from("..."), name.to_string()));
        } else if let Some((key, reader)) = piece.split_once(':') {
            found.push((key.trim().to_string(), reader.trim().to_string()));
        } else if !piece.is_empty() {
            found.push((piece.to_string(), piece.to_string()));
        }
    };
    for (at, one) in body.char_indices() {
        match one {
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' => depth -= 1,
            ',' if depth == 0 => cut(at, &mut found),
            _ => {}
        }
    }
    cut(body.len(), &mut found);
    found
}

/// The fields `const <name>: Record<string, Read> = { ... };` holds in `screen.ts`.
fn record_in_screen_ts(name: &str) -> Vec<(String, String)> {
    let start = SCREEN_TS.find(&format!("const {name}: Record<string, Read> = {{")).unwrap_or_else(|| panic!("screen.ts has no {name}"));
    pieces(balanced(&SCREEN_TS[start..], '{', '}').0)
}

/// Each `spec("type", fields, [required])` of `SPECS` in `screen.ts`: the type, its fields with their readers, and what it requires.
fn specs_in_screen_ts() -> Vec<(String, Vec<(String, String)>, Vec<String>)> {
    let start = SCREEN_TS.find("const SPECS = new Map").expect("screen.ts no longer builds SPECS");
    let block = &SCREEN_TS[start..];
    let block = &block[..block.find("\n]);").expect("SPECS closes")];
    let mut found = Vec::new();
    for entry in block.split("\n  spec(\"").skip(1) {
        let (name, rest) = entry.split_once('"').unwrap();
        let rest = rest.trim_start_matches(',').trim_start();
        let (fields, after) = if rest.starts_with('{') {
            let (body, end) = balanced(rest, '{', '}');
            (pieces(body), &rest[end..])
        } else {
            let (spread, after) = rest.split_once(',').unwrap_or((rest.trim_end_matches("),"), ""));
            (vec![(String::from("..."), spread.trim().to_string())], after)
        };
        let mut all = Vec::new();
        for (key, reader) in fields {
            if key == "..." {
                all.extend(record_in_screen_ts(&reader));
            } else {
                all.push((key, reader));
            }
        }
        let required = match after.find('[') {
            Some(_) => balanced(after, '[', ']').0.split('"').skip(1).step_by(2).map(String::from).collect(),
            None => Vec::new(),
        };
        found.push((name.to_string(), all, required));
    }
    found
}

/// A reader as `screen.ts` writes it, spaces aside.
fn reader_of(read: Read) -> String {
    match read {
        Read::Text => "text".into(),
        Read::Flag => "flag".into(),
        Read::Number => "num".into(),
        Read::Whole(least, most) => format!("whole({least},{most})"),
        Read::Size => "size".into(),
        Read::Choice(name, _) => format!("choice({name})"),
        Read::Icon => "icon".into(),
        Read::Act => "act".into(),
        Read::Many(one) => format!("many({})", reader_of(*one)),
        Read::Row => format!("record({{{}}},[\"text\"])", record_of(&ROW_FIELDS)),
        Read::Message => format!("record({{{}}},[\"from\",\"text\"])", record_of(&MESSAGE_FIELDS)),
        Read::Children => "children".into(),
    }
}

fn record_of(fields: &[(&str, Read)]) -> String {
    let said: Vec<String> = fields
        .iter()
        .map(|(name, read)| match read {
            Read::Choice(_, list) => format!("{name}:choice([{}])", list.iter().map(|one| format!("\"{one}\"")).collect::<Vec<_>>().join(",")),
            other if reader_of(*other) == *name => name.to_string(),
            other => format!("{name}:{}", reader_of(*other)),
        })
        .collect();
    said.join(",")
}

fn bare(text: &str) -> String {
    text.chars().filter(|one| !one.is_whitespace()).collect()
}

#[test]
fn the_screen_types_fields_readers_and_required_parts_are_the_ones_screen_ts_reads() {
    let readers = |fields: &[(&str, Read)]| -> Vec<(String, String)> {
        let mut said: Vec<_> = fields.iter().map(|(name, read)| (name.to_string(), reader_of(*read))).collect();
        said.sort();
        said
    };
    let theirs = |fields: Vec<(String, String)>| -> Vec<(String, String)> {
        let mut said: Vec<_> = fields.into_iter().map(|(name, reader)| (name, bare(&reader))).collect();
        said.sort();
        said
    };
    assert_eq!(readers(&BASE), theirs(record_in_screen_ts("BASE")), "BASE differs from screen.ts");
    assert_eq!(readers(&LAYOUT), theirs(record_in_screen_ts("LAYOUT")), "LAYOUT differs from screen.ts");

    let mut types: Vec<_> = specs_in_screen_ts();
    types.sort();
    let mut ours: Vec<_> = SPECS.iter().collect();
    ours.sort_by_key(|spec| spec.name);
    assert_eq!(types.iter().map(|(name, ..)| name.as_str()).collect::<Vec<_>>(), ours.iter().map(|spec| spec.name).collect::<Vec<_>>());
    for ((name, fields, required), spec) in types.into_iter().zip(ours) {
        // `screen.ts` adds BASE and `type` to every spec itself, so they are not in its list.
        let ours_own: Vec<(&str, Read)> = fields_of(spec).into_iter().filter(|(key, _)| !BASE.iter().any(|(base, _)| base == key)).collect();
        assert_eq!(readers(&ours_own), theirs(fields), "the fields of `{name}` differ from screen.ts");
        let mut required = required;
        required.sort();
        let mut mine: Vec<&str> = spec.required.to_vec();
        mine.sort_unstable();
        assert_eq!(required, mine, "what `{name}` requires differs from screen.ts");
    }
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
    for word in SCENE_KINDS.iter().chain(WIRE_PARTS.iter()).chain(SCENE_ICONS.iter()).chain(SCENE_SHAPES.iter()).chain(SCENE_COLORS.iter()) {
        assert!(OPERATION.contains(word), "show_sketch.toml does not name `{word}`");
    }
    assert!(OPERATION.contains("ASCII"), "the description must forbid ASCII art");
}

#[test]
fn the_tool_description_names_every_screen_element_type_and_enum_value() {
    for spec in &SPECS {
        assert!(OPERATION.contains(spec.name), "show_sketch.toml does not name the screen element `{}`", spec.name);
        for (field, read) in fields_of(spec) {
            assert!(OPERATION.contains(field), "show_sketch.toml does not name `{field}` of `{}`", spec.name);
            if let Read::Choice(_, list) = read {
                for word in list {
                    assert!(OPERATION.contains(word), "show_sketch.toml does not name `{word}` of `{field}`");
                }
            }
        }
    }
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

/// The "Team settings" screen of the mock's `screen-wireframes` walk, as an agent sends it.
fn team_settings() -> serde_json::Value {
    // Text and not `json!`, whose expansion is too deep for a tree this size.
    serde_json::from_str(
        r#"{
        "type": "window", "chrome": "browser", "title": "Team settings", "url": "app.tallyboard.example/settings/team",
        "children": [
            { "type": "nav", "brand": "Tallyboard", "items": ["Projects", "Reports", "Team"], "active": "Team",
              "actions": [{ "icon": "search" }, { "icon": "bell" }] },
            { "type": "row", "gap": "none", "size": "fill", "children": [
                { "type": "nav", "vertical": true, "brand": "Settings", "active": "Team",
                  "items": [{ "text": "Team", "icon": "user" }, { "text": "Security", "icon": "lock" }] },
                { "type": "column", "size": "fill", "gap": "md", "pad": "lg", "children": [
                    { "type": "text", "style": "heading", "text": "Team" },
                    { "type": "tabs", "items": ["General", "Members", "Billing"], "active": "Members" },
                    { "type": "row", "gap": "sm", "children": [
                        { "type": "field", "kind": "search", "placeholder": "Find a member", "size": "fill" },
                        { "type": "button", "text": "Invite", "variant": "primary", "icon": "plus" }
                    ] },
                    { "type": "table", "columns": ["Name", "Role"], "rows": [["Ada Okafor", "Owner"], ["Marcus Webb", "Admin"]] },
                    { "type": "panel", "title": "Invites", "actions": [{ "text": "Copy link", "icon": "link" }], "children": [
                        { "type": "grid", "cols": 2, "gap": "md", "children": [
                            { "type": "field", "label": "Role", "kind": "select", "value": "Member" },
                            { "type": "field", "label": "Require approval", "kind": "toggle", "value": "on" }
                        ] },
                        { "type": "divider" },
                        { "type": "list", "rows": [{ "text": "Standup", "icon": "clock", "meta": "9:30", "selected": true }] },
                        { "type": "chat", "messages": [{ "from": "me", "text": "Hi" }, { "from": "them", "text": "Hello" }] },
                        { "type": "row", "gap": "sm", "children": [{ "type": "spacer" }, { "type": "button", "text": "Save" }] }
                    ] }
                ] }
            ] },
            { "type": "modal", "title": "Find", "step": 2, "children": [{ "type": "badge", "text": "New" }, { "type": "avatar", "text": "A" }, { "type": "image" }] }
        ]
    }"#,
    )
    .unwrap()
}

fn screen_node(tree: serde_json::Value) -> serde_json::Value {
    json!({ "nodes": [
        { "id": "a", "kind": "box", "x": 0, "y": 0 },
        { "id": "b", "kind": "box", "x": 0, "y": 200 },
        { "id": "team", "kind": "screen", "x": 600, "y": 0, "w": 900, "title": "Team", "screen": tree },
    ], "edges": [] })
}

/// The tree of `team_settings` with the value at `path` replaced, or set where the field was absent.
fn team_with(path: &str, value: serde_json::Value) -> serde_json::Value {
    let mut tree = team_settings();
    let (parent, key) = path.rsplit_once('/').unwrap();
    let held = tree.pointer_mut(parent).unwrap_or_else(|| panic!("no {parent}"));
    match held {
        serde_json::Value::Object(fields) => {
            fields.insert(key.to_string(), value);
        }
        other => *other.pointer_mut(&format!("/{key}")).unwrap_or_else(|| panic!("no {path}")) = value,
    }
    tree
}

#[test]
fn a_screen_node_with_a_layout_tree_is_kept_whole() {
    let kept = sent("Team", screen_node(team_settings())).checked().unwrap();
    assert_eq!((kept.nodes, kept.edges), (3, 0));
    let back: serde_json::Value = serde_json::from_str(&kept.scene).unwrap();
    assert_eq!(back, screen_node(team_settings()));
}

#[test]
fn a_screen_and_the_screen_it_is_drawn_against_fit_the_cap_with_room() {
    let kept = sent("Team", screen_node(team_settings())).checked().unwrap();
    let against = sent("Before", screen_node(team_settings())).checked().unwrap();
    let both = kept.read_against("Before", &against.scene).unwrap();
    let weight = both.scene.len() + both.against.as_ref().unwrap().len();
    assert!(weight * 4 < MOST_SCENE_BYTES, "{weight} bytes is not a fraction of the cap");
}

#[test]
fn a_screen_node_needs_its_tree_and_only_a_screen_node_takes_one() {
    let mut scene = screen_node(team_settings());
    scene["nodes"][2].as_object_mut().unwrap().remove("screen");
    scene["nodes"][1]["screen"] = team_settings();
    let said = refused(scene);
    assert!(said.contains("nodes[2] `team`: a `screen` needs `screen`, its layout tree"), "{said}");
    assert!(said.contains("nodes[1] `b`: `screen` is for a `screen` node, and this is a `box`"), "{said}");
    assert!(refused(screen_node(json!(null))).contains("nodes[2] `team` screen: must be an object, is null"));
}

/// Where in the tree a JSON pointer lands, as a refusal says it.
fn shown(pointer: &str) -> String {
    let mut said = String::from("nodes[2] `team` screen");
    for part in pointer.split('/').skip(1) {
        said += &if part.parse::<usize>().is_ok() { format!("[{part}]") } else { format!(".{part}") };
    }
    said
}

/// The main column, the panel in it, and the panel's grid, list, chat and last row.
const COLUMN: &str = "/children/1/children/1";
const PANEL: &str = "/children/1/children/1/children/4";

#[test]
fn an_unknown_element_type_names_the_element_and_lists_the_types() {
    let at = format!("{PANEL}/children/1");
    let said = refused(screen_node(team_with(&at, json!({ "type": "carousel" }))));
    assert!(said.contains(&format!("{}.type is `carousel`. Allowed: row, column, grid, window,", shown(&at))), "{said}");
    assert!(refused(screen_node(json!({ "children": [] }))).contains("nodes[2] `team` screen.type is missing. Allowed: row,"));
}

#[test]
fn a_field_an_element_does_not_take_is_refused_with_the_fields_it_does() {
    let at = format!("{COLUMN}/children/0");
    let said = refused(screen_node(team_with(&format!("{at}/titel"), json!("Team"))));
    assert!(said.contains(&format!("{}: has `titel`, which it does not take. Fields: type, id, step, size, align,", shown(&at))), "{said}");
    let said = refused(screen_node(team_with("/children/0/gap", json!("sm"))));
    assert!(said.contains("screen.children[0]: has `gap`, which it does not take"), "{said}");
}

#[test]
fn a_bad_size_align_gap_or_count_names_the_field_and_what_is_allowed() {
    let grid = format!("{PANEL}/children/0");
    for (path, value, said) in [
        ("/children/1/size".to_string(), json!("huge"), "It must be `auto`, `fill` or a number above 0"),
        ("/children/1/size".to_string(), json!(0), "It must be `auto`, `fill` or a number above 0"),
        (format!("{COLUMN}/align"), json!("middle"), "Allowed: start, center, end, stretch"),
        (format!("{COLUMN}/gap"), json!("wide"), "Allowed: none, sm, md, lg"),
        (format!("{COLUMN}/pad"), json!(3), "Allowed: none, sm, md, lg"),
        ("/chrome".to_string(), json!("tablet"), "Allowed: desktop, browser, phone"),
        (format!("{grid}/cols"), json!(13), "It must be a whole number from 1 to 12"),
        (format!("{grid}/cols"), json!(2.5), "It must be a whole number from 1 to 12"),
        ("/children/0/active".to_string(), json!(7), "It must be a string"),
    ] {
        let got = refused(screen_node(team_with(&path, value.clone())));
        let kind = if value.is_string() { format!("`{}`", value.as_str().unwrap()) } else { String::from("a number") };
        assert!(got.contains(&format!("{} is {kind}. {said}", shown(&path))), "{path}: {got}");
    }
}

#[test]
fn an_unknown_icon_names_the_path_to_it_and_lists_the_glyphs() {
    let at = format!("{PANEL}/children/2/rows/0/icon");
    let said = refused(screen_node(team_with(&at, json!("home"))));
    assert!(said.contains(&format!("{} is `home`. Allowed: arrow-left, arrow-right,", shown(&at))), "{said}");
    assert!(said.contains(", layout-dashboard,"), "{said}");
    let at = "/children/0/actions/0/icon";
    let said = refused(screen_node(team_with(at, json!("magnifier"))));
    assert!(said.contains(&format!("{} is `magnifier`. Allowed:", shown(at))), "{said}");
}

#[test]
fn wrong_shapes_name_the_part_and_what_it_must_be() {
    let row = format!("{PANEL}/children/2/rows/0");
    let message = format!("{PANEL}/children/3/messages/0");
    for (path, value, said) in [
        ("/children/0/items".to_string(), json!("Home"), "is `Home`. It must be a list of strings or objects with `text` and `icon`"),
        ("/children/0/items/0".to_string(), json!(3), "is a number. It must be a string or an object with `text` and `icon`"),
        (format!("{COLUMN}/children/3/rows/0"), json!("Ada"), "is `Ada`. It must be a list of strings"),
        (row.clone(), json!("Standup"), "is `Standup`. It must be an object with `text` and, optionally,"),
        (format!("{message}/from"), json!("you"), "is `you`. Allowed: me, them"),
        (format!("{row}/selected"), json!("yes"), "is `yes`. It must be true or false"),
        (format!("{COLUMN}/children"), json!({}), "is an object. It must be a list of elements"),
        ("/children/0/vertical".to_string(), json!(null), "is null. It must be true or false"),
    ] {
        let got = refused(screen_node(team_with(&path, value)));
        assert!(got.contains(&format!("{} {said}", shown(&path))), "wanted `{said}` at {path} in {got}");
    }
    let got = refused(screen_node(team_with(&format!("{row}/text"), json!(null))));
    assert!(got.contains(&format!("{}.text is null. It must be a string", shown(&row))), "{got}");
}

#[test]
fn a_required_field_that_is_missing_is_named_with_what_it_holds() {
    let mut tree = team_settings();
    tree["children"][1].as_object_mut().unwrap().remove("children");
    tree["children"][2].as_object_mut().unwrap().remove("children");
    tree.as_object_mut().unwrap().remove("chrome");
    let said = refused(screen_node(tree));
    assert!(said.contains("nodes[2] `team` screen.children[1]: needs `children`, a list of elements"), "{said}");
    assert!(said.contains("screen.children[2]: needs `children`, a list of elements"), "{said}");
    assert!(said.contains("nodes[2] `team` screen: needs `chrome`, one of desktop, browser, phone"), "{said}");

    let mut tree = team_settings();
    tree["children"][1]["children"][1]["children"][4]["children"][0].as_object_mut().unwrap().remove("cols");
    assert!(refused(screen_node(tree)).contains("needs `cols`, a whole number from 1 to 12"));
}

#[test]
fn a_tree_nests_a_bounded_number_of_levels_and_says_where_it_stopped() {
    let mut tree = json!({ "type": "spacer" });
    for _ in 0..MOST_SCREEN_DEPTH + 40 {
        tree = json!({ "type": "column", "children": [tree] });
    }
    let said = refused(screen_node(tree));
    assert_eq!(said.matches("levels deep").count(), 1, "{said}");
    assert!(said.contains("nests 13 levels deep and a screen holds at most 12"), "{said}");

    let mut tree = json!({ "type": "spacer" });
    for _ in 1..MOST_SCREEN_DEPTH {
        tree = json!({ "type": "column", "children": [tree] });
    }
    sent("Deep", screen_node(tree)).checked().unwrap();
}

#[test]
fn a_screen_with_many_faults_says_a_dozen_of_them_and_counts_the_rest() {
    let rows: Vec<_> = (0..20).map(|_| json!({ "type": "button", "icon": "nope" })).collect();
    let said = refused(screen_node(json!({ "type": "column", "children": rows })));
    assert_eq!(said.matches("`nope`").count(), 12, "{said}");
    assert!(said.contains("; and 8 more"), "{said}");
}
