//! A sketch read on its own, as Electron's `SketchReading` reads it in the panel: the whole drawing
//! fitted to a sheet over the window, its steps playing round, read-only. A wheel or `+`, `-` and
//! `0` zoom it, a drag or the arrows move it, and Esc, Close or a press beside it shut it.

use std::rc::Rc;
use std::time::Instant;

use gpui::{
    anchored, deferred, div, point, prelude::*, px, Anchor, Context, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, Role,
    ScrollDelta, ScrollEvent, SharedString, Window,
};
use ipc::{Scene, ThreadItem, ThreadSketch};

use super::cards::shown_scene;
use super::rows::{glyph, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::scene::{Fit, Lens};

/// The sketch open over the window.
pub(super) struct Reading {
    key: String,
    title: SharedString,
    scene: Rc<Scene>,
    since: Instant,
    focus: FocusHandle,
    back: Option<FocusHandle>,
    lens: Lens,
    /// The press held, and where the pointer was last.
    drag: Option<gpui::Point<f32>>,
}

/// A key's zoom step and pan, and the most and least the reader may zoom past the fit.
const KEY_STEP: f32 = 1.5;
const KEY_PAN: f32 = 64.;
const MOST: f32 = 4.;
const LEAST: f32 = 0.5;
const WHEEL_SPEED: f32 = 0.002;
const LINE_HEIGHT: f32 = 16.;

/// The card with this id: the pinned one, else the one in the thread.
fn card_of<'a>(head: &'a ipc::ThreadHead, items: &'a [ThreadItem], id: &str) -> Option<&'a ThreadSketch> {
    head.sketch.as_ref().filter(|card| card.id == id).or_else(|| {
        items.iter().find_map(|item| match item {
            ThreadItem::Sketch(card) if card.id == id => Some(card),
            _ => None,
        })
    })
}

impl JobView {
    /// A press on a sketch card: its drawing, the revision shown under `shown`, opens over the window.
    pub(super) fn read_sketch(&mut self, key: &str, shown: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let picked = self.pages.get(&job).and_then(|page| page.cards.revision.get(shown).copied());
        let Some((title, (scene_key, scene))) = self.thread(&job, cx).and_then(|held| card_of(&held.head, &held.items, key)).map(|card| (card.title.clone(), shown_scene(card, picked))) else {
            return;
        };
        let Some(scene) = scene else { return };
        let focus = cx.focus_handle();
        let back = window.focused(cx);
        window.focus(&focus, cx);
        self.sketch_reading = Some(Reading { key: scene_key, title: title.into(), scene, since: Instant::now(), focus, back, lens: Lens::default(), drag: None });
        cx.notify();
    }

    fn close_reading(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(back) = self.sketch_reading.take().and_then(|reading| reading.back) {
            window.focus(&back, cx);
        }
        cx.notify();
    }

    fn zoom_sketch(&mut self, by: f32, cx: &mut Context<Self>) {
        if let Some(reading) = self.sketch_reading.as_mut() {
            reading.lens.zoom = (reading.lens.zoom * by).clamp(LEAST, MOST);
        }
        cx.notify();
    }

    fn pan_sketch(&mut self, x: f32, y: f32, cx: &mut Context<Self>) {
        if let Some(reading) = self.sketch_reading.as_mut() {
            reading.lens.pan = (reading.lens.pan.0 + x, reading.lens.pan.1 + y);
        }
        cx.notify();
    }

    fn sketch_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.platform || key.modifiers.control || key.modifiers.alt {
            return;
        }
        match key.key.as_str() {
            "escape" => self.close_reading(window, cx),
            "+" | "=" => self.zoom_sketch(KEY_STEP, cx),
            "-" => self.zoom_sketch(1. / KEY_STEP, cx),
            "0" => {
                if let Some(reading) = self.sketch_reading.as_mut() {
                    reading.lens = Lens::default();
                }
                cx.notify();
            }
            "left" => self.pan_sketch(KEY_PAN, 0., cx),
            "right" => self.pan_sketch(-KEY_PAN, 0., cx),
            "up" => self.pan_sketch(0., KEY_PAN, cx),
            "down" => self.pan_sketch(0., -KEY_PAN, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    /// The sheet: the scrim over the window, and on it a frame with the sketch's band and its drawing.
    pub(super) fn render_sketch_reading(&self, window: &Window, cx: &Context<Self>) -> Option<impl IntoElement> {
        let reading = self.sketch_reading.as_ref()?;
        let look = cx.look();
        let viewport = window.viewport_size();
        let inset = look.length(token::SPACE_6);
        let button = |id: &'static str, icon: &'static str, said: &'static str, key: &'static str, act: fn(&mut Self, &mut Window, &mut Context<Self>)| {
            let key = gpui::Keystroke::parse(key).ok();
            labelled(
                div()
                    .id(id)
                    .size(look.length(token::H_CONTROL_SM))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED)))
                    .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said).key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, window, cx| act(this, window, cx))),
                Role::Button,
                said,
            )
        };
        let band = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_h(look.length(token::H_ROW) * 1.25)
            .pl(look.length(token::SPACE_4))
            .pr(look.length(token::SPACE_2))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_RAISED))
            .child(glyph(look, "pencil-ruler", Size::Small, look.color(token::FG_MUTED)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_SM))
                    .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(reading.title.clone()),
            )
            .child(button("sketch-zoom-out", "minus", "Zoom out", "-", |this, _, cx| this.zoom_sketch(1. / KEY_STEP, cx)))
            .child(button("sketch-zoom-in", "plus", "Zoom in", "+", |this, _, cx| this.zoom_sketch(KEY_STEP, cx)))
            .child(button("sketch-close", "x", "Close", "escape", |this, window, cx| this.close_reading(window, cx)));
        let drawing = div()
            .id("sketch-drawing")
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .bg(look.color(token::BG_SUNKEN))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, _, cx| {
                if let Some(reading) = this.sketch_reading.as_mut() {
                    reading.drag = Some(point(f32::from(event.position.x), f32::from(event.position.y)));
                }
                cx.stop_propagation();
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                let Some(was) = this.sketch_reading.as_ref().and_then(|reading| reading.drag) else { return };
                if event.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                let now = point(f32::from(event.position.x), f32::from(event.position.y));
                if let Some(reading) = this.sketch_reading.as_mut() {
                    reading.drag = Some(now);
                }
                this.pan_sketch(now.x - was.x, now.y - was.y, cx);
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, _| {
                if let Some(reading) = this.sketch_reading.as_mut() {
                    reading.drag = None;
                }
            }))
            .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| {
                let delta = match event.delta {
                    ScrollDelta::Pixels(by) => f32::from(by.y),
                    ScrollDelta::Lines(by) => by.y * LINE_HEIGHT,
                };
                this.zoom_sketch((delta * WHEEL_SPEED).exp(), cx);
            }))
            .child(crate::scene::view(reading.key.clone().into(), reading.scene.clone(), Fit::Reading, Some(reading.since), reading.lens));
        Some(
            deferred(
                anchored().position(point(px(0.), px(0.))).anchor(Anchor::TopLeft).child(
                    div()
                        .id("sketch-view")
                        .occlude()
                        .track_focus(&reading.focus)
                        .w(viewport.width)
                        .h(viewport.height)
                        .p(inset)
                        .flex()
                        .bg(look.color(token::SCRIM))
                        .on_key_down(cx.listener(Self::sketch_key))
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_reading(window, cx)))
                        .child(labelled(
                            div()
                                .id("sketch-sheet")
                                .flex_1()
                                .flex()
                                .flex_col()
                                .overflow_hidden()
                                .rounded(look.length(token::RADIUS_LG))
                                .border(look.length(token::BORDER_WIDTH))
                                .border_color(look.color(token::BORDER_DEFAULT))
                                .shadow(look.shadows(token::SHADOW_OVERLAY))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(band)
                                .child(drawing),
                            Role::Dialog,
                            format!("Sketch: {}", reading.title),
                        )),
                ),
            )
            .with_priority(3),
        )
    }
}
