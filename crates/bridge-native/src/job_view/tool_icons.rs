//! The kind icons a tool call's row draws (take A, approved 10 Nov): each is painted from its own paths
//! in a 24-unit box, so its parts can move while the call runs. A finished call's icon stands still on
//! its last frame. Web is the lucide globe, turning.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use gpui::{canvas, div, point, prelude::*, px, svg, Animation, AnimationExt, AnyElement, Bounds, Hsla, PathBuilder, PathStyle, Pixels, Point, SharedString, StrokeOptions, Window};
use lyon::tessellation::{LineCap, LineJoin};

use crate::assets::icon_path;

/// What a call does, by the icon and rail colour it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Read,
    Run,
    Edit,
    Search,
    Agent,
    Web,
    /// A tool the row has no kind for: a still wrench.
    Other,
}

/// Seconds on the one clock every running icon reads, so two rows of a kind move together.
fn clock() -> f32 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f32()
}

fn phase(seconds: f32, period: f32) -> f32 {
    (seconds % period) / period
}

/// Where `t` falls from `from` to `to`, held to 0..1.
fn span(t: f32, from: f32, to: f32) -> f32 {
    ((t - from) / (to - from)).clamp(0., 1.)
}

/// The value at `t` between keyframes, eased within each step when `eased`.
fn keyed(t: f32, frames: &[(f32, (f32, f32))], eased: bool) -> (f32, f32) {
    let Some(first) = frames.first() else { return (0., 0.) };
    if t <= first.0 {
        return first.1;
    }
    for pair in frames.windows(2) {
        let ((t0, a), (t1, b)) = (pair[0], pair[1]);
        if t <= t1 {
            let k = span(t, t0, t1);
            let k = if eased { gpui::ease_in_out(k) } else { k };
            return (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
        }
    }
    frames.last().map_or((0., 0.), |last| last.1)
}

/// The wave the pencil writes: `M3 20q1.5 -2 3 0t3 0t3 0t3 0`, as points along it.
fn wave() -> Vec<(f32, f32)> {
    let mut points = vec![(3., 20.)];
    for step in 0..4 {
        let x0 = 3. + 3. * step as f32;
        let crest = if step % 2 == 0 { 18. } else { 22. };
        for at in 1..=8 {
            let t = at as f32 / 8.;
            let x = x0 + 3. * t;
            let y = (1. - t) * (1. - t) * 20. + 2. * (1. - t) * t * crest + t * t * 20.;
            points.push((x, y));
        }
    }
    points
}

/// The first `length` units of a line through `points`.
fn part(points: &[(f32, f32)], length: f32) -> Vec<(f32, f32)> {
    let mut kept = Vec::new();
    let mut left = length;
    for pair in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if kept.is_empty() {
            kept.push((x0, y0));
        }
        let each = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        if left >= each {
            kept.push((x1, y1));
            left -= each;
        } else {
            let k = left / each;
            kept.push((x0 + (x1 - x0) * k, y0 + (y1 - y0) * k));
            break;
        }
    }
    kept
}

/// Corners rounded by `r`, as points round a rectangle.
fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<(f32, f32)> {
    let corners = [(x + w - r, y + r, -0.25), (x + w - r, y + h - r, 0.), (x + r, y + h - r, 0.25), (x + r, y + r, 0.5)];
    let mut points = Vec::new();
    for (cx, cy, start) in corners {
        for at in 0..=6 {
            let turn = (start + 0.25 * at as f32 / 6.) * std::f32::consts::TAU;
            points.push((cx + r * turn.cos(), cy + r * turn.sin()));
        }
    }
    points
}

fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<(f32, f32)> {
    (0..24)
        .map(|at| {
            let turn = at as f32 / 24. * std::f32::consts::TAU;
            (cx + rx * turn.cos(), cy + ry * turn.sin())
        })
        .collect()
}

/// Paints in the 24-unit box at `at`, `unit` pixels to a unit.
struct Pen<'a> {
    at: Point<Pixels>,
    unit: f32,
    ink: Hsla,
    window: &'a mut Window,
}

impl Pen<'_> {
    fn p(&self, (x, y): (f32, f32)) -> Point<Pixels> {
        point(self.at.x + px(x * self.unit), self.at.y + px(y * self.unit))
    }

    fn pen(&self, width: f32) -> PathBuilder {
        let options = StrokeOptions::default().with_line_width(width * self.unit).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
        PathBuilder::stroke(px(width * self.unit)).with_style(PathStyle::Stroke(options))
    }

    fn paint(&mut self, path: PathBuilder, alpha: f32) {
        if let Ok(path) = path.build() {
            self.window.paint_path(path, self.ink.opacity(alpha));
        }
    }

    fn line(&mut self, points: &[(f32, f32)], width: f32, closed: bool, alpha: f32) {
        if points.len() < 2 || alpha <= 0. {
            return;
        }
        let mut path = self.pen(width);
        let points: Vec<Point<Pixels>> = points.iter().map(|&one| self.p(one)).collect();
        path.add_polygon(&points, closed);
        self.paint(path, alpha);
    }

    fn fill(&mut self, points: &[(f32, f32)], alpha: f32) {
        if alpha <= 0. {
            return;
        }
        let mut path = PathBuilder::fill();
        let points: Vec<Point<Pixels>> = points.iter().map(|&one| self.p(one)).collect();
        path.add_polygon(&points, true);
        self.paint(path, alpha);
    }

    /// An arc of radius `r` from `from` to `to`, turning clockwise when `sweep`.
    fn arc(&mut self, from: (f32, f32), r: f32, sweep: bool, to: (f32, f32), width: f32, alpha: f32) {
        if alpha <= 0. {
            return;
        }
        let mut path = self.pen(width);
        path.move_to(self.p(from));
        path.arc_to(point(px(r * self.unit), px(r * self.unit)), px(0.), false, sweep, self.p(to));
        self.paint(path, alpha);
    }
}

const STROKE: f32 = 1.8;
const LINES: f32 = 1.6;

/// One frame of `kind`: `at` is the clock while the call runs, `None` once it is done.
fn paint(kind: Kind, at: Option<f32>, pen: &mut Pen<'_>) {
    match kind {
        Kind::Read => {
            pen.line(&[(6., 3.), (14., 3.), (18., 7.), (18., 21.), (6., 21.)], STROKE, true, 1.);
            pen.line(&[(14., 3.), (14., 7.), (18., 7.)], STROKE, false, 1.);
            let t = at.map(|seconds| phase(seconds, 2.));
            let rows = [((8.5, 10.), (15.5, 10.), 0.08), ((8.5, 13.), (15.5, 13.), 0.24), ((8.5, 16.), (13.5, 16.), 0.40)];
            for (from, to, start) in rows {
                // Each line is a dash of 7 drawn in over 8% of the loop; all three erase over the last 8%.
                let drawn = match t {
                    None => 1.,
                    Some(t) => span(t, start, start + 0.08) * (1. - span(t, 0.92, 1.)),
                };
                pen.line(&part(&[from, to], 7. * drawn), LINES, false, 1.);
            }
            if let Some(t) = t {
                let alpha = 0.6 * (1. - span(t, 0.55, 0.62));
                let y = 8. + 10. * span(t, 0., 0.55);
                pen.fill(&rounded(7., y, 10., 1.4, 0.01), alpha);
            }
        }
        Kind::Run => {
            pen.line(&[(4., 17.), (10., 11.), (4., 5.)], STROKE, false, 1.);
            let on = at.is_none_or(|seconds| phase(seconds, 1.1) < 0.5);
            if on {
                pen.fill(&rounded(12., 16., 8., 3.2, 0.8), 1.);
            }
        }
        Kind::Edit => {
            let t = at.map(|seconds| phase(seconds, 2.4));
            let (drawn, (dx, dy)) = match t {
                None => (1., (12., 0.)),
                Some(t) => {
                    let steps: Vec<(f32, (f32, f32))> = [(0., 0.), (1.5, -1.), (3., 0.), (4.5, 1.), (6., 0.), (7.5, -1.), (9., 0.), (10.5, 1.), (12., 0.)]
                        .iter()
                        .enumerate()
                        .map(|(at, &to)| (at as f32 * 0.1, to))
                        .chain([(0.92, (12., 0.)), (1., (0., 0.))])
                        .collect();
                    (span(t, 0., 0.8) * (1. - span(t, 0.92, 1.)), keyed(t, &steps, false))
                }
            };
            pen.line(&part(&wave(), 16. * drawn), STROKE, false, 1.);
            let moved = |points: &[(f32, f32)]| -> Vec<(f32, f32)> { points.iter().map(|&(x, y)| (x + dx, y + dy)).collect() };
            pen.line(&moved(&[(3., 20.), (3.8, 16.8), (10., 10.6), (12.4, 13.), (6.2, 19.2)]), 1.5, true, 1.);
            pen.line(&moved(&[(8.8, 11.8), (11.2, 14.2)]), 1.5, false, 1.);
        }
        Kind::Search => {
            for (from, to) in [((3., 6.), (21., 6.)), ((3., 12.), (21., 12.)), ((3., 18.), (15., 18.))] {
                pen.line(&[from, to], LINES, false, 0.35);
            }
            let t = at.map(|seconds| phase(seconds, 2.4));
            let shown = t.is_none_or(|t| (0.64..=0.85).contains(&t));
            if shown {
                pen.fill(&rounded(8., 10., 7., 4., 1.), 0.45);
            }
            let (x, y) = match t {
                None => (11.5, 12.),
                Some(t) => keyed(t, &[(0., (4., 6.)), (0.25, (19., 6.)), (0.45, (4., 12.)), (0.6, (11.5, 12.)), (0.85, (11.5, 12.)), (1., (4., 6.))], true),
            };
            pen.line(&ellipse(x, y, 3.6, 3.6), STROKE, true, 1.);
            pen.line(&[(x + 2.6, y + 2.6), (x + 5.4, y + 5.4)], STROKE, false, 1.);
        }
        Kind::Agent => {
            pen.line(&rounded(4., 9., 16., 11., 3.), STROKE, true, 1.);
            pen.line(&[(12., 9.), (12., 5.8)], STROKE, false, 1.);
            pen.fill(&ellipse(12., 4.5, 1.3, 1.3), 1.);
            let (dx, squash) = match at {
                None => (0., 1.),
                Some(seconds) => {
                    let t = phase(seconds, 2.6);
                    let (dx, _) = keyed(t, &[(0., (0., 0.)), (0.2, (0., 0.)), (0.3, (-1.5, 0.)), (0.5, (-1.5, 0.)), (0.6, (1.5, 0.)), (0.8, (1.5, 0.)), (0.9, (0., 0.)), (1., (0., 0.))], true);
                    let (squash, _) = keyed(t, &[(0.9, (1., 0.)), (0.95, (0.15, 0.)), (1., (1., 0.))], false);
                    (dx, squash)
                }
            };
            pen.fill(&ellipse(9. + dx, 14.5, 1.4, 1.4 * squash), 1.);
            pen.fill(&ellipse(15. + dx, 14.5, 1.4, 1.4 * squash), 1.);
            if let Some(seconds) = at {
                let t = phase(seconds, 1.6);
                let alpha = span(t, 0., 0.3) * (1. - span(t, 0.3, 1.));
                pen.arc((8.4, 2.2), 4.5, false, (8.4, 6.8), 1.5, alpha);
                pen.arc((15.6, 2.2), 4.5, true, (15.6, 6.8), 1.5, alpha);
            }
        }
        Kind::Web | Kind::Other => {}
    }
}

/// The icon of `kind` in a `side` box, its glyph 22/24 of the box. `key` names its animation, and is
/// unique to the row.
pub(super) fn icon(kind: Kind, running: bool, key: SharedString, ink: Hsla, side: Pixels) -> AnyElement {
    let glyph = side * (22. / 24.);
    let frame = div().flex_none().flex().items_center().justify_center().size(side);
    match kind {
        Kind::Web => {
            let globe = svg().path(icon_path("globe")).size(glyph).text_color(ink);
            if !running {
                return frame.child(globe).into_any_element();
            }
            frame
                .child(globe.with_animation(key, Animation::new(Duration::from_secs(3)).repeat(), |globe, delta| {
                    globe.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta)))
                }))
                .into_any_element()
        }
        Kind::Other => frame.child(svg().path(icon_path("wrench")).size(glyph).text_color(ink)).into_any_element(),
        _ => {
            let drawn = move |at: Option<f32>| {
                canvas(
                    |_, _, _| {},
                    move |bounds: Bounds<Pixels>, _, window, _| {
                        let unit = f32::from(glyph) / 24.;
                        let inset = (bounds.size.width - glyph) / 2.;
                        let mut pen = Pen { at: point(bounds.origin.x + inset, bounds.origin.y + (bounds.size.height - glyph) / 2.), unit, ink, window };
                        paint(kind, at, &mut pen);
                    },
                )
                .size_full()
            };
            if !running {
                return frame.child(drawn(None)).into_any_element();
            }
            // The animation only asks for frames while the row is drawn; each frame reads the shared clock.
            frame
                .with_animation(key, Animation::new(Duration::from_secs(1)).repeat(), move |frame, _| frame.child(drawn(Some(clock()))))
                .into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_drawn_to_the_length_asked_and_no_further() {
        assert_eq!(part(&[(0., 0.), (10., 0.)], 4.), [(0., 0.), (4., 0.)]);
        assert_eq!(part(&[(0., 0.), (10., 0.)], 20.), [(0., 0.), (10., 0.)]);
        assert_eq!(part(&[(0., 0.), (3., 0.), (3., 4.)], 5.), [(0., 0.), (3., 0.), (3., 2.)]);
    }

    #[test]
    fn keyframes_hold_before_the_first_and_after_the_last() {
        let frames = [(0.2, (1., 1.)), (0.6, (3., 5.))];
        assert_eq!(keyed(0., &frames, false), (1., 1.));
        assert_eq!(keyed(0.4, &frames, false), (2., 3.));
        assert_eq!(keyed(0.9, &frames, false), (3., 5.));
    }

    #[test]
    fn the_wave_ends_where_the_pencil_stops() {
        let wave = wave();
        assert_eq!(wave.first(), Some(&(3., 20.)));
        let end = wave.last().copied().unwrap_or_default();
        assert!((end.0 - 15.).abs() < 1e-4 && (end.1 - 20.).abs() < 1e-4, "{end:?}");
    }
}
