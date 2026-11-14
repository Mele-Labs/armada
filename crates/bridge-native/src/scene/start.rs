//! Which node a sketch starts at, and where its "Start" tab sits. One rule, shared by the drawing in the
//! app, the page and the picture a sketch exports as, so the mark is the same wherever it is drawn.

use std::collections::HashSet;

use ipc::{Scene, SceneKind, SceneNode, SceneShape};

use super::geometry::Rect;

/// The tab's height: one line, so its word stands on the line box it is drawn in.
pub(super) const TAB_H: f32 = 18.;
/// Room each side of the word inside the tab.
pub(super) const TAB_PAD: f32 = 8.;
/// What the tab rises above the node's top edge: it stands on the edge and shares the frame's border.
pub(super) const TAB_RISE: f32 = TAB_H - super::BORDER;
/// The word on the tab.
pub(super) const TAB_WORD: &str = "Start";

/// A group, a lane and a label are the page the nodes stand on, never a step in the flow.
fn takes_mark(node: &SceneNode) -> bool {
    !matches!(node.kind, SceneKind::Group | SceneKind::Lane | SceneKind::Label)
}

/// The ids of the nodes a reader's eyes start at; empty where the scene does not say.
///
/// 1. A scene with steps starts at the lowest step: the nodes at it, and the node an arrow at it leaves.
/// 2. Else the one node with arrows leaving it and none arriving.
/// 3. Else the one pill with an arrow leaving it.
/// 4. Else nothing: a start is never guessed.
pub(super) fn nodes(scene: &Scene) -> HashSet<&str> {
    let by_id = |id: &str| scene.nodes.iter().find(|node| node.id == id);
    let eligible = |id: &str| by_id(id).is_some_and(takes_mark);

    let lowest = scene
        .nodes
        .iter()
        .filter_map(|node| node.step)
        .chain(scene.edges.iter().filter_map(|edge| edge.step))
        .filter(|step| step.is_finite())
        .min_by(f64::total_cmp);
    if let Some(lowest) = lowest {
        return scene
            .nodes
            .iter()
            .filter(|node| node.step == Some(lowest))
            .map(|node| node.id.as_str())
            .chain(scene.edges.iter().filter(|edge| edge.step == Some(lowest)).map(|edge| edge.from.as_str()))
            .filter(|id| eligible(id))
            .collect();
    }

    let leaves = |node: &SceneNode| scene.edges.iter().any(|edge| edge.from == node.id);
    let arrives = |node: &SceneNode| scene.edges.iter().any(|edge| edge.to == node.id);

    let sources = scene.nodes.iter().filter(|node| takes_mark(node) && leaves(node) && !arrives(node)).collect();
    let pills = scene.nodes.iter().filter(|node| takes_mark(node) && node.kind == SceneKind::Box && node.shape == Some(SceneShape::Pill) && leaves(node)).collect();
    only(sources).or_else(|| only(pills)).into_iter().collect()
}

/// The id of the one node in `found`, and none where there are none or several.
fn only(found: Vec<&SceneNode>) -> Option<&str> {
    match found[..] {
        [one] => Some(one.id.as_str()),
        _ => None,
    }
}

/// Where the tab stands for a node at `rect`, its word `word_w` wide. At the top left of the frame, past its
/// corner; a circle, a diamond and a cylinder have no flat top edge, so it stands over their top.
pub(super) fn tab(node: &SceneNode, rect: Rect, word_w: f32) -> Rect {
    let w = word_w + 2. * TAB_PAD;
    let shape = (node.kind == SceneKind::Box).then_some(node.shape).flatten();
    let x = match shape {
        Some(SceneShape::Circle | SceneShape::Diamond | SceneShape::Cylinder) => rect.x + (rect.w - w) / 2.,
        Some(SceneShape::Pill) => rect.x + rect.h / 2.,
        Some(SceneShape::Hexagon) => rect.x + rect.w * 0.12,
        _ => rect.x + super::RADIUS,
    };
    Rect { x, y: rect.y + super::BORDER - TAB_H, w, h: TAB_H }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(json: &str) -> Scene {
        ipc::decode::<Scene>("Scene", json.as_bytes()).unwrap()
    }

    fn start(json: &str) -> Vec<String> {
        let mut found: Vec<String> = nodes(&scene(json)).into_iter().map(str::to_string).collect();
        found.sort();
        found
    }

    #[test]
    fn the_lowest_step_is_the_start_and_an_arrow_at_it_starts_at_the_node_it_leaves() {
        let nodes = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"step":2},{"id":"b","kind":"box","x":0,"y":0,"step":3},{"id":"c","kind":"box","x":0,"y":0}],"#;
        assert_eq!(start(&format!(r#"{nodes}"edges":[]}}"#)), ["a"]);
        // Both nodes at the lowest step start it.
        let both = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"step":1},{"id":"b","kind":"box","x":0,"y":0,"step":1},{"id":"c","kind":"box","x":0,"y":0,"step":2}],"edges":[]}"#;
        assert_eq!(start(both), ["a", "b"]);
        // An arrow at the lowest step names the node it leaves, whatever the steps of the nodes.
        let arrow = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"step":2},{"id":"b","kind":"box","x":0,"y":0},{"id":"c","kind":"box","x":0,"y":0}],
            "edges":[{"id":"e","from":"c","to":"b","step":1},{"id":"f","from":"b","to":"a","step":2}]}"#;
        assert_eq!(start(arrow), ["c"]);
    }

    #[test]
    fn steps_win_over_a_lone_source() {
        let json = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0},{"id":"b","kind":"box","x":0,"y":0,"step":1}],"edges":[{"id":"e","from":"a","to":"b"}]}"#;
        assert_eq!(start(json), ["b"]);
    }

    #[test]
    fn the_one_node_arrows_leave_and_none_reach_is_the_start() {
        let json = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0},{"id":"b","kind":"box","x":0,"y":0},{"id":"c","kind":"box","x":0,"y":0},{"id":"d","kind":"box","x":0,"y":0}],
            "edges":[{"id":"e1","from":"a","to":"b"},{"id":"e2","from":"b","to":"c"}]}"#;
        // d joins nothing and is not a start; a is the only source.
        assert_eq!(start(json), ["a"]);
    }

    #[test]
    fn two_sources_are_no_start() {
        let json = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0},{"id":"b","kind":"box","x":0,"y":0},{"id":"c","kind":"box","x":0,"y":0}],
            "edges":[{"id":"e1","from":"a","to":"c"},{"id":"e2","from":"b","to":"c"}]}"#;
        assert!(start(json).is_empty());
    }

    #[test]
    fn a_cycle_has_no_source_and_a_lone_pill_with_an_arrow_out_is_the_start() {
        let ring = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0},{"id":"go","kind":"box","shape":"pill","x":0,"y":0},{"id":"c","kind":"box","x":0,"y":0}],
            "edges":[{"id":"e1","from":"a","to":"go"},{"id":"e2","from":"go","to":"c"},{"id":"e3","from":"c","to":"a"}]}"#;
        assert_eq!(start(ring), ["go"]);
        // Two sources and one pill: the pill breaks the tie.
        let tie = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0},{"id":"b","kind":"box","x":0,"y":0},{"id":"end","kind":"box","shape":"pill","x":0,"y":0},{"id":"c","kind":"box","x":0,"y":0}],
            "edges":[{"id":"e1","from":"a","to":"end"},{"id":"e2","from":"b","to":"end"},{"id":"e3","from":"end","to":"c"}]}"#;
        assert_eq!(start(tie), ["end"]);
    }

    #[test]
    fn two_pills_or_a_pill_with_no_arrow_out_are_no_start() {
        let two = r#"{"nodes":[{"id":"a","kind":"box","shape":"pill","x":0,"y":0},{"id":"b","kind":"box","shape":"pill","x":0,"y":0}],
            "edges":[{"id":"e1","from":"a","to":"b"},{"id":"e2","from":"b","to":"a"}]}"#;
        assert!(start(two).is_empty());
        let none_out = r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0},{"id":"b","kind":"box","x":0,"y":0},{"id":"end","kind":"box","shape":"pill","x":0,"y":0}],
            "edges":[{"id":"e1","from":"a","to":"end"},{"id":"e2","from":"b","to":"end"}]}"#;
        assert!(start(none_out).is_empty());
    }

    #[test]
    fn a_group_a_lane_and_a_label_never_take_the_mark() {
        let grounds = r#"{"nodes":[
              {"id":"g","kind":"group","x":0,"y":0,"w":10,"h":10,"step":1},
              {"id":"l","kind":"lane","x":0,"y":0,"w":10,"h":10},
              {"id":"t","kind":"label","x":0,"y":0},
              {"id":"a","kind":"box","x":0,"y":0,"step":2}],
            "edges":[]}"#;
        // The lowest step is a group's, so nothing is marked: the next step is not guessed at.
        assert!(start(grounds).is_empty());
        let sources = r#"{"nodes":[{"id":"g","kind":"group","x":0,"y":0,"w":10,"h":10},{"id":"t","kind":"label","x":0,"y":0},{"id":"a","kind":"box","x":0,"y":0},{"id":"b","kind":"box","x":0,"y":0}],
            "edges":[{"id":"e1","from":"g","to":"a"},{"id":"e2","from":"t","to":"b"},{"id":"e3","from":"a","to":"b"}]}"#;
        // g and t have arrows leaving them and none arriving, but are not eligible; a is reached, b is reached.
        assert!(start(sources).is_empty());
        let one = r#"{"nodes":[{"id":"g","kind":"group","x":0,"y":0,"w":10,"h":10},{"id":"a","kind":"box","x":0,"y":0},{"id":"b","kind":"box","x":0,"y":0}],
            "edges":[{"id":"e1","from":"g","to":"b"},{"id":"e2","from":"a","to":"b"}]}"#;
        assert_eq!(start(one), ["a"]);
    }

    #[test]
    fn a_scene_with_nothing_joined_has_no_start() {
        assert!(start(r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0}],"edges":[]}"#).is_empty());
        assert!(start(r#"{"nodes":[],"edges":[]}"#).is_empty());
    }

    #[test]
    fn the_tab_stands_on_the_top_edge_at_the_left_or_over_a_round_top() {
        let node = |shape: &str| scene(&format!(r#"{{"nodes":[{{"id":"a","kind":"box","shape":"{shape}","x":0,"y":0}}],"edges":[]}}"#)).nodes.remove(0);
        let rect = Rect { x: 100., y: 50., w: 200., h: 60. };
        let plain = tab(&node("box"), rect, 30.);
        assert_eq!((plain.x, plain.w, plain.h), (106., 46., TAB_H));
        // Its foot shares the frame's border, so the two read as one.
        assert_eq!(plain.y + plain.h, 50. + crate::scene::BORDER);
        assert_eq!(tab(&node("pill"), rect, 30.).x, 130.);
        assert_eq!(tab(&node("circle"), rect, 30.).x, 100. + (200. - 46.) / 2.);
    }
}
