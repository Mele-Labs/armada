//! The busy ring: while something loads, its border carries a bright arc with a fading tail that
//! circles the rounded edge, in place of a spinner glyph. One colour, the tail fading to clear over
//! [`TAIL_DEGREES`], the head the brightest and nearest white. A turn takes the seconds he set in
//! Settings, Motion (six where he has set none), read from `settings::motion`.
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

/// How many times a second the ring asks for a new frame. Each ask redraws the whole view the ring sits in,
/// so a turn that takes seconds does not need sixty.
pub const RING_FPS: f32 = 30.;

/// How many times a second a running call's clock asks for a new frame: it changes once a second.
pub const COUNT_FPS: f32 = 2.;

/// The animation a running call's elapsed time counts on.
pub fn count_animation() -> Animation {
    Animation::new(std::time::Duration::from_secs(1)).repeat().with_max_fps(COUNT_FPS)
}

/// The ring over an element's border: absolutely placed, its own size, turning once per the seconds he set.
/// `radius` is the element's corner radius; `ink` is the arc's colour.
pub fn ring_overlay(look: &Look, id: impl Into<ElementId>, radius: Pixels, ink: Hsla) -> impl IntoElement {
    ring_overlay_in(look, id, radius, ink, crate::settings::motion::ring_seconds())
}

/// The same ring turning once per `seconds`: Settings, Motion draws its preview at the speed on the stepper,
/// before it is saved.
pub fn ring_overlay_in(look: &Look, id: impl Into<ElementId>, radius: Pixels, ink: Hsla, seconds: u32) -> impl IntoElement {
    let edge = look.length(token::BORDER_WIDTH);
    let (width, reach) = (edge * 2., edge * 0.5);
    div().absolute().inset_0().with_animation(
        id,
        Animation::new(std::time::Duration::from_secs(u64::from(seconds))).repeat().with_max_fps(RING_FPS),
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

    use gpui::{list, AnyElement, Context, Entity, ListAlignment, ListState, Render, SharedString, Window};
    use std::{cell::Cell, rc::Rc, time::Duration};

    thread_local! {
        static PHASES: std::cell::RefCell<Vec<f32>> = std::cell::RefCell::default();
    }

    /// One list row like the live calls card: a 1 s counting timer, and the ring over it when `ringed`.
    struct Thread {
        state: ListState,
        ringed: bool,
        drawn: Rc<Cell<u32>>,
        counted: Rc<Cell<u32>>,
    }

    impl Render for Thread {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = self.state.clone();
            let (ringed, drawn, counted) = (self.ringed, self.drawn.clone(), self.counted.clone());
            let _ = cx;
            list(state, move |_, _, _| {
                drawn.set(drawn.get() + 1);
                let counted = counted.clone();
                let look = Look::dark();
                let mut card = div().relative().w(px(300.)).h(px(80.)).child(div().with_animation(
                    SharedString::from("count"),
                    count_animation(),
                    move |time, delta| {
                        counted.set(counted.get() + 1);
                        PHASES.with(|all| all.borrow_mut().push(delta));
                        time.child("0s")
                    },
                ));
                if ringed {
                    card = card.child(ring_overlay_in(&look, SharedString::from("ring"), px(8.), look.color(token::STATUS_RUNNING), 6));
                }
                card.into_any_element() as AnyElement
            })
            .size_full()
        }
    }

    /// Frames of 16 ms over `frames`: how many times the row was drawn, and its timer asked for its value.
    fn timer_ticks(ringed: bool, frames: u32, cx: &mut gpui::TestAppContext) -> (u32, u32) {
        cx.update(crate::init_app);
        let (drawn, counted) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let (view, cx): (Entity<Thread>, _) = {
            let (d, c) = (drawn.clone(), counted.clone());
            cx.add_window_view(move |_, _| {
                let state = ListState::new(1, ListAlignment::Top, px(200.));
                Thread { state, ringed, drawn: d, counted: c }
            })
        };
        let _ = view;
        for _ in 0..frames {
            cx.executor().advance_clock(Duration::from_millis(16));
            cx.update(|window, cx| {
                window.simulate_next_frame(cx);
            });
            cx.run_until_parked();
        }
        (drawn.get(), counted.get())
    }

    #[gpui::test]
    fn a_list_rows_timer_keeps_ticking_with_and_without_the_ring(cx: &mut gpui::TestAppContext) {
        let plain = timer_ticks(false, 240, cx);
        let ringed = timer_ticks(true, 240, cx);
        let phases = PHASES.with(|all| all.borrow().clone());
        assert!(phases.windows(2).filter(|pair| pair[1] > pair[0]).count() >= 6, "the animation's phase advances: {phases:?}");
        assert!(plain.1 >= 6 && ringed.1 >= 6, "the timer ticks alone {plain:?} and beside the ring {ringed:?}");
    }

    #[gpui::test]
    fn the_ring_and_the_clock_do_not_redraw_the_view_every_frame(cx: &mut gpui::TestAppContext) {
        // About two seconds at sixty frames a second: a redraw every frame would draw the row some 120 times.
        let (drawn, _) = timer_ticks(true, 120, cx);
        assert!(drawn <= 80, "the row was drawn {drawn} times in two seconds");
    }
}
