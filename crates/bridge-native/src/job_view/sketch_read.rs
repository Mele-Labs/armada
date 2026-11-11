//! A sketch read on its own, as Electron's `SketchReading` reads it in the panel: the whole drawing
//! fitted to a sheet over the window, its steps playing round, read-only. Its rail, as
//! `GraphCanvas` draws one, stops and restarts the steps and zooms; a wheel or `+`, `-` and `0`
//! zoom it too, a drag or the arrows move it, and Esc, Close or a press beside it shut it.

use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    anchored, deferred, div, point, prelude::*, Anchor, AnyElement, Context, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, Role,
    ScrollDelta, ScrollEvent, SharedString, Window,
};
use ipc::{Scene, ThreadItem, ThreadSketch};

use super::cards::{shown_against, shown_scene};
use super::rows::{glyph, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::dropdown::{Entry, Row};
use crate::scene::{export, Clock, Fit, Lens};

/// How long a save or a copy is said in the sheet's band.
const SAID_FOR: Duration = Duration::from_secs(4);

/// What Save as image or Save as web page hands the save dialog.
enum Shared {
    Page(String),
    /// The standalone SVG: written as it is where he names an `.svg`, else drawn as PNG.
    Picture(String),
}

/// What a rail button draws: a glyph, or a sign where no glyph is right.
#[derive(Clone, Copy)]
enum Face {
    Glyph(&'static str),
    Sign(&'static str),
}

/// The sketch open over the window.
pub(super) struct Reading {
    key: String,
    title: SharedString,
    scene: Rc<Scene>,
    /// The sketch it is read as a change to, its title and drawing, and whether it is read so
    /// (Changes) or alone (Whole).
    against: Option<(SharedString, Rc<Scene>)>,
    changes: bool,
    /// When the steps started playing, or `None` while they are stopped.
    steps: Option<Instant>,
    /// When the sheet opened: the flows crawl from it while no step plays.
    opened: Instant,
    focus: FocusHandle,
    back: Option<FocusHandle>,
    lens: Lens,
    /// The press held, and where the pointer was last.
    drag: Option<gpui::Point<f32>>,
    /// What a save or a copy came to, said in the band for a moment.
    said: Option<SharedString>,
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
        let Some((title, (scene_key, scene), against)) = self
            .thread(&job, cx)
            .and_then(|held| card_of(&held.head, &held.items, key))
            .map(|card| (card.title.clone(), shown_scene(card, picked), shown_against(card, picked).map(|(title, scene)| (title.into(), scene))))
        else {
            return;
        };
        let Some(scene) = scene else { return };
        let focus = cx.focus_handle();
        let back = window.focused(cx);
        window.focus(&focus, cx);
        let now = Instant::now();
        let steps = crate::scene::plays(&scene).then_some(now);
        self.sketch_reading = Some(Reading { key: scene_key, title: title.into(), scene, against, changes: true, steps, opened: now, focus, back, lens: Lens::default(), drag: None, said: None });
        cx.notify();
    }

    /// Play stops the steps while they play and starts them from the first while they do not;
    /// Restart starts them from the first either way.
    fn play_sketch(&mut self, restart: bool, cx: &mut Context<Self>) {
        if let Some(reading) = self.sketch_reading.as_mut().filter(|reading| crate::scene::plays(&reading.scene)) {
            reading.steps = if reading.steps.is_some() && !restart { None } else { Some(Instant::now()) };
        }
        cx.notify();
    }

    /// Changes reads the sketch against the one it changes; Whole reads it alone.
    fn read_changes(&mut self, changes: bool, cx: &mut Context<Self>) {
        if let Some(reading) = self.sketch_reading.as_mut() {
            reading.changes = changes;
        }
        cx.notify();
    }

    /// Share opens a menu of three, as Electron's rail does: a page, a picture, or Mermaid text.
    fn share_sketch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(back) = self.sketch_reading.as_ref().map(|reading| reading.focus.clone()) else { return };
        let entries = vec![
            Entry::Row(Row::new("html", "Save as web page")),
            Entry::Row(Row::new("image", "Save as image")),
            Entry::Row(Row::new("mermaid", "Copy as Mermaid")),
        ];
        let at = window.mouse_position();
        self.open_pop("Share", entries, at, false, Some(back), |this, id, window, cx| this.share_as(id, window, cx), window, cx);
    }

    fn share_as(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(reading) = self.sketch_reading.as_ref() else { return };
        let (key, scene, title): (SharedString, Rc<Scene>, String) = (reading.key.clone().into(), reading.scene.clone(), reading.title.to_string());
        match id {
            "mermaid" => {
                let text = export::mermaid(&scene, &cx.look());
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                self.say_shared(format!("{} is on the clipboard.", export::MERMAID_COPIED).into(), cx);
            }
            "html" => {
                let page = export::web_page(&key, &scene, &title, window, cx);
                self.save_shared(export::file_name(&title, "html"), Shared::Page(page), cx);
            }
            "image" => {
                let svg = export::svg(&key, &scene, window, cx);
                self.save_shared(export::file_name(&title, "png"), Shared::Picture(svg), cx);
            }
            _ => {}
        }
    }

    /// The place is his, picked in the save dialog. A dialog he closes says nothing, as Electron's `savedSaid`.
    fn save_shared(&mut self, name: String, what: Shared, cx: &mut Context<Self>) {
        let folder = std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Downloads")).unwrap_or_else(std::env::temp_dir);
        let picked = cx.prompt_for_new_path(&folder, Some(&name));
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = picked.await else { return };
            let saved = path.file_name().map_or_else(|| name.clone(), |file| file.to_string_lossy().into_owned());
            let bytes = match what {
                Shared::Page(page) => Ok(page.into_bytes()),
                Shared::Picture(svg) if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("svg")) => Ok(svg.into_bytes()),
                Shared::Picture(svg) => export::png(&svg),
            };
            let said = match bytes.map(|bytes| std::fs::write(&path, bytes)) {
                Ok(Ok(())) => format!("Saved {saved}."),
                _ => format!("{saved} was not saved."),
            };
            let _ = this.update(cx, |this, cx| this.say_shared(said.into(), cx));
        })
        .detach();
    }

    /// A save or a copy is otherwise silent, so the band says it for a moment.
    fn say_shared(&mut self, said: SharedString, cx: &mut Context<Self>) {
        let Some(reading) = self.sketch_reading.as_mut() else { return };
        reading.said = Some(said.clone());
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAID_FOR).await;
            let _ = this.update(cx, |this, cx| {
                if let Some(reading) = this.sketch_reading.as_mut().filter(|reading| reading.said.as_ref() == Some(&said)) {
                    reading.said = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn fit_sketch(&mut self, cx: &mut Context<Self>) {
        if let Some(reading) = self.sketch_reading.as_mut() {
            reading.lens = Lens::default();
        }
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
            "0" => self.fit_sketch(cx),
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
        let room = crate::room::of(window, cx);
        let viewport = room.size;
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
            .children(reading.said.clone().map(|said| {
                labelled(
                    div().id("sketch-said").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(said.clone()),
                    Role::Status,
                    said,
                )
            }))
            .child(button("sketch-close", "x", "Close", "escape", |this, window, cx| this.close_reading(window, cx)));
        // `GraphCanvasRailGroup`'s buttons: ghost and icon-only, a sign where no glyph is right, a
        // lamp in the corner while one is on, and the reason in the tooltip while one is off.
        let rail_button = |id: &'static str, face: Face, said: &'static str, key: Option<&'static str>, pressed: Option<bool>, off: Option<&'static str>, act: fn(&mut Self, &mut Window, &mut Context<Self>)| {
            let key = key.and_then(|key| gpui::Keystroke::parse(key).ok());
            let tip: SharedString = off.unwrap_or(said).into();
            let ink = if off.is_some() {
                look.color(token::FG_SUBTLE)
            } else if pressed == Some(true) {
                look.color(token::FG_DEFAULT)
            } else {
                look.color(token::FG_MUTED)
            };
            let mut press = div()
                .id(id)
                .relative()
                .size(look.length(token::H_CONTROL_SM))
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .text_size(look.length(token::TEXT_SM))
                .text_color(ink)
                .child(match face {
                    Face::Glyph(name) => glyph(look, name, Size::Small, ink).into_any_element(),
                    Face::Sign(sign) => div().child(sign).into_any_element(),
                })
                .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(tip.clone()).key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
            press = if off.is_some() {
                press.bg(look.color(token::BG_SUNKEN)).border_color(look.color(token::BORDER_STRONG))
            } else {
                press.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, window, cx| act(this, window, cx)))
            };
            if pressed == Some(true) {
                let inset = look.length(token::SPACE_1) * 0.5;
                press = press.child(div().absolute().top(inset).right(inset).size(look.length(token::DOT)).rounded_full().bg(look.color(token::FG_DEFAULT)));
            }
            labelled(press, Role::Button, said)
        };
        let group = |id: &'static str, said: &'static str, buttons: Vec<AnyElement>| {
            labelled(
                div()
                    .id(id)
                    .flex()
                    .flex_col()
                    .flex_none()
                    .gap(look.length(token::SPACE_1))
                    .p(look.length(token::SPACE_1))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::BG_OVERLAY))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .children(buttons),
                Role::Group,
                said,
            )
        };
        let plays = crate::scene::plays(&reading.scene);
        let no_order = (!plays).then_some("This sketch has no order to play.");
        let playing = reading.steps.is_some();
        let rail = div()
            .flex()
            .flex_col()
            .flex_none()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_3))
            .child(group(
                "sketch-acts",
                "What you can do with the sketch",
                vec![
                    rail_button(
                        "sketch-play",
                        Face::Glyph(if playing { "pause" } else { "play" }),
                        if playing { "Stop the steps" } else { "Play the steps" },
                        None,
                        Some(playing),
                        no_order,
                        |this, _, cx| this.play_sketch(false, cx),
                    )
                    .into_any_element(),
                    rail_button("sketch-restart", Face::Glyph("rotate-cw"), "Restart the steps", None, None, no_order, |this, _, cx| this.play_sketch(true, cx)).into_any_element(),
                    rail_button("sketch-share", Face::Glyph("external-link"), "Share", None, None, None, |this, window, cx| this.share_sketch(window, cx)).into_any_element(),
                ],
            ))
            .child(group(
                "sketch-view-acts",
                "How you are looking at this",
                vec![
                    rail_button("sketch-zoom-out", Face::Sign("−"), "Zoom out", Some("-"), None, None, |this, _, cx| this.zoom_sketch(1. / KEY_STEP, cx)).into_any_element(),
                    rail_button("sketch-zoom-in", Face::Sign("+"), "Zoom in", Some("+"), None, None, |this, _, cx| this.zoom_sketch(KEY_STEP, cx)).into_any_element(),
                    rail_button("sketch-fit", Face::Glyph("maximize"), "Fit", Some("0"), None, None, |this, _, cx| this.fit_sketch(cx)).into_any_element(),
                ],
            ));
        let pane = div()
            .id("sketch-drawing")
            .flex_1()
            .min_w_0()
            .overflow_hidden()
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
            .child({
                let clock = Clock { steps: reading.steps, flows: Some(reading.opened) };
                match reading.against.as_ref().filter(|_| reading.changes) {
                    Some((_, before)) => {
                        let key = format!("{}:changes", reading.key);
                        let (scene, changes) = crate::scene::against(&key, before, &reading.scene);
                        crate::scene::view_changes(key.into(), scene, Some(changes), Fit::Reading, clock, reading.lens).into_any_element()
                    }
                    None => crate::scene::view(reading.key.clone().into(), reading.scene.clone(), Fit::Reading, clock, reading.lens).into_any_element(),
                }
            });
        // `SketchReading`'s band: what the sketch is read against, and Changes or Whole.
        let against = reading.against.as_ref().map(|(title, _)| {
            let read = |id: &'static str, said: &'static str, on: bool, changes: bool| {
                let weight = if on { crate::titlebar::parts::Weight::Secondary } else { crate::titlebar::parts::Weight::Ghost };
                labelled(
                    crate::titlebar::parts::Btn::new(id, said, weight)
                        .draw(look, cx)
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _, _, cx| this.read_changes(changes, cx))),
                    Role::Button,
                    said,
                )
            };
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(look.length(token::SPACE_4))
                .min_h(look.length(token::H_ROW) * 1.25)
                .pl(look.length(token::SPACE_4))
                .pr(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_1))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .text_color(look.color(token::FG_MUTED))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
                        .child(format!("Against {title}")),
                )
                .child(labelled(
                    div().id("sketch-reads").flex().flex_none().gap(look.length(token::SPACE_1)).child(read("sketch-changes", "Changes", reading.changes, true)).child(read("sketch-whole", "Whole", !reading.changes, false)),
                    Role::Group,
                    "Read as",
                ))
        });
        // The rail is a column of the frame beside the drawing, never a layer over it.
        let drawing = div().flex().flex_1().min_h_0().bg(look.color(token::BG_SUNKEN)).child(rail).child(pane);
        Some(
            deferred(
                anchored().position(room.origin).anchor(Anchor::TopLeft).child(
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
                                .children(against)
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
