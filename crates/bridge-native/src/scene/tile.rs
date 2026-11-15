//! Take B of the sketch look (owner, 14 Nov): every box and block of code is a tile with a head band, a title and a line
//! of body, and the words that say what a node is. One set of numbers and rules, shared by the drawing in the app, the page
//! and the picture a sketch exports as, so a tile is the same wherever it is drawn.

use ipc::{Scene, SceneColor, SceneEdge, SceneKind, SceneNode, SceneShape};

/// The tile's corner, and its frame: one hairline.
pub(super) const RADIUS: f32 = 12.;
pub(super) const HAIRLINE: f32 = 1.;
/// A frame that says something (the start, a change): a little heavier than the hairline.
pub(super) const FRAME: f32 = 1.5;
/// The canvas a drawing stands on: its corner, and the air kept round what it holds.
pub(super) const CANVAS_RADIUS: f32 = 14.;
pub(super) const CANVAS_PAD: f32 = 24.;

/// The head band: its height, the room each side of its words, and the registry glyph in it.
pub(super) const BAND_H: f32 = 28.;
pub(super) const BAND_PAD: f32 = 12.;
pub(super) const BAND_ICON: f32 = 13.;
pub(super) const BAND_GAP: f32 = 6.;
/// The hue's share of the band, over the tile's surface.
pub(super) const BAND_TINT: f32 = 0.16;
/// The kind word and a ground's label: 10px, tracked at `.09em`.
pub(super) const KIND_AT: f32 = 10.;
pub(super) const KIND_WEIGHT: f32 = 700.;
pub(super) const TRACKING: f32 = 0.09;

/// The tile's words: the title, then the body as one mono line under it.
pub(super) const TITLE_AT: f32 = 14.;
pub(super) const TITLE_LEAD: f32 = 20.;
pub(super) const TITLE_WEIGHT: f32 = 650.;
pub(super) const BODY_AT: f32 = 11.;
pub(super) const BODY_LEAD: f32 = 16.;
pub(super) const BODY_PAD_X: f32 = 14.;
pub(super) const BODY_PAD_TOP: f32 = 10.;
pub(super) const BODY_PAD_BOTTOM: f32 = 12.;
pub(super) const BODY_GAP: f32 = 4.;

/// The step's badge: a circle on the tile's top right corner, standing out past it.
pub(super) const BADGE: f32 = 20.;
pub(super) const BADGE_OUT: f32 = 6.;
pub(super) const BADGE_AT: f32 = 11.;
pub(super) const BADGE_WEIGHT: f32 = 800.;

/// The Start pill: its height, the size of its word, and the room each side of the word.
pub(super) const PILL_H: f32 = 16.;
pub(super) const PILL_AT: f32 = 9.;
pub(super) const PILL_PAD: f32 = 6.;
/// What the pill keeps clear of the badge, which stands in from the corner.
pub(super) const PILL_CLEAR: f32 = 8.;
/// The gap a pill keeps above a node that has no head band.
pub(super) const PILL_RISE: f32 = 4.;

/// An arrow: its width, its head, and its words.
pub(super) const ARROW: f32 = 3.;
pub(super) const LABEL_AT: f32 = 11.;
pub(super) const LABEL_WEIGHT: f32 = 600.;
pub(super) const LABEL_LEAD: f32 = 15.;

/// Whether a node is drawn as a tile: a box or a block of code.
pub(super) fn is_tile(node: &SceneNode) -> bool {
    matches!(node.kind, SceneKind::Box | SceneKind::Code)
}

/// What a node is, in the one word its head band carries. `leaves` is whether an arrow goes out of it: a pill with none is
/// where the flow ends. `started` is whether the scene starts at the node, which wears the Start pill and so is never also
/// called START or END.
pub(super) fn kind_word(node: &SceneNode, leaves: bool, started: bool) -> &'static str {
    if node.kind == SceneKind::Code {
        return "CODE";
    }
    match node.shape.unwrap_or(SceneShape::Box) {
        SceneShape::Diamond => "DECIDE",
        SceneShape::Cylinder => "STORE",
        SceneShape::Hexagon => "PROCESS",
        SceneShape::Pill if started || leaves => "STEP",
        SceneShape::Pill => "END",
        SceneShape::Circle | SceneShape::Box => "STEP",
    }
}

/// Whether an arrow leaves the node.
pub(super) fn leaves(scene: &Scene, node: &SceneNode) -> bool {
    scene.edges.iter().any(|edge| edge.from == node.id)
}

/// The badge's number: `3`, and `1.5` where a step sits between two.
pub(super) fn step_text(step: f64) -> String {
    if step.fract() == 0. && step.abs() < 1e9 {
        format!("{}", step as i64)
    } else {
        format!("{step}")
    }
}

/// The hue an arrow is drawn in: the one it says, else the hue of the node it leaves, else grey.
pub(super) fn arrow_color(scene: &Scene, edge: &SceneEdge) -> SceneColor {
    edge.color.or_else(|| scene.nodes.iter().find(|node| node.id == edge.from).and_then(|node| node.color)).unwrap_or(SceneColor::Grey)
}

/// The step badge of a node at `rect`: a circle standing on its top right corner, and its number. A node with no step has
/// none, and neither has a ground or a label, which are no step in the flow.
pub(super) fn badge(node: &SceneNode, rect: super::geometry::Rect) -> Option<(super::geometry::Rect, String)> {
    let step = node.step.filter(|step| step.is_finite())?;
    if matches!(node.kind, SceneKind::Group | SceneKind::Lane | SceneKind::Label) {
        return None;
    }
    Some((super::geometry::Rect { x: rect.x + rect.w - BADGE + BADGE_OUT, y: rect.y - BADGE_OUT, w: BADGE, h: BADGE }, step_text(step)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(json: &str) -> Scene {
        ipc::decode::<Scene>("Scene", json.as_bytes()).unwrap()
    }

    fn word(shape: &str, edges: &str) -> &'static str {
        let scene = scene(&format!(r#"{{"nodes":[{{"id":"a","kind":"box","shape":"{shape}","x":0,"y":0}},{{"id":"b","kind":"box","x":0,"y":0}}],"edges":[{edges}]}}"#));
        kind_word(&scene.nodes[0], leaves(&scene, &scene.nodes[0]), false)
    }

    #[test]
    fn each_shape_has_its_kind_word() {
        assert_eq!(word("diamond", ""), "DECIDE");
        assert_eq!(word("cylinder", ""), "STORE");
        assert_eq!(word("hexagon", ""), "PROCESS");
        assert_eq!(word("circle", ""), "STEP");
        assert_eq!(word("box", ""), "STEP");
        let plain = scene(r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0},{"id":"c","kind":"code","x":0,"y":0}],"edges":[]}"#);
        assert_eq!(kind_word(&plain.nodes[0], false, false), "STEP");
        assert_eq!(kind_word(&plain.nodes[1], false, false), "CODE");
    }

    #[test]
    fn a_pill_is_a_step_where_an_arrow_leaves_it_and_the_end_where_none_does() {
        // No pill is called START: the one the scene starts at wears the Start pill, and a second START would read as a second start.
        assert_eq!(word("pill", r#"{"id":"e","from":"a","to":"b"}"#), "STEP");
        assert_eq!(word("pill", r#"{"id":"e","from":"b","to":"a"}"#), "END");
        assert_eq!(word("pill", ""), "END");
        // The start itself is a step even where nothing leaves it.
        let scene = scene(r#"{"nodes":[{"id":"a","kind":"box","shape":"pill","x":0,"y":0}],"edges":[]}"#);
        assert_eq!(kind_word(&scene.nodes[0], false, true), "STEP");
    }

    #[test]
    fn an_arrow_takes_the_hue_of_the_node_it_leaves() {
        let scene = scene(
            r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"color":"teal"},{"id":"b","kind":"box","x":0,"y":0,"color":"red"},{"id":"c","kind":"box","x":0,"y":0}],
                "edges":[{"id":"e1","from":"a","to":"b"},{"id":"e2","from":"b","to":"c"},{"id":"e3","from":"c","to":"a"},{"id":"e4","from":"a","to":"c","color":"amber"}]}"#,
        );
        let hue = |at: usize| arrow_color(&scene, &scene.edges[at]);
        assert_eq!(hue(0), SceneColor::Teal);
        assert_eq!(hue(1), SceneColor::Red);
        // A node with no colour is grey, and so is the arrow it sends.
        assert_eq!(hue(2), SceneColor::Grey);
        // An arrow that names its own hue keeps it.
        assert_eq!(hue(3), SceneColor::Amber);
    }

    #[test]
    fn a_step_is_a_whole_number_where_it_is_one() {
        assert_eq!(step_text(3.), "3");
        assert_eq!(step_text(1.5), "1.5");
    }

    #[test]
    fn the_badge_stands_on_the_top_right_corner_and_only_where_a_step_is_set() {
        use super::super::geometry::Rect;
        let scene = scene(r#"{"nodes":[{"id":"a","kind":"box","x":0,"y":0,"step":3},{"id":"b","kind":"box","x":0,"y":0},{"id":"g","kind":"group","x":0,"y":0,"step":1}],"edges":[]}"#);
        let rect = Rect { x: 100., y: 50., w: 200., h: 80. };
        let (at, number) = badge(&scene.nodes[0], rect).unwrap();
        assert_eq!(number, "3");
        // It overlaps the corner: some of it inside the tile, some past its top and right edges.
        assert!(at.x < 300. && at.x + at.w > 300. && at.y < 50. && at.y + at.h > 50.);
        assert_eq!((at.w, at.h), (BADGE, BADGE));
        assert!(badge(&scene.nodes[1], rect).is_none());
        assert!(badge(&scene.nodes[2], rect).is_none());
    }
}
