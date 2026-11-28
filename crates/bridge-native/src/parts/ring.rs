//! The busy ring: while something loads, its border carries a bright arc with a fading tail that
//! circles the rounded edge, in place of a spinner glyph. One colour, the tail fading to clear over
//! [`TAIL_DEGREES`], the head the brightest and nearest white. A turn takes the spin duration.
//!
//! Place [`ring_overlay`] inside a relatively positioned element; it paints nothing else and takes
//! no press. The geometry is pure ([`arc_segments`], [`perimeter_point`]) so it is tested without a window.

use gpui::{
    canvas, div, point, prelude::*, px, Animation, AnimationExt as _, Bounds, ElementId, Hsla, PathBuilder, Pixels,
};

use crate::look::{token, Look};

/// How far behind the head the tail runs, as a share of one turn read in degrees.
pub const TAIL_DEGREES: f32 = 140.;
/// How many short strokes lay the tail, each a step fainter than the one ahead of it.
pub const SEGMENTS: usize = 24;
/// Points along one stroke, so a corner bends instead of cutting across.
const STEPS: usize = 3;

/// One short stroke of the tail. `start` is where it begins, in degrees clockwise from twelve
/// o'clock, wrapped to 0..360; it runs `sweep` degrees on from there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub start: f32,
    pub sweep: f32,
    pub alpha: f32,
}

impl Segment {
    /// Where the stroke ends, not wrapped.
    pub fn end(&self) -> f32 {
        self.start + self.sweep
    }
}

/// The tail at `phase` (0..1, one turn): `count` strokes from the head backwards, the first the brightest.
pub fn arc_segments(phase: f32, tail: f32, count: usize) -> Vec<Segment> {
    let head = phase.rem_euclid(1.) * 360.;
    let sweep = tail / count as f32;
    (0..count)
        .map(|at| Segment {
            start: (head - (at as f32 + 1.) * sweep).rem_euclid(360.),
            sweep,
            alpha: 1. - (at as f32 + 0.5) / count as f32,
        })
        .collect()
}

/// The point `turn` (0..1, from the top centre, clockwise) of the way round a `width` by `height`
/// rounded rectangle with corner `radius`, from its top-left corner.
pub fn perimeter_point(width: f32, height: f32, radius: f32, turn: f32) -> (f32, f32) {
    let r = radius.clamp(0., width.min(height) / 2.);
    let half = width / 2. - r;
    let side = height - 2. * r;
    let flat = width - 2. * r;
    let quarter = std::f32::consts::FRAC_PI_2 * r;
    let total = 2. * half + flat + 2. * side + 4. * quarter;
    let mut left = turn.rem_euclid(1.) * total;
    let arc = |centre: (f32, f32), from: f32, along: f32| {
        let angle = from + if r > 0. { along / r } else { 0. };
        (centre.0 + r * angle.cos(), centre.1 + r * angle.sin())
    };
    let pi = std::f32::consts::PI;
    let lengths = [half, quarter, side, quarter, flat, quarter, side, quarter, half];
    for (piece, length) in lengths.into_iter().enumerate() {
        if left <= length {
            return match piece {
                0 => (width / 2. + left, 0.),
                1 => arc((width - r, r), -pi / 2., left),
                2 => (width, r + left),
                3 => arc((width - r, height - r), 0., left),
                4 => (width - r - left, height),
                5 => arc((r, height - r), pi / 2., left),
                6 => (0., height - r - left),
                7 => arc((r, r), pi, left),
                _ => (r + left, 0.),
            };
        }
        left -= length;
    }
    (width / 2., 0.)
}

/// A stroke's colour: the ink faded by `alpha`, the head leaning toward white.
fn shade(ink: Hsla, alpha: f32) -> Hsla {
    let lean = alpha.powi(6) * 0.7;
    Hsla { l: ink.l + (1. - ink.l) * lean, s: ink.s * (1. - lean * 0.6), a: ink.a * alpha, ..ink }
}

fn sweep(phase: f32, radius: Pixels, width: Pixels, reach: Pixels, ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds: Bounds<Pixels>, _, window, _| {
            let origin = point(bounds.origin.x - reach, bounds.origin.y - reach);
            let (w, h) = (f32::from(bounds.size.width + reach * 2.), f32::from(bounds.size.height + reach * 2.));
            let r = f32::from(radius + reach);
            for one in arc_segments(phase, TAIL_DEGREES, SEGMENTS) {
                let mut stroke = PathBuilder::stroke(width);
                for step in 0..=STEPS {
                    let turn = (one.start + one.sweep * step as f32 / STEPS as f32) / 360.;
                    let (x, y) = perimeter_point(w, h, r, turn);
                    let at = point(origin.x + px(x), origin.y + px(y));
                    if step == 0 {
                        stroke.move_to(at);
                    } else {
                        stroke.line_to(at);
                    }
                }
                if let Ok(path) = stroke.build() {
                    window.paint_path(path, shade(ink, one.alpha));
                }
            }
        },
    )
    .size_full()
}

/// The ring over an element's border: absolutely placed, its own size, turning once per three spins.
/// `radius` is the element's corner radius; `ink` is the arc's colour.
pub fn ring_overlay(look: &Look, id: impl Into<ElementId>, radius: Pixels, ink: Hsla) -> impl IntoElement {
    let edge = look.length(token::BORDER_WIDTH);
    let (width, reach) = (edge * 2., edge * 0.5);
    div().absolute().inset_0().with_animation(
        id,
        // Three spins of the glyph spinner: a border is a long path, so the same pace reads as a blur.
        Animation::new(look.duration(token::DURATION_SPIN) * 3).repeat(),
        move |layer, phase| layer.child(sweep(phase, radius, width, reach, ink)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tail_fades_from_the_head_back() {
        let tail = arc_segments(0.3, TAIL_DEGREES, SEGMENTS);
        assert_eq!(tail.len(), SEGMENTS);
        assert!(tail.windows(2).all(|pair| pair[0].alpha > pair[1].alpha));
        assert!(tail[0].alpha > 0.9 && tail[SEGMENTS - 1].alpha > 0. && tail[SEGMENTS - 1].alpha < 0.1);
    }

    #[test]
    fn the_head_is_at_the_phase_angle() {
        let tail = arc_segments(0.25, TAIL_DEGREES, SEGMENTS);
        assert!((tail[0].end() - 90.).abs() < 1e-3);
    }

    #[test]
    fn the_tail_covers_its_degrees_and_wraps_past_360() {
        let tail = arc_segments(0.1, TAIL_DEGREES, SEGMENTS);
        let covered: f32 = tail.iter().map(|one| one.sweep).sum();
        assert!((covered - TAIL_DEGREES).abs() < 1e-3);
        assert!(tail.iter().all(|one| (0. ..360.).contains(&one.start)));
        // Head at 36 degrees: the tail's far end is 104 degrees behind zero, at 256.
        assert!((tail[SEGMENTS - 1].start - 256.).abs() < 1e-3);
        assert_eq!(arc_segments(1.25, TAIL_DEGREES, SEGMENTS), arc_segments(0.25, TAIL_DEGREES, SEGMENTS));
    }

    #[test]
    fn the_perimeter_starts_at_the_top_centre_and_goes_clockwise() {
        let near = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 1e-2 && (a.1 - b.1).abs() < 1e-2;
        assert!(near(perimeter_point(100., 100., 10., 0.), (50., 0.)));
        assert!(near(perimeter_point(100., 100., 10., 0.25), (100., 50.)));
        assert!(near(perimeter_point(100., 100., 10., 0.5), (50., 100.)));
        assert!(near(perimeter_point(100., 100., 10., 0.75), (0., 50.)));
        assert!(near(perimeter_point(100., 100., 10., 1.), (50., 0.)));
    }
}
