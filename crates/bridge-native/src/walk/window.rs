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
use gpui_component::input::{Input, InputEvent, InputState};

use super::bar::{button, Weight};
use super::fleet::{NoteOn, WalkFleet};
use super::stage::{find, perform, Stage};
use super::Walk;
use crate::labels::{label_root, labelled};
use crate::look::{token, ActiveLook};

actions!(walk, [Next, Back, Approve, TakeNote, CancelNote]);

/// The bar's keys. **Electron's walk bar binds these in `CaptureBar.tsx` (`QUEUE_KEYS`)**, not in
/// the action registry, so they are restated here until the registry carries them.
pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("alt-cmd-right", Next, Some("walk")),
        KeyBinding::new("alt-cmd-left", Back, Some("walk")),
        KeyBinding::new("cmd-enter", Approve, Some("walk")),
        KeyBinding::new("alt-cmd-c", TakeNote, Some("walk")),
        KeyBinding::new("escape", CancelNote, Some("walk-note")),
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

pub struct WalkWindow {
    walk: &'static Walk,
    stage: Option<Stage>,
    /// The step on screen, from 0.
    at: usize,
    playing: bool,
    fleet: Option<WalkFleet>,
    note: Option<(Entity<InputState>, Subscription)>,
    said: Option<Said>,
    focus: FocusHandle,
    _play: Option<Task<()>>,
}

impl WalkWindow {
    pub fn new(walk: &'static Walk, fleet: Option<WalkFleet>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this =
            WalkWindow { walk, stage: None, at: 0, playing: false, fleet, note: None, said: None, focus: cx.focus_handle(), _play: None };
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

    fn approve(&mut self, _: &Approve, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else {
            self.said = Some(Said { text: "Not sent: no Job opened this walk".into(), fault: false });
            cx.notify();
            return;
        };
        self.send(cx, "Approved", move || fleet.approve());
    }

    fn take_note(&mut self, _: &TakeNote, window: &mut Window, cx: &mut Context<Self>) {
        if self.note.is_some() {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("What should change here"));
        let watch = cx.subscribe_in(&input, window, |this, input, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                let said = input.read(cx).value().to_string();
                this.send_note(said, window, cx);
            }
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        self.note = Some((input, watch));
        cx.notify();
    }

    fn cancel_note(&mut self, _: &CancelNote, window: &mut Window, cx: &mut Context<Self>) {
        self.note = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn send_note(&mut self, said: String, window: &mut Window, cx: &mut Context<Self>) {
        if said.trim().is_empty() {
            return;
        }
        self.note = None;
        window.focus(&self.focus, cx);
        let Some(fleet) = self.fleet.clone() else {
            self.said = Some(Said { text: "Not sent: no Job opened this walk".into(), fault: false });
            cx.notify();
            return;
        };
        let step = self.walk.steps[self.at];
        let (walk, at, of) = (self.walk.name, self.at as u32 + 1, self.walk.steps.len() as u32);
        self.send(cx, "Note sent", move || {
            fleet.note(&said, NoteOn { walk, step: at, of, role: step.target.role, name: step.target.name })
        });
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
        let look = cx.look();
        let steps = self.walk.steps;
        let step = steps[self.at];
        let first = self.at == 0;
        let last = self.at + 1 == steps.len();
        let said = self.said.clone();
        let top = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_ROW_HEADER))
            .px(look.length(token::SPACE_4))
            .bg(look.color(token::BG_SUNKEN))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(labelled(
                div()
                    .id("walk-name")
                    .text_size(look.length(token::TEXT_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(self.walk.title),
                Role::Heading,
                self.walk.title,
            ))
            .child(div().flex_1())
            .children(said.map(|said| {
                labelled(
                    div()
                        .id("walk-said")
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(look.color(if said.fault { token::ERROR } else { token::FG_MUTED }))
                        .child(said.text.clone()),
                    Role::Status,
                    said.text,
                )
            }))
            .child(button("walk-note", "Note", Some("⌥⌘C"), Weight::Quiet, self.note.is_none(), cx, cx.listener(|this, _, window, cx| this.take_note(&TakeNote, window, cx))))
            .child(button("walk-approve", "Approve", Some("⌘↵"), Weight::Primary, true, cx, cx.listener(|this, _, window, cx| this.approve(&Approve, window, cx))));
        let line = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_ROW))
            .px(look.length(token::SPACE_4))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(
                div()
                    .w(look.length(token::SPACE_12))
                    .flex_none()
                    .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(format!("{} of {}", self.at + 1, steps.len())),
            )
            .child(labelled(
                div()
                    .id("walk-step")
                    .flex_1()
                    .truncate()
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(step.say),
                Role::Status,
                step.say,
            ))
            .child(button("walk-back", "Back", Some("⌥⌘←"), Weight::Quiet, !first && !self.playing, cx, cx.listener(|this, _, window, cx| this.back(&Back, window, cx))))
            .child(button("walk-next", "Next", Some("⌥⌘→"), Weight::Quiet, !last && !self.playing, cx, cx.listener(|this, _, window, cx| this.next(&Next, window, cx))));
        let note = self.note.as_ref().map(|(input, _)| {
            let input = input.clone();
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .h(look.length(token::H_ROW_HEADER))
                .px(look.length(token::SPACE_4))
                .bg(look.color(token::BG_RAISED))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .key_context("walk-note")
                .child(
                    div()
                        .flex_none()
                        .text_size(look.length(token::TEXT_XS))
                        .text_color(look.color(token::FG_MUTED))
                        .child(format!("On {}", step.target.name)),
                )
                .child(labelled(div().id("walk-note-field").flex_1().child(Input::new(&input)), Role::TextInput, "Note"))
                .child(button("walk-note-send", "Send", Some("↵"), Weight::Primary, true, cx, cx.listener(move |this, _, window, cx| {
                    let said = input.read(cx).value().to_string();
                    this.send_note(said, window, cx);
                })))
                .child(button("walk-note-cancel", "Cancel", Some("esc"), Weight::Quiet, true, cx, cx.listener(|this, _, window, cx| this.cancel_note(&CancelNote, window, cx))))
        });
        labelled(div().id("walk-bar").flex().flex_col().flex_none().child(top).child(line).children(note), Role::Toolbar, "Walk")
    }
}

impl Render for WalkWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bar = self.render_bar(cx);
        let look = cx.look();
        let ring = if self.playing { None } else { find(&self.walk.steps[self.at], window, cx).ok() };
        let gap = look.length(token::FOCUS_RING_WIDTH) * 2.;
        label_root(
            div()
                .id("walk-window")
                .key_context("walk")
                .track_focus(&self.focus)
                .on_action(cx.listener(Self::next))
                .on_action(cx.listener(Self::back))
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
                .child(div().flex_1().min_h(px(0.)).children(self.stage.as_ref().map(|stage| stage.view.clone())))
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
                })),
        )
    }
}
