//! One zoom and pan for every picture view: the picture viewer, a sketch read over the window and any scene drawn large.
//! A wheel with ⌘ held, or a pinch, zooms about the pointer; a drag and a two-finger scroll move the picture, which a clamp
//! keeps over the view; a double press toggles fit and 100%; ⌘0 fits, ⌘= and ⌘- zoom. A picture in the reading column does
//! not bind it: there the wheel scrolls the page.
//!
//! The numbers are `ZoomablePicture`'s (`SessionThread.tsx`): fit is 1, a key steps by 1.5 and pans 64, a press that travels
//! further than 8 is a drag. A scale is always read against the fit, and `Stage::actual` is the scale where the picture is
//! drawn at its own size.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{canvas, div, point, prelude::*, px, size, AnyElement, App, Bounds, Context, Keystroke, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PinchEvent, Pixels, Point, Role, ScrollDelta, ScrollEvent, SharedString, Size, Window};

use super::rows::{glyph, Size as GlyphSize};
use crate::labels::labelled;
use crate::look::{token, Look};

pub(crate) const MOST: f32 = 8.;
pub(crate) const KEY_STEP: f32 = 1.5;
const KEY_PAN: f32 = 64.;
const TAP_SLOP: f32 = 8.;
const PINCH_SPEED: f32 = 0.01;
const WHEEL_SPEED: f32 = 0.002;
const LINE_HEIGHT: f32 = 16.;
/// What a double press from fit goes to where the picture is already drawn at its own size.
const NEAR: f32 = 2.;

/// What the view and the picture are, which is all the zoom needs to hold the picture over the view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Stage {
    /// The view the picture is held over.
    pub view: Size<f32>,
    /// The picture at fit: its shape inside the view, never larger than itself.
    pub fit: Size<f32>,
    /// The scale, against the fit, at which the picture is drawn at its own size.
    pub actual: f32,
}

impl Stage {
    /// A picture of `natural` pixels fitted inside `view` less `inset` on each side, and never made larger than it is.
    pub(crate) fn of_picture(natural: Size<f32>, view: Size<f32>, inset: f32) -> Stage {
        let room = size((view.width - 2. * inset).max(1.), (view.height - 2. * inset).max(1.));
        let fit = fitted(natural, room);
        let actual = if fit.width > 0. && natural.width > 0. { (natural.width / fit.width).max(1.) } else { 1. };
        Stage { view, fit, actual }
    }

    /// The most it zooms: 8 times the fit, or past its own size where that is further.
    fn most(&self) -> f32 {
        MOST.max(self.actual * 2.)
    }
}

/// `natural` made to fit inside `most`, keeping its shape and never made larger.
pub(crate) fn fitted(natural: Size<f32>, most: Size<f32>) -> Size<f32> {
    if natural.width <= 0. || natural.height <= 0. {
        return most;
    }
    let scale = (most.width / natural.width).min(most.height / natural.height).min(1.);
    size(natural.width * scale, natural.height * scale)
}

/// How far one axis may be moved: an edge of a picture larger than the view never comes inside it, so the picture never
/// leaves the view, and one that is smaller stays in the middle.
pub(crate) fn panned(pan: f32, length: f32, scale: f32, whole: f32) -> f32 {
    let half = length * scale / 2.;
    if half * 2. <= whole {
        return 0.;
    }
    pan.clamp(whole / 2. - half, half - whole / 2.)
}

/// A press held: where it began and where the pointer was last, from the view's middle, and whether it has travelled.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Press {
    from: Point<f32>,
    last: Point<f32>,
    moved: bool,
}

/// What a press let go of was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Tap {
    /// It travelled: the picture was dragged.
    Dragged,
    /// A second press in a row: fit and 100% were toggled.
    Toggled,
    /// One press that did not travel, at this point from the view's middle.
    Pressed(Point<f32>),
}

/// How far a picture is zoomed past its fit, and moved from the view's middle.
#[derive(Clone, Debug)]
pub(crate) struct Zoomer {
    pub scale: f32,
    pub pan: Point<f32>,
    press: Option<Press>,
    /// Where the view was last drawn, so a pointer's place is read from its middle.
    seen: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl Default for Zoomer {
    fn default() -> Self {
        Zoomer { scale: 1., pan: point(0., 0.), press: None, seen: Rc::default() }
    }
}

impl Zoomer {
    pub(crate) fn is_fit(&self) -> bool {
        self.scale <= 1. && self.pan == point(0., 0.)
    }

    pub(crate) fn fit(&mut self) {
        self.scale = 1.;
        self.pan = point(0., 0.);
    }

    /// Zoom to `scale` about `at`, a point from the view's middle; the pan follows, held over the view.
    pub(crate) fn zoom_about(&mut self, stage: Stage, at: Point<f32>, scale: f32) {
        let to = scale.clamp(1., stage.most());
        let by = to / self.scale;
        self.scale = to;
        self.pan = held(stage, self.scale, point(at.x - (at.x - self.pan.x) * by, at.y - (at.y - self.pan.y) * by));
    }

    pub(crate) fn zoom_by(&mut self, stage: Stage, at: Point<f32>, by: f32) {
        self.zoom_about(stage, at, self.scale * by);
    }

    pub(crate) fn pan_by(&mut self, stage: Stage, by: Point<f32>) {
        self.pan = held(stage, self.scale, point(self.pan.x + by.x, self.pan.y + by.y));
    }

    /// Fit and 100%: from fit it goes to the picture's own size about `at`, from anything else back to fit.
    pub(crate) fn toggle(&mut self, stage: Stage, at: Point<f32>) {
        if self.scale > 1.001 {
            self.fit();
        } else {
            self.zoom_about(stage, at, if stage.actual > 1.05 { stage.actual } else { NEAR });
        }
    }

    /// The figure beside the Fit button: the picture's size against its own, in whole percent.
    pub(crate) fn readout(&self, stage: Stage) -> String {
        format!("{}%", (self.scale / stage.actual * 100.).round() as i64)
    }

    /// A key the view takes: ⌘0 or 0 fits, ⌘= ⌘+ ⌘- or the bare keys zoom, and the arrows move a zoomed picture.
    pub(crate) fn key(&mut self, stage: Stage, key: &Keystroke) -> bool {
        let modifiers = key.modifiers;
        if modifiers.control || modifiers.alt || modifiers.function {
            return false;
        }
        let zoomed = self.scale > 1.;
        let centre = point(0., 0.);
        match key.key.as_str() {
            "+" | "=" => self.zoom_by(stage, centre, KEY_STEP),
            "-" => self.zoom_by(stage, centre, 1. / KEY_STEP),
            "0" => self.fit(),
            _ if modifiers.platform => return false,
            "left" if zoomed => self.pan_by(stage, point(KEY_PAN, 0.)),
            "right" if zoomed => self.pan_by(stage, point(-KEY_PAN, 0.)),
            "up" if zoomed => self.pan_by(stage, point(0., KEY_PAN)),
            "down" if zoomed => self.pan_by(stage, point(0., -KEY_PAN)),
            _ => return false,
        }
        true
    }

    pub(crate) fn press(&mut self, at: Point<f32>) {
        self.press = Some(Press { from: at, last: at, moved: false });
    }

    /// The pointer moved with the press held: past the slop the picture follows it. Whether it moved the picture.
    pub(crate) fn drag(&mut self, stage: Stage, at: Point<f32>) -> bool {
        let Some(press) = self.press.as_mut() else { return false };
        let by = point(at.x - press.last.x, at.y - press.last.y);
        press.last = at;
        press.moved |= (at.x - press.from.x).hypot(at.y - press.from.y) > TAP_SLOP;
        if !press.moved {
            return false;
        }
        self.pan_by(stage, by);
        true
    }

    /// The press let go: a drag, a double press that toggles fit and 100%, or one press that did not travel.
    pub(crate) fn release(&mut self, stage: Stage, at: Point<f32>, clicks: usize) -> Option<Tap> {
        let press = self.press.take()?;
        if press.moved {
            return Some(Tap::Dragged);
        }
        if clicks >= 2 {
            self.toggle(stage, at);
            return Some(Tap::Toggled);
        }
        Some(Tap::Pressed(at))
    }

    /// A wheel: with ⌘ held (or ⌃, which a trackpad's pinch is on some mice) it zooms about the pointer; else it moves the picture.
    pub(crate) fn wheel(&mut self, stage: Stage, at: Point<f32>, delta: ScrollDelta, zoom: bool, pinch: bool) {
        let (x, y) = match delta {
            ScrollDelta::Pixels(by) => (f32::from(by.x), f32::from(by.y)),
            ScrollDelta::Lines(by) => (by.x * LINE_HEIGHT, by.y * LINE_HEIGHT),
        };
        if zoom {
            // gpui's wheel is the page's moving, the DOM's `deltaY` the finger's: the sign turns.
            let speed = if pinch { PINCH_SPEED } else { WHEEL_SPEED };
            self.zoom_by(stage, at, (y * speed).exp());
        } else {
            self.pan_by(stage, point(x, y));
        }
    }

    /// A pinch about its own place.
    pub(crate) fn pinch(&mut self, stage: Stage, at: Point<f32>, delta: f32) {
        self.zoom_by(stage, at, (1. + delta).max(0.05));
    }

    /// A point of the window from the view's middle, where the view has been drawn.
    fn from_middle(&self, at: Point<Pixels>) -> Option<Point<f32>> {
        let middle = self.seen.get()?.center();
        Some(point(f32::from(at.x - middle.x), f32::from(at.y - middle.y)))
    }

    /// The view's size as last drawn.
    pub(crate) fn view(&self) -> Option<Size<f32>> {
        self.seen.get().map(|bounds| size(f32::from(bounds.size.width), f32::from(bounds.size.height)))
    }

    /// A zero-size element that notes where the view stands: put it inside the view, at its full size.
    pub(crate) fn probe(&self) -> AnyElement {
        let seen = self.seen.clone();
        canvas(move |bounds, _, _| seen.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full().into_any_element()
    }
}

/// The pan held so the picture stays over the view.
fn held(stage: Stage, scale: f32, pan: Point<f32>) -> Point<f32> {
    point(panned(pan.x, stage.fit.width, scale, stage.view.width), panned(pan.y, stage.fit.height, scale, stage.view.height))
}

/// A view that takes the zoom's events: it names its own zoomer and the stage the picture stands on.
pub(crate) trait Zooms: Sized + 'static {
    fn zoomer(&mut self) -> Option<&mut Zoomer>;
    /// The stage the open picture stands on in a view of `view` size.
    fn stage(&self, view: Size<f32>, window: &Window, cx: &App) -> Option<Stage>;
}

/// The zoom's stage and zoomer together, read at the moment of an event.
fn at_event<T: Zooms>(this: &mut T, window: &Window, cx: &App) -> Option<Stage> {
    let view = this.zoomer()?.view()?;
    this.stage(view, window, cx)
}

/// Bind the zoom's events on `surface`: a drag moves it, ⌘ and a wheel or a pinch zoom it, a two-finger scroll moves it, a
/// double press toggles fit and 100%. `tapped` hears one press that did not travel, at the window's point.
pub(crate) fn bind<T: Zooms, E: InteractiveElement>(surface: E, cx: &Context<T>, tapped: fn(&mut T, Point<Pixels>, &mut Window, &mut Context<T>)) -> E {
    surface
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this: &mut T, event: &MouseDownEvent, _, _| {
                if let Some(zoomer) = this.zoomer() {
                    if let Some(at) = zoomer.from_middle(event.position) {
                        zoomer.press(at);
                    }
                }
            }),
        )
        .on_mouse_move(cx.listener(|this: &mut T, event: &MouseMoveEvent, window, cx| {
            if event.pressed_button != Some(MouseButton::Left) {
                return;
            }
            let Some(stage) = at_event(this, window, cx) else { return };
            let Some(zoomer) = this.zoomer() else { return };
            let Some(at) = zoomer.from_middle(event.position) else { return };
            if zoomer.drag(stage, at) {
                cx.notify();
            }
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |this: &mut T, event: &MouseUpEvent, window, cx| {
                let Some(stage) = at_event(this, window, cx) else { return };
                let Some(zoomer) = this.zoomer() else { return };
                let Some(at) = zoomer.from_middle(event.position) else { return };
                match zoomer.release(stage, at, event.click_count) {
                    Some(Tap::Toggled) => cx.notify(),
                    Some(Tap::Pressed(_)) => tapped(this, event.position, window, cx),
                    Some(Tap::Dragged) | None => {}
                }
            }),
        )
        .on_scroll(cx.listener(|this: &mut T, event: &ScrollEvent, window, cx| {
            let Some(stage) = at_event(this, window, cx) else { return };
            let Some(zoomer) = this.zoomer() else { return };
            let Some(at) = zoomer.from_middle(event.position) else { return };
            zoomer.wheel(stage, at, event.delta, event.modifiers.platform || event.modifiers.control, event.modifiers.control);
            cx.notify();
        }))
        .on_pinch(cx.listener(|this: &mut T, event: &PinchEvent, window, cx| {
            let Some(stage) = at_event(this, window, cx) else { return };
            let Some(zoomer) = this.zoomer() else { return };
            let Some(at) = zoomer.from_middle(event.position) else { return };
            zoomer.pinch(stage, at, event.delta);
            cx.notify();
        }))
}

/// The zoom's keys, for a view's key handler: whether the key was one of them.
pub(crate) fn keyed<T: Zooms>(this: &mut T, key: &Keystroke, window: &Window, cx: &mut Context<T>) -> bool {
    let Some(stage) = at_event(this, window, cx) else { return false };
    let taken = this.zoomer().is_some_and(|zoomer| zoomer.key(stage, key));
    if taken {
        cx.notify();
    }
    taken
}

/// A zoom by `by` about the view's middle, as a button or a menu asks.
pub(crate) fn step<T: Zooms>(this: &mut T, by: f32, window: &Window, cx: &mut Context<T>) {
    let Some(stage) = at_event(this, window, cx) else { return };
    if let Some(zoomer) = this.zoomer() {
        zoomer.zoom_by(stage, point(0., 0.), by);
        cx.notify();
    }
}

/// Fit, as the button and ⌘0 do.
pub(crate) fn refit<T: Zooms>(this: &mut T, cx: &mut Context<T>) {
    if let Some(zoomer) = this.zoomer() {
        zoomer.fit();
        cx.notify();
    }
}

/// The corner: the zoom as a figure, and Fit. `fit` is what the button does; the keys fit and zoom all the same.
pub(crate) fn corner<T: Zooms>(look: &Look, readout: String, fit: fn(&mut T, &mut Window, &mut Context<T>), cx: &Context<T>) -> AnyElement {
    let said = SharedString::from(format!("Zoom {readout}"));
    let key = Keystroke::parse("cmd-0").ok();
    div()
        .id("zoom-corner")
        .absolute()
        .right(look.length(token::SPACE_3))
        .bottom(look.length(token::SPACE_3))
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .pl(look.length(token::SPACE_3))
        .pr(look.length(token::SPACE_1))
        .h(look.length(token::H_CONTROL))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_OVERLAY))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(labelled(
            div().id("zoom-readout").text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(readout),
            Role::Status,
            said,
        ))
        .child(labelled(
            div()
                .id("zoom-fit")
                .size(look.length(token::H_CONTROL_SM))
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(glyph(look, "maximize", GlyphSize::Small, look.color(token::FG_DEFAULT)))
                .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new("Fit").key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                .on_click(cx.listener(move |this, _, window, cx| fit(this, window, cx))),
            Role::Button,
            "Fit",
        ))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage() -> Stage {
        // A 2000 by 1000 picture in a 1000 by 800 view: fitted to 1000 by 500, drawn at its own size at 2.
        Stage::of_picture(size(2000., 1000.), size(1000., 800.), 0.)
    }

    fn key(text: &'static str) -> Keystroke {
        Keystroke::parse(text).expect("a key")
    }

    #[test]
    fn a_picture_is_fitted_and_its_own_size_is_the_scale_it_is_drawn_at_without_zoom() {
        let stage = stage();
        assert_eq!(stage.fit, size(1000., 500.));
        assert_eq!(stage.actual, 2.);
        let small = Stage::of_picture(size(300., 200.), size(1000., 800.), 0.);
        assert_eq!((small.fit, small.actual), (size(300., 200.), 1.));
    }

    #[test]
    fn a_zoom_about_the_pointer_keeps_the_point_under_it_where_it_was() {
        let (stage, mut zoomer) = (stage(), Zoomer::default());
        let at = point(120., -40.);
        // The picture's point under `at` before: its offset from the picture's middle, at scale 1 and no pan.
        zoomer.zoom_about(stage, at, 2.);
        let under = |zoomer: &Zoomer| point((at.x - zoomer.pan.x) / zoomer.scale, (at.y - zoomer.pan.y) / zoomer.scale);
        assert_eq!(under(&zoomer), point(120., -40.));
        zoomer.zoom_about(stage, at, 4.);
        assert_eq!(under(&zoomer), point(120., -40.));
    }

    #[test]
    fn a_picture_never_leaves_the_view_however_far_it_is_dragged() {
        let (stage, mut zoomer) = (stage(), Zoomer::default());
        zoomer.zoom_about(stage, point(0., 0.), 3.);
        zoomer.pan_by(stage, point(100_000., -100_000.));
        // Three times 1000 by 500 is 3000 by 1500 over 1000 by 800: each edge stops at the view's.
        assert_eq!(zoomer.pan, point(1000., -350.));
        assert_eq!(panned(-9e9, 200., 3., 800.), 0., "an axis that fits stays in the middle");
        zoomer.fit();
        zoomer.pan_by(stage, point(500., 500.));
        assert_eq!(zoomer.pan, point(0., 0.), "at fit there is nowhere to go");
    }

    #[test]
    fn a_double_press_toggles_fit_and_one_hundred_percent() {
        let (stage, mut zoomer) = (stage(), Zoomer::default());
        assert_eq!(zoomer.readout(stage), "50%");
        zoomer.press(point(10., 10.));
        assert_eq!(zoomer.release(stage, point(10., 10.), 2), Some(Tap::Toggled));
        assert_eq!((zoomer.scale, zoomer.readout(stage)), (2., "100%".to_string()));
        zoomer.press(point(10., 10.));
        assert_eq!(zoomer.release(stage, point(10., 10.), 2), Some(Tap::Toggled));
        assert!(zoomer.is_fit());
    }

    #[test]
    fn a_press_that_travels_is_a_drag_and_one_that_does_not_is_a_press() {
        let (stage, mut zoomer) = (stage(), Zoomer::default());
        zoomer.zoom_about(stage, point(0., 0.), 2.);
        zoomer.press(point(0., 0.));
        assert!(!zoomer.drag(stage, point(3., 0.)), "inside the slop");
        assert!(zoomer.drag(stage, point(40., 0.)));
        assert_eq!(zoomer.release(stage, point(40., 0.), 1), Some(Tap::Dragged));
        assert_eq!(zoomer.pan.x, 37., "only what came after the slop moved it");
        zoomer.press(point(5., 5.));
        assert_eq!(zoomer.release(stage, point(5., 5.), 1), Some(Tap::Pressed(point(5., 5.))));
    }

    #[test]
    fn a_wheel_with_command_zooms_and_a_two_finger_scroll_moves_the_picture() {
        let (stage, mut zoomer) = (stage(), Zoomer::default());
        zoomer.wheel(stage, point(0., 0.), ScrollDelta::Pixels(point(px(0.), px(120.))), true, false);
        assert!(zoomer.scale > 1.2, "a wheel with command held zooms in");
        let was = zoomer.pan;
        zoomer.wheel(stage, point(0., 0.), ScrollDelta::Pixels(point(px(-20.), px(0.))), false, false);
        assert_eq!(zoomer.pan.x, was.x - 20., "the same wheel without it moves the picture");
    }

    #[test]
    fn the_keys_fit_and_zoom_and_the_zoom_stays_between_fit_and_its_most() {
        let (stage, mut zoomer) = (stage(), Zoomer::default());
        assert!(zoomer.key(stage, &key("cmd-=")));
        assert_eq!(zoomer.scale, KEY_STEP);
        assert!(zoomer.key(stage, &key("cmd--")));
        assert_eq!(zoomer.scale, 1.);
        assert!(zoomer.key(stage, &key("cmd--")), "a key taken even where it does nothing");
        assert_eq!(zoomer.scale, 1., "it never goes under the fit");
        for _ in 0..20 {
            zoomer.key(stage, &key("cmd-="));
        }
        assert_eq!(zoomer.scale, stage.most());
        assert!(zoomer.key(stage, &key("cmd-0")));
        assert!(zoomer.is_fit());
        assert!(!zoomer.key(stage, &key("cmd-c")), "any other chord is not the zoom's");
        assert!(!zoomer.key(stage, &key("left")), "an arrow moves a zoomed picture only");
    }

    #[test]
    fn a_picture_drawn_larger_than_eight_fits_still_reaches_its_own_size() {
        let stage = Stage::of_picture(size(10_000., 8_000.), size(800., 600.), 0.);
        assert!(stage.actual > 12.);
        let mut zoomer = Zoomer::default();
        zoomer.toggle(stage, point(0., 0.));
        assert_eq!(zoomer.readout(stage), "100%");
    }
}
