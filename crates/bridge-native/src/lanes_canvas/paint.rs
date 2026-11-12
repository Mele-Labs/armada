//! What the pane paints under the nodes: the lanes' lines, the edges and the watches' hexagons.

use std::collections::HashMap;

use gpui::{point, px, Bounds, Hsla, PathBuilder, Pixels, Window};
use ipc::{JobLanesView, LaneAutonomy, LaneKind, LaneNodeKind};

use super::layout::bezier;
use super::wears::{rule_hue, wears};
use super::{Pick, Placement, Rect, View};
use crate::look::{token, Look};

/// A line along a bezier, dashed as `dash` asks (on, off), with a filled arrowhead `head` long at its end.
pub(super) fn paint_curve(window: &mut Window, at: impl Fn((f32, f32)) -> gpui::Point<Pixels>, curve: [(f32, f32); 4], color: Hsla, width: Pixels, dash: Option<(f32, f32)>, head: f32) {
    let point_at = |t: f32| {
        let u = 1. - t;
        let [a, b, c, d] = curve;
        (u * u * u * a.0 + 3. * u * u * t * b.0 + 3. * u * t * t * c.0 + t * t * t * d.0, u * u * u * a.1 + 3. * u * u * t * b.1 + 3. * u * t * t * c.1 + t * t * t * d.1)
    };
    let points: Vec<(f32, f32)> = (0..=48).map(|step| point_at(step as f32 / 48.)).collect();
    let mut line = PathBuilder::stroke(width);
    match dash {
        None => {
            line.move_to(at(curve[0]));
            line.cubic_bezier_to(at(curve[3]), at(curve[1]), at(curve[2]));
        }
        Some((on, off)) => {
            let period = on + off;
            let mut walked = 0.;
            for pair in points.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let long = (b.0 - a.0).hypot(b.1 - a.1);
                if long <= 0. {
                    continue;
                }
                let along = |s: f32| (a.0 + (b.0 - a.0) * s / long, a.1 + (b.1 - a.1) * s / long);
                let mut s = 0.;
                while s < long {
                    let into = (walked + s) % period;
                    let (lit, left) = if into < on { (true, on - into) } else { (false, period - into) };
                    let end = (s + left.max(0.01)).min(long);
                    if lit {
                        line.move_to(at(along(s)));
                        line.line_to(at(along(end)));
                    }
                    s = end;
                }
                walked += long;
            }
        }
    }
    if let Ok(path) = line.build() {
        window.paint_path(path, color);
    }
    let tip = curve[3];
    let before = points[points.len() - 3];
    let (dx, dy) = (tip.0 - before.0, tip.1 - before.1);
    let long = dx.hypot(dy).max(0.001);
    let (ux, uy) = (dx / long, dy / long);
    let half = head / 2.;
    let mut arrow = PathBuilder::fill();
    arrow.add_polygon(&[at(tip), at((tip.0 - ux * head - uy * half, tip.1 - uy * head + ux * half)), at((tip.0 - ux * head + uy * half, tip.1 - uy * head - ux * half))], true);
    if let Ok(path) = arrow.build() {
        window.paint_path(path, color);
    }
}

/// A closed polygon, filled and outlined, its outline dashed where asked.
pub(super) fn paint_polygon(window: &mut Window, points: &[gpui::Point<Pixels>], fill: Hsla, stroke: Hsla, width: Pixels, dash: Option<f32>) {
    let mut body = PathBuilder::fill();
    body.add_polygon(points, true);
    if let Ok(path) = body.build() {
        window.paint_path(path, fill);
    }
    let mut line = PathBuilder::stroke(width);
    let ring: Vec<gpui::Point<Pixels>> = points.iter().copied().chain(points.first().copied()).collect();
    match dash {
        None => {
            line.move_to(ring[0]);
            for one in &ring[1..] {
                line.line_to(*one);
            }
        }
        Some(on) => {
            for pair in ring.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let long = f32::from(b.x - a.x).hypot(f32::from(b.y - a.y));
                let mut s = 0.;
                while s < long {
                    let end = (s + on).min(long);
                    line.move_to(a + (b - a) * (s / long));
                    line.line_to(a + (b - a) * (end / long));
                    s += on * 2.;
                }
            }
        }
    }
    if let Ok(path) = line.build() {
        window.paint_path(path, stroke);
    }
}

/// What the pane paints under the nodes: each lane's line, the ways a state is left, the rules' edges and
/// the hexagons a watch is drawn in.
pub(super) struct PaintPlan {
    bands: Vec<(f32, f32, Hsla)>,
    curves: Vec<([(f32, f32); 4], Hsla, f32, Option<(f32, f32)>, f32)>,
    hexagons: Vec<(Rect, Hsla, Hsla, bool)>,
    hairline: f32,
    space_1: f32,
}

impl PaintPlan {
    pub(super) fn of(lanes: &JobLanesView, placement: &Placement, picked: Option<&Pick>, look: &Look) -> Self {
        let hairline = f32::from(look.length(token::BORDER_WIDTH));
        let space_1 = f32::from(look.length(token::SPACE_1));
        let space_2 = f32::from(look.length(token::SPACE_2));
        let bands = lanes.lanes.iter().zip(&placement.bands).map(|(lane, band)| (band.y + band.h, band.w, look.color(if lane.kind == LaneKind::Workflow { token::BORDER_STRONG } else { token::BORDER_SUBTLE }))).collect();
        let kinds: HashMap<&str, LaneNodeKind> = lanes.lanes.iter().flat_map(|lane| lane.nodes.iter()).map(|node| (node.id.as_str(), node.kind)).collect();
        let mut curves = Vec::new();
        // A way a state is left within its lane: thin and solid, dashed where it leads to a wait (`MoveEdge`).
        for (from, to) in lanes.lanes.iter().flat_map(|lane| lane.moves.iter()) {
            let (Some(a), Some(b)) = (placement.items.get(from), placement.items.get(to)) else { continue };
            let wait = kinds.get(to.as_str()) == Some(&LaneNodeKind::Watch);
            curves.push((bezier(a, b), look.color(token::BORDER_STRONG), hairline, wait.then_some((space_1, space_1)), 6. * hairline));
        }
        for edge in &lanes.rules {
            let (Some(a), Some(b)) = (placement.items.get(&edge.from), placement.items.get(&edge.to)) else { continue };
            let is_picked = picked == Some(&Pick::Rule(edge.rule.clone()));
            let (hue, dashed) = rule_hue(look, edge, is_picked);
            let width = hairline * if is_picked { 2.5 } else { 1.5 };
            let dash = if edge.autonomy == LaneAutonomy::Off { Some((hairline, space_1)) } else { dashed.then_some((space_2, space_1)) };
            curves.push((bezier(a, b), hue, width, dash, 7. * hairline * 1.5));
        }
        let hexagons = lanes
            .lanes
            .iter()
            .flat_map(|lane| lane.nodes.iter())
            .filter(|node| node.kind == LaneNodeKind::Watch)
            .filter_map(|node| {
                let rect = *placement.items.get(&node.id)?;
                let w = wears(look, node, picked == Some(&Pick::Node(node.id.clone())));
                Some((rect, w.ground, w.frame, w.dashed))
            })
            .collect();
        Self { bands, curves, hexagons, hairline, space_1 }
    }

    pub(super) fn paint(&self, origin: gpui::Point<Pixels>, view: View, window: &mut Window) {
        let at = |(x, y): (f32, f32)| origin + point(px(view.x + x * view.zoom), px(view.y + y * view.zoom));
        for &(bottom, wide, color) in &self.bands {
            let from = at((0., bottom)) - point(px(0.), px(self.hairline * view.zoom));
            window.paint_quad(gpui::fill(Bounds { origin: from, size: gpui::size(px(wide * view.zoom), px(self.hairline * view.zoom)) }, color));
        }
        for &(curve, color, width, dash, head) in &self.curves {
            paint_curve(window, at, curve, color, px(width * view.zoom), dash.map(|(on, off)| (on * view.zoom, off * view.zoom)), head * view.zoom);
        }
        for &(rect, ground, frame, dashed) in &self.hexagons {
            let p = |fx: f32, fy: f32| at((rect.x + rect.w * fx, rect.y + rect.h * fy));
            let points = [p(0.08, 0.), p(0.92, 0.), p(1., 0.5), p(0.92, 1.), p(0.08, 1.), p(0., 0.5)];
            paint_polygon(window, &points, ground, frame, px(self.hairline), dashed.then_some(self.space_1 * view.zoom));
        }
    }
}
