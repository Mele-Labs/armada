//! The dispatch sheet: one line to describe the work, a drawer under it for the repository, the
//! model and the act (`DP/DispatchPanel.tsx`, `DP/dispatching.tsx`).
//!
//! It draws a [`Face`] and holds only what a hand has changed: the words, the pick, the
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
use gpui_component::input::{InputEvent, Position, TextareaState};

use super::face::{Face, Mark, Offered, Part, Reading, Row, Trigger, Wait};
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
    Queue { words: String, repository: String, model: Option<String>, effort: Option<String> },
    /// New Job here, from the card under the sheet: what ⌘T starts, in the repository the picker named if it did. The sheet has shut.
    JobHere(Option<String>),
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
    Repository,
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
    pub repository: Rc<Cell<Option<Bounds<Pixels>>>>,
}

pub struct DispatchSheet {
    pub face: Face,
    pub(super) open: bool,
    pub(super) request: Entity<TextareaState>,
    _typed: Subscription,
    pub(super) attached: Vec<SharedString>,
    /// The act is out: Start is pending.
    pub(super) sending: bool,
    /// The act out is Draft: the button reads Drafting, and a start waiting on the repository stays a draft.
    pub(super) drafting: bool,
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
    keys_were: Option<FocusHandle>,
    /// The keys the request would take for itself (↑ ↓ ↵) are read before it sees them while the sheet is open.
    intercept: Option<Subscription>,
    pub(super) frame: FocusHandle,
    pub(super) row: FocusHandle,
    pub(super) parts: [FocusHandle; 4],
    /// Where the card under the sheet starts a Job, as the title bar names it: none draws no card.
    pub(super) here: Option<SharedString>,
    pub(super) here_key: FocusHandle,
    pub(super) model_key: FocusHandle,
    pub(super) effort_key: FocusHandle,
    pub(super) queue_key: FocusHandle,
    pub(super) draft_key: FocusHandle,
    pub(super) act_key: FocusHandle,
}

impl Focusable for DispatchSheet {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.request.focus_handle(cx)
    }
}

impl DispatchSheet {
    pub fn new(face: Face, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let request = cx.new(|cx| TextareaState::new(window, cx).auto_grow(5, 5).placeholder(PLACEHOLDER));
        let typed = cx.subscribe_in(&request, window, |this, request, event: &InputEvent, window, cx| match event {
            InputEvent::Change => this.typed(request, window, cx),
            _ => {}
        });
        DispatchSheet {
            wait: face.wait.clone(),
            face,
            open: false,
            request,
            _typed: typed,
            attached: Vec::new(),
            sending: false,
            drafting: false,
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
            keys_were: None,
            intercept: None,
            frame: cx.focus_handle(),
            row: cx.focus_handle(),
            parts: [cx.focus_handle(), cx.focus_handle(), cx.focus_handle(), cx.focus_handle()],
            here: None,
            here_key: cx.focus_handle(),
            model_key: cx.focus_handle(),
            effort_key: cx.focus_handle(),
            queue_key: cx.focus_handle(),
            draft_key: cx.focus_handle(),
            act_key: cx.focus_handle(),
        }
    }

    /// The project or repository the card under the sheet starts a Job in.
    pub fn set_here(&mut self, here: Option<SharedString>, cx: &mut Context<Self>) {
        self.here = here;
        cx.notify();
    }

    /// The repository the picker names, chosen by the words or by hand. Where the rail is scoped it names the title bar's, not one picked.
    pub(super) fn named_repository(&self) -> Option<String> {
        if self.face.reading == Reading::Scoped {
            return None;
        }
        self.face.chosen().map(|chip| chip.name.to_string())
    }

    /// Where the card under the sheet, and ⌘T over it, start a Job: the repository the picker names, else where the title bar is.
    pub(super) fn here_in(&self) -> Option<SharedString> {
        let here = self.here.as_ref()?;
        Some(self.named_repository().map_or_else(|| here.clone(), SharedString::from))
    }

    /// The card under the sheet: the sheet shuts, and the Job ⌘T starts is asked for.
    pub(super) fn job_here(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.here.is_none() {
            return;
        }
        let picked = self.named_repository();
        self.close(window, cx);
        cx.emit(Asked::JobHere(picked));
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
        self.asking = false;
        self.face.fresh();
        let words = crate::drafts::kept(KEEP_REQUEST, cx).unwrap_or_default();
        let tuning = crate::drafts::kept(KEEP_TUNING, cx).unwrap_or_default();
        let mut parts = tuning.split('\n');
        self.model = parts.next().filter(|one| !one.is_empty()).map(|one| one.to_string().into());
        self.effort = parts.next().filter(|one| !one.is_empty()).map(|one| one.to_string().into());
        let words = format!("{words}{add}");
        self.request.update(cx, |state, cx| {
            state.set_placeholder(PLACEHOLDER, window, cx);
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
        // Every change clears the refusal and any pending act.
        self.face.refused = None;
        self.asking = false;
        self.sending = false;
        // A start waiting on the repository read stays what was pressed.
        self.drafting = self.drafting && self.waiting;
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
        // A repository picked by hand, or the one the window is scoped to, stands until the sheet is opened again.
        if !matches!(self.face.reading, Reading::Picked | Reading::Scoped) {
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

    /// A repository chosen for the sheet from elsewhere.
    pub(super) fn pick_repository(&mut self, name: &str, cx: &mut Context<Self>) {
        self.face.pick(name);
        cx.notify();
    }

    // -- the act ----------------------------------------------------------------------------------

    pub(super) fn repository(&self) -> Option<String> {
        self.face.chosen().map(|chip| chip.name.to_string())
    }

    /// Start, from the button or ↵: it runs at once, pending in this frame.
    pub fn act(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.begin(false, window, cx);
    }

    /// Draft: the same start, held on its start card until Start is pressed there. A wait has no card.
    pub(super) fn draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.wait.is_some() {
            return;
        }
        self.begin(true, window, cx);
    }

    fn begin(&mut self, held: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.face.connected || self.sending || self.queueing || !self.worded(cx) {
            return;
        }
        if self.face.reading == Reading::Detecting && self.repository().is_none() {
            // Nothing starts without a repository: a read still out is waited for.
            self.sending = true;
            self.drafting = held;
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
        self.drafting = held;
        let words = self.words(cx);
        self.face.refused = None;
        cx.emit(Asked::Start {
            words,
            repository,
            model: self.model.as_ref().map(|one| one.to_string()),
            effort: self.effort.as_ref().map(|one| one.to_string()),
            held,
        });
        cx.notify();
    }

    /// The words and the repository of a start out for Fleet's answer, for a walk's scene to answer with.
    pub(super) fn started(&self, cx: &App) -> Option<(String, String)> {
        self.sending.then(|| self.repository()).flatten().map(|repository| (self.words(cx), repository))
    }

    /// No repository stands: the picker takes the call hue and its list opens until one is picked.
    fn ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = true;
        let row = self.row.clone();
        window.focus(&row, cx);
        self.open_repositories(window, cx);
        cx.notify();
    }

    /// Queue for later: the idea is saved here and the sheet goes.
    pub(super) fn queue(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.face.connected || self.sending || self.queueing || !self.face.queue || self.words(cx).trim().is_empty() {
            return;
        }
        let Some(repository) = self.repository() else {
            self.ask(window, cx);
            return;
        };
        self.queueing = true;
        cx.emit(Asked::Queue {
            words: self.words(cx),
            repository,
            model: self.model.as_ref().map(|one| one.to_string()),
            effort: self.effort.as_ref().map(|one| one.to_string()),
        });
        cx.notify();
    }

    /// Fleet took it: the words go, the sheet clears and shuts.
    pub fn answered(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sending = false;
        self.drafting = false;
        self.queueing = false;
        self.waiting = false;
        crate::drafts::clear(KEEP_REQUEST, cx);
        self.attached.clear();
        self.wait = None;
        self.tagged = None;
        self.request.update(cx, |state, cx| state.set_value("", window, cx));
        self.close(window, cx);
    }

    /// Fleet refused, in its own words: the sheet is as it was, words and pick kept, and the row says why.
    pub fn refused(&mut self, why: SharedString, cx: &mut Context<Self>) {
        self.face.refused = Some(why);
        self.sending = false;
        self.drafting = false;
        self.queueing = false;
        self.waiting = false;
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
            self.drafting = false;
        }
        if let Some(wait) = self.face.wait.clone() {
            self.wait = Some(wait);
        }
        if self.waiting && self.face.reading != Reading::Detecting {
            self.waiting = false;
            self.sending = false;
            let held = self.drafting;
            self.begin(held, window, cx);
        }
        cx.notify();
    }

    /// What Fleet read of the brief's tag: the sentence over the request, or none where the tag names no wait.
    pub fn set_wait(&mut self, wait: Option<Wait>, cx: &mut Context<Self>) {
        self.wait = wait;
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
        self.face.refused = None;
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

    // -- keys ----------------------------------------------------------------------------------------

    pub(super) fn on_act(&mut self, key: &Act, window: &mut Window, cx: &mut Context<Self>) {
        match key.id {
            // The sheet is a dialog, and the key does nothing while one is up.
            "new_job" | "send_message" => {}
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
            "draft_session" => self.draft(window, cx),
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
            // Tab goes round the sheet and the card under it, and never out to the window behind.
            "tab" if plain && self.popup.is_none() => self.step_focus(mods.shift, window, cx),
            "enter" if plain && !mods.shift && in_request => self.act(window, cx),
            "enter" | "space" | "down" if plain && self.row.is_focused(window) => self.open_repositories(window, cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    // -- the order Tab takes ---------------------------------------------------------------------------

    /// Whether Queue for later can be pressed: the stop is left out while it cannot.
    pub(super) fn queue_live(&self, cx: &App) -> bool {
        self.face.connected && !self.sending && !self.queueing && !self.words(cx).trim().is_empty()
    }

    /// Whether Draft and Start can be pressed.
    pub(super) fn act_live(&self, cx: &App) -> bool {
        self.face.connected && !self.queueing && self.worded(cx) && !self.sending
    }

    /// Every stop Tab visits, in order: the wait sentence's parts when there is one, the request, the
    /// repository, Model, Effort, Queue for later, Draft, Start, then the card under the sheet. A
    /// control that is off, or has no card to draw, has no stop.
    pub(super) fn stops(&self, cx: &App) -> Vec<FocusHandle> {
        let mut stops = Vec::new();
        if let Some(wait) = &self.wait {
            let parts = if wait.needs.is_some() { 4 } else { 3 };
            stops.extend(self.parts[..parts].iter().cloned());
        }
        stops.push(self.request.focus_handle(cx));
        if self.face.connected {
            stops.extend([self.row.clone(), self.model_key.clone(), self.effort_key.clone()]);
        }
        let act = self.act_live(cx);
        if self.face.queue && self.attached.is_empty() && self.queue_live(cx) {
            stops.push(self.queue_key.clone());
        }
        if self.wait.is_none() && act {
            stops.push(self.draft_key.clone());
        }
        if act {
            stops.push(self.act_key.clone());
        }
        if self.here_in().is_some() {
            stops.push(self.here_key.clone());
        }
        stops
    }

    /// Tab, or Shift-Tab with `back`, from whichever stop has the keys.
    pub(super) fn step_focus(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let stops = self.stops(cx);
        let now = stops.iter().position(|stop| stop.is_focused(window));
        self.wait_menu = None;
        window.focus(&stops[next_stop(stops.len(), now, back)], cx);
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
            Pop::Repository => {
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

    /// The repository picker's list: every repository, the first nine with their ⌥ keys, and a search
    /// over them once there are more than nine.
    pub(super) fn open_repositories(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(from) = self.at.repository.get() else { return };
        let mut entries = Vec::new();
        if !self.face.more.is_empty() {
            entries.push(Entry::Search { label: "Find a repository", to_rule: false });
        }
        for chip in self.face.chips.iter().chain(&self.face.more) {
            let row = crate::dropdown::Row::new(id_of(format!("repo:{}", chip.name)), chip.name.clone()).ticked(chip.mark == Mark::Chosen);
            let row = match chip.key.and_then(|n| crate::keymap::shown_digit("dispatch_repository", n)) {
                Some(key) => row.key(key),
                None => row,
            };
            entries.push(Entry::Row(row));
        }
        self.open_pop(Pop::Repository, from, "Repository", entries, window, cx);
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

/// The stop after `now` among `len`, wrapping at both ends. Keys on nothing in the order start at its first stop, or its last going back.
fn next_stop(len: usize, now: Option<usize>, back: bool) -> usize {
    match (now, back) {
        (Some(at), false) => (at + 1) % len,
        (Some(at), true) => (at + len - 1) % len,
        (None, false) => 0,
        (None, true) => len - 1,
    }
}

const PLACEHOLDER: &str = "Describe the work, or paste a link to a ticket.";

/// The caret at the end of `text`, which `set_value` leaves at the start.
fn end_of(state: &mut TextareaState, text: &str, window: &mut Window, cx: &mut Context<TextareaState>) {
    let line = text.matches('\n').count() as u32;
    let last = text.rsplit('\n').next().unwrap_or("");
    state.set_cursor_position(Position::new(line, last.encode_utf16().count() as u32), window, cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};

    fn open<'a>(face: Face, here: Option<&'static str>, cx: &'a mut TestAppContext) -> (Entity<DispatchSheet>, &'a mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (sheet, cx) = cx.add_window_view(|window, cx| DispatchSheet::new(face, window, cx));
        sheet.update_in(cx, |sheet, window, cx| {
            sheet.set_here(here.map(|name| name.to_string().into()), cx);
            sheet.request.update(cx, |state, cx| state.set_value("Fix the flaky login test", window, cx));
            let request = sheet.request.focus_handle(cx);
            window.focus(&request, cx);
        });
        (sheet, cx)
    }

    /// The stops as the names this module gives its handles, in the order Tab visits them.
    fn named(sheet: &DispatchSheet, cx: &App) -> Vec<&'static str> {
        sheet
            .stops(cx)
            .iter()
            .map(|stop| {
                let is = |handle: &FocusHandle| handle == stop;
                if is(&sheet.request.focus_handle(cx)) {
                    "request"
                } else if is(&sheet.row) {
                    "repository"
                } else if is(&sheet.model_key) {
                    "model"
                } else if is(&sheet.effort_key) {
                    "effort"
                } else if is(&sheet.queue_key) {
                    "queue"
                } else if is(&sheet.draft_key) {
                    "draft"
                } else if is(&sheet.act_key) {
                    "start"
                } else if is(&sheet.here_key) {
                    "card"
                } else if sheet.parts.iter().any(is) {
                    "part"
                } else {
                    "unknown"
                }
            })
            .collect()
    }

    /// Where the keys are, by name.
    fn at(sheet: &DispatchSheet, window: &Window, cx: &App) -> &'static str {
        let now = sheet.stops(cx).iter().position(|stop| stop.is_focused(window)).expect("the keys are on a stop");
        named(sheet, cx)[now]
    }

    #[test]
    fn the_next_stop_wraps_at_both_ends() {
        assert_eq!(next_stop(3, Some(0), false), 1);
        assert_eq!(next_stop(3, Some(2), false), 0);
        assert_eq!(next_stop(3, Some(0), true), 2);
        assert_eq!(next_stop(3, Some(1), true), 0);
        assert_eq!(next_stop(3, None, false), 0);
        assert_eq!(next_stop(3, None, true), 2);
    }

    #[gpui::test]
    fn tab_goes_round_the_sheet_and_shift_tab_goes_back_the_same_way(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), Some("sample-api"), cx);
        let order = ["request", "repository", "model", "effort", "queue", "draft", "start", "card"];
        sheet.update_in(cx, |sheet, window, cx| {
            assert_eq!(named(sheet, cx), order);
            assert_eq!(at(sheet, window, cx), "request");
            for next in order.iter().skip(1).chain(Some(&"request")) {
                sheet.step_focus(false, window, cx);
                assert_eq!(at(sheet, window, cx), *next, "Tab");
            }
            // Back from the request: the last stop, then each one before it, to the request again.
            for next in order.iter().rev() {
                sheet.step_focus(true, window, cx);
                assert_eq!(at(sheet, window, cx), *next, "Shift-Tab");
            }
        });
    }

    #[gpui::test]
    fn a_stop_that_cannot_act_is_left_out(cx: &mut TestAppContext) {
        let (sheet, cx) = open(crate::dispatch::scenes::rest(&[]), None, cx);
        sheet.update_in(cx, |sheet, window, cx| {
            assert_eq!(named(sheet, cx), ["request", "repository", "model", "effort", "queue", "draft", "start"], "no card where the title bar names nowhere");
            sheet.request.update(cx, |state, cx| state.set_value("", window, cx));
            assert_eq!(named(sheet, cx), ["request", "repository", "model", "effort"], "nothing to start or queue");
            sheet.face.connected = false;
            assert_eq!(named(sheet, cx), ["request"], "Fleet unreachable: everything else is off");
        });
    }

    #[gpui::test]
    fn the_wait_sentence_stands_before_the_request(cx: &mut TestAppContext) {
        let face = crate::dispatch::scenes::waiting(crate::dispatch::scenes::rest(&[]), None);
        let (sheet, cx) = open(face, Some("sample-api"), cx);
        sheet.update_in(cx, |sheet, window, cx| {
            assert_eq!(named(sheet, cx), ["part", "part", "part", "part", "request", "repository", "model", "effort", "queue", "start", "card"], "a wait has no Draft");
            sheet.step_focus(true, window, cx);
            assert_eq!(at(sheet, window, cx), "part", "Shift-Tab from the request enters the sentence");
            assert!(sheet.parts[3].is_focused(window));
        });
    }
}
