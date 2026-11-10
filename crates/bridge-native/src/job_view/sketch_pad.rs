//! Draw sketch: the composer's pencil-ruler opens a sheet over the app with a pad on it, and what
//! is drawn there goes with the next message as a picture (`sessions.tsx` `SketchSheet` on
//! `SketchPad`). Boxes hold words, joins run between two of them, and the pen draws by hand; the
//! rail holds the pen, Undo and Add a box over the view's Zoom out, Zoom in and Fit, as
//! `GraphCanvas` draws them.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, Anchor, AnyElement, Bounds, Context, FocusHandle, FontWeight, Hsla, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels, Role, ScrollEvent, SharedString, Stateful, Subscription, Window,
};
use gpui_component::input::{InputEvent, Textarea, TextareaState};

use super::acts::{button, Tone};
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::sketch::{self, Drawing, Point, Shape};

const BOX_PLACEHOLDER: &str = "A panel, a read, a step";
/// React Flow's own bounds and step, which `GraphCanvas` leaves as they are.
const ZOOM_LEAST: f32 = 0.5;
const ZOOM_MOST: f32 = 2.;
const ZOOM_STEP: f32 = 1.2;
/// Room around a fitted picture (`SketchPad.tsx` `FIT`).
const FIT_PAD: f32 = 0.1;
/// The grid a new box looks for room on, as a share of its width (`Paste.tsx` `A_SEARCH_STEP`).
const SEARCH_STEP: f32 = 1. / 8.;

/// What a press on the pad is moving until the pointer lifts.
enum Grab {
    Box { id: String, from: gpui::Point<Pixels>, was: (f32, f32) },
    Pan { from: gpui::Point<Pixels>, was: (f32, f32) },
}

/// The pad of one Job, kept while its sheet is shut, as Electron's sheet keeps its drawing.
pub(super) struct Pad {
    job: SharedString,
    drawing: Drawing,
    areas: HashMap<String, (gpui::Entity<TextareaState>, Subscription)>,
    picked: Vec<String>,
    pen: bool,
    ink: Option<Vec<Point>>,
    grab: Option<Grab>,
    pan: (f32, f32),
    zoom: f32,
    /// Where the pane was drawn, and each box, in the window, as the last frame placed them.
    pane: Rc<Cell<Bounds<Pixels>>>,
    placed: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    focus: FocusHandle,
}

impl Pad {
    fn new(job: SharedString, cx: &mut Context<JobView>) -> Pad {
        Pad {
            job,
            drawing: Drawing::default(),
            areas: HashMap::new(),
            picked: Vec::new(),
            pen: false,
            ink: None,
            grab: None,
            pan: (0., 0.),
            zoom: 1.,
            pane: Rc::new(Cell::new(Bounds::default())),
            placed: Rc::new(RefCell::new(HashMap::new())),
            focus: cx.focus_handle(),
        }
    }

    /// A place on the pad, from a place in the window.
    fn to_pad(&self, at: gpui::Point<Pixels>) -> Point {
        let pane = self.pane.get();
        Point { x: (f32::from(at.x - pane.origin.x) - self.pan.0) / self.zoom, y: (f32::from(at.y - pane.origin.y) - self.pan.1) / self.zoom }
    }

    /// A place in the window, from a place on the pad.
    fn to_window(&self, at: &Point) -> gpui::Point<Pixels> {
        let pane = self.pane.get();
        point(pane.origin.x + px(self.pan.0 + at.x * self.zoom), pane.origin.y + px(self.pan.1 + at.y * self.zoom))
    }

    /// A box's size on the pad: as drawn, or a new box's before it is.
    fn size_of(&self, id: &str, wide: f32) -> (f32, f32) {
        self.placed.borrow().get(id).map_or((wide, wide * SEARCH_STEP * 2.), |bounds| (f32::from(bounds.size.width) / self.zoom, f32::from(bounds.size.height) / self.zoom))
    }

    /// Zoom about a place in the pane, held under the pointer as React Flow holds it.
    fn zoom_about(&mut self, to: f32, about: (f32, f32)) {
        let to = to.clamp(ZOOM_LEAST, ZOOM_MOST);
        let at = ((about.0 - self.pan.0) / self.zoom, (about.1 - self.pan.1) / self.zoom);
        self.zoom = to;
        self.pan = (about.0 - at.0 * to, about.1 - at.1 * to);
    }

    fn middle(&self) -> (f32, f32) {
        let pane = self.pane.get();
        (f32::from(pane.size.width) / 2., f32::from(pane.size.height) / 2.)
    }

    /// Where a new box goes (`Paste.tsx` `useMiddle`): the free spot nearest the middle of what is
    /// in view, or the middle a step off anything put down exactly there.
    fn spot(&self, wide: f32) -> (f32, f32) {
        let pane = self.pane.get();
        let from = self.to_pad(pane.origin);
        let to = self.to_pad(pane.origin + point(pane.size.width, pane.size.height));
        let view = (from.x, from.y, to.x - from.x, to.y - from.y);
        let tall = self.drawing.shapes.iter().map(|one| self.size_of(&one.id, wide).1).filter(|tall| *tall > 0.).fold(f32::INFINITY, f32::min);
        let size = (wide, if tall.is_finite() { tall } else { wide * SEARCH_STEP * 2. });
        let taken: Vec<(f32, f32, f32, f32)> = self.drawing.shapes.iter().map(|one| {
            let (w, h) = self.size_of(&one.id, wide);
            (one.x, one.y, w, h)
        }).collect();
        let step = (wide * SEARCH_STEP).max(1.);
        let centre = (view.0 + view.2 / 2., view.1 + view.3 / 2.);
        let clear = |x: f32, y: f32| {
            taken.iter().all(|one| x + size.0 + step <= one.0 || one.0 + one.2 + step <= x || y + size.1 + step <= one.1 || one.1 + one.3 + step <= y)
        };
        let mut spots = Vec::new();
        let mut y = view.1;
        while y + size.1 <= view.1 + view.3 {
            let mut x = view.0;
            while x + size.0 <= view.0 + view.2 {
                spots.push((x, y, (x + size.0 / 2. - centre.0).hypot(y + size.1 / 2. - centre.1)));
                x += step;
            }
            y += step;
        }
        spots.sort_by(|a, b| a.2.total_cmp(&b.2));
        if let Some(found) = spots.into_iter().find(|spot| clear(spot.0, spot.1)) {
            return (found.0.round(), found.1.round());
        }
        let mut at = (centre.0 - size.0 / 2., centre.1 - size.1 / 2.);
        while self.drawing.shapes.iter().any(|one| (one.x - at.0).abs() < step && (one.y - at.1).abs() < step) {
            at = (at.0 + step, at.1 + step);
        }
        (at.0.clamp(view.0, (view.0 + view.2 - size.0).max(view.0)).round(), at.1.clamp(view.1, (view.1 + view.3 - size.1).max(view.1)).round())
    }
}

impl JobView {
    /// The pencil-ruler under the box: the sheet opens on the Job's pad, as it was left.
    pub(super) fn open_sketch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if self.pad.as_ref().is_none_or(|pad| pad.job != job) {
            self.pad = Some(Pad::new(job, cx));
        }
        self.sketch_open = true;
        if let Some(pad) = &self.pad {
            window.focus(&pad.focus, cx);
        }
        cx.notify();
    }

    fn close_sketch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sketch_open = false;
        if let Some(pad) = self.pad.as_mut() {
            pad.grab = None;
            pad.ink = None;
        }
        self.to_composer(window, cx);
        cx.notify();
    }

    /// A box with these words, where there is room for it; the pen is put down, as a box is typed in.
    fn add_box(&mut self, body: String, window: &mut Window, cx: &mut Context<Self>) {
        let wide = f32::from(cx.look().length(token::W_SKETCH_BOX));
        let Some(pad) = self.pad.as_mut() else { return };
        pad.pen = false;
        let (x, y) = pad.spot(wide);
        let id = pad.drawing.next_shape_id();
        pad.drawing.with_shape(Shape { id: id.clone(), x, y, body: body.clone() });
        let area = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx).auto_grow(2, 8).placeholder(BOX_PLACEHOLDER);
            state.set_value(body, window, cx);
            state
        });
        let typed_in = id.clone();
        let typed = cx.subscribe_in(&area, window, move |this, area, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                let body = area.read(cx).value().to_string();
                if let Some(pad) = this.pad.as_mut() {
                    pad.drawing.with_body(&typed_in, &body);
                }
                cx.notify();
            }
        });
        pad.areas.insert(id.clone(), (area, typed));
        pad.picked = vec![id];
        cx.notify();
    }

    fn pad_act(&mut self, act: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pad) = self.pad.as_mut() else { return };
        match act {
            "draw" => pad.pen = !pad.pen,
            "undo" => pad.drawing.without_last_stroke(),
            "add" => return self.add_box(String::new(), window, cx),
            "out" | "in" => {
                let to = if act == "in" { pad.zoom * ZOOM_STEP } else { pad.zoom / ZOOM_STEP };
                let about = pad.middle();
                pad.zoom_about(to, about);
            }
            "fit" => self.fit_pad(cx),
            "join" => {
                if let [from, to] = pad.picked.as_slice() {
                    let (from, to) = (from.clone(), to.clone());
                    pad.drawing.with_join(&from, &to);
                }
            }
            "remove" => {
                let gone = std::mem::take(&mut pad.picked);
                pad.drawing.without_shapes(&gone);
                for id in &gone {
                    pad.areas.remove(id);
                    pad.placed.borrow_mut().remove(id);
                }
            }
            _ => {}
        }
        cx.notify();
    }

    /// The whole drawing in view, at no more than React Flow's largest zoom.
    fn fit_pad(&mut self, cx: &mut Context<Self>) {
        let wide = f32::from(cx.look().length(token::W_SKETCH_BOX));
        let Some(pad) = self.pad.as_mut() else { return };
        let mut seen: Vec<(f32, f32)> = Vec::new();
        for one in &pad.drawing.shapes {
            let (w, h) = pad.size_of(&one.id, wide);
            seen.extend([(one.x, one.y), (one.x + w, one.y + h)]);
        }
        seen.extend(pad.drawing.strokes.iter().flat_map(|stroke| stroke.points.iter().map(|at| (at.x, at.y))));
        if seen.is_empty() {
            pad.zoom = 1.;
            pad.pan = (0., 0.);
            return;
        }
        let (left, top) = seen.iter().fold((f32::INFINITY, f32::INFINITY), |(x, y), at| (x.min(at.0), y.min(at.1)));
        let (right, bottom) = seen.iter().fold((f32::NEG_INFINITY, f32::NEG_INFINITY), |(x, y), at| (x.max(at.0), y.max(at.1)));
        let (room_w, room_h) = { let pane = pad.pane.get(); (f32::from(pane.size.width), f32::from(pane.size.height)) };
        let (w, h) = ((right - left).max(1.), (bottom - top).max(1.));
        let zoom = (room_w / (w * (1. + FIT_PAD * 2.))).min(room_h / (h * (1. + FIT_PAD * 2.))).clamp(ZOOM_LEAST, ZOOM_MOST);
        pad.zoom = zoom;
        pad.pan = (room_w / 2. - (left + w / 2.) * zoom, room_h / 2. - (top + h / 2.) * zoom);
    }

    /// Attach sketch: the drawing goes in the box's tray as a chip, and the pad starts again.
    fn attach_sketch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pad) = self.pad.as_mut() else { return };
        if !pad.drawing.is_drawn() {
            return;
        }
        let drawing = std::mem::take(&mut pad.drawing);
        pad.areas.clear();
        pad.picked.clear();
        pad.placed.borrow_mut().clear();
        let id = format!("d{}", chrono::Utc::now().timestamp_millis());
        let job = pad.job.clone();
        self.composer.box_of(&job).drawn.push(sketch::attach(&drawing, id));
        self.close_sketch(window, cx);
    }

    pub(super) fn remove_drawn(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(job) = self.box_key() else { return };
        self.composer.box_of(&job).drawn.retain(|one| one.id != id);
        cx.notify();
    }

    // ---- the pointer on the pad -------------------------------------------------------------

    fn pad_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pad) = self.pad.as_mut() else { return };
        window.focus(&pad.focus, cx);
        if pad.pen {
            pad.ink = Some(vec![pad.to_pad(event.position)]);
        } else {
            pad.picked.clear();
            pad.grab = Some(Grab::Pan { from: event.position, was: pad.pan });
        }
        cx.notify();
    }

    fn box_down(&mut self, id: &str, event: &MouseDownEvent, grips: bool, cx: &mut Context<Self>) {
        let Some(pad) = self.pad.as_mut() else { return };
        // ⌘ or ⌃ held, a box joins the pick rather than replacing it, which is how two are picked to join.
        if event.modifiers.platform || event.modifiers.control {
            if let Some(at) = pad.picked.iter().position(|one| one == id) {
                pad.picked.remove(at);
            } else {
                pad.picked.push(id.to_string());
            }
        } else if !pad.picked.iter().any(|one| one == id) {
            pad.picked = vec![id.to_string()];
        }
        if grips {
            if let Some(shape) = pad.drawing.shapes.iter().find(|one| one.id == id) {
                pad.grab = Some(Grab::Box { id: id.to_string(), from: event.position, was: (shape.x, shape.y) });
            }
        }
        cx.notify();
    }

    fn pad_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(pad) = self.pad.as_mut() else { return };
        if let Some(ink) = pad.ink.as_mut() {
            let pane = pad.pane.get();
            let at = Point { x: (f32::from(event.position.x - pane.origin.x) - pad.pan.0) / pad.zoom, y: (f32::from(event.position.y - pane.origin.y) - pad.pan.1) / pad.zoom };
            ink.push(at);
            return cx.notify();
        }
        match &pad.grab {
            Some(Grab::Box { id, from, was }) => {
                let by = event.position - *from;
                let (x, y) = (was.0 + f32::from(by.x) / pad.zoom, was.1 + f32::from(by.y) / pad.zoom);
                let id = id.clone();
                // A box is reported where it lands; while it travels the pad holds it, as React Flow does.
                if let Some(shape) = pad.drawing.shapes.iter_mut().find(|one| one.id == id) {
                    shape.x = x;
                    shape.y = y;
                }
            }
            Some(Grab::Pan { from, was }) => {
                let by = event.position - *from;
                pad.pan = (was.0 + f32::from(by.x), was.1 + f32::from(by.y));
            }
            None => return,
        }
        cx.notify();
    }

    fn pad_up(&mut self, cx: &mut Context<Self>) {
        let Some(pad) = self.pad.as_mut() else { return };
        if let Some(points) = pad.ink.take() {
            pad.drawing.with_stroke(points);
        }
        if let Some(Grab::Box { id, .. }) = pad.grab.take() {
            if let Some((x, y)) = pad.drawing.shapes.iter().find(|one| one.id == id).map(|one| (one.x, one.y)) {
                pad.drawing.with_place(&id, x, y);
            }
        }
        cx.notify();
    }

    fn pad_wheel(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) {
        let Some(pad) = self.pad.as_mut() else { return };
        let dy = f32::from(event.delta.pixel_delta(px(16.)).y);
        let pane = pad.pane.get();
        let about = (f32::from(event.position.x - pane.origin.x), f32::from(event.position.y - pane.origin.y));
        pad.zoom_about(pad.zoom * (dy * 0.002).exp(), about);
        cx.notify();
    }

    /// ⌘V on the pad, outside a box: the words land at once as a box holding them.
    fn pad_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keys = &event.keystroke;
        if keys.key == "escape" {
            cx.stop_propagation();
            return self.close_sketch(window, cx);
        }
        let on_pad = self.pad.as_ref().is_some_and(|pad| pad.focus.is_focused(window));
        if on_pad && keys.key == "v" && keys.modifiers.platform {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()).filter(|text| !text.trim().is_empty()) {
                cx.stop_propagation();
                self.add_box(text, window, cx);
            }
        }
    }

    // ---- drawing --------------------------------------------------------------------------

    /// The pencil-ruler beside Attach in the box's bar.
    pub(super) fn sketch_button(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        labelled(
            super::head::icon_button(look, "draw-sketch", "pencil-ruler", "Draw a sketch to send with the message".into(), false, false)
                .on_click(cx.listener(|this, _, window, cx| this.open_sketch(window, cx))),
            Role::Button,
            "Draw sketch",
        )
        .into_any_element()
    }

    /// The tray's chip for a sketch waiting to go: its title, and Sketch where a file says its kind.
    pub(super) fn drawn_tiles(&self, id: &str, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        let Some(held) = self.composer.boxes.get(id) else { return Vec::new() };
        held.drawn
            .iter()
            .map(|one| {
                let name: SharedString = one.title.clone().into();
                let gone = one.id.clone();
                let remove = labelled(
                    super::head::icon_button(look, "remove-drawn", "trash", format!("Remove {}", one.title).into(), false, false)
                        .absolute()
                        .top(look.length(token::SPACE_1))
                        .right(look.length(token::SPACE_1))
                        .size(look.length(token::SPACE_6))
                        .rounded_full()
                        .bg(look.color(token::BG_OVERLAY))
                        .on_click(cx.listener(move |this, _, _, cx| this.remove_drawn(&gone, cx))),
                    Role::Button,
                    format!("Remove {}", one.title),
                );
                labelled(
                    div()
                        .id(SharedString::from(format!("drawn-{}", one.id)))
                        .relative()
                        .flex_none()
                        .h(look.length(token::SPACE_12) + look.length(token::SPACE_4))
                        .w(look.length(token::SPACE_12) * 3.)
                        .overflow_hidden()
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .bg(look.color(token::BG_RAISED))
                        .child(
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .justify_between()
                                .px(look.length(token::SPACE_3))
                                .py(look.length(token::SPACE_2))
                                .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).overflow_hidden().child(name.clone()))
                                .child(div().text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_SUBTLE)).child("Sketch")),
                        )
                        .child(remove)
                        .tooltip(tip(name.clone())),
                    Role::ListItem,
                    name,
                )
                .into_any_element()
            })
            .collect()
    }

    /// The sheet, over the app from under the title bar and at the right, as a floating wide `Sheet`.
    pub(super) fn render_sketch(&self, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.sketch_open {
            return None;
        }
        let pad = self.pad.as_ref()?;
        let look = cx.look();
        let viewport = window.viewport_size();
        let margin = look.length(token::SPACE_4);
        let close = labelled(
            div()
                .id("sketch-close")
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_LG))
                .rounded(look.length(token::RADIUS_LG))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Close \u{b7} Esc"))
                .child(glyph(look, "x", Size::Mark, look.color(token::FG_MUTED)))
                .on_click(cx.listener(|this, _, window, cx| this.close_sketch(window, cx))),
            Role::Button,
            "Close",
        );
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_LG))
                    .line_height(look.length(token::LEADING_LG))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child("Draw a sketch"),
            )
            .child(close);
        let drawn = pad.drawing.is_drawn();
        let attach = labelled(
            button(look, "attach-sketch", "Attach sketch", Tone::Primary, false, None, drawn)
                .w_full()
                .when(drawn, |button| button.on_click(cx.listener(|this, _, window, cx| this.attach_sketch(window, cx)))),
            Role::Button,
            "Attach sketch",
        );
        let body = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .p(look.length(token::SPACE_4))
            .child(self.render_pad(pad, look, cx))
            .child(attach);
        let sheet = labelled(
            div()
                .id("sketch-sheet")
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .capture_key_down(cx.listener(Self::pad_key))
                .w(viewport.width * 0.62)
                .h(viewport.height - look.length(token::H_TITLE_BAR) - margin * 2.)
                .m(margin)
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .child(head)
                .child(body),
            Role::Dialog,
            "Draw a sketch",
        );
        Some(
            deferred(
                anchored().position(point(px(0.), look.length(token::H_TITLE_BAR))).anchor(Anchor::TopLeft).child(
                    div()
                        .id("sketch-scrim")
                        .occlude()
                        .w(viewport.width)
                        .h(viewport.height - look.length(token::H_TITLE_BAR))
                        .flex()
                        .justify_end()
                        .bg(look.color(token::SCRIM))
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_sketch(window, cx)))
                        .child(sheet),
                ),
            )
            .with_priority(0)
            .into_any_element(),
        )
    }

    /// The pad: the rail as a column of the frame, and the pane with the joins, the boxes and the pen.
    fn render_pad(&self, pad: &Pad, look: &Look, cx: &Context<Self>) -> AnyElement {
        let off_undo = pad.drawing.strokes.is_empty();
        let draw = rail_button(look, "draw", "pencil", if pad.pen { "Draw" } else { "Draw" }, pad.pen, false, cx);
        let undo = rail_button(look, "undo", "undo-2", if off_undo { "Nothing has been drawn by hand." } else { "Undo" }, false, off_undo, cx);
        let add = rail_button(look, "add", "square-plus", "Add a box", false, false, cx);
        let out = rail_sign(look, "out", "\u{2212}", "Zoom out", cx);
        let into = rail_sign(look, "in", "+", "Zoom in", cx);
        let fit = rail_button(look, "fit", "maximize", "Fit", false, false, cx);
        let rail = div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_3))
            .child(rail_group(look, "What you can draw", vec![draw, undo, add]))
            .child(rail_group(look, "How you are looking at this", vec![out, into, fit]));

        let wide = look.length(token::W_SKETCH_BOX) * pad.zoom;
        let pane_cell = Rc::clone(&pad.pane);
        let placed = Rc::clone(&pad.placed);
        let joins: Vec<(String, String)> = pad.drawing.joins.iter().map(|one| (one.from.clone(), one.to.clone())).collect();
        let edge = look.color(token::BORDER_STRONG);
        let hairline = look.length(token::BORDER_WIDTH);
        let joins_layer = canvas(
            move |bounds, _, _| pane_cell.set(bounds),
            move |_, _, window, _| {
                let placed = placed.borrow();
                for (from, to) in &joins {
                    if let (Some(a), Some(b)) = (placed.get(from), placed.get(to)) {
                        paint_join(window, *a, *b, edge, hairline);
                    }
                }
            },
        )
        .absolute()
        .size_full();

        let boxes: Vec<AnyElement> = pad.drawing.shapes.iter().enumerate().map(|(at, shape)| self.render_box(pad, at, shape, wide, look, cx)).collect();
        let mut strokes: Vec<Vec<gpui::Point<Pixels>>> = pad.drawing.strokes.iter().map(|stroke| stroke.points.iter().map(|at| pad.to_window(at)).collect()).collect();
        strokes.extend(pad.ink.iter().map(|ink| ink.iter().map(|at| pad.to_window(at)).collect()));
        let ink = look.color(token::FG_MUTED);
        let pen_width = look.length(token::EDGE_ACTIVE);
        let ink_layer = canvas(|_, _, _| {}, move |_, _, window, _| {
            for points in &strokes {
                paint_stroke(window, points, ink, pen_width);
            }
        })
        .absolute()
        .size_full();
        let bar = (!pad.pen && !pad.picked.is_empty()).then(|| self.render_node_bar(pad, look, cx)).flatten();
        let pen_layer = pad.pen.then(|| {
            labelled(div()
                .id("sketch-pen")
                .absolute()
                .size_full()
                .cursor(gpui::CursorStyle::Crosshair)
                .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.pad_down(event, window, cx);
                })), Role::Group, "Where the pen draws")
        });
        let pane = div()
            .id("sketch-pane")
            .track_focus(&pad.focus)
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, window, cx| this.pad_down(event, window, cx)))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| this.pad_move(event, cx)))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _: &MouseUpEvent, _, cx| this.pad_up(cx)))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _: &MouseUpEvent, _, cx| this.pad_up(cx)))
            .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| this.pad_wheel(event, cx)))
            .child(joins_layer)
            .children(boxes)
            .child(ink_layer)
            .children(pen_layer)
            .children(bar);
        labelled(
            div()
                .id("sketch-pad")
                .flex()
                .h(look.length(token::H_SKETCH_PAD))
                .flex_none()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::SURFACE_CANVAS))
                .overflow_hidden()
                .child(rail)
                .child(pane),
            Role::Group,
            "Sketch for the message",
        )
        .into_any_element()
    }

    /// One box: the card is the grip, the words inside it are not, so a drag in them selects text.
    fn render_box(&self, pad: &Pad, at: usize, shape: &Shape, wide: Pixels, look: &Look, cx: &Context<Self>) -> AnyElement {
        let place = pad.to_window(&Point { x: shape.x, y: shape.y }) - pad.pane.get().origin;
        let picked = pad.picked.iter().any(|one| *one == shape.id);
        let name: SharedString = if shape.body.trim().is_empty() { "An empty box".into() } else { format!("Box: {}", shape.body).into() };
        let store = Rc::clone(&pad.placed);
        let kept = shape.id.clone();
        let (grip, well) = (shape.id.clone(), shape.id.clone());
        let words = pad.areas.get(&shape.id).map(|(area, _)| {
            labelled(
                div()
                    .id(("sketch-words", at))
                    .key_context(crate::keymap::TEXT_BOX)
                    .text_size(look.length(token::TEXT_SM) * pad.zoom)
                    .line_height(look.length(token::LEADING_SM) * pad.zoom)
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        this.box_down(&well, event, false, cx);
                    }))
                    .child(Textarea::new(area).aria_label("The words in this box")),
                Role::TextInput,
                "The words in this box",
            )
        });
        labelled(
            div()
                .id(("sketch-box", at))
                .absolute()
                .left(place.x)
                .top(place.y)
                .w(wide)
                .p(look.length(token::SPACE_2) * pad.zoom)
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_RAISED))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if picked { look.color(token::EDGE_FOCUS) } else { gpui::transparent_black() })
                .cursor(gpui::CursorStyle::OpenHand)
                .child(canvas(move |bounds, _, _| { store.borrow_mut().insert(kept, bounds); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full())
                .on_mouse_down(MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.box_down(&grip, event, true, cx);
                }))
                .children(words),
            Role::Group,
            name,
        )
        .into_any_element()
    }

    /// Join and Remove over the boxes picked (`BoxActs`); under them where there is no room over them.
    fn render_node_bar(&self, pad: &Pad, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let placed = pad.placed.borrow();
        let picked: Vec<Bounds<Pixels>> = pad.picked.iter().filter_map(|id| placed.get(id).copied()).collect();
        let first = picked.first()?;
        let pane = pad.pane.get();
        let (left, right) = picked.iter().fold((first.left(), first.right()), |(l, r), one| (l.min(one.left()), r.max(one.right())));
        let (top, bottom) = picked.iter().fold((first.top(), first.bottom()), |(t, b), one| (t.min(one.top()), b.max(one.bottom())));
        let gap = look.length(token::SPACE_2);
        let tall = look.length(token::H_CONTROL_SM) + look.length(token::SPACE_1) * 2. + look.length(token::BORDER_WIDTH) * 2.;
        let y = if top - pane.top() >= tall + gap { top - pane.top() - gap - tall } else { bottom - pane.top() + gap };
        let joinable = pad.picked.len() == 2;
        let join = button(look, "sketch-join", "Join", Tone::Ghost, false, None, joinable)
            .h(look.length(token::H_CONTROL_SM))
            .px(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .when(!joinable, |button| button.tooltip(tip("Pick two boxes to join them.")))
            .when(joinable, |button| button.on_click(cx.listener(|this, _, window, cx| this.pad_act("join", window, cx))));
        let danger = look.color(token::STATUS_COMPLETED_FAILED);
        let remove = div()
            .id("sketch-remove")
            .flex()
            .items_center()
            .justify_center()
            .size(look.length(token::H_CONTROL_SM))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(danger)
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::STATUS_COMPLETED_FAILED_BG)))
            .tooltip(tip("Remove"))
            .child(glyph(look, "trash", Size::Small, danger))
            .on_click(cx.listener(|this, _, window, cx| this.pad_act("remove", window, cx)));
        let middle = (left + right) / 2. - pane.left();
        Some(
            div()
                .absolute()
                .top(y)
                .left(middle)
                .child(
                    labelled(
                        div()
                            .id("sketch-node-bar")
                            .relative()
                            .left(-(look.length(token::H_CONTROL_SM) * 3.) / 2.)
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_1))
                            .p(look.length(token::SPACE_1))
                            .rounded(look.length(token::RADIUS_MD))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::BORDER_SUBTLE))
                            .bg(look.color(token::BG_OVERLAY))
                            .shadow(look.shadows(token::SHADOW_OVERLAY))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(labelled(join, Role::Button, "Join"))
                            .child(labelled(remove, Role::Button, "Remove")),
                        Role::Toolbar,
                        "What you can do with the boxes you picked",
                    ),
                )
                .into_any_element(),
        )
    }
}

/// One act on the rail: a small icon button with its tooltip, lit while it is on.
fn rail_button(look: &Look, act: &'static str, icon: &'static str, said: &'static str, on: bool, off: bool, cx: &Context<JobView>) -> AnyElement {
    let name = match act {
        "draw" => "Draw",
        "undo" => "Undo",
        "add" => "Add a box",
        _ => said,
    };
    let ink = if on { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) };
    labelled(
        rail_frame(look, act, on, off)
            .tooltip(tip(said))
            .child(glyph(look, icon, Size::Small, ink))
            .when(!off, |button| button.on_click(cx.listener(move |this, _, window, cx| this.pad_act(act, window, cx)))),
        Role::Button,
        name,
    )
    .into_any_element()
}

/// Zoom out and Zoom in, which draw their sign rather than a glyph.
fn rail_sign(look: &Look, act: &'static str, sign: &'static str, said: &'static str, cx: &Context<JobView>) -> AnyElement {
    labelled(
        rail_frame(look, act, false, false)
            .tooltip(tip(said))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_MUTED))
            .child(sign)
            .on_click(cx.listener(move |this, _, window, cx| this.pad_act(act, window, cx))),
        Role::Button,
        said,
    )
    .into_any_element()
}

fn rail_frame(look: &Look, act: &'static str, on: bool, off: bool) -> Stateful<gpui::Div> {
    div()
        .id(SharedString::from(format!("rail-{act}")))
        .flex()
        .items_center()
        .justify_center()
        .size(look.length(token::H_CONTROL_SM))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if on { look.color(token::ACCENT) } else { gpui::transparent_black() })
        .when(on, |button| button.bg(look.color(token::ACCENT_MUTED)))
        .when(off, |button| button.opacity(0.4))
        .when(!off, |button| button.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

fn rail_group(look: &Look, said: &'static str, acts: Vec<AnyElement>) -> AnyElement {
    labelled(
        div()
            .id(said)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .children(acts),
        Role::Group,
        said,
    )
    .into_any_element()
}

/// A join, from the side of one box that faces the other to the side facing back (`facingSides`),
/// curved as React Flow's default edge and closed with an arrowhead.
fn paint_join(window: &mut Window, from: Bounds<Pixels>, to: Bounds<Pixels>, color: Hsla, width: Pixels) {
    let (a, b) = (from.center(), to.center());
    let (dx, dy) = (f32::from(b.x - a.x), f32::from(b.y - a.y));
    let across = dx.abs() >= dy.abs();
    let (out, into) = match (across, dx >= 0., dy >= 0.) {
        (true, true, _) => ((from.right(), a.y), (to.left(), b.y)),
        (true, false, _) => ((from.left(), a.y), (to.right(), b.y)),
        (false, _, true) => ((a.x, from.bottom()), (b.x, to.top())),
        (false, _, false) => ((a.x, from.top()), (b.x, to.bottom())),
    };
    let (sx, sy, tx, ty) = (f32::from(out.0), f32::from(out.1), f32::from(into.0), f32::from(into.1));
    // React Flow's `calculateControlOffset` at its default curvature.
    let offset = |distance: f32| if distance >= 0. { 0.5 * distance } else { 0.25 * 25. * (-distance).sqrt() };
    let (c1, c2) = match (across, dx >= 0., dy >= 0.) {
        (true, true, _) => ((sx + offset(tx - sx), sy), (tx - offset(tx - sx), ty)),
        (true, false, _) => ((sx - offset(sx - tx), sy), (tx + offset(sx - tx), ty)),
        (false, _, true) => ((sx, sy + offset(ty - sy)), (tx, ty - offset(ty - sy))),
        (false, _, false) => ((sx, sy - offset(sy - ty)), (tx, ty + offset(sy - ty))),
    };
    let mut line = PathBuilder::stroke(width);
    line.move_to(point(px(sx), px(sy)));
    line.cubic_bezier_to(point(px(tx), px(ty)), point(px(c1.0), px(c1.1)), point(px(c2.0), px(c2.1)));
    if let Ok(path) = line.build() {
        window.paint_path(path, color);
    }
    let (ux, uy) = { let (x, y) = (tx - c2.0, ty - c2.1); let long = x.hypot(y).max(0.001); (x / long, y / long) };
    let (long, half) = (7., 4.5);
    let mut head = PathBuilder::fill();
    head.add_polygon(
        &[
            point(px(tx), px(ty)),
            point(px(tx - ux * long - uy * half), px(ty - uy * long + ux * half)),
            point(px(tx - ux * long + uy * half), px(ty - uy * long - ux * half)),
        ],
        true,
    );
    if let Ok(path) = head.build() {
        window.paint_path(path, color);
    }
}

/// A line drawn by hand, the same width at every zoom, round at its ends.
fn paint_stroke(window: &mut Window, points: &[gpui::Point<Pixels>], color: Hsla, width: Pixels) {
    let Some((first, rest)) = points.split_first() else { return };
    let mut line = PathBuilder::stroke(width);
    line.move_to(*first);
    if rest.is_empty() {
        line.line_to(*first + point(px(0.5), px(0.)));
    }
    for at in rest {
        line.line_to(*at);
    }
    if let Ok(path) = line.build() {
        window.paint_path(path, color);
    }
}
