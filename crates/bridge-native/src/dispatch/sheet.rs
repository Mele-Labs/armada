//! The dispatch sheet: one line to describe the work, a drawer under it for the repository, the
//! kind, the model and the act (`DP/DispatchPanel.tsx`, `DP/dispatching.tsx`).
//!
//! It draws a [`Face`] and holds only what a hand has changed: the words, the kind, the pick, the
//! sentence's cycles. A press answers in the same frame (the act turns pending, a chip fills) and
//! tells the app through [`Asked`]; the app answers with [`DispatchSheet::answered`] or
//! [`DispatchSheet::refused`], which puts the sheet back with its words kept. This file is its
//! state and keys; `view.rs` draws it.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    point, AppContext, Anchor, App, Bounds, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, KeyDownEvent, Pixels,
    SharedString, Subscription, Window,
};
use gpui_component::input::{InputEvent, InputState, Position, TextareaState};

use super::face::{Chip, Face, Kind, Mark, Offered, Part, Reading, Row, Sent, Trigger, Wait};
use crate::dropdown::{Dropdown, Entry, Picked};
use crate::keymap::Act;
use crate::look::{token, ActiveLook};
use crate::titlebar::id_of;

/// Where the request's words are kept between openings, and the model and effort picked.
pub const KEEP_REQUEST: &str = "dispatch:request";
const KEEP_TUNING: &str = "dispatch:session-tuning";

/// What the sheet asks of the app. The wiring sends these to Fleet; a walk answers them by scene.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    /// The sheet shut. The draft stays.
    Closed,
    /// The words rested: which repository do they name.
    ReadRepository(String),
    /// A Job is tagged: read the wait the words name.
    ReadWait { job: String, words: String },
    Start { words: String, repository: String, model: Option<String>, effort: Option<String>, held: bool },
    Queue { words: String, repository: String },
    ReadAutopilot { words: String, repository: String },
    StartAutopilot { repository: String, instruction: String },
}

impl EventEmitter<Asked> for DispatchSheet {}

/// The `@` or `#` being typed, and the rows it offers.
pub(super) struct Menu {
    pub trigger: Trigger,
    /// Where the trigger stands in the words, in bytes.
    pub from: usize,
    pub query: String,
    pub rows: Vec<Row>,
    pub active: usize,
}

/// The popover a trigger in the drawer opens.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pop {
    Model,
    Effort,
    More,
}

pub(super) struct Popup {
    pub dropdown: Entity<Dropdown>,
    _picked: Subscription,
    _dismissed: Subscription,
}

#[derive(Default)]
pub(super) struct Anchors {
    pub model: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub effort: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub more: Rc<Cell<Option<Bounds<Pixels>>>>,
}

pub struct DispatchSheet {
    pub face: Face,
    pub(super) open: bool,
    pub(super) kind: Kind,
    pub(super) request: Entity<TextareaState>,
    _typed: Subscription,
    pub(super) attached: Vec<SharedString>,
    /// The act is out: Start, Read it or Start Autopilot is pending.
    pub(super) sending: bool,
    pub(super) queueing: bool,
    /// A Job was started with no repository named; the row takes the keys until one is picked.
    pub(super) asking: bool,
    /// The act is waiting for the repository read to answer.
    waiting: bool,
    pub(super) menu: Option<Menu>,
    /// The Job menu on the sentence, with its active row.
    pub(super) wait_menu: Option<usize>,
    /// The Job the words tag, by id and title.
    pub(super) tagged: Option<(SharedString, SharedString)>,
    pub(super) wait: Option<Wait>,
    pub(super) model: Option<SharedString>,
    pub(super) effort: Option<SharedString>,
    pub(super) popup: Option<Popup>,
    pub(super) at: Anchors,
    /// The Autopilot read's fields, as the person edits them.
    pub(super) instruction: Option<Entity<InputState>>,
    pub(super) fields: Vec<(&'static str, Entity<InputState>, Option<SharedString>)>,
    keys_were: Option<FocusHandle>,
    /// The keys the request would take for itself (↑ ↓ ↵) are read before it sees them while the sheet is open.
    intercept: Option<Subscription>,
    pub(super) frame: FocusHandle,
    pub(super) row: FocusHandle,
    pub(super) parts: [FocusHandle; 4],
    /// The chip the row's arrow keys stand on.
    pub(super) rover: usize,
}

impl Focusable for DispatchSheet {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.request.focus_handle(cx)
    }
}

impl DispatchSheet {
    pub fn new(face: Face, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let request = cx.new(|cx| TextareaState::new(window, cx).auto_grow(5, 5).placeholder(placeholder(Kind::Job)));
        let typed = cx.subscribe_in(&request, window, |this, request, event: &InputEvent, window, cx| match event {
            InputEvent::Change => this.typed(request, window, cx),
            InputEvent::PressEnter { secondary: true, .. } => {
                if this.kind == Kind::Autopilot {
                    this.act(window, cx);
                }
            }
            _ => {}
        });
        DispatchSheet {
            wait: face.wait.clone(),
            face,
            open: false,
            kind: Kind::Job,
            request,
            _typed: typed,
            attached: Vec::new(),
            sending: false,
            queueing: false,
            asking: false,
            waiting: false,
            menu: None,
            wait_menu: None,
            tagged: None,
            model: None,
            effort: None,
            popup: None,
            at: Anchors::default(),
            instruction: None,
            fields: Vec::new(),
            keys_were: None,
            intercept: None,
            frame: cx.focus_handle(),
            row: cx.focus_handle(),
            parts: [cx.focus_handle(), cx.focus_handle(), cx.focus_handle(), cx.focus_handle()],
            rover: 0,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    // -- opening and closing ----------------------------------------------------------------

    /// Open on the draft as it stands: the words kept, a Job. `add` is typed after them: the `@` that
    /// opens its menu, or the key that began the next draft.
    pub fn show(&mut self, add: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            return;
        }
        self.keys_were = window.focused(cx);
        self.open = true;
        let me = cx.entity().downgrade();
        self.intercept = Some(cx.intercept_keystrokes(move |event, window, cx| {
            let Some(me) = me.upgrade() else { return };
            me.update(cx, |this, cx| {
                if this.open && this.frame.contains_focused(window, cx) {
                    let down = KeyDownEvent { keystroke: event.keystroke.clone(), is_held: false, prefer_character_input: false };
                    this.on_key(&down, window, cx);
                }
            });
        }));
        self.kind = Kind::Job;
        self.asking = false;
        self.face.fresh();
        let words = crate::drafts::kept(KEEP_REQUEST, cx).unwrap_or_default();
        let tuning = crate::drafts::kept(KEEP_TUNING, cx).unwrap_or_default();
        let mut parts = tuning.split('\n');
        self.model = parts.next().filter(|one| !one.is_empty()).map(|one| one.to_string().into());
        self.effort = parts.next().filter(|one| !one.is_empty()).map(|one| one.to_string().into());
        let words = format!("{words}{add}");
        self.request.update(cx, |state, cx| {
            state.set_placeholder(placeholder(Kind::Job), window, cx);
            state.set_value(words.clone(), window, cx);
            end_of(state, &words, window, cx);
        });
        let focus = self.request.focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
        cx.notify();
    }

    /// Esc, or a press on the scrim: the sheet goes and the draft stays, the keys back where they were.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        self.intercept = None;
        self.menu = None;
        self.wait_menu = None;
        self.popup = None;
        self.asking = false;
        if let Some(keys) = self.keys_were.take() {
            window.focus(&keys, cx);
        }
        cx.emit(Asked::Closed);
        cx.notify();
    }

    /// The Job the words tag, as the screen's mock Fleet reads the wait on it.
    pub fn tagged(&self) -> Option<Offered> {
        self.tagged.as_ref().map(|(id, title)| Offered { id: id.clone(), title: title.clone() })
    }

    pub(super) fn words(&self, cx: &App) -> String {
        self.request.read(cx).value().to_string()
    }

    pub(super) fn worded(&self, cx: &App) -> bool {
        !self.words(cx).trim().is_empty() || !self.attached.is_empty()
    }

    // -- the words ----------------------------------------------------------------------------

    fn typed(&mut self, request: &Entity<TextareaState>, _window: &mut Window, cx: &mut Context<Self>) {
        let words = request.read(cx).value().to_string();
        crate::drafts::keep(KEEP_REQUEST, &words, cx);
        // Every change clears the Autopilot read, the refusal and any pending act.
        self.face.read = None;
        self.face.refused = None;
        self.instruction = None;
        self.fields.clear();
        self.asking = false;
        self.sending = false;
        let cursor = request.read(cx).cursor().min(words.len());
        self.menu = self.mention(&words, cursor).filter(|menu| !menu.rows.is_empty());
        self.reread(&words, cx);
        cx.notify();
    }

    /// The words rested: the repository is read again, and the Job the words tag is looked for.
    fn reread(&mut self, words: &str, cx: &mut Context<Self>) {
        self.tagged = self
            .face
            .mentions
            .jobs
            .iter()
            .filter(|job| words.contains(&format!("@{}", job.title)))
            .max_by_key(|job| job.title.len())
            .map(|job| (job.id.clone(), job.title.clone()));
        if words.trim().is_empty() {
            self.wait = None;
            return;
        }
        // A repository picked by hand stands until the sheet is opened again.
        if self.face.reading != Reading::Picked {
            self.face.reading = Reading::Detecting;
            cx.emit(Asked::ReadRepository(words.to_string()));
        }
        match &self.tagged {
            Some((job, _)) => cx.emit(Asked::ReadWait { job: job.to_string(), words: words.to_string() }),
            None => self.wait = None,
        }
    }

    /// The `@` or `#` being typed before the caret (`DP/SessionMenu.tsx`, `useAtMenu`).
    fn mention(&self, words: &str, cursor: usize) -> Option<Menu> {
        let before = words.get(..cursor)?;
        let mut found = None;
        for (at, ch) in before.char_indices().rev() {
            if ch == '\n' {
                break;
            }
            if ch == '#' || ch == '@' {
                if before[..at].chars().last().map_or(true, char::is_whitespace) {
                    found = Some((at, ch));
                }
                break;
            }
        }
        let (at, ch) = found?;
        let query = before[at + 1..].to_string();
        let trigger = if ch == '@' { Trigger::At } else { Trigger::Hash };
        let spaced = query.chars().any(char::is_whitespace);
        if trigger == Trigger::Hash && spaced {
            return None;
        }
        // `@` narrows while some Job title still starts with what was typed, whitespace included.
        if trigger == Trigger::At && spaced {
            let lower = query.to_lowercase();
            if !self.face.mentions.jobs.iter().any(|job| job.title.to_lowercase().starts_with(&lower)) {
                return None;
            }
        }
        let rows = self.face.mentions.rows(trigger, &query);
        Some(Menu { trigger, from: at, query, rows, active: 0 })
    }

    pub(super) fn choose_row(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.take() else { return };
        let Some(row) = menu.rows.get(at).cloned() else { return };
        let words = self.words(cx);
        let cursor = self.request.read(cx).cursor().min(words.len());
        let next = format!("{}{}{}", &words[..menu.from], row.insert, &words[cursor..]);
        let end = menu.from + row.insert.len();
        self.request.update(cx, |state, cx| {
            state.set_value(next.clone(), window, cx);
            end_of(state, &next[..end], window, cx);
        });
        // Setting the value says nothing to the field's watchers: the draft, the read and the tag follow here.
        let request = self.request.clone();
        self.typed(&request, window, cx);
    }

    // -- kinds ----------------------------------------------------------------------------------

    pub(super) fn set_kind(&mut self, kind: Kind, window: &mut Window, cx: &mut Context<Self>) {
        if self.kind == kind || !self.face.kinds.contains(&kind) {
            return;
        }
        self.kind = kind;
        self.sending = false;
        self.face.read = None;
        self.instruction = None;
        self.fields.clear();
        self.request.update(cx, |state, cx| state.set_placeholder(placeholder(kind), window, cx));
        cx.notify();
    }

    fn step_kind(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let kinds = self.face.kinds.clone();
        if kinds.len() < 2 {
            return;
        }
        let at = kinds.iter().position(|kind| *kind == self.kind).unwrap_or(0);
        let to = if back { (at + kinds.len() - 1) % kinds.len() } else { (at + 1) % kinds.len() };
        self.set_kind(kinds[to], window, cx);
    }

    // -- the act ----------------------------------------------------------------------------------

    fn repository(&self) -> Option<String> {
        self.face.chosen().map(|chip| chip.name.to_string())
    }

    /// Start, Read it or Start Autopilot, from the button or ↵: pending in this frame.
    pub fn act(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.face.connected || self.sending || self.queueing || !self.worded(cx) {
            return;
        }
        if self.face.reading == Reading::Detecting && self.repository().is_none() {
            // Nothing starts without a repository: a read still out is waited for.
            self.sending = true;
            self.waiting = true;
            cx.notify();
            return;
        }
        let Some(repository) = self.repository() else {
            self.ask(window, cx);
            return;
        };
        self.asking = false;
        self.sending = true;
        let words = self.words(cx);
        match self.kind {
            Kind::Job => {
                let held = self.wait.is_none();
                cx.emit(Asked::Start {
                    words,
                    repository,
                    model: self.model.as_ref().map(|one| one.to_string()),
                    effort: self.effort.as_ref().map(|one| one.to_string()),
                    held,
                });
            }
            Kind::Autopilot if self.face.read.is_none() => {
                self.face.refused = None;
                cx.emit(Asked::ReadAutopilot { words, repository });
            }
            Kind::Autopilot => {
                let instruction = self.instruction.as_ref().map(|input| input.read(cx).value().to_string()).unwrap_or_default();
                cx.emit(Asked::StartAutopilot { repository, instruction });
            }
        }
        cx.notify();
    }

    /// No repository stands: the frame takes the call hue and the row the keys until one is picked.
    fn ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = true;
        let row = self.row.clone();
        window.focus(&row, cx);
        cx.notify();
    }

    /// Queue for later: the idea is saved here and the sheet goes.
    pub(super) fn queue(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.face.connected || self.sending || self.queueing || self.kind != Kind::Job || !self.face.queue || self.words(cx).trim().is_empty() {
            return;
        }
        let Some(repository) = self.repository() else {
            self.ask(window, cx);
            return;
        };
        self.queueing = true;
        cx.emit(Asked::Queue { words: self.words(cx), repository });
        cx.notify();
    }

    /// Fleet took it: the words go, the sheet clears and shuts. An Autopilot leaves a strip.
    pub fn answered(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<Sent> {
        let kind = self.kind;
        let title = self.repository().unwrap_or_default();
        self.sending = false;
        self.queueing = false;
        self.waiting = false;
        crate::drafts::clear(KEEP_REQUEST, cx);
        self.attached.clear();
        self.wait = None;
        self.tagged = None;
        self.face.read = None;
        self.instruction = None;
        self.fields.clear();
        self.request.update(cx, |state, cx| state.set_value("", window, cx));
        self.close(window, cx);
        (kind == Kind::Autopilot).then(|| Sent { kind, title: title.into() })
    }

    /// Fleet refused: the sheet is as it was, words and pick kept.
    pub fn refused(&mut self, cx: &mut Context<Self>) {
        self.sending = false;
        self.queueing = false;
        self.waiting = false;
        cx.notify();
    }

    /// Fleet read an Autopilot sentence: the instruction and the fields appear, and the act turns to Start Autopilot.
    pub fn read_answered(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sending = false;
        let Some(read) = self.face.read.clone() else {
            cx.notify();
            return;
        };
        let instruction = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(read.instruction.to_string(), window, cx);
            state
        });
        self.instruction = Some(instruction);
        self.fields = read
            .fields
            .iter()
            .map(|field| {
                let input = cx.new(|cx| {
                    let mut state = InputState::new(window, cx).placeholder(field.placeholder);
                    state.set_value(field.value.to_string(), window, cx);
                    state
                });
                (field.label, input, field.quoted.clone())
            })
            .collect();
        cx.notify();
    }

    /// The app changed the face under the sheet: a read answered, a wait arrived. A start that was
    /// waiting on the repository goes on.
    pub fn set_face(&mut self, face: Face, window: &mut Window, cx: &mut Context<Self>) {
        let hand = (self.face.reading == Reading::Picked).then(|| self.face.chosen().map(|chip| chip.name.clone())).flatten();
        let refused = self.face.refused.clone();
        self.face = face;
        if let Some(name) = hand {
            self.face.pick(&name);
        }
        if self.face.refused.is_none() {
            self.face.refused = refused.filter(|_| false);
        } else {
            self.sending = false;
        }
        if let Some(wait) = self.face.wait.clone() {
            self.wait = Some(wait);
        }
        if self.waiting && self.face.reading != Reading::Detecting {
            self.waiting = false;
            self.sending = false;
            self.act(window, cx);
        }
        if self.face.read.is_some() && self.instruction.is_none() {
            self.read_answered(window, cx);
        }
        cx.notify();
    }

    /// A pasted snippet kept as a file.
    pub fn attach(&mut self, name: &str, cx: &mut Context<Self>) {
        self.attached.push(name.to_string().into());
        cx.notify();
    }

    // -- the repository row -------------------------------------------------------------------------

    pub(super) fn pick(&mut self, name: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let was_asking = self.asking;
        self.face.pick(&name);
        self.asking = false;
        if was_asking {
            // The act was waiting on a repository: it goes at once.
            self.act(window, cx);
        } else {
            let focus = self.request.focus_handle(cx);
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    fn all_chips(&self) -> Vec<Chip> {
        self.face.chips.iter().chain(&self.face.more).cloned().collect()
    }

    // -- keys ----------------------------------------------------------------------------------------

    pub(super) fn on_act(&mut self, key: &Act, window: &mut Window, cx: &mut Context<Self>) {
        match key.id {
            // The sheet is a dialog, and the key does nothing while one is up.
            "new_job" => {}
            "dispatch_after" => self.request.update(cx, |state, cx| state.insert("@", window, cx)),
            "dispatch_repository" => {
                let Some(digit) = key.digit else { return };
                let Some(chip) = self.face.chips.iter().find(|chip| chip.key == Some(digit)).cloned() else { return };
                let none_chosen = self.repository().is_none();
                self.face.pick(&chip.name);
                self.asking = false;
                // With none chosen, the key also sends.
                if none_chosen && self.worded(cx) {
                    self.act(window, cx);
                }
                cx.notify();
            }
            "queue_session" => self.queue(window, cx),
            "send_message" => {
                if self.kind == Kind::Autopilot {
                    self.act(window, cx);
                }
            }
            _ => cx.propagate(),
        }
    }

    pub(super) fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let mods = &event.keystroke.modifiers;
        let plain = !mods.platform && !mods.control && !mods.alt;
        let in_request = self.request.focus_handle(cx).is_focused(window);
        // The `@` menu: arrows wrap, ↵ or ⇥ choose, Esc closes it and nothing else.
        if let Some(menu) = &mut self.menu {
            let len = menu.rows.len().max(1);
            match key {
                "down" => menu.active = (menu.active + 1) % len,
                "up" => menu.active = (menu.active + len - 1) % len,
                "enter" | "tab" if plain => {
                    let at = menu.active;
                    self.choose_row(at, window, cx);
                }
                "escape" => self.menu = None,
                _ => return,
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        match key {
            "escape" => {
                if self.wait_menu.take().is_some() || self.popup.take().is_some() {
                    // Only the menu closes.
                } else if self.asking {
                    self.asking = false;
                    let focus = self.request.focus_handle(cx);
                    window.focus(&focus, cx);
                } else {
                    self.close(window, cx);
                }
            }
            "tab" if plain && in_request => {
                if mods.shift && self.kind == Kind::Job && self.wait.is_some() {
                    let first = self.parts[0].clone();
                    window.focus(&first, cx);
                } else if mods.shift || self.face.kinds.len() < 2 {
                    return;
                } else {
                    self.step_kind(false, window, cx);
                }
            }
            "enter" if plain && !mods.shift && in_request && self.kind == Kind::Job => self.act(window, cx),
            "left" | "right" if plain && self.row.is_focused(window) => {
                let len = self.all_chips().len().max(1);
                self.rover = if key == "right" { (self.rover + 1) % len } else { (self.rover + len - 1) % len };
            }
            "enter" | "space" if plain && self.row.is_focused(window) => {
                if let Some(chip) = self.all_chips().get(self.rover).cloned() {
                    self.pick(chip.name, window, cx);
                }
            }
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    // -- popovers ----------------------------------------------------------------------------------------

    fn open_pop(&mut self, kind: Pop, from: Bounds<Pixels>, name: &'static str, entries: Vec<Entry>, window: &mut Window, cx: &mut Context<Self>) {
        let gap = cx.look().length(token::SPACE_1);
        let at = point(from.origin.x, from.origin.y + from.size.height + gap);
        let dropdown = cx.new(|cx| Dropdown::new(name, entries, at, Anchor::TopLeft, false, cx));
        let picked = cx.subscribe_in(&dropdown, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, .. } = picked;
            this.popup = None;
            this.picked(kind, id, window, cx);
        });
        let dismissed = cx.subscribe_in(&dropdown, window, |this, _, _: &DismissEvent, window, cx| {
            this.popup = None;
            let focus = this.request.focus_handle(cx);
            window.focus(&focus, cx);
            cx.notify();
        });
        let keys = dropdown.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.popup = Some(Popup { dropdown, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }

    fn picked(&mut self, kind: Pop, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.request.focus_handle(cx);
        window.focus(&focus, cx);
        match kind {
            Pop::Model => self.model = id.strip_prefix("model:").filter(|one| !one.is_empty()).map(|one| one.to_string().into()),
            Pop::Effort => self.effort = id.strip_prefix("effort:").filter(|one| !one.is_empty()).map(|one| one.to_string().into()),
            Pop::More => {
                if let Some(name) = id.strip_prefix("repo:") {
                    self.pick(name.to_string().into(), window, cx);
                }
                return;
            }
        }
        let kept = format!("{}\n{}", self.model.as_deref().unwrap_or(""), self.effort.as_deref().unwrap_or(""));
        crate::drafts::keep(KEEP_TUNING, &kept, cx);
        cx.notify();
    }

    pub(super) fn open_model(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = self.at.model.get() else { return };
        let auto = self.face.tuning.default_model.clone().unwrap_or_else(|| "Auto".into());
        let mut entries = vec![Entry::Choice { id: "model:", label: auto, on: self.model.is_none() }];
        for model in &self.face.tuning.models {
            entries.push(Entry::Choice { id: id_of(format!("model:{model}")), label: model.clone(), on: self.model.as_ref() == Some(model) });
        }
        self.open_pop(Pop::Model, from, "Model", entries, window, cx);
    }

    pub(super) fn open_effort(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = self.at.effort.get() else { return };
        let mut entries = vec![Entry::Choice { id: "effort:", label: "Auto".into(), on: self.effort.is_none() }];
        for effort in &self.face.tuning.efforts {
            entries.push(Entry::Choice { id: id_of(format!("effort:{effort}")), label: effort.clone(), on: self.effort.as_ref() == Some(effort) });
        }
        self.open_pop(Pop::Effort, from, "Effort", entries, window, cx);
    }

    pub(super) fn open_more(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = self.at.more.get() else { return };
        let mut entries = vec![Entry::Search { label: "Find a repository", to_rule: false }];
        for chip in &self.face.more {
            entries.push(Entry::Choice { id: id_of(format!("repo:{}", chip.name)), label: chip.name.clone(), on: chip.mark == Mark::Chosen });
        }
        self.open_pop(Pop::More, from, "More repositories", entries, window, cx);
    }

    // -- the wait sentence ------------------------------------------------------------------------------

    pub(super) fn cycle(&mut self, part: Part, back: bool, cx: &mut Context<Self>) {
        if let Some(wait) = &mut self.wait {
            wait.cycled(part, back);
        }
        cx.notify();
    }

    pub(super) fn wait_on(&mut self, at: usize, cx: &mut Context<Self>) {
        self.wait_menu = None;
        let id = self.wait.as_ref().and_then(|wait| wait.others.get(at)).map(|one| one.id.clone());
        if let (Some(id), Some(wait)) = (id, self.wait.as_mut()) {
            wait.waits_on(&id);
        }
        cx.notify();
    }

    pub(super) fn part_keys(&mut self, part: Part, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if key == "escape" {
            if self.wait_menu.take().is_none() {
                let focus = self.request.focus_handle(cx);
                window.focus(&focus, cx);
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if part == Part::On {
            let others = self.wait.as_ref().map_or(0, |wait| wait.others.len()).max(1);
            match (key, self.wait_menu) {
                ("down" | "up" | "space" | "enter", None) => self.wait_menu = Some(0),
                ("down", Some(at)) => self.wait_menu = Some((at + 1) % others),
                ("up", Some(at)) => self.wait_menu = Some((at + others - 1) % others),
                ("space" | "enter", Some(at)) => self.wait_on(at, cx),
                _ => return,
            }
        } else {
            match key {
                "down" | "space" | "enter" => self.cycle(part, false, cx),
                "up" => self.cycle(part, true, cx),
                _ => return,
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
}

pub(super) fn placeholder(kind: Kind) -> &'static str {
    match kind {
        Kind::Job => "Describe the work, or paste a link to a ticket.",
        Kind::Autopilot => "What should it do?",
    }
}

/// The caret at the end of `text`, which `set_value` leaves at the start.
fn end_of(state: &mut TextareaState, text: &str, window: &mut Window, cx: &mut Context<TextareaState>) {
    let line = text.matches('\n').count() as u32;
    let last = text.rsplit('\n').next().unwrap_or("");
    state.set_cursor_position(Position::new(line, last.encode_utf16().count() as u32), window, cx);
}
