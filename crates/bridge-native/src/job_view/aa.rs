//! Aa, in the toolbelt: the thread's look in a panel, changed live (`ThreadSettings.tsx`, section 1.5).
//!
//! The panel holds no look of its own. It reads `thread.look` through the Job view, every act is a
//! plain write of that setting (drawn as done the same frame, put back with Fleet's reason if Fleet
//! refuses), and the rows of the thread are drawn by it the next frame. What it does keep is what
//! is being typed: the agent's name (written 400 ms after the last key), the new quick reply and
//! a response being edited, which survive the panel closing as drafts.

use std::time::Duration;

use gpui::{
    anchored, deferred, div, prelude::*, svg, Anchor, Animation, AnimationExt, AnyElement, App, ClickEvent, Context, DismissEvent,
    Div, ElementId, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, KeyDownEvent, MouseButton, Pixels, Point, Role,
    ScrollHandle, SharedString, Stateful, Subscription, Task, Toggled, Window,
};
use gpui_component::input::{Escape, Input, InputEvent, InputState};

use super::head::tip;
use super::quick::{act_on_responses, first_one, is_emoji, Kind, Line, Quick, Response, ResponseAct, MAX_RESPONSES, REACTIONS};
use super::JobView;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::whisper::State as Whisper;

/// What the new quick reply's box is kept under, so a line half typed survives the panel closing.
/// Name, switches, selects, responses, quick replies, voice.
const SECTIONS: usize = 6;

const ADDING: &str = "thread-look:new-line";

const TEXT_SIZES: [(&str, &str); 4] = [("xs", "13 px"), ("sm", "14 px"), ("base", "15 px"), ("lg", "18 px")];
const SPACINGS: [(&str, &str); 3] = [("compact", "Compact"), ("default", "Default"), ("roomy", "Roomy")];
const VOICES: [(&str, &str); 3] = [("apple", "Apple on-device"), ("whisper", "Whisper"), ("dictation", "macOS Dictation")];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Pick {
    TextSize,
    Spacing,
    Voice,
}

impl Pick {
    const ALL: [Pick; 3] = [Pick::TextSize, Pick::Spacing, Pick::Voice];

    fn at(self) -> usize {
        Self::ALL.iter().position(|one| *one == self).unwrap_or(0)
    }

    fn label(self) -> &'static str {
        match self {
            Pick::TextSize => "Text size",
            Pick::Spacing => "Spacing",
            Pick::Voice => "Voice",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Pick::TextSize => "textSize",
            Pick::Spacing => "spacing",
            Pick::Voice => "voice",
        }
    }

    fn default(self) -> &'static str {
        match self {
            Pick::TextSize => "base",
            Pick::Spacing => "default",
            Pick::Voice => "apple",
        }
    }

    fn options(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Pick::TextSize => &TEXT_SIZES,
            Pick::Spacing => &SPACINGS,
            Pick::Voice => &VOICES,
        }
    }
}

/// A quick reply being reworded in its row.
struct LineEdit {
    id: String,
    input: Entity<InputState>,
    _subs: Vec<Subscription>,
}

/// A response being edited under its row of buttons, or the blank one that adds.
struct ResponseEdit {
    /// `None` is the blank editor.
    id: Option<String>,
    emoji: Entity<InputState>,
    name: Entity<InputState>,
    /// A name he typed stays; otherwise the name follows the reaction picked.
    typed: bool,
    _subs: Vec<Subscription>,
}

pub(super) struct Aa {
    view: Entity<JobView>,
    job: Option<SharedString>,
    /// The Aa button's top left: the panel opens upward from it.
    at: Point<Pixels>,
    focus: FocusHandle,
    selects: [FocusHandle; 3],
    /// One for each section, so the panel scrolls to the one the keys enter.
    sections: [FocusHandle; SECTIONS],
    scroll: ScrollHandle,
    name: Entity<InputState>,
    adding: Entity<InputState>,
    /// The list of the select that is open, and the option it is on.
    open: Option<(Pick, usize)>,
    line: Option<LineEdit>,
    response: Option<ResponseEdit>,
    whisper: Option<Whisper>,
    /// Voice is drawn only where the screen that mounts the panel asks for it, as `voice` is optional in
    /// `ThreadSettings.tsx`. The composer's Aa asks for it: native has no other place to pick a voice.
    voice: bool,
    /// The look as it was before the agent name's last burst of keys, until it is written.
    unwritten: Option<Option<std::collections::BTreeMap<String, ipc::SettingValue>>>,
    saving: Option<Task<()>>,
    watching: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl EventEmitter<DismissEvent> for Aa {}

impl Focusable for Aa {
    /// The panel takes the focus in its first field as it opens.
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.name.read(cx).focus_handle(cx)
    }
}

fn response_key(id: &Option<String>, field: &str) -> String {
    format!("{ADDING}:response:{}:{field}", id.as_deref().unwrap_or("new"))
}

impl Aa {
    /// The Job view is being updated as the panel is made, so what it holds comes in with the call.
    pub fn new(
        view: Entity<JobView>,
        job: Option<SharedString>,
        at: Point<Pixels>,
        (agent, draft, mocked): (String, String, Option<Whisper>),
        voice: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx));
        name.update(cx, |input, cx| input.set_value(agent, window, cx));
        let adding = cx.new(|cx| InputState::new(window, cx));
        adding.update(cx, |input, cx| input.set_value(draft, window, cx));
        let sections: [FocusHandle; SECTIONS] = std::array::from_fn(|_| cx.focus_handle());
        let mut subs = vec![
            cx.observe(&view, |_, _, cx| cx.notify()),
            cx.subscribe_in(&name, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.name_typed(window, cx);
                }
            }),
            cx.subscribe_in(&adding, window, |this, input, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    let text = input.read(cx).value().to_string();
                    this.view.update(cx, |view, cx| view.keep_text(ADDING, text, cx));
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.add_line(window, cx),
                _ => {}
            }),
        ];
        for (ix, handle) in sections.iter().enumerate() {
            subs.push(cx.on_focus_in(handle, window, move |this, _, cx| {
                this.scroll.scroll_to_item(ix);
                cx.notify();
            }));
        }
        let mut aa = Aa {
            view,
            job,
            at,
            focus: cx.focus_handle(),
            selects: [cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true), cx.focus_handle().tab_stop(true)],
            sections,
            scroll: ScrollHandle::new(),
            name,
            adding,
            open: None,
            line: None,
            response: None,
            whisper: mocked,
            voice,
            unwritten: None,
            saving: None,
            watching: None,
            _subs: Vec::new(),
        };
        aa._subs.append(&mut subs);
        if voice && mocked.is_none() {
            aa.read_whisper(cx);
        }
        aa
    }

    // ---- writing the look ----------------------------------------------------------------

    /// One act on the look: drawn at once, written now.
    fn change(&mut self, window: &mut Window, cx: &mut Context<Self>, act: impl FnOnce(&mut Quick)) {
        self.flush(false, window, cx);
        self.view.update(cx, |view, cx| {
            if !view.composer.quick.loaded() {
                return;
            }
            let was = view.composer.quick.look.clone();
            act(&mut view.composer.quick);
            view.write_look(was, window, cx);
        });
        cx.notify();
    }

    /// The agent's name is written 400 ms after the last key, as Electron saves it.
    fn name_typed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.name.read(cx).value().to_string();
        let moved = self.view.update(cx, |view, _| {
            let quick = &mut view.composer.quick;
            if !quick.loaded() || quick.panel().agent_name == text {
                return None;
            }
            let was = quick.look.clone();
            quick.set_text("agentName", &text);
            Some(was)
        });
        let Some(was) = moved else { return };
        self.unwritten.get_or_insert(was);
        self.saving = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(400)).await;
            let _ = this.update_in(cx, |this, window, cx| this.flush(false, window, cx));
        }));
        cx.notify();
    }

    /// Write what the agent name's keys changed, now. `cancel` also drops the timer that would.
    fn flush(&mut self, cancel: bool, window: &mut Window, cx: &mut Context<Self>) {
        if cancel {
            self.saving = None;
        }
        if let Some(was) = self.unwritten.take() {
            self.view.update(cx, |view, cx| view.write_look(was, window, cx));
        }
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush(true, window, cx);
        cx.emit(DismissEvent);
    }

    /// Esc puts away what is open inside the panel first: a list, an editor, then the panel itself.
    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((which, _)) = self.open.take() {
            window.focus(&self.selects[which.at()], cx);
        } else if self.response.take().is_some() || self.line.take().is_some() {
            window.focus(&self.focus, cx);
        } else {
            self.dismiss(window, cx);
        }
        cx.notify();
    }

    // ---- the selects -----------------------------------------------------------------------

    fn open_select(&mut self, which: Pick, window: &mut Window, cx: &mut Context<Self>) {
        let current = self.view.read(cx).composer.quick.text_or(which.key(), which.default());
        let at = which.options().iter().position(|one| one.0 == current).unwrap_or(0);
        window.focus(&self.selects[which.at()], cx);
        self.open = Some((which, at));
        cx.notify();
    }

    fn pick(&mut self, which: Pick, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.open = None;
        window.focus(&self.selects[which.at()], cx);
        let Some((value, _)) = which.options().get(at).copied() else { return };
        self.change(window, cx, |quick| quick.set_text(which.key(), value));
        if which == Pick::Voice && value == "whisper" {
            self.read_whisper(cx);
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let alt = event.keystroke.modifiers.alt;
        if let Some((which, at)) = self.open {
            let last = which.options().len() - 1;
            match key {
                "down" => self.open = Some((which, (at + 1).min(last))),
                "up" => self.open = Some((which, at.saturating_sub(1))),
                "enter" | "space" | " " => self.pick(which, at, window, cx),
                "escape" => self.escape(window, cx),
                _ => return,
            }
        } else if key == "escape" {
            self.escape(window, cx);
        } else if matches!(key, "down" | "up") && !alt {
            let Some(which) = Pick::ALL.into_iter().find(|one| self.selects[one.at()].is_focused(window)) else { return };
            self.open_select(which, window, cx);
        } else {
            return;
        }
        cx.stop_propagation();
        cx.notify();
    }

    // ---- quick replies -----------------------------------------------------------------------

    fn add_line(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.adding.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.change(window, cx, |quick| {
            let mut lines = quick.kept();
            lines.push(Line { id: Some(format!("saved:{}", chrono::Utc::now().timestamp_millis())), text, kind: Kind::Saved, pinned: false });
            quick.set_kept(lines);
        });
        // Add goes grey with the box empty, so the keys go back to the box for the next line.
        self.adding.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
        self.view.update(cx, |view, cx| view.keep_text(ADDING, String::new(), cx));
    }

    fn move_line(&mut self, id: String, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        self.change(window, cx, move |quick| {
            quick.move_line(&id, by);
        });
    }

    fn act_on(&mut self, id: String, act: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.flush(false, window, cx);
        self.view.update(cx, |view, cx| view.act_on_line(id, act, window, cx));
        cx.notify();
    }

    fn edit_line(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = self.view.read(cx).composer.quick.kept().into_iter().find(|one| one.id.as_deref() == Some(id.as_str())).map(|one| one.text) else {
            return;
        };
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |input, cx| {
            input.set_value(text, window, cx);
            input.select_all(window, cx);
            input.focus(window, cx);
        });
        let saved = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                this.finish_line(true, window, cx);
            }
        });
        self.line = Some(LineEdit { id, input, _subs: vec![saved] });
        cx.notify();
    }

    /// Enter and leaving save, and a line left empty or unchanged is a cancel.
    fn finish_line(&mut self, keep: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.line.take() else { return };
        window.focus(&self.focus, cx);
        let said = edit.input.read(cx).value().to_string();
        if keep {
            self.change(window, cx, move |quick| {
                quick.reword_line(&edit.id, &said);
            });
        }
        cx.notify();
    }

    // ---- quick responses ---------------------------------------------------------------------

    fn open_response(&mut self, id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if self.response.as_ref().is_some_and(|one| one.id == id) {
            self.response = None;
            cx.notify();
            return;
        }
        let was = id.as_ref().and_then(|id| self.view.read(cx).composer.quick.responses().into_iter().find(|one| one.id == *id));
        let (emoji_draft, name_draft) = {
            let view = self.view.read(cx);
            (view.kept_text(&response_key(&id, "emoji")), view.kept_text(&response_key(&id, "name")))
        };
        let emoji_now = if emoji_draft.is_empty() { was.as_ref().map(|one| one.emoji.clone()).unwrap_or_default() } else { emoji_draft };
        let name_now = if name_draft.is_empty() { was.as_ref().map(|one| one.name.clone()).unwrap_or_default() } else { name_draft };
        let emoji = cx.new(|cx| InputState::new(window, cx));
        emoji.update(cx, |input, cx| {
            input.set_value(emoji_now, window, cx);
            input.select_all(window, cx);
            input.focus(window, cx);
        });
        let name = cx.new(|cx| InputState::new(window, cx));
        name.update(cx, |input, cx| input.set_value(name_now, window, cx));
        let key = id.clone();
        let subs = vec![
            cx.subscribe_in(&emoji, window, {
                let key = key.clone();
                move |this, input, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => {
                        let text = input.read(cx).value().to_string();
                        let one = first_one(&text);
                        if one != text {
                            input.update(cx, |input, cx| input.set_value(one.clone(), window, cx));
                        }
                        this.view.update(cx, |view, cx| view.keep_text(&response_key(&key, "emoji"), one, cx));
                        cx.notify();
                    }
                    InputEvent::PressEnter { .. } => this.save_response(window, cx),
                    _ => {}
                }
            }),
            cx.subscribe_in(&name, window, {
                let key = key.clone();
                move |this, input, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => {
                        let text = input.read(cx).value().to_string();
                        if let Some(edit) = this.response.as_mut() {
                            edit.typed = true;
                        }
                        this.view.update(cx, |view, cx| view.keep_text(&response_key(&key, "name"), text, cx));
                        cx.notify();
                    }
                    InputEvent::PressEnter { .. } => this.save_response(window, cx),
                    _ => {}
                }
            }),
        ];
        self.response = Some(ResponseEdit { id, emoji, name, typed: false, _subs: subs });
        cx.notify();
    }

    fn react_with(&mut self, emoji: &str, said: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.response.as_mut() else { return };
        let follow = !edit.typed || edit.name.read(cx).value().trim().is_empty();
        let (emoji_box, name_box) = (edit.emoji.clone(), edit.name.clone());
        emoji_box.update(cx, |input, cx| input.set_value(emoji.to_string(), window, cx));
        if follow {
            name_box.update(cx, |input, cx| input.set_value(said.to_string(), window, cx));
            if let Some(edit) = self.response.as_mut() {
                edit.typed = false;
            }
        }
        cx.notify();
    }

    fn save_response(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.response.as_ref() else { return };
        let (emoji, name) = (edit.emoji.read(cx).value().to_string(), edit.name.read(cx).value().trim().to_string());
        if !is_emoji(&emoji) || name.is_empty() {
            return;
        }
        let id = edit.id.clone();
        self.respond(
            match id.clone() {
                Some(id) => ResponseAct::Edit { id, emoji, name },
                None => ResponseAct::Add { emoji, name },
            },
            window,
            cx,
        );
        self.view.update(cx, |view, cx| {
            view.keep_text(&response_key(&id, "emoji"), String::new(), cx);
            view.keep_text(&response_key(&id, "name"), String::new(), cx);
        });
        self.response = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn respond(&mut self, act: ResponseAct, window: &mut Window, cx: &mut Context<Self>) {
        self.change(window, cx, move |quick| {
            let list = act_on_responses(quick.responses(), act);
            quick.set_responses(list);
        });
    }

    // ---- Whisper -----------------------------------------------------------------------------

    /// Read Whisper's state off the main thread, and keep reading while it comes down.
    fn read_whisper(&mut self, cx: &mut Context<Self>) {
        self.watching = Some(cx.spawn(async move |this, cx| loop {
            let state = cx.background_executor().spawn(async { crate::whisper::state() }).await;
            let more = this.update(cx, |this, cx| {
                if this.whisper != Some(state) {
                    this.whisper = Some(state);
                    cx.notify();
                }
                matches!(state, Whisper::Installing(_))
            });
            if !more.unwrap_or(false) {
                break;
            }
            cx.background_executor().timer(Duration::from_millis(300)).await;
        }));
    }

    fn install_whisper(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The button is replaced by the bar, so the keys wait in the panel.
        window.focus(&self.focus, cx);
        let mocked = self.view.update(cx, |view, _| {
            let quick = &mut view.composer.quick;
            quick.is_mock().then(|| quick.whisper = Some(Whisper::Installing(0.4)))
        });
        if mocked.is_some() {
            self.whisper = Some(Whisper::Installing(0.4));
        } else {
            crate::whisper::install();
            self.read_whisper(cx);
        }
        cx.notify();
    }
}

// ---- drawing -------------------------------------------------------------------------------------

/// A small text button: the quiet one the panel's rows use.
fn button(look: &Look, id: impl Into<ElementId>, said: &str, off: bool, primary: bool) -> Stateful<Div> {
    let on_accent = primary && !off;
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .h(look.length(token::H_CONTROL))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .text_size(look.length(token::TEXT_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(if off {
            look.color(token::FG_SUBTLE).opacity(0.5)
        } else if on_accent {
            look.color(token::TEXT_ON_ACCENT)
        } else {
            look.color(token::FG_MUTED)
        })
        .when(on_accent, |button| button.bg(look.color(token::ACCENT)).hover(|style| style.bg(look.color(token::ACCENT_HOVER))))
        .when(!off && !on_accent, |button| {
            button.hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
        })
        .when(!off, |button| button.cursor_pointer().tab_index(0).focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT))))
        .child(SharedString::from(said.to_string()))
}

/// A switch as Electron's `Switch`: its words, then at the row's end a recessed track with the knob
/// and ON or OFF in the half the knob leaves. Pressing it anywhere turns it.
pub(super) fn switch(look: &Look, id: impl Into<ElementId>, said: &str, on: bool, off: bool) -> Stateful<Div> {
    let edge = look.length(token::BORDER_WIDTH);
    let tall = look.length(token::H_BADGE) + look.length(token::SPACE_2);
    let pad = edge * 3.;
    let knob = tall - pad * 2.;
    let air = look.length(token::SPACE_2);
    let word = div()
        .flex_none()
        .w(look.length(token::SPACE_4) + look.length(token::SPACE_1))
        .flex()
        .justify_center()
        .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
        .text_size(look.length(token::TEXT_SWITCH))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(if on && !off { look.color(token::FG_INVERSE) } else { look.color(token::FG_MUTED) })
        .child(if on { "ON" } else { "OFF" });
    let disc = div()
        .flex_none()
        .size(knob)
        .rounded_full()
        .border(edge * 2.)
        .border_color(if on && !off { look.color(token::ACCENT).opacity(0.6) } else { look.color(token::BORDER_STRONG) })
        .bg(if on { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) });
    let lit = on && !off;
    let track = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(air)
        .h(tall)
        .pl(pad - edge)
        .pr(pad - edge)
        .border(edge)
        .border_color(if off { look.color(token::BORDER_SUBTLE) } else if lit { look.color(token::ACCENT) } else { look.color(token::BORDER_STRONG) })
        .rounded(tall / 2.)
        .bg(if lit { look.color(token::ACCENT) } else { look.color(token::BG_SUNKEN) })
        .map(|track| if on { track.pl(air).child(word).child(disc) } else { track.pr(air).child(disc).child(word) });
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .gap(look.length(token::SPACE_3))
        .min_h(look.length(token::H_CONTROL))
        .text_size(look.length(token::TEXT_SM))
        .text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) })
        .aria_toggled(if on { Toggled::True } else { Toggled::False })
        .when(!off, |row| row.cursor_pointer().tab_index(0).focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT))))
        .child(div().min_w_0().child(SharedString::from(said.to_string())))
        .child(track)
}

/// A heading over a group of the panel's controls.
fn heading(look: &Look, said: &str) -> Div {
    div()
        .text_size(look.length(token::TEXT_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
        .text_color(look.color(token::FG_MUTED))
        .child(SharedString::from(said.to_string()))
}

/// A single-line text box with its words over it, the box under the Input key context.
fn field(look: &Look, said: &str, input: &Entity<InputState>, id: &'static str) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.to_string())))
        .child(boxed(look, said, input, id))
        .into_any_element()
}

fn boxed(look: &Look, said: &str, input: &Entity<InputState>, id: &'static str) -> AnyElement {
    labelled(
        div()
            .id(id)
            .key_context(crate::keymap::TEXT_BOX)
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .h(look.length(token::H_CONTROL_LG))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(Input::new(input).appearance(false).cleanable(false)),
        Role::TextInput,
        SharedString::from(said.to_string()),
    )
    .into_any_element()
}

impl Aa {
    fn select(&self, look: &Look, which: Pick, current: &str, cx: &Context<Self>) -> AnyElement {
        let said = which.label();
        let options = which.options();
        let shown = options.iter().find(|one| one.0 == current).map_or(current, |one| one.1);
        let open = self.open.filter(|(open, _)| *open == which).map(|(_, at)| at);
        let trigger = labelled(
            div()
                .id(SharedString::from(format!("select:{said}")))
                .track_focus(&self.selects[which.at()])
                .flex()
                .items_center()
                .justify_between()
                .h(look.length(token::H_CONTROL_LG))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .cursor_pointer()
                .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                .aria_expanded(open.is_some())
                .aria_value(shown.to_string())
                .child(shown.to_string())
                .child(svg().path(icon_path("chevron-down")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if this.open.is_some_and(|(open, _)| open == which) {
                        this.open = None;
                        cx.notify();
                    } else {
                        this.open_select(which, window, cx);
                    }
                })),
            Role::ComboBox,
            said,
        );
        let list = open.map(|highlight| {
            labelled(
                div()
                    .id(SharedString::from(format!("options:{said}")))
                    .flex()
                    .flex_col()
                    .p(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_RAISED))
                    .children(options.iter().enumerate().map(|(at, (value, label))| {
                        labelled(
                            div()
                                .id(SharedString::from(format!("option:{said}:{value}")))
                                .flex()
                                .items_center()
                                .justify_between()
                                .h(look.length(token::H_CONTROL))
                                .px(look.length(token::SPACE_2))
                                .rounded(look.length(token::RADIUS_SM))
                                .text_size(look.length(token::TEXT_SM))
                                .text_color(look.color(token::FG_DEFAULT))
                                .cursor_pointer()
                                .when(at == highlight, |row| row.bg(look.color(token::BG_HOVER)))
                                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                                .aria_selected(*value == current)
                                .child(label.to_string())
                                .when(*value == current, |row| {
                                    row.child(svg().path(icon_path("check")).size(look.length(token::SPACE_3)).text_color(look.color(token::ACCENT)))
                                })
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.pick(which, at, window, cx))),
                            Role::ListBoxOption,
                            label.to_string(),
                        )
                    })),
                Role::ListBox,
                format!("{said} options"),
            )
        });
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(said))
            .child(trigger)
            .children(list)
            .into_any_element()
    }

    fn responses_section(&self, look: &Look, responses: &[Response], cx: &Context<Self>) -> AnyElement {
        let full = responses.len() >= MAX_RESPONSES;
        let editing = self.response.as_ref().map(|one| one.id.clone());
        let buttons = responses.iter().map(|one| {
            let (id, name) = (one.id.clone(), one.name.clone());
            let (press, left, right) = (id.clone(), id.clone(), id.clone());
            let open = editing == Some(Some(id.clone()));
            labelled(
                div()
                    .id(SharedString::from(format!("response:{id}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL_LG))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_BASE))
                    .cursor_pointer()
                    .tab_index(0)
                    .when(open, |button| button.bg(look.color(token::BG_HOVER)))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                    .aria_expanded(open)
                    .aria_keyshortcuts("Alt+ArrowLeft Alt+ArrowRight")
                    .child(one.emoji.clone())
                    .tooltip(tip(name.clone().into()))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.open_response(Some(press.clone()), window, cx)))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if !event.keystroke.modifiers.alt {
                            return;
                        }
                        let by = match event.keystroke.key.as_str() {
                            "left" => -1,
                            "right" => 1,
                            _ => return,
                        };
                        cx.stop_propagation();
                        let id = if by < 0 { left.clone() } else { right.clone() };
                        this.respond(ResponseAct::Move { id, by }, window, cx);
                    })),
                Role::Button,
                format!("Edit {name}"),
            )
        });
        let add = labelled(
            div()
                .id("response:new")
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_LG))
                .rounded(look.length(token::RADIUS_MD))
                .text_color(if full { look.color(token::FG_SUBTLE).opacity(0.5) } else { look.color(token::FG_MUTED) })
                .when(!full, |button| {
                    button
                        .cursor_pointer()
                        .tab_index(0)
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.open_response(None, window, cx)))
                })
                .aria_expanded(editing == Some(None))
                .child(svg().path(icon_path("plus")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)))
                .tooltip(tip("Add response".into())),
            Role::Button,
            "Add response",
        );
        let editor = self.response.as_ref().map(|edit| self.response_editor(look, edit, responses.len() > 1, cx));
        labelled(
            div()
                .id("section:responses")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(heading(look, "Quick responses"))
                .child(div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_1)).children(buttons).child(add))
                .children(editor),
            Role::Group,
            "Quick responses",
        )
        .into_any_element()
    }

    fn response_editor(&self, look: &Look, edit: &ResponseEdit, removable: bool, cx: &Context<Self>) -> AnyElement {
        let (emoji, name) = (edit.emoji.read(cx).value().to_string(), edit.name.read(cx).value().trim().to_string());
        let valid = is_emoji(&emoji) && !name.is_empty();
        let id = edit.id.clone();
        let grid = labelled(
            div().id("reactions").flex().flex_wrap().gap(look.length(token::SPACE_1)).children(REACTIONS.iter().map(|(one, said)| {
                let on = emoji == *one;
                labelled(
                    div()
                        .id(SharedString::from(format!("reaction:{said}")))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(look.length(token::H_CONTROL))
                        .rounded(look.length(token::RADIUS_MD))
                        .text_size(look.length(token::TEXT_BASE))
                        .cursor_pointer()
                        .tab_index(0)
                        .when(on, |button| button.bg(look.color(token::BG_HOVER)).border_1().border_color(look.color(token::ACCENT)))
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                        .aria_toggled(if on { Toggled::True } else { Toggled::False })
                        .child(one.to_string())
                        .tooltip(tip(said.to_string().into()))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.react_with(one, said, window, cx))),
                    Role::Button,
                    said.to_string(),
                )
            })),
            Role::Group,
            "Reactions",
        );
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(grid)
            .child(
                div()
                    .flex()
                    .gap(look.length(token::SPACE_3))
                    .child(div().w(look.length(token::SPACE_12) * 2.).child(field(look, "Emoji", &edit.emoji, "response-emoji")))
                    .child(div().flex_1().min_w_0().child(field(look, "Name", &edit.name, "response-name"))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .when(removable && id.is_some(), |row| {
                        let id = id.clone().unwrap_or_default();
                        row.child(labelled(
                            button(look, "response-remove", "Remove", false, false).on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.respond(ResponseAct::Remove(id.clone()), window, cx);
                                this.response = None;
                                window.focus(&this.focus, cx);
                                cx.notify();
                            })),
                            Role::Button,
                            "Remove",
                        ))
                    })
                    .child(div().flex_1())
                    .child(labelled(
                        button(look, "response-cancel", "Cancel", false, false).on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.response = None;
                            window.focus(&this.focus, cx);
                            cx.notify();
                        })),
                        Role::Button,
                        "Cancel",
                    ))
                    .child(labelled(
                        button(look, "response-save", "Save", !valid, true).when(valid, |save| {
                            save.on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.save_response(window, cx)))
                        }),
                        Role::Button,
                        "Save",
                    )),
            )
            .into_any_element()
    }

    /// One quick reply: its marks, its words, and its acts in fixed columns.
    fn reply_row(&self, look: &Look, line: &Line, at: usize, last: usize, cx: &Context<Self>) -> AnyElement {
        // A row keeps its keys with the line when it moves, so its elements are named by the line.
        let place = line.id.clone().unwrap_or_else(|| line.text.clone());
        let (glyph, word) = match line.kind {
            Kind::Learnt => ("repeat", "Learnt"),
            _ => ("message-square-quote", "Saved"),
        };
        let marks = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .child(
                div()
                    .id(SharedString::from(format!("reply-kind:{place}")))
                    .child(svg().path(icon_path(glyph)).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
                    .tooltip(tip(word.into())),
            )
            .when(line.pinned, |marks| {
                marks.child(
                    div()
                        .id(SharedString::from(format!("reply-pin:{place}")))
                        .child(svg().path(icon_path("arrow-up-to-line")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
                        .tooltip(tip("Pinned".into())),
                )
            });
        let editing = self.line.as_ref().zip(line.id.as_ref()).filter(|(edit, id)| edit.id == **id).map(|(edit, _)| edit);
        let body: AnyElement = match editing {
            Some(edit) => div()
                .flex_1()
                .min_w_0()
                .child(boxed(look, &format!("Edit {}", line.text), &edit.input, "reply-edit"))
                .into_any_element(),
            None => div().flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(line.text.clone()).into_any_element(),
        };
        let cell = |content: Option<AnyElement>| div().flex_none().children(content);
        let acts = (editing.is_none()).then(|| {
            let text = line.text.clone();
            match line.id.clone() {
                // A line Fleet learnt is read and kept as it is; Save makes it his own.
                None => {
                    let said = line.text.clone();
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .child(cell(Some(
                            labelled(
                                button(look, SharedString::from(format!("reply-save:{place}")), "Save", false, false).on_click(cx.listener(
                                    move |this, _: &ClickEvent, window, cx| {
                                        let said = said.clone();
                                        this.view.update(cx, |view, cx| view.save_line(said, window, cx));
                                    },
                                )),
                                Role::Button,
                                format!("Save {text}"),
                            )
                            .into_any_element(),
                        )))
                        .into_any_element()
                }
                Some(id) => {
                    let act = |key: &'static str, name: &'static str, said: String, off: bool, run: Box<dyn Fn(&mut Aa, &mut Window, &mut Context<Aa>)>| {
                        cell(Some(
                            labelled(
                                button(look, SharedString::from(format!("reply-{key}:{place}")), name, off, false).when(!off, |button| {
                                    button.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| run(this, window, cx)))
                                }),
                                Role::Button,
                                said,
                            )
                            .into_any_element(),
                        ))
                    };
                    let (edit_id, pin_id, up_id, down_id, delete_id) = (id.clone(), id.clone(), id.clone(), id.clone(), id.clone());
                    let pinned = line.pinned;
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .child(act("edit", "Edit", format!("Edit {text}"), false, Box::new(move |this, window, cx| this.edit_line(edit_id.clone(), window, cx))))
                        .child(act(
                            "pin",
                            if pinned { "Unpin" } else { "Pin" },
                            format!("{} {text}", if pinned { "Unpin" } else { "Pin" }),
                            false,
                            Box::new(move |this, window, cx| this.act_on(pin_id.clone(), super::quick::PIN, window, cx)),
                        ))
                        .child(act("up", "Up", format!("Move {text} up"), at == 0, Box::new(move |this, window, cx| this.move_line(up_id.clone(), -1, window, cx))))
                        .child(act("down", "Down", format!("Move {text} down"), at == last, Box::new(move |this, window, cx| this.move_line(down_id.clone(), 1, window, cx))))
                        .child(act("delete", "Delete", format!("Delete {text}"), false, Box::new(move |this, window, cx| this.act_on(delete_id.clone(), super::quick::DELETE, window, cx))))
                        .into_any_element()
                }
            }
        });
        let line_id = line.id.clone();
        // The acts lie over the row's end and show while it is pointed at or holds the keys, as Electron's.
        let group = SharedString::from(format!("reply-row:{place}"));
        let lit = look.color(token::BG_HOVER);
        let acts = acts.map(|acts| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right(look.length(token::SPACE_1))
                .flex()
                .items_center()
                .pl(look.length(token::SPACE_2))
                .bg(look.color(token::BG_HOVER))
                .opacity(0.)
                .group_hover(group.clone(), |style| style.opacity(1.))
                .child(acts)
        });
        div()
            .id(SharedString::from(format!("reply:{place}")))
            .group(group)
            .relative()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_h(look.length(token::H_ROW))
            .px(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .hover(move |style| style.bg(lit))
            .child(marks)
            .child(body)
            .children(acts)
            // ⌥↑ and ⌥↓ move a line from anywhere on its row.
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                let (Some(id), true) = (line_id.clone(), event.keystroke.modifiers.alt) else { return };
                let by = match event.keystroke.key.as_str() {
                    "up" => -1,
                    "down" => 1,
                    _ => return,
                };
                cx.stop_propagation();
                this.move_line(id, by, window, cx);
            }))
            .into_any_element()
    }

    fn whisper_row(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let state = self.whisper?;
        let name = div().flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child("Whisper model");
        let status: AnyElement = match state {
            Whisper::Absent => labelled(
                button(look, "whisper-install", "Install", false, false)
                    .border_1()
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.install_whisper(window, cx))),
                Role::Button,
                "Install",
            )
            .into_any_element(),
            Whisper::Installing(progress) => {
                let percent = (progress.clamp(0., 1.) * 100.).round();
                labelled(
                    div()
                        .id("whisper-progress")
                        .flex_none()
                        .w(look.length(token::SPACE_12) * 3.)
                        .h(look.length(token::SPACE_2))
                        .rounded_full()
                        .overflow_hidden()
                        .bg(look.color(token::FG_DEFAULT).opacity(0.12))
                        .aria_min_numeric_value(0.)
                        .aria_max_numeric_value(100.)
                        .aria_numeric_value(percent as f64)
                        .child(div().h_full().w(gpui::relative(percent as f32 / 100.)).rounded_full().bg(look.color(token::ACCENT))),
                    Role::ProgressIndicator,
                    "Installing Whisper model",
                )
                .into_any_element()
            }
            Whisper::Installed => div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child("Installed").into_any_element(),
        };
        Some(
            labelled(
                div().id("whisper-model").flex().items_center().gap(look.length(token::SPACE_3)).min_h(look.length(token::H_CONTROL)).child(name).child(status),
                Role::Group,
                "Whisper model",
            )
            .into_any_element(),
        )
    }

    /// Shapes in the places of the controls, while Fleet has not answered for the look.
    fn skeleton(look: &Look) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .children((0..5).map(|at| {
                div()
                    .h(look.length(token::H_CONTROL_LG))
                    .w(gpui::relative(0.4 + 0.12 * at as f32))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::FG_DEFAULT).opacity(0.07))
                    .with_animation(
                        SharedString::from(format!("aa-skeleton-{at}")),
                        Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                        |shape, at| shape.opacity(at),
                    )
            }))
            .into_any_element()
    }
}

impl Render for Aa {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let (panel, kept, learnt, loaded, this_job) = {
            let quick = &self.view.read(cx).composer.quick;
            let this_job = self.job.as_ref().map(|job| quick.shown_in(job));
            (quick.panel(), quick.kept(), quick.learnt_lines(), quick.loaded(), this_job)
        };
        let room = self.at.y - look.length(token::SPACE_8);
        let _ = window;
        let body: Vec<AnyElement> = if !loaded {
            vec![Self::skeleton(look)]
        } else {
            let lines: Vec<Line> = kept.into_iter().chain(learnt).collect();
            let last = lines.len().saturating_sub(1);
            let job = self.job.clone();
            let quick_section = labelled(
                div()
                    .id("section:replies")
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .child(heading(look, "Quick replies"))
                    .children(this_job.zip(job).map(|(on, job)| {
                        labelled(
                            switch(look, "switch:this-job", "In this Job", on && !panel.quick_off, panel.quick_off).when(!panel.quick_off, |row| {
                                row.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    let job = job.clone();
                                    this.change(window, cx, move |quick| quick.set_shown_in(&job, !on));
                                }))
                            }),
                            Role::Switch,
                            "In this Job",
                        )
                    }))
                    .child(labelled(
                        switch(look, "switch:all-jobs", "In all Jobs", !panel.quick_off, false).on_click(cx.listener({
                            let off = panel.quick_off;
                            move |this, _: &ClickEvent, window, cx| this.change(window, cx, move |quick| quick.set_flag("quickOff", !off))
                        })),
                        Role::Switch,
                        "In all Jobs",
                    ))
                    .child(labelled(
                        div()
                            .id("replies")
                            .flex()
                            .flex_col()
                            .children(lines.iter().enumerate().map(|(at, line)| self.reply_row(look, line, at, last, cx))),
                        Role::List,
                        "Quick replies",
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .child(boxed(look, "New quick reply", &self.adding, "new-reply"))
                            .child(labelled(
                                button(look, "reply-add", "Add", self.adding.read(cx).value().trim().is_empty(), false)
                                    .border_1()
                                    .border_color(look.color(token::BORDER_DEFAULT))
                                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.add_line(window, cx))),
                                Role::Button,
                                "Add",
                            )),
                    )
                    .child(labelled(
                        switch(look, "switch:learn", "Learn from my messages", panel.learn, false).on_click(cx.listener({
                            let learn = panel.learn;
                            move |this, _: &ClickEvent, window, cx| this.change(window, cx, move |quick| quick.set_flag("learn", !learn))
                        })),
                        Role::Switch,
                        "Learn from my messages",
                    )),
                Role::Group,
                "Quick replies",
            );
            let voice = labelled(
                div()
                    .id("section:voice")
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .child(self.select(look, Pick::Voice, &panel.voice, cx))
                    .children((panel.voice == "whisper").then(|| self.whisper_row(look, cx)).flatten()),
                Role::Group,
                "Voice",
            );
            let size_key = self.view.read(cx).composer.quick.text_or("textSize", "base");
            let spacing_key = self.view.read(cx).composer.quick.text_or("spacing", "default");
            let switches = div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .child(labelled(
                    switch(look, "switch:agent-name", "Show agent name", panel.show_agent_name, false).on_click(cx.listener({
                        let on = panel.show_agent_name;
                        move |this, _: &ClickEvent, window, cx| this.change(window, cx, move |quick| quick.set_flag("showAgentName", !on))
                    })),
                    Role::Switch,
                    "Show agent name",
                ))
                .child(labelled(
                    switch(look, "switch:times", "Show times", panel.thread.show_times, false).on_click(cx.listener({
                        let on = panel.thread.show_times;
                        move |this, _: &ClickEvent, window, cx| this.change(window, cx, move |quick| quick.set_flag("showTimes", !on))
                    })),
                    Role::Switch,
                    "Show times",
                ));
            let selects = div()
                .flex()
                .gap(look.length(token::SPACE_3))
                .child(self.select(look, Pick::TextSize, &size_key, cx))
                .child(self.select(look, Pick::Spacing, &spacing_key, cx));
            vec![
                field(look, "Agent name", &self.name, "agent-name"),
                switches.into_any_element(),
                selects.into_any_element(),
                self.responses_section(look, &panel.responses, cx),
                quick_section.into_any_element(),
                voice.into_any_element(),
            ]
            .into_iter()
            .enumerate()
            .filter(|(ix, _)| self.voice || *ix + 1 < SECTIONS)
            .map(|(_, part)| part)
            .collect()
        };
        deferred(
            anchored().position(self.at).anchor(Anchor::BottomLeft).snap_to_window_with_margin(look.length(token::SPACE_1)).child(labelled(
                div()
                    .id("thread-look")
                    .track_focus(&self.focus)
                    .capture_key_down(cx.listener(Self::on_key))
                    .capture_action(cx.listener(|this, _: &Escape, window, cx| {
                        this.escape(window, cx);
                        cx.stop_propagation();
                    }))
                    .on_mouse_down_out(cx.listener(|this, _, window, cx| this.dismiss(window, cx)))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .occlude()
                    .mb(look.length(token::SPACE_2))
                    .w(look.length(token::SPACE_12) * 7. + look.length(token::SPACE_4) * 2.)
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_OVERLAY))
                    .shadow(look.shadows(token::SHADOW_OVERLAY))
                    .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(
                        div()
                            .id("thread-look-scroll")
                            .track_scroll(&self.scroll)
                            .max_h(room)
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_3))
                            .p(look.length(token::SPACE_4))
                            .children(body.into_iter().enumerate().map(|(ix, part)| {
                                div().id(SharedString::from(format!("aa-section:{ix}"))).track_focus(&self.sections[ix.min(SECTIONS - 1)]).child(part)
                            })),
                    ),
                Role::Dialog,
                "Thread look",
            )),
        )
        .with_priority(1)
    }
}
