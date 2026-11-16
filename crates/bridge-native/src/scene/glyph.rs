//! The small outline at a node's left that says what it is: a pill for a start or an end, a rounded box for a step, a diamond
//! for a decision, a cylinder for a store and a hexagon for a process. One drawing, in a square of `SIZE` units, that the
//! app strokes and the page and the picture write as a path, so a kind looks the same wherever it is drawn.

use ipc::{SceneKind, SceneNode, SceneShape};

/// The square a glyph is drawn in, in scene units, and the width of its line.
pub(super) const SIZE: f32 = 14.;
pub(super) const STROKE: f32 = 1.4;

/// What a node is, as far as its outline says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mark {
    Pill,
    Step,
    Decision,
    Store,
    Process,
}

/// One command of an outline, in the glyph's own square.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Cmd {
    Move(f32, f32),
    Line(f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// The mark of a node: its shape's, and a pill for the node the sketch starts at unless its shape says another. A block of
/// code has none: its language, at its top right, says what it is.
pub(super) fn mark_of(node: &SceneNode, started: bool) -> Option<Mark> {
    if node.kind != SceneKind::Box {
        return None;
    }
    Some(match node.shape.unwrap_or(SceneShape::Box) {
        SceneShape::Diamond => Mark::Decision,
        SceneShape::Cylinder => Mark::Store,
        SceneShape::Hexagon => Mark::Process,
        SceneShape::Pill => Mark::Pill,
        SceneShape::Circle | SceneShape::Box if started => Mark::Pill,
        SceneShape::Circle | SceneShape::Box => Mark::Step,
    })
}

/// A quarter of a circle's cubic: the control's share of the radius.
const K: f32 = 0.552_284_75;

fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<Cmd> {
    let (right, foot, c) = (x + w, y + h, r * K);
    vec![
        Cmd::Move(x + r, y),
        Cmd::Line(right - r, y),
        Cmd::Cubic(right - r + c, y, right, y + r - c, right, y + r),
        Cmd::Line(right, foot - r),
        Cmd::Cubic(right, foot - r + c, right - r + c, foot, right - r, foot),
        Cmd::Line(x + r, foot),
        Cmd::Cubic(x + r - c, foot, x, foot - r + c, x, foot - r),
        Cmd::Line(x, y + r),
        Cmd::Cubic(x, y + r - c, x + r - c, y, x + r, y),
        Cmd::Close,
    ]
}

fn polygon(points: &[(f32, f32)]) -> Vec<Cmd> {
    let mut out = vec![Cmd::Move(points[0].0, points[0].1)];
    out.extend(points[1..].iter().map(|&(x, y)| Cmd::Line(x, y)));
    out.push(Cmd::Close);
    out
}

/// A cylinder: the ellipse of its top, and its sides closed by the lower half of the same ellipse.
fn cylinder() -> Vec<Cmd> {
    let (left, right, mid, top, foot) = (1.5, 12.5, 7., 3.5, 10.5);
    let (rx, ry) = ((right - left) / 2., 2.);
    let (cx, cy) = (rx * K, ry * K);
    vec![
        Cmd::Move(left, top),
        Cmd::Cubic(left, top - cy, mid - cx, top - ry, mid, top - ry),
        Cmd::Cubic(mid + cx, top - ry, right, top - cy, right, top),
        Cmd::Cubic(right, top + cy, mid + cx, top + ry, mid, top + ry),
        Cmd::Cubic(mid - cx, top + ry, left, top + cy, left, top),
        Cmd::Line(left, foot),
        Cmd::Cubic(left, foot + cy, mid - cx, foot + ry, mid, foot + ry),
        Cmd::Cubic(mid + cx, foot + ry, right, foot + cy, right, foot),
        Cmd::Line(right, top),
    ]
}

/// The outline of a mark, in a square of `SIZE` units.
pub(super) fn outline(mark: Mark) -> Vec<Cmd> {
    match mark {
        Mark::Pill => rounded(1., 3.5, 12., 7., 3.5),
        Mark::Step => rounded(1.5, 2.5, 11., 9., 2.5),
        Mark::Decision => polygon(&[(7., 1.), (13., 7.), (7., 13.), (1., 7.)]),
        Mark::Store => cylinder(),
        Mark::Process => polygon(&[(3.5, 2.), (10.5, 2.), (13., 7.), (10.5, 12.), (3.5, 12.), (1., 7.)]),
    }
}

/// The outline as the text of an SVG path.
pub(super) fn path_text(mark: Mark) -> String {
    let mut out = String::new();
    for cmd in outline(mark) {
        match cmd {
            Cmd::Move(x, y) => out.push_str(&format!("M{x} {y}")),
            Cmd::Line(x, y) => out.push_str(&format!("L{x} {y}")),
            Cmd::Cubic(a, b, c, d, e, f) => out.push_str(&format!("C{a} {b} {c} {d} {e} {f}")),
            Cmd::Close => out.push('Z'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(kind: &str, shape: Option<&str>) -> SceneNode {
        let shape = shape.map(|shape| format!(r#","shape":"{shape}""#)).unwrap_or_default();
        ipc::decode("a node", format!(r#"{{"id":"a","kind":"{kind}","x":0,"y":0{shape}}}"#).as_bytes()).unwrap()
    }

    #[test]
    fn each_shape_has_its_own_mark() {
        assert_eq!(mark_of(&node("box", Some("diamond")), false), Some(Mark::Decision));
        assert_eq!(mark_of(&node("box", Some("cylinder")), false), Some(Mark::Store));
        assert_eq!(mark_of(&node("box", Some("hexagon")), false), Some(Mark::Process));
        assert_eq!(mark_of(&node("box", Some("pill")), false), Some(Mark::Pill));
        assert_eq!(mark_of(&node("box", Some("circle")), false), Some(Mark::Step));
        assert_eq!(mark_of(&node("box", None), false), Some(Mark::Step));
    }

    #[test]
    fn the_start_is_a_pill_unless_its_shape_says_another_and_code_has_none() {
        assert_eq!(mark_of(&node("box", None), true), Some(Mark::Pill));
        assert_eq!(mark_of(&node("box", Some("diamond")), true), Some(Mark::Decision));
        assert_eq!(mark_of(&node("code", None), false), None);
        assert_eq!(mark_of(&node("mock", None), false), None);
    }

    #[test]
    fn each_mark_draws_its_own_outline_inside_its_square() {
        let marks = [Mark::Pill, Mark::Step, Mark::Decision, Mark::Store, Mark::Process];
        for (at, one) in marks.iter().enumerate() {
            let drawn = outline(*one);
            assert!(drawn.len() >= 4, "{one:?} draws something");
            for other in &marks[at + 1..] {
                assert_ne!(drawn, outline(*other), "{one:?} and {other:?} are not drawn alike");
            }
            let at_most = |value: f32| (0. ..=SIZE).contains(&value);
            for cmd in drawn {
                let values: Vec<f32> = match cmd {
                    Cmd::Move(x, y) | Cmd::Line(x, y) => vec![x, y],
                    Cmd::Cubic(a, b, c, d, e, f) => vec![a, b, c, d, e, f],
                    Cmd::Close => vec![],
                };
                assert!(values.into_iter().all(at_most), "{one:?} stays in its square");
            }
        }
    }
}
