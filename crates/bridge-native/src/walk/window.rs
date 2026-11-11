//! The walk window: the walk bar over the screen, with the step's target ringed.
//!
//! Reaching a step plays it (a press, a key) and rings its target. Back plays the walk again from
//! a fresh screen up to the step before, so a step always shows what its steps before it made.
//! A note is held on the window until Send notes, which sends every held note as one turn, as
//! Electron's bar does; Approve takes its place while any are held. Approve, the notes and the
//! window closing go to Fleet through [`super::fleet`]; opened without a Job, nothing is sent and
//! the bar says so.

use gpui::{
    actions, div, prelude::*, px, AnyElement, App, Context, Entity, FocusHandle, FontWeight, KeyBinding, Role,
    SharedString, Subscription, Task, Window,
};
use std::cell::Cell;
use std::rc::Rc;

use gpui::{canvas, point, Anchor, AnimationExt as _, Focusable, Bounds, DismissEvent, Pixels};

use super::bar::{chord, icon_button, Weight};
use crate::capture::{self, ringed, Capture, CaptureEvent, Held, Words};
use crate::dropdown::{Dropdown, Entry, Picked};
use super::fleet::WalkFleet;
use super::stage::{find, gone, perform, Stage};
use super::{Verb, Walk};
use crate::labels::{label_root, labelled};
use crate::look::{token, ActiveLook};

actions!(walk, [Next, Back, Reload, Approve, TakeNote, CancelNote, OpenTakes]);

/// The bar's keys. **Electron's walk bar binds these in `CaptureBar.tsx` (`QUEUE_KEYS`)**, not in
/// the action registry, so they are restated here until the registry carries them.
pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("alt-cmd-right", Next, Some("walk")),
        KeyBinding::new("alt-cmd-left", Back, Some("walk")),
        KeyBinding::new("cmd-enter", Approve, Some("walk")),
        KeyBinding::new("cmd-r", Reload, Some("walk")),
        KeyBinding::new("alt-cmd-a", TakeNote, Some("walk")),
        KeyBinding::new("alt-cmd-t", OpenTakes, Some("walk")),
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

/// One take of a walk with variations: what the bar calls it, and the walk it plays.
pub struct Take {
    pub name: SharedString,
    pub walk: &'static Walk,
}

/// The takes the command line named, each `(name, walk)`; one naming no walk here is left out
/// and said in the log, so the rest still walk.
pub fn takes_of(named: &[(String, String)]) -> Vec<Take> {
    named
        .iter()
        .filter_map(|(name, walk)| match super::named(walk) {
            Some(walk) => Some(Take { name: name.clone().into(), walk }),
            None => {
                crate::log::Note::warn("no walk by that name for a take").field("walk", walk.as_str()).emit();
                None
            }
        })
        .collect()
}

/// A take's letter, as Electron's bar gives it (`variationLetter`): A, B, C.
fn letter(at: usize) -> String {
    char::from(b'A' + (at % 26) as u8).to_string()
}

/// The ids the takes menu answers to: a menu entry's id lives as long as the app.
const TAKE: [&str; 12] = ["take-1", "take-2", "take-3", "take-4", "take-5", "take-6", "take-7", "take-8", "take-9", "take-10", "take-11", "take-12"];

/// A note written and not sent yet.
#[derive(Clone)]
struct HeldNote {
    said: String,
    /// The take, walk and length it was written on: a switch of take leaves it naming its own.
    take: Option<super::fleet::Taken>,
    walk: &'static str,
    of: u32,
    step: u32,
    role: String,
    name: String,
    ring: Bounds<Pixels>,
    /// Shared, so a send can hand the notes back if Fleet refuses them without a copy.
    picture: Option<(std::sync::Arc<image::RgbaImage>, f32)>,
}

/// The ids the notes menu's removals answer to: a menu entry's id lives as long as the app.
const REMOVE: [&str; 12] =
    ["remove-1", "remove-2", "remove-3", "remove-4", "remove-5", "remove-6", "remove-7", "remove-8", "remove-9", "remove-10", "remove-11", "remove-12"];

pub struct WalkWindow {
    walk: &'static Walk,
    /// The walk's takes, and the one on screen. Empty: one walk.
    takes: Vec<Take>,
    take: usize,
    takes_at: Rc<Cell<Option<Bounds<Pixels>>>>,
    stage: Option<Stage>,
    /// The step on screen, from 0.
    at: usize,
    playing: bool,
    fleet: Option<WalkFleet>,
    /// Capture while it is armed over the screen.
    capture: Option<(Entity<Capture>, Subscription)>,
    held: Vec<HeldNote>,
    /// Notes on their way to Fleet: they left the bar the moment Send notes was pressed.
    sending: bool,
    /// The menu beside Send notes, while it is open.
    menu: Option<(Entity<Dropdown>, Subscription, Subscription)>,
    caret_at: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Where the screen is drawn in the window, for what Capture may hold.
    stage_at: Rc<Cell<Option<Bounds<Pixels>>>>,
    said: Option<Said>,
    focus: FocusHandle,
    /// What had the keys before Capture or the notes menu took them, so they go back there and a
    /// screen's own keys, such as `/`, work again.
    keys_were: Option<FocusHandle>,
    _play: Option<Task<()>>,
}

impl WalkWindow {
    pub fn new(walk: &'static Walk, fleet: Option<WalkFleet>, takes: Vec<Take>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this =
            WalkWindow {
            walk,
            take: takes.iter().position(|take| std::ptr::eq(take.walk, walk)).unwrap_or(0),
            takes,
            takes_at: Rc::default(),
            stage: None,
            at: 0,
            playing: false,
            fleet,
            capture: None,
            held: Vec::new(),
            sending: false,
            menu: None,
            caret_at: Rc::default(),
            stage_at: Rc::default(),
            said: None,
            focus: cx.focus_handle(),
            keys_were: None,
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
                let step = walk.steps[at];
                if step.verb == Verb::Fresh {
                    // A part of a long walk starts on a screen built again, with nothing of the last part open.
                    let rebuilt = this.update_in(cx, |this, window, cx| match Stage::new(walk, window, cx) {
                        Ok(stage) => {
                            this.stage = Some(stage);
                            cx.notify();
                            Ok(())
                        }
                        Err(why) => Err(why),
                    });
                    if let Ok(Err(why)) = rebuilt {
                        let _ = this.update(cx, |this, cx| {
                            this.said = Some(Said { text: format!("Step {}: {why}", at + 1).into(), fault: true });
                            this.playing = false;
                            cx.notify();
                        });
                        return;
                    }
                }
                let Ok(Some(focus)) = this.read_with(cx, |this, _| this.stage.as_ref().map(|stage| stage.focus.clone())) else { return };
                let person = step.verb == Verb::NoteKey;
                if person {
                    let _ = this.update(cx, |this, _| this.playing = false);
                }
                let done = cx.update(|window, cx| {
                    window.draw(cx).clear(cx);
                    perform(&step, &focus, window, cx)
                });
                if person {
                    let _ = this.update(cx, |this, _| this.playing = true);
                }
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

    /// ⌘↵: Send notes while any are held, else Approve. Nothing while notes are on their way, so an
    /// approval never reaches the Job before them.
    fn approve(&mut self, _: &Approve, _window: &mut Window, cx: &mut Context<Self>) {
        if self.sending {
            return;
        }
        if self.held.is_empty() {
            self.approve_only(cx);
        } else {
            self.send_notes(false, cx);
        }
    }

    fn fleet_or_say(&mut self, cx: &mut Context<Self>) -> Option<WalkFleet> {
        if self.fleet.is_none() {
            self.said = Some(Said { text: "Not sent: no Job opened this walk".into(), fault: false });
            cx.notify();
        }
        self.fleet.clone()
    }

    /// Approve, leaving any held notes held.
    fn approve_only(&mut self, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet_or_say(cx) else { return };
        let take = self.taken();
        self.send(cx, "Approved", |_| {}, |_| {}, move || fleet.approve(take.as_ref()));
    }

    /// Every held note as one turn, each with the window's picture, its area outlined and
    /// numbered by its place among them; then Approve where `and_approve`.
    fn send_notes(&mut self, and_approve: bool, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet_or_say(cx) else { return };
        let look = cx.look();
        let bytes = |token| {
            let rgba = look.color(token).to_rgb();
            [rgba.r, rgba.g, rgba.b, rgba.a].map(|part| (part * 255.).round() as u8)
        };
        let (ink, on_ink) = (bytes(token::ACCENT), bytes(token::TEXT_ON_ACCENT));
        if self.sending || self.held.is_empty() {
            return;
        }
        // The notes leave the bar now, and come back in front of any written since if Fleet
        // refuses them.
        let held = std::mem::take(&mut self.held);
        let back = held.clone();
        self.sending = true;
        self.said = Some(Said { text: if held.len() == 1 { "Sending the note".into() } else { "Sending the notes".into() }, fault: false });
        cx.notify();
        let done = if and_approve { "Notes sent and approved" } else { "Notes sent" };
        let take = self.taken();
        let sent = |this: &mut Self| this.sending = false;
        let refused = move |this: &mut Self| {
            this.sending = false;
            this.held.splice(0..0, back);
        };
        self.send(cx, done, sent, refused, move || {
            let notes = held
                .into_iter()
                .enumerate()
                .map(|(at, note)| super::fleet::NoteOn {
                    picture: note.picture.and_then(|(picture, scale)| ringed(&picture, note.ring, scale, at + 1, ink, on_ink)),
                    said: note.said,
                    take: note.take,
                    walk: note.walk,
                    step: note.step,
                    of: note.of,
                    role: note.role,
                    name: note.name,
                })
                .collect();
            fleet.notes(notes)?;
            if and_approve {
                fleet.approve(take.as_ref())?;
            }
            Ok(())
        });
    }

    /// The take on screen, as Approve and a note name it. `None` for a walk with no takes.
    fn taken(&self) -> Option<super::fleet::Taken> {
        self.takes.get(self.take).map(|take| (letter(self.take), take.name.to_string()))
    }

    /// Put another take on screen, from its first step, as Electron's bar swaps the page.
    fn pick_take(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(take) = self.takes.get(at) else { return };
        if at == self.take {
            return;
        }
        self.take = at;
        self.walk = take.walk;
        self.capture = None;
        self.said = None;
        self.at = 0;
        self.play_to(0, true, window, cx);
    }

    /// ⌥⌘T, or a press on the takes: the menu of takes under its trigger, the one on screen ticked.
    fn open_takes(&mut self, _: &OpenTakes, window: &mut Window, cx: &mut Context<Self>) {
        if self.takes.len() < 2 || self.menu.is_some() {
            return;
        }
        let Some(bounds) = self.takes_at.get() else { return };
        let entries = self
            .takes
            .iter()
            .enumerate()
            .take(TAKE.len())
            .map(|(at, take)| Entry::Choice { id: TAKE[at], label: format!("{} {}", letter(at), take.name).into(), on: at == self.take })
            .collect();
        let at = point(bounds.origin.x, bounds.origin.y + bounds.size.height + cx.look().length(token::SPACE_1));
        let menu = cx.new(|cx| Dropdown::new("Takes", entries, at, Anchor::TopLeft, false, cx));
        let picked = cx.subscribe_in(&menu, window, |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.menu = None;
            this.give_keys_back(window, cx);
            if let Some(at) = TAKE.iter().position(|one| one == id) {
                this.pick_take(at, window, cx);
            }
            cx.notify();
        });
        let dismissed = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            this.give_keys_back(window, cx);
            cx.notify();
        });
        self.keys_were = window.focused(cx).or(self.keys_were.take());
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some((menu, picked, dismissed));
        cx.notify();
    }

    /// The takes' trigger, as Electron's `TakesMenu`: the letter and name on screen and a caret.
    fn takes_trigger(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let take = self.takes.get(self.take).filter(|_| self.takes.len() > 1)?;
        let look = cx.look();
        let named = format!("{} {}", letter(self.take), take.name);
        let takes_at = Rc::clone(&self.takes_at);
        Some(labelled(
            div()
                .id("walk-takes")
                .relative()
                .flex_none()
                .h(look.length(token::H_CONTROL_SM))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(canvas(move |bounds, _, _| takes_at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .child(named.clone())
                .child(gpui::svg().path(crate::assets::icon_path("chevron-down")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
                .tooltip(|window, cx| gpui_component::tooltip::Tooltip::new("Takes  ⌥⌘T").build(window, cx))
                .on_click(cx.listener(|this, _, window, cx| this.open_takes(&OpenTakes, window, cx))),
            Role::Button,
            format!("Takes, {named}"),
        ))
    }

    /// The menu beside Send notes: send and approve, approve alone, or take a held note back.
    fn open_notes_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(bounds) = self.caret_at.get() else { return };
        let mut entries = vec![
            Entry::Act { id: "send-approve", label: "Send notes and approve".into(), icon: Some("send"), danger: false },
            Entry::Act { id: "approve", label: "Approve without notes".into(), icon: Some("check"), danger: false },
        ];
        if !self.held.is_empty() {
            entries.push(Entry::Rule);
        }
        for (at, note) in self.held.iter().enumerate().take(REMOVE.len()) {
            let cut: String = note.said.chars().take(32).collect();
            let more = if note.said.chars().count() > 32 { "…" } else { "" };
            entries.push(Entry::Act { id: REMOVE[at], label: format!("Remove {}. {cut}{more}", at + 1).into(), icon: Some("x"), danger: true });
        }
        let at = point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height + cx.look().length(token::SPACE_1));
        let menu = cx.new(|cx| Dropdown::new("Notes", entries, at, Anchor::TopRight, false, cx));
        let picked = cx.subscribe(&menu, |this, _, picked: &Picked, cx| {
            let Picked::Entry { id, .. } = picked;
            match *id {
                "send-approve" => this.send_notes(true, cx),
                "approve" => this.approve_only(cx),
                id => {
                    if let Some(at) = REMOVE.iter().position(|one| *one == id).filter(|at| *at < this.held.len()) {
                        this.held.remove(at);
                        this.said = Some(Said { text: format!("Note {} removed", at + 1).into(), fault: false });
                    }
                }
            }
            this.menu = None;
            cx.notify();
        });
        let dismissed = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            this.give_keys_back(window, cx);
            cx.notify();
        });
        self.keys_were = window.focused(cx).or(self.keys_were.take());
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some((menu, picked, dismissed));
        cx.notify();
    }

    /// ⌥⌘A arms Capture, as in Electron, and again puts it away.
    fn take_note(&mut self, _: &TakeNote, window: &mut Window, cx: &mut Context<Self>) {
        if self.capture.is_some() {
            self.cancel_note(&CancelNote, window, cx);
            return;
        }
        self.keys_were = window.focused(cx);
        let within = Rc::clone(&self.stage_at);
        let words = Words { save: "Send", band: None, card: "New note", tools: false };
        let keep = format!("walk-note:{}", self.walk.name);
        let capture = cx.new(|cx| Capture::new(within, words, Some(keep), cx));
        let watch = cx.subscribe_in(&capture, window, |this, _, event: &CaptureEvent, window, cx| match event {
            CaptureEvent::Noted(noted) => this.noted(noted, window, cx),
            CaptureEvent::Closed => this.cancel_note(&CancelNote, window, cx),
        });
        let focus = capture.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
        self.capture = Some((capture, watch));
        cx.notify();
    }

    /// The keys go back to what had them before Capture or the menu, else to the window.
    fn give_keys_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.keys_were.take() {
            Some(keys) => window.focus(&keys, cx),
            None => window.focus(&self.focus, cx),
        }
    }

    fn cancel_note(&mut self, _: &CancelNote, window: &mut Window, cx: &mut Context<Self>) {
        if self.capture.take().is_some() {
            self.give_keys_back(window, cx);
            cx.notify();
        } else {
            // Esc is the screen's own while nothing is being captured.
            cx.propagate();
        }
    }

    /// The card's note joins the held ones, for Send notes.
    fn noted(&mut self, noted: &capture::Noted, window: &mut Window, cx: &mut Context<Self>) {
        self.capture = None;
        self.give_keys_back(window, cx);
        let (role, name) = match &noted.held {
            Held::Element(drawn) => (capture::role_word(drawn.role).unwrap_or("element").to_lowercase(), drawn.name.to_string()),
            Held::Region(bounds) => (
                "region".to_string(),
                format!(
                    "{:.0},{:.0} {:.0}×{:.0}",
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height)
                ),
            ),
            // A walk window never arms a whole-window note: the layer does.
            Held::Window { .. } => ("window".to_string(), "Whole window".to_string()),
        };
        self.held.push(HeldNote {
            said: noted.said.clone(),
            take: self.taken(),
            walk: self.walk.name,
            of: self.walk.steps.len() as u32,
            step: self.at as u32 + 1,
            role,
            name,
            ring: noted.held.bounds(),
            picture: noted.picture.clone(),
        });
        self.said = Some(Said { text: format!("Note {} held", self.held.len()).into(), fault: false });
        cx.notify();
    }

    /// Send off the main thread, and show what came back.
    /// `sent` runs on the window once Fleet took it.
    fn send(
        &mut self,
        cx: &mut Context<Self>,
        done: &'static str,
        sent: impl FnOnce(&mut Self) + 'static,
        refused: impl FnOnce(&mut Self) + 'static,
        call: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) {
        cx.spawn(async move |this, cx| {
            let answer = cx.background_executor().spawn(async move { call() }).await;
            let _ = this.update(cx, |this, cx| {
                this.said = Some(match answer {
                    Ok(()) => {
                        sent(this);
                        Said { text: done.into(), fault: false }
                    }
                    Err(why) => {
                        refused(this);
                        Said { text: why.into(), fault: true }
                    }
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
            missing: if self.playing {
                Some("still playing".into())
            } else {
                let step = &self.walk.steps[self.at];
                match (step.verb, find(step, window, cx)) {
                    (Verb::Gone, _) if gone(step, window, cx) => None,
                    (Verb::Gone, _) => Some(format!("the {} named \"{}\" is still drawn", step.target.role, step.target.name)),
                    (_, found) => found.err().map(|missed| missed.0),
                }
            },
        }
    }

    fn render_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let takes = self.takes_trigger(cx);
        let approve = if self.sending {
            sending_button(cx).into_any_element()
        } else if self.held.is_empty() {
            approve_button(self.takes.get(self.take).map(|take| take.name.clone()), cx).into_any_element()
        } else {
            self.send_notes_button(cx).into_any_element()
        };
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
            .children(takes)
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
                Some("⌥⌘A"),
                if self.capture.is_some() { Weight::Primary } else { Weight::Quiet },
                true,
                cx,
                cx.listener(|this, _, window, cx| {
                    this.take_note(&TakeNote, window, cx)
                }),
            ))
            .child(
                // Approve keeps one width, `calc(var(--space-8) * 6)`, so nothing beside it moves; wider
                // for a walk with takes, as it names the one on screen.
                div().flex_none().w(look.length(token::SPACE_8) * if self.takes.is_empty() { 6. } else { 8. }).child(approve),
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

impl WalkWindow {
    /// Send notes in Approve's place while notes are held (`CaptureBar.tsx`): the primary button,
    /// ⌘↵, and a caret for approving with them or without.
    fn send_notes_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let caret_at = Rc::clone(&self.caret_at);
        let accent = look.color(token::ACCENT);
        let ink = look.color(token::TEXT_ON_ACCENT);
        let part = |id: &'static str| {
            div()
                .id(id)
                .h(look.length(token::H_CONTROL_SM))
                .flex()
                .items_center()
                .justify_center()
                .bg(accent)
                .text_color(ink)
                .cursor_pointer()
                .hover(|style| style.opacity(0.9))
        };
        div()
            .w_full()
            .flex()
            .gap(look.length(token::BORDER_WIDTH))
            .child(labelled(
                part("walk-send-notes")
                    .flex_1()
                    .min_w_0()
                    .gap(look.length(token::SPACE_2))
                    .rounded_l(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .child("Send notes")
                    .child(chord("⌘↵", cx))
                    .tooltip(|window, cx| gpui_component::tooltip::Tooltip::new("Sends every held note to the Job as one turn").build(window, cx))
                    .on_click(cx.listener(|this, _, _, cx| this.send_notes(false, cx))),
                Role::Button,
                "Send notes",
            ))
            .child(labelled(
                part("walk-notes-menu")
                    .relative()
                    .flex_none()
                    .w(look.length(token::H_CONTROL_SM))
                    .rounded_r(look.length(token::RADIUS_MD))
                    .child(canvas(move |bounds, _, _| caret_at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                    .child(gpui::svg().path(crate::assets::icon_path("chevron-down")).size(look.length(token::SPACE_3)).text_color(ink))
                    .on_click(cx.listener(|this, _, window, cx| this.open_notes_menu(window, cx))),
                Role::Button,
                "More ways to send",
            ))
    }
}

/// Send notes while the notes are on their way: the same accent, a turning mark, and no press.
fn sending_button(cx: &mut Context<WalkWindow>) -> impl IntoElement {
    let look = cx.look();
    let ink = look.color(token::TEXT_ON_ACCENT);
    labelled(
        div()
            .id("walk-sending")
            .w_full()
            .h(look.length(token::H_CONTROL_SM))
            .flex()
            .items_center()
            .justify_center()
            .gap(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::ACCENT).opacity(0.7))
            .text_color(ink)
            .text_size(look.length(token::TEXT_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .child(
                gpui::svg()
                    .path(crate::assets::icon_path("loader-circle"))
                    .size(look.length(token::SPACE_4))
                    .text_color(ink)
                    .with_animation("walk-sending-turn", gpui::Animation::new(look.duration(token::DURATION_SPIN)).repeat(), |mark, at| {
                        mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(at)))
                    }),
            )
            .child("Sending"),
        Role::Button,
        "Sending",
    )
}

/// Approve as Electron's walk bar draws it: a secondary button filling its slot, the word and a
/// check centred, and the ⌘↵ chord. With takes it names the one on screen, which Electron's does not.
fn approve_button(take: Option<SharedString>, cx: &mut Context<WalkWindow>) -> impl IntoElement {
    let look = cx.look();
    let said: SharedString = match take {
        Some(take) => format!("Approve {take}").into(),
        None => "Approve".into(),
    };
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
            .child(div().min_w_0().truncate().child(said.clone()))
            .child(gpui::svg().path(crate::assets::icon_path("check")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_DEFAULT)))
            .child(chord("⌘↵", cx))
            .on_click(cx.listener(|this, _, window, cx| this.approve(&Approve, window, cx))),
        Role::Button,
        said,
    )
}

impl Render for WalkWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bar = self.render_bar(cx);
        // The app draws below the bar: its sheets and dialogs start there, so the step stays readable.
        // The bar is two rows, each as `render_bar`'s `row` sets it.
        let barred = {
            let look = cx.look();
            (look.length(token::SPACE_8) + look.length(token::SPACE_2) + look.length(token::SPACE_1)) * 2.
        };
        let viewport = window.viewport_size();
        crate::room::set(window, Bounds::new(point(px(0.), barred), gpui::size(viewport.width, viewport.height - barred)), cx);
        let look = cx.look();
        let ring = if self.playing || self.capture.is_some() { None } else { find(&self.walk.steps[self.at], window, cx).ok() };
        let gap = look.length(token::FOCUS_RING_WIDTH) * 2.;
        let stage_at = Rc::clone(&self.stage_at);
        let chrome = |element: AnyElement| gpui::deferred(element).with_priority(crate::room::WALK_CHROME);
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
                // In a development build the layer's ⌥⌘A is bound with no context, which gpui ranks
                // above the walk's own; taken here first, it is the walk's note wherever the keys are.
                // A step's key while playing goes on to the screen, as the notes walk shows the layer.
                .capture_action(cx.listener(|this, _: &crate::annotate::Toggle, window, cx| {
                    if !this.playing {
                        cx.stop_propagation();
                        this.take_note(&TakeNote, window, cx);
                    }
                }))
                .on_action(cx.listener(Self::cancel_note))
                .on_action(cx.listener(Self::open_takes))
                .relative()
                .size_full()
                .flex()
                .flex_col()
                .bg(look.color(token::SURFACE_CANVAS))
                // What a library widget inherits, such as the note's typed text; gpui's own is black.
                .text_color(look.color(token::FG_DEFAULT))
                .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                // Walk chrome paints above every app layer: the bar, the ring on the target, the note and the takes menu.
                .child(chrome(bar.into_any_element()))
                .child(
                    div()
                        .relative()
                        .flex_1()
                        .min_h(px(0.))
                        .children(self.stage.as_ref().map(|stage| stage.view.clone()))
                        .child(canvas(move |bounds, _, _| stage_at.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full()),
                )
                .children(ring.map(|drawn| {
                    let shown = drawn.shown;
                    chrome(div()
                        .absolute()
                        .left(shown.origin.x - gap)
                        .top(shown.origin.y - gap)
                        .w(shown.size.width + gap * 2.)
                        .h(shown.size.height + gap * 2.)
                        // The ring follows the element's own corners, out by the gap; one that sets none takes the card's.
                        .rounded(drawn.radius.unwrap_or(look.length(token::RADIUS_LG)) + gap)
                        .border(look.length(token::FOCUS_RING_WIDTH))
                        .border_color(look.color(token::ACCENT))
                        .into_any_element())
                }))
                .children(self.capture.as_ref().map(|(capture, _)| chrome(capture.clone().into_any_element())))
                .children(self.menu.as_ref().map(|(menu, ..)| chrome(menu.clone().into_any_element()))),
        )
    }
}
