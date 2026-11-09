//! The walk window: the walk bar over the screen, with the step's target ringed.
//!
//! Reaching a step plays it (a press, a key) and rings its target. Back plays the walk again from
//! a fresh screen up to the step before, so a step always shows what its steps before it made.
//! Approve, a note and the window closing go to Fleet through [`super::fleet`]; opened without a
//! Job, nothing is sent and the bar says so.

use gpui::{
    actions, div, prelude::*, px, App, Context, Entity, FocusHandle, FontWeight, KeyBinding, Role,
    SharedString, Subscription, Task, Window,
};
use std::cell::Cell;
use std::rc::Rc;

use gpui::{anchored, canvas, deferred, point, Bounds, CursorStyle, MouseButton, Pixels, Point};
use gpui_component::input::{InputEvent, Textarea, TextareaState};

use super::bar::{button, chord, icon_button, Weight};
use super::capture::{element_at, region, Held};
use super::fleet::{NoteOn, WalkFleet};
use super::stage::{find, perform, Stage};
use super::Walk;
use crate::labels::{label_root, labelled, Drawn, LabelBook};
use crate::look::{token, ActiveLook};

actions!(walk, [Next, Back, Reload, Approve, TakeNote, CancelNote]);

/// The bar's keys. **Electron's walk bar binds these in `CaptureBar.tsx` (`QUEUE_KEYS`)**, not in
/// the action registry, so they are restated here until the registry carries them.
pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("alt-cmd-right", Next, Some("walk")),
        KeyBinding::new("alt-cmd-left", Back, Some("walk")),
        KeyBinding::new("cmd-enter", Approve, Some("walk")),
        KeyBinding::new("cmd-r", Reload, Some("walk")),
        KeyBinding::new("alt-cmd-c", TakeNote, Some("walk")),
        KeyBinding::new("escape", CancelNote, Some("walk-note")),
        KeyBinding::new("escape", CancelNote, Some("walk")),
    ]);
}

pub struct Reading {
    pub at: usize,
    pub fault: Option<String>,
    pub missing: Option<String>,
}

/// A line the bar shows beside its acts: what Fleet answered, or why nothing was sent.
#[derive(Clone)]
struct Said {
    text: SharedString,
    fault: bool,
}

/// Capture while it is armed: what the pointer is over, a box being dragged, and once a press
/// lets go, what is held, its picture and the note being written on it.
#[derive(Default)]
struct Capturing {
    hover: Option<Drawn>,
    from: Option<Point<Pixels>>,
    to: Option<Point<Pixels>>,
    held: Option<Held>,
    /// The frame drawn for the picture: nothing of Capture's over the screen.
    clean: bool,
    picture: Option<String>,
    note: Option<(Entity<TextareaState>, Subscription)>,
}

pub struct WalkWindow {
    walk: &'static Walk,
    stage: Option<Stage>,
    /// The step on screen, from 0.
    at: usize,
    playing: bool,
    fleet: Option<WalkFleet>,
    capture: Option<Capturing>,
    /// Where the screen is drawn in the window, for what Capture may hold.
    stage_at: Rc<Cell<Option<Bounds<Pixels>>>>,
    said: Option<Said>,
    focus: FocusHandle,
    _play: Option<Task<()>>,
}

impl WalkWindow {
    pub fn new(walk: &'static Walk, fleet: Option<WalkFleet>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this =
            WalkWindow {
            walk,
            stage: None,
            at: 0,
            playing: false,
            fleet,
            capture: None,
            stage_at: Rc::default(),
            said: None,
            focus: cx.focus_handle(),
            _play: None,
        };
        this.play_to(0, true, window, cx);
        this
    }

    /// Play from a fresh screen (`fresh`), or from the step on screen, up to step `to`.
    fn play_to(&mut self, to: usize, fresh: bool, window: &mut Window, cx: &mut Context<Self>) {
        if to >= self.walk.steps.len() {
            return;
        }
        let walk = self.walk;
        let from = if fresh { 0 } else { self.at + 1 };
        self.playing = true;
        if fresh {
            match Stage::new(walk, window, cx) {
                Ok(stage) => self.stage = Some(stage),
                Err(why) => {
                    self.said = Some(Said { text: why.into(), fault: true });
                    return;
                }
            }
        }
        cx.notify();
        self._play = Some(cx.spawn_in(window, async move |this, cx| {
            for at in from..=to {
                let Ok(Some(focus)) = this.read_with(cx, |this, _| this.stage.as_ref().map(|stage| stage.focus.clone())) else { return };
                let step = walk.steps[at];
                let done = cx.update(|window, cx| {
                    window.draw(cx).clear(cx);
                    perform(&step, &focus, window, cx)
                });
                if let Ok(Err(missed)) = done {
                    let _ = this.update(cx, |this, cx| {
                        this.said = Some(Said { text: format!("Step {}: {}", at + 1, missed.0).into(), fault: true });
                        this.playing = false;
                        cx.notify();
                    });
                    return;
                }
            }
            let _ = cx.update(|window, cx| window.draw(cx).clear(cx));
            let _ = this.update(cx, |this, cx| {
                this.at = to;
                this.playing = false;
                cx.notify();
            });
        }));
    }

    fn next(&mut self, _: &Next, window: &mut Window, cx: &mut Context<Self>) {
        if !self.playing && self.at + 1 < self.walk.steps.len() {
            self.play_to(self.at + 1, false, window, cx);
        }
    }

    fn back(&mut self, _: &Back, window: &mut Window, cx: &mut Context<Self>) {
        if !self.playing && self.at > 0 {
            self.play_to(self.at - 1, true, window, cx);
        }
    }

    /// Play the step on screen again from a fresh screen.
    fn reload(&mut self, _: &Reload, window: &mut Window, cx: &mut Context<Self>) {
        if !self.playing {
            self.play_to(self.at, true, window, cx);
        }
    }

    fn approve(&mut self, _: &Approve, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else {
            self.said = Some(Said { text: "Not sent: no Job opened this walk".into(), fault: false });
            cx.notify();
            return;
        };
        self.send(cx, "Approved", move || fleet.approve());
    }

    /// ⌥⌘C arms Capture, and again puts it away.
    fn take_note(&mut self, _: &TakeNote, window: &mut Window, cx: &mut Context<Self>) {
        if self.capture.is_some() {
            self.cancel_note(&CancelNote, window, cx);
            return;
        }
        self.capture = Some(Capturing::default());
        cx.notify();
    }

    fn cancel_note(&mut self, _: &CancelNote, window: &mut Window, cx: &mut Context<Self>) {
        if self.capture.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        } else {
            // Esc is the screen's own while nothing is being captured.
            cx.propagate();
        }
    }

    fn pointer_moved(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let within = self.stage_at.get();
        let Some(capture) = self.capture.as_mut().filter(|capture| capture.held.is_none()) else { return };
        if capture.from.is_some() {
            capture.to = Some(at);
        } else {
            let drawn = cx.try_global::<LabelBook>().map(|book| book.drawn(window.window_handle().window_id()).to_vec()).unwrap_or_default();
            capture.hover = within.and_then(|within| element_at(&drawn, at, within));
        }
        cx.notify();
    }

    fn pressed(&mut self, at: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(capture) = self.capture.as_mut().filter(|capture| capture.held.is_none()) else { return };
        capture.from = Some(at);
        capture.to = Some(at);
        cx.notify();
    }

    /// A press lets go: a box if it was dragged, else the element under it. The window draws once
    /// with nothing of Capture's over the screen, and that frame is the note's picture.
    fn released(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(capture) = self.capture.as_mut().filter(|capture| capture.held.is_none()) else { return };
        let from = capture.from.take().unwrap_or(at);
        capture.to = None;
        let held = region(from, at).map(Held::Region).or_else(|| capture.hover.take().map(Held::Element));
        let Some(held) = held else {
            cx.notify();
            return;
        };
        capture.held = Some(held);
        capture.clean = true;
        cx.notify();
        let this = cx.entity().downgrade();
        window.on_next_frame(move |window, _cx| {
            window.refresh();
            window.on_next_frame(move |window, cx| {
                let _ = this.update(cx, |this, cx| this.pictured(window, cx));
            });
        });
    }

    fn pictured(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let scale = window.scale_factor();
        let picture = window.render_to_image().ok();
        let Some(capture) = self.capture.as_mut() else { return };
        capture.clean = false;
        capture.picture = picture.zip(capture.held.as_ref()).and_then(|(picture, held)| super::capture::crop(&picture, held.bounds(), scale));
        let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 8).placeholder("What should change here"));
        let watch = cx.subscribe_in(&input, window, |this, input, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { secondary: true, .. } = event {
                let said = input.read(cx).value().to_string();
                this.send_note(said, window, cx);
            }
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        capture.note = Some((input, watch));
        cx.notify();
    }

    fn send_note(&mut self, said: String, window: &mut Window, cx: &mut Context<Self>) {
        if said.trim().is_empty() {
            return;
        }
        let Some(capture) = self.capture.take() else { return };
        window.focus(&self.focus, cx);
        let Some(fleet) = self.fleet.clone() else {
            self.said = Some(Said { text: "Not sent: no Job opened this walk".into(), fault: false });
            cx.notify();
            return;
        };
        let (walk, at, of) = (self.walk.name, self.at as u32 + 1, self.walk.steps.len() as u32);
        let (role, name) = match &capture.held {
            Some(Held::Element(drawn)) => (super::capture::role_word(drawn.role).unwrap_or("element").to_lowercase(), drawn.name.to_string()),
            Some(Held::Region(bounds)) => (
                "region".to_string(),
                format!(
                    "{:.0},{:.0} {:.0}×{:.0}",
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height)
                ),
            ),
            None => return,
        };
        let crop = capture.picture;
        self.send(cx, "Note sent", move || fleet.note(&said, NoteOn { walk, step: at, of, role: &role, name: &name, crop }));
    }

    /// Send off the main thread, and show what came back.
    fn send(&mut self, cx: &mut Context<Self>, done: &'static str, call: impl FnOnce() -> Result<(), String> + Send + 'static) {
        cx.spawn(async move |this, cx| {
            let answer = cx.background_executor().spawn(async move { call() }).await;
            let _ = this.update(cx, |this, cx| {
                this.said = Some(match answer {
                    Ok(()) => Said { text: done.into(), fault: false },
                    Err(why) => Said { text: why.into(), fault: true },
                });
                cx.notify();
            });
        })
        .detach();
    }

    /// Where the walk stands, for the check: the step on screen, a fault the bar shows, and a
    /// target the window did not draw.
    pub fn reading(&self, window: &Window, cx: &App) -> Reading {
        Reading {
            at: self.at,
            fault: self.said.as_ref().filter(|said| said.fault).map(|said| said.text.to_string()),
            missing: if self.playing { Some("still playing".into()) } else { find(&self.walk.steps[self.at], window, cx).err().map(|missed| missed.0) },
        }
    }

    fn render_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let approve = approve_button(cx);
        let look = cx.look();
        let steps = self.walk.steps;
        let step = steps[self.at];
        let first = self.at == 0;
        let last = self.at + 1 == steps.len();
        let said = self.said.clone();
        // Electron's walk bar (`CaptureBar.tsx`, `WalkBar`): two rows of 44px, each padded 12 with 8 between.
        let row = || {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::SPACE_8) + look.length(token::SPACE_2) + look.length(token::SPACE_1))
                .px(look.length(token::SPACE_3))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
        };
        // The bar is the window's top, so it clears the traffic lights and a press on its empty part
        // moves the window, as the title row does.
        let drag = |id: &'static str| {
            div().id(id).h_full().on_mouse_down(gpui::MouseButton::Left, |event, window, _cx| {
                if event.click_count == 2 {
                    window.titlebar_double_click();
                } else {
                    window.start_window_move();
                }
            })
        };
        let top = row()
            .child(drag("walk-lights").flex_none().w(look.length(token::SPACE_12) * 2. - look.length(token::SPACE_3)))
            .child(drag("walk-room").flex_1().min_w_0().flex().items_center().children(said.map(|said| {
                labelled(
                    div()
                        .id("walk-said")
                        .truncate()
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(look.color(if said.fault { token::ERROR } else { token::FG_MUTED }))
                        .child(said.text.clone()),
                    Role::Status,
                    said.text,
                )
            })))
            .child(icon_button("walk-reload", "refresh-cw", "Reload", Some("⌘R"), Weight::Quiet, !self.playing, cx, cx.listener(|this, _, window, cx| this.reload(&Reload, window, cx))))
            .child(icon_button(
                "walk-note",
                "crosshair",
                if self.capture.is_some() { "Capturing" } else { "Capture" },
                Some("⌥⌘C"),
                if self.capture.is_some() { Weight::Primary } else { Weight::Quiet },
                true,
                cx,
                cx.listener(|this, _, window, cx| {
                    this.take_note(&TakeNote, window, cx)
                }),
            ))
            .child(
                // Approve keeps one width, `calc(var(--space-8) * 6)`, so nothing beside it moves.
                div().flex_none().w(look.length(token::SPACE_8) * 6.).child(approve),
            );
        let line = row()
            .child(icon_button("walk-back", "chevron-left", "Replay to the step before", Some("⌥⌘←"), Weight::Quiet, !first && !self.playing, cx, cx.listener(|this, _, window, cx| this.back(&Back, window, cx))))
            .child(
                div()
                    .flex_none()
                    .min_w(look.length(token::SPACE_8) + look.length(token::SPACE_4))
                    .flex()
                    .justify_center()
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(format!("{} of {}", self.at + 1, steps.len())),
            )
            .child(icon_button("walk-next", "chevron-right", "Act on the ringed target and go on", Some("⌥⌘→"), Weight::Primary, !last && !self.playing, cx, cx.listener(|this, _, window, cx| this.next(&Next, window, cx))))
            .child(labelled(
                div()
                    .id("walk-step")
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(step.say),
                Role::Status,
                step.say,
            ));
        labelled(
            div().id("walk-bar").flex().flex_col().flex_none().bg(look.color(token::BG_SUNKEN)).child(top).child(line),
            Role::Toolbar,
            format!("Walk {}", self.walk.title),
        )
    }
}

/// Approve as Electron's walk bar draws it: a secondary button filling its slot, the word and a
/// check centred, and the ⌘↵ chord.
fn approve_button(cx: &mut Context<WalkWindow>) -> impl IntoElement {
    let look = cx.look();
    labelled(
        div()
            .id("walk-approve")
            .w_full()
            .h(look.length(token::H_CONTROL_SM))
            .flex()
            .items_center()
            .justify_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_RAISED))
            .text_size(look.length(token::TEXT_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(look.color(token::FG_DEFAULT))
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .child("Approve")
            .child(gpui::svg().path(crate::assets::icon_path("check")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_DEFAULT)))
            .child(chord("⌘↵", cx))
            .on_click(cx.listener(|this, _, window, cx| this.approve(&Approve, window, cx))),
        Role::Button,
        "Approve",
    )
}

impl Render for WalkWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bar = self.render_bar(cx);
        let capture = self.render_capture(cx);
        let look = cx.look();
        let ring = if self.playing || self.capture.is_some() { None } else { find(&self.walk.steps[self.at], window, cx).ok() };
        let gap = look.length(token::FOCUS_RING_WIDTH) * 2.;
        let stage_at = Rc::clone(&self.stage_at);
        label_root(
            div()
                .id("walk-window")
                .key_context("walk")
                .track_focus(&self.focus)
                .on_action(cx.listener(Self::next))
                .on_action(cx.listener(Self::back))
                .on_action(cx.listener(Self::reload))
                .on_action(cx.listener(Self::approve))
                .on_action(cx.listener(Self::take_note))
                .on_action(cx.listener(Self::cancel_note))
                .relative()
                .size_full()
                .flex()
                .flex_col()
                .bg(look.color(token::SURFACE_CANVAS))
                // What a library widget inherits, such as the note's typed text; gpui's own is black.
                .text_color(look.color(token::FG_DEFAULT))
                .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                .child(bar)
                .child(
                    div()
                        .relative()
                        .flex_1()
                        .min_h(px(0.))
                        .children(self.stage.as_ref().map(|stage| stage.view.clone()))
                        .child(canvas(move |bounds, _, _| stage_at.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full()),
                )
                .children(ring.map(|drawn| {
                    div()
                        .absolute()
                        .left(drawn.bounds.origin.x - gap)
                        .top(drawn.bounds.origin.y - gap)
                        .w(drawn.bounds.size.width + gap * 2.)
                        .h(drawn.bounds.size.height + gap * 2.)
                        .rounded(look.length(token::RADIUS_SM))
                        .border(look.length(token::FOCUS_RING_WIDTH))
                        .border_color(look.color(token::ACCENT))
                }))
                .children(capture),
        )
    }
}

impl WalkWindow {
    /// What Capture draws over the screen: a crosshair cover that takes the pointer, the outline
    /// and name of what it is over or the box being dragged, and the note card on what is held.
    fn render_capture(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let capture = self.capture.as_ref()?;
        let within = self.stage_at.get()?;
        let look = cx.look();
        let edge = look.length(token::FOCUS_RING_WIDTH);
        let outline = |bounds: Bounds<Pixels>| {
            div()
                .absolute()
                .left(bounds.origin.x)
                .top(bounds.origin.y)
                .w(bounds.size.width)
                .h(bounds.size.height)
                .border(edge)
                .border_color(look.color(token::ACCENT))
                .bg(look.color(token::ACCENT).opacity(0.08))
        };
        let mut layer = div().absolute().inset_0();
        if capture.clean {
            return Some(layer.into_any_element());
        }
        if capture.held.is_none() {
            layer = layer.child(
                div()
                    .id("capture-cover")
                    .absolute()
                    .left(within.origin.x)
                    .top(within.origin.y)
                    .w(within.size.width)
                    .h(within.size.height)
                    .cursor(CursorStyle::Crosshair)
                    .occlude()
                    .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, window, cx| this.pointer_moved(event.position, window, cx)))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| this.pressed(event.position, cx)))
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, event: &gpui::MouseUpEvent, window, cx| this.released(event.position, window, cx))),
            );
        }
        let dragged = capture.from.zip(capture.to).and_then(|(from, to)| region(from, to));
        if let Some(bounds) = dragged {
            layer = layer.child(outline(bounds));
        } else if let Some(hover) = capture.hover.as_ref().filter(|_| capture.held.is_none()) {
            let name = Held::Element(hover.clone()).said();
            layer = layer.child(outline(hover.bounds)).child(
                div()
                    .absolute()
                    .left(hover.bounds.origin.x)
                    .top(hover.bounds.origin.y - look.length(token::H_BADGE) - edge)
                    .h(look.length(token::H_BADGE))
                    .px(look.length(token::SPACE_2))
                    .flex()
                    .items_center()
                    .rounded(look.length(token::RADIUS_SM))
                    .bg(look.color(token::ACCENT))
                    .text_size(look.length(token::TEXT_2XS))
                    .text_color(look.color(token::TEXT_ON_ACCENT))
                    .child(name),
            );
        }
        if let Some(held) = &capture.held {
            let bounds = held.bounds();
            layer = layer.child(outline(bounds));
            if let Some((input, _)) = &capture.note {
                let input = input.clone();
                let sent = input.clone();
                let gap = look.length(token::SPACE_2);
                let card = div()
                    .id("capture-card")
                    .key_context("walk-note")
                    // The field would keep Esc for itself.
                    .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape" {
                            cx.stop_propagation();
                            this.cancel_note(&CancelNote, window, cx);
                        }
                    }))
                    .w(look.length(token::SPACE_8) * 10.)
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_OVERLAY))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .child(
                        div()
                            .truncate()
                            .text_size(look.length(token::TEXT_2XS))
                            .line_height(look.length(token::LEADING_2XS))
                            .text_color(look.color(token::FG_MUTED))
                            .child(held.said()),
                    )
                    .child(labelled(div().id("capture-note").child(Textarea::new(&input)), Role::TextInput, "Note"))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(look.length(token::SPACE_2))
                            .child(button("capture-cancel", "Cancel", Some("esc"), Weight::Quiet, true, cx, cx.listener(|this, _, window, cx| this.cancel_note(&CancelNote, window, cx))))
                            .child(button("capture-send", "Send", Some("⌘↵"), Weight::Primary, true, cx, cx.listener(move |this, _, window, cx| {
                                let said = sent.read(cx).value().to_string();
                                this.send_note(said, window, cx);
                            }))),
                    );
                layer = layer.child(deferred(
                    anchored()
                        .position(point(bounds.origin.x, bounds.origin.y + bounds.size.height + gap))
                        .snap_to_window_with_margin(gap)
                        .child(card),
                ));
            }
        }
        Some(layer.into_any_element())
    }
}
