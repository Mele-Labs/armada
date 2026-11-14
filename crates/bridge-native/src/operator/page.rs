//! The page's state and acts: which item is open, what he answered this sitting, the chat on Ask. Every
//! press answers at once: the page moves first, says so with a pending mark, and takes Fleet's page when
//! it comes. A refusal puts the item back and says why on it.

use std::collections::{BTreeSet, HashMap};

use gpui::{App, AppContext as _, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable as _, ScrollHandle, SharedString, Subscription, Task, Window};
use ipc::operator_board::{DispatchWay, OperatorDispatched};
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorAnswered, OperatorPage, OperatorThen, OperatorTurnItem};
use ipc::{SessionId, WaitingAct};

use super::chat::{ChatAsked, OperatorChat};
use super::composer::{Composed, OperatorComposer, Outgoing};
use super::face::{self, Kind};
use super::parked::{ParkedAsked, ParkedIdeas};
use super::scenes::{self, MockFleet};
use super::wire::{Failed, Wire};
use crate::dropdown::{Dropdown, Entry, Picked, Row};
use crate::look::ActiveLook as _;
use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::log::Note;

/// An item answered this sitting, struck through under Your turn.
#[derive(Clone, Debug, PartialEq)]
pub struct Done {
    pub id: String,
    pub kind: Kind,
    pub title: SharedString,
    /// What came of it, in a few words. Absent while Fleet has not answered.
    pub said: Option<SharedString>,
    /// When he pressed it: Answered today holds the presses of his local day.
    pub at: chrono::DateTime<chrono::Local>,
}

/// What the page asks of the window around it: a place it does not draw.
#[derive(Clone, Debug, PartialEq)]
pub enum Opened {
    /// The act of an item Fleet cannot do itself, such as a walk to open or a pull request, from the
    /// Job that holds it.
    Act(WaitingAct, Option<SessionId>),
    /// A Job, from a row of the digest.
    Job(SessionId),
}

/// Items that joined Your turn since the page was last read, oldest first: what raises a toast. Jobs
/// screens own toasts (`banner.rs`, `notify.rs`) and subscribe to this on [`super::live`].
#[derive(Clone, Debug, PartialEq)]
pub struct Arrived(pub Vec<OperatorTurnItem>);

/// The digest's tab stops: each tile's head, and the links under them.
pub const STOPS: [&str; 8] = ["Landed", "What I handled", "Running", "Coming up", "Learned", "Holds", "The whole Log", "Every rule"];

impl EventEmitter<Opened> for OperatorPageView {}
impl EventEmitter<Arrived> for OperatorPageView {}

pub struct OperatorPageView {
    pub(super) wire: Wire,
    /// Fleet's page, with this sitting's presses already on it. `None` until it is read: a skeleton.
    pub(super) page: Option<OperatorPage>,
    /// The date, and when he came back, as RFC 3339 so it flips with the page's other times.
    pub(super) day: Option<SharedString>,
    pub(super) back_at: Option<String>,
    /// The item open, by id: one at a time.
    pub(super) open: Option<String>,
    /// The open card is to be scrolled into view at the next frame: set when the keys or an answer move it.
    pub(super) reveal: bool,
    pub(super) done: Vec<Done>,
    /// The items whose press is in flight.
    pub(super) pending: BTreeSet<String>,
    /// A press Fleet refused, on that item, with why.
    pub(super) refused: Option<(String, SharedString)>,
    /// The Then he picked on a go-ahead to file, where he picked one; else the Operator's.
    pub(super) then: HashMap<String, OperatorThen>,
    /// The option he picked on an item that offers more than one, by its label; else the Operator's.
    pub(super) option: HashMap<String, String>,
    pub(super) chat: Option<Entity<OperatorChat>>,
    /// The box at the page's foot, whose send is a dispatch to the Operator.
    pub(super) composer: Entity<OperatorComposer>,
    /// The ideas the Operator kept for later.
    pub(super) parked: Entity<ParkedIdeas>,
    parked_reading: bool,
    /// The digest's tiles he has folded or opened, by name, against how they open.
    pub(super) folded: HashMap<&'static str, bool>,
    pub focus: FocusHandle,
    /// The pane from a Job: the chat and a short list.
    pub(super) pane: bool,
    /// Every time on the page read as its age, after a press on one: they flip together, as the thread's do.
    pub(super) aged: bool,
    /// The clock the page reads its times against: the walk's 13:05, else this Mac's.
    pub(super) clock_now: Option<chrono::DateTime<chrono::Local>>,
    /// Switch on pressed, and Fleet has not answered; or why it refused.
    pub(super) switching: bool,
    pub(super) switch_refused: Option<SharedString>,
    /// A Mac waiting to pair whose Confirm or Decline is in flight, by its call's key.
    pub(super) pairing_pending: BTreeSet<String>,
    /// What the Gateway refused on a Mac waiting to pair, by its call's key, with its sentence.
    pub(super) pairing_refused: Option<(String, SharedString)>,
    /// The Mac waiting to pair whose card a banner brought to the front.
    pub(super) pairing_front: Option<String>,
    /// The front pairing card's Confirm, where a banner's press puts the keys.
    pub(super) pairing_focus: FocusHandle,
    /// The front card waits to be scrolled to and given the keys at the next frame.
    pub(super) pairing_reveal: bool,
    /// The digest's tiles and links, each a tab stop that Enter presses, by name.
    pub(super) stops: HashMap<&'static str, FocusHandle>,
    /// Tiles drawing every row, past the first few.
    pub(super) whole: BTreeSet<&'static str>,
    /// Every rule, on a sheet over the page, with the row the arrows are on. `None` while it is shut.
    pub(super) rules: Option<usize>,
    pub(super) rules_focus: FocusHandle,
    pub(super) rules_scroll: ScrollHandle,
    /// The week's Log, on a sheet over the page: `Some("")` while it is read, its words once read.
    pub(super) log: Option<String>,
    pub(super) log_focus: FocusHandle,
    pub(super) log_scroll: ScrollHandle,
    pub(super) main_scroll: ScrollHandle,
    /// The page's first tab stop, its first pill: where Tab goes from the page itself.
    pub(super) first_stop: FocusHandle,
    /// The open card's Later, so a test can put the keys on a press.
    pub(super) later_focus: FocusHandle,
    /// Where each ask's Later is drawn, by its id, so the menu opens under the open ask's.
    pub(super) later_bounds: std::rc::Rc<std::cell::RefCell<HashMap<String, gpui::Bounds<gpui::Pixels>>>>,
    /// Where each ask of a Job's card sits in the scrolled content, by its id, so a key that opens one shows it.
    pub(super) ask_bounds: std::rc::Rc<std::cell::RefCell<HashMap<String, gpui::Bounds<gpui::Pixels>>>>,
    /// Later's menu of when the item comes back, while it is open.
    pub(super) later_menu: Option<(Entity<Dropdown>, [Subscription; 2])>,
    /// The turn ids already seen, so a new one is told once. `None` before the first read.
    seen: Option<BTreeSet<String>>,
    read_at: u64,
    reading: bool,
    subs: Vec<Subscription>,
}

impl OperatorPageView {
    fn new(wire: Wire, pane: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subs = Vec::new();
        // Whenever the window gains focus or is shown: Fleet keeps when he was last in front.
        subs.push(cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.in_front(cx);
            }
        }));
        if pane {
            cx.defer_in(window, |this, window, cx| this.open_pane_chat(window, cx));
        }
        let fleet = match &wire {
            Wire::Live(fleet) => Some(fleet.clone()),
            Wire::Mock(_) => None,
        };
        let composer = cx.new(|cx| OperatorComposer::new(fleet, window, cx));
        let parked = cx.new(|_| ParkedIdeas::new(None));
        subs.push(cx.subscribe_in(&composer, window, |this, _, event: &Composed, window, cx| this.composed(event, window, cx)));
        subs.push(cx.subscribe(&parked, |this, _, asked: &ParkedAsked, cx| this.idea_asked(asked, cx)));
        OperatorPageView {
            wire,
            page: None,
            day: None,
            back_at: None,
            open: None,
            reveal: false,
            done: Vec::new(),
            pending: BTreeSet::new(),
            refused: None,
            then: HashMap::new(),
            option: HashMap::new(),
            chat: None,
            composer,
            parked,
            parked_reading: false,
            folded: HashMap::new(),
            focus: cx.focus_handle(),
            pane,
            aged: false,
            clock_now: None,
            switching: false,
            switch_refused: None,
            pairing_pending: BTreeSet::new(),
            pairing_refused: None,
            pairing_front: None,
            pairing_focus: cx.focus_handle(),
            pairing_reveal: false,
            stops: STOPS.iter().map(|name| (*name, cx.focus_handle())).collect(),
            whole: BTreeSet::new(),
            rules: None,
            rules_focus: cx.focus_handle(),
            rules_scroll: ScrollHandle::new(),
            log: None,
            log_focus: cx.focus_handle(),
            log_scroll: ScrollHandle::new(),
            main_scroll: ScrollHandle::new(),
            first_stop: cx.focus_handle(),
            later_focus: cx.focus_handle(),
            later_bounds: Default::default(),
            ask_bounds: Default::default(),
            later_menu: None,
            seen: None,
            read_at: 0,
            reading: false,
            subs,
        }
    }

    /// The page on the owner's Fleet: read when the port is known, and again on each `operator.page_changed`.
    pub fn live(fleet: &Entity<FleetConnection>, pane: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Wire::Live(fleet.clone()), pane, window, cx);
        this.subs.push(cx.observe(fleet, |this, fleet, cx| {
            let held = fleet.read(cx).held();
            // A Mac confirmed or declined stays pressed until Fleet's calls drop it.
            let waiting: Vec<String> = held.calls.iter().flat_map(|calls| calls.calls.iter()).filter(|call| call.kind == ipc::StandingCallKind::MacPairing).map(|call| call.key.clone()).collect();
            this.pairing_pending.retain(|key| waiting.contains(key));
            let moved = held.operator_page_moved;
            let ready = held.port().is_some();
            if ready && (moved != this.read_at || (this.page.is_none() && !this.reading)) {
                this.read_at = moved;
                this.read(cx);
            }
            if ready && !this.parked.read(cx).is_read() && !this.parked_reading {
                this.read_parked(cx);
            }
        }));
        // An idea kept for later is on the list as the Operator says it was kept.
        this.subs.push(cx.subscribe(fleet, |this, _, done: &crate::fleet::Dispatched, cx| {
            if done.0.iter().any(|one| one.way == DispatchWay::Parked) {
                this.read_parked(cx);
            }
        }));
        if window.is_window_active() {
            this.in_front(cx);
        }
        this.read(cx);
        this
    }

    /// The page on the walk's Fleet, at 13:05 as the sketch drew it.
    pub fn mock(pane: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Wire::Mock(Box::new(MockFleet::new())), pane, window, cx);
        this.day = Some(scenes::day());
        this.back_at = Some(scenes::now().to_rfc3339());
        this.clock_now = Some(scenes::now());
        this.parked.update(cx, |parked, _| parked.set_clock(Some(scenes::now())));
        this.read(cx);
        this.read_parked(cx);
        this
    }

    pub fn mock_fleet(&mut self) -> Option<&mut MockFleet> {
        match &mut self.wire {
            Wire::Mock(mock) => Some(mock),
            Wire::Live(_) => None,
        }
    }

    /// The owner's Fleet, where the page reads from it; `None` on the walk's.
    pub fn fleet(&self) -> Option<&Entity<FleetConnection>> {
        match &self.wire {
            Wire::Live(fleet) => Some(fleet),
            Wire::Mock(_) => None,
        }
    }

    /// Whether Fleet's page has been read once: before it, Your turn is not known to be empty.
    pub fn is_read(&self) -> bool {
        self.page.is_some()
    }

    pub fn items(&self) -> &[OperatorTurnItem] {
        self.page.as_ref().map_or(&[], |page| page.turn.as_slice())
    }

    pub fn open_item(&self) -> Option<&OperatorTurnItem> {
        let open = self.open.as_deref()?;
        self.items().iter().find(|turn| turn.id == open)
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.items().iter().position(|turn| turn.id == id)
    }

    /// The Then a Yes would carry: his pick, else the Operator's. `None` on anything but a file.
    pub fn then_of(&self, turn: &OperatorTurnItem) -> Option<OperatorThen> {
        self.then.get(&turn.id).copied().or_else(|| turn.then.iter().find(|offer| offer.pick).map(|offer| offer.then))
    }

    /// The option a Yes would carry, on an item that offers more than one: his pick, else the Operator's,
    /// else the first. `None` on any other item.
    pub fn option_of(&self, turn: &OperatorTurnItem) -> Option<SharedString> {
        if !face::chooses(turn) {
            return None;
        }
        let offered = |label: &str| turn.item.options.iter().any(|option| option.label == label);
        self.option
            .get(&turn.id)
            .filter(|label| offered(label))
            .map(|label| SharedString::from(label.clone()))
            .or_else(|| face::pick_of(turn).filter(|pick| offered(pick)))
            .or_else(|| turn.item.options.first().map(|option| SharedString::from(option.label.clone())))
    }

    /// A request's follow-up runs to its end however the page moves on: each holds the page weakly.
    fn track(&mut self, task: Task<()>) {
        task.detach();
    }

    // -- reading -----------------------------------------------------------------------------------

    pub fn read(&mut self, cx: &mut Context<Self>) {
        self.reading = true;
        let task = self.wire.page(cx);
        let task = cx.spawn(async move |this, cx| {
            let read = task.await;
            let _ = this.update(cx, |this, cx| {
                this.reading = false;
                match read {
                    Ok(page) => this.set_page(page, cx),
                    Err(why) => Note::warn("the Operator's page could not be read").field("why", why).emit(),
                }
            });
        });
        self.track(task);
    }

    /// Fleet's page replaces the one held. The open item stays open where it is still there, and an item
    /// not seen before is told once.
    /// An item he answered here stays out of Your turn, though Fleet keeps one whose act it left to him
    /// to open until he does it.
    pub fn set_page(&mut self, mut page: OperatorPage, cx: &mut Context<Self>) {
        let ids: BTreeSet<String> = page.turn.iter().map(|turn| turn.id.clone()).collect();
        if let Some(seen) = &self.seen {
            let new: Vec<OperatorTurnItem> = page.turn.iter().filter(|turn| !seen.contains(&turn.id)).cloned().collect();
            if !new.is_empty() {
                cx.emit(Arrived(new));
            }
        }
        self.seen = Some(ids.clone());
        page.turn.retain(|turn| !self.done.iter().any(|done| done.id == turn.id));
        let ids: BTreeSet<String> = page.turn.iter().map(|turn| turn.id.clone()).collect();
        self.then.retain(|id, _| ids.contains(id));
        let keep = self.open.as_ref().filter(|open| ids.contains(*open)).cloned();
        self.open = keep.or_else(|| page.turn.first().map(|turn| turn.id.clone()));
        self.page = Some(page);
        cx.notify();
    }

    // -- the keys and presses ----------------------------------------------------------------------

    pub fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        // ⌥M puts the caret in the box from anywhere on the page, as it does in a Job's view.
        if act.is("message_job") && !self.pane {
            return self.composer.update(cx, |composer, cx| composer.focus(window, cx));
        }
        if self.composer.read(cx).has_keys(window, cx) {
            // ⌘Enter sends from the box; every other act is the box's to take, or goes on.
            if act.is("send_message") {
                self.composer.update(cx, |composer, cx| composer.send(window, cx));
            } else {
                cx.propagate();
            }
            return;
        }
        if self.chat_has_keys(window, cx) {
            return cx.propagate();
        }
        // The Log's sheet scrolls on the arrows, and no key reaches Your turn behind it.
        if self.log.is_some() {
            match act.id {
                "call_move_focus" => self.scroll_log(if act.slot % 2 == 0 { 1. } else { -1. }, cx),
                id if id.starts_with("call_") => {}
                _ => cx.propagate(),
            }
            return;
        }
        // The sheet over the page takes the arrows and Enter, and no key reaches Your turn behind it.
        if self.rules.is_some() {
            match act.id {
                "call_move_focus" => self.step_rule(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
                "call_send" => cx.propagate(),
                "call_forget" => self.forget_selected_rule(cx),
                id if id.starts_with("call_") => {}
                _ => cx.propagate(),
            }
            return;
        }
        match act.id {
            // The registry's slots are j, k, ↓, ↑: even slots go down.
            "call_move_focus" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "call_yes" => self.answer(OperatorAnswer::Yes, window, cx),
            "call_always_here" => self.answer(OperatorAnswer::AlwaysHere, window, cx),
            "call_always" => self.answer(OperatorAnswer::Always, window, cx),
            "call_dismiss" => self.answer(OperatorAnswer::No, window, cx),
            "call_send" => self.enter(window, cx),
            "call_send_changes" => self.send_changes(cx),
            "call_later" => self.open_later(true, window, cx),
            "call_ask" => self.ask(window, cx),
            // ⌥↑ then ⌥↓.
            "call_reorder" => self.shove(if act.slot == 0 { -1 } else { 1 }, cx),
            "call_pick" => self.digit(act.digit.unwrap_or(0) as usize, cx),
            // `f` among them: a rule's or a hold's Forget takes it itself, where it has the keys.
            _ => cx.propagate(),
        }
    }

    fn chat_has_keys(&self, window: &Window, cx: &App) -> bool {
        self.chat.as_ref().is_some_and(|chat| chat.read(cx).focus_handle(cx).contains_focused(window, cx))
    }

    /// Enter where the page itself has the keys: Yes. A press with the keys, such as Later, a tile or a
    /// time, takes Enter itself and does what a click does: gpui runs the binding first, so it is handed on.
    fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus.is_focused(window) {
            self.answer(OperatorAnswer::Yes, window, cx);
        } else {
            cx.propagate();
        }
    }

    /// A tile's head folds or opens it; a link does what it says.
    pub fn press_stop(&mut self, name: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        match name {
            "The whole Log" => self.open_log(window, cx),
            "Every rule" => self.open_rules(window, cx),
            tile => {
                let open = self.folded.get(tile).copied().unwrap_or(super::view::opens_unpressed(tile));
                self.folded.insert(tile, !open);
                cx.notify();
            }
        }
    }

    /// Send changes, on a plan: its Job, where the plan takes changes. Nothing is answered.
    pub fn send_changes(&mut self, cx: &mut Context<Self>) {
        let Some(turn) = self.open_item().filter(|turn| face::sends_changes(turn)) else { return };
        if let Some(job) = turn.session_id.clone() {
            cx.emit(Opened::Job(job));
        }
    }

    /// A press on a time: every time on the page reads the other way.
    pub fn flip_stamps(&mut self, cx: &mut Context<Self>) {
        self.aged = !self.aged;
        cx.notify();
    }

    /// Switch on, where the page says the Operator is off: the setting Electron's Permissions tab saves.
    /// The page reads on at once, and goes back with why if Fleet refuses. Saved, Fleet's
    /// `operator.page_changed` redraws it; the mock sends no event, so it is read in its place.
    pub fn switch_on(&mut self, cx: &mut Context<Self>) {
        if self.switching {
            return;
        }
        self.switching = true;
        self.switch_refused = None;
        if let Some(page) = self.page.as_mut() {
            page.on = true;
        }
        cx.notify();
        let task = self.wire.switch(true, cx);
        let task = cx.spawn(async move |this, cx| {
            let saved = task.await;
            let _ = this.update(cx, |this, cx| {
                this.switching = false;
                match saved {
                    Ok(()) if !this.wire.is_live() => this.read(cx),
                    Ok(()) => cx.notify(),
                    Err(why) => {
                        if let Some(page) = this.page.as_mut() {
                            page.on = false;
                        }
                        this.switch_refused = Some(why.into());
                        cx.notify();
                    }
                }
            });
        });
        self.track(task);
    }

    // -- the Log ----------------------------------------------------------------------------------

    /// The whole Log: the week's journal on a sheet, read from where Fleet says it is.
    pub fn open_log(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.page.as_ref().and_then(|page| page.digest.handled.log.as_ref()).map(|log| log.path.clone()) else { return };
        self.log = Some(String::new());
        window.focus(&self.log_focus, cx);
        cx.notify();
        let read = self.wire.log(path, cx);
        let task = cx.spawn(async move |this, cx| {
            let read = read.await;
            let _ = this.update(cx, |this, cx| {
                if this.log.is_some() {
                    this.log = Some(read.unwrap_or_else(|why| format!("The Log did not read: {why}")));
                    cx.notify();
                }
            });
        });
        self.track(task);
    }

    /// Esc on the Log: shut, and the keys back on the link that opened it.
    pub fn close_log(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.log = None;
        match self.stops.get("The whole Log") {
            Some(keys) => window.focus(keys, cx),
            None => window.focus(&self.focus, cx),
        }
        cx.notify();
    }

    fn scroll_log(&mut self, by: f32, cx: &mut Context<Self>) {
        let step = gpui::px(60.);
        let offset = self.log_scroll.offset();
        self.log_scroll.set_offset(gpui::point(offset.x, (offset.y - step * by).min(gpui::px(0.))));
        cx.notify();
    }

    // -- every rule --------------------------------------------------------------------------------

    /// Every rule, in the sheet's order: everywhere first, then each project by name.
    pub fn rules_in_order(&self) -> Vec<&ipc::operator_page::OperatorLearned> {
        let mut rules: Vec<_> = self.page.as_ref().map_or(Vec::new(), |page| page.digest.learned.iter().collect());
        rules.sort_by(|a, b| (a.repository.is_some(), a.repository.as_deref()).cmp(&(b.repository.is_some(), b.repository.as_deref())));
        rules
    }

    pub fn open_rules(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.rules = Some(0);
        window.focus(&self.rules_focus, cx);
        cx.notify();
    }

    /// Esc on the sheet: shut, and the keys back on the link that opened it.
    pub fn close_rules(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.rules = None;
        match self.stops.get("Every rule") {
            Some(keys) => window.focus(keys, cx),
            None => window.focus(&self.focus, cx),
        }
        cx.notify();
    }

    fn step_rule(&mut self, by: isize, cx: &mut Context<Self>) {
        let rules = self.rules_in_order();
        let Some(at) = self.rules else { return };
        let at = (at as isize + by).clamp(0, rules.len().saturating_sub(1) as isize) as usize;
        // The sheet's list holds a heading before each place's first rule: the row's place counts them.
        let heads = rules.iter().take(at + 1).enumerate().filter(|(n, rule)| *n == 0 || rules[n - 1].repository != rule.repository).count();
        self.rules_scroll.scroll_to_item(at + heads);
        self.rules = Some(at);
        cx.notify();
    }

    /// Forget: the rule or hold stops standing. Its row goes at once; Fleet's `operator.page_changed`
    /// redraws the page, and a refusal reads it again, which puts the row back.
    pub fn forget(&mut self, id: i64, cx: &mut Context<Self>) {
        if let Some(page) = self.page.as_mut() {
            page.digest.learned.retain(|rule| rule.id != id);
            page.digest.holds.retain(|hold| hold.id != id);
        }
        if let Some(at) = self.rules {
            self.rules = Some(at.min(self.rules_in_order().len().saturating_sub(1)));
        }
        cx.notify();
        let task = self.wire.forget(id, cx);
        let task = cx.spawn(async move |this, cx| {
            let forgot = task.await;
            let _ = this.update(cx, |this, cx| match forgot {
                Ok(Some(page)) => this.set_page(page, cx),
                Ok(None) => {}
                Err(why) => {
                    Note::warn("the Operator's page did not forget it").field("why", why).emit();
                    this.read(cx);
                }
            });
        });
        self.track(task);
    }

    /// Forget pressed on a row: the row and its Forget go, so the keys go to the sheet, or the tile's
    /// head. A tile left with nothing is not drawn, so then the keys go to the other tile, or the page.
    pub fn forget_pressed(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        let held = self.page.as_ref().is_some_and(|page| page.digest.holds.iter().any(|hold| hold.id == id));
        self.forget(id, cx);
        let (rules, holds) = self.page.as_ref().map_or((false, false), |page| (!page.digest.learned.is_empty(), !page.digest.holds.is_empty()));
        let tile = match (held, holds, rules) {
            (true, true, _) | (false, true, false) => Some("Holds"),
            (_, _, true) => Some("Learned"),
            _ => None,
        };
        let keys = if self.rules.is_some() { Some(&self.rules_focus) } else { tile.and_then(|tile| self.stops.get(tile)) };
        window.focus(keys.unwrap_or(&self.focus), cx);
    }

    /// `f` on Every rule: the selected rule.
    fn forget_selected_rule(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.rules.and_then(|at| self.rules_in_order().get(at).map(|rule| rule.id)) {
            self.forget(id, cx);
        }
    }

    /// Open the next or the previous item.
    pub fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let count = self.items().len();
        if count == 0 {
            return;
        }
        let at = self.open.as_deref().and_then(|id| self.index_of(id)).unwrap_or(0) as isize;
        let to = (at + by).clamp(0, count as isize - 1) as usize;
        self.open = Some(self.items()[to].id.clone());
        self.reveal = true;
        cx.notify();
    }

    pub fn open_id(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.index_of(id).is_some() {
            self.open = Some(id.to_string());
            cx.notify();
        }
    }

    /// A digit: on a go-ahead to file, the Then in that place; on an item with options, the option in that
    /// place. Each answer has a letter of its own.
    pub fn digit(&mut self, digit: usize, cx: &mut Context<Self>) {
        let Some(turn) = self.open_item().cloned() else { return };
        if face::chooses(&turn) {
            if let Some(option) = digit.checked_sub(1).and_then(|at| turn.item.options.get(at)) {
                self.option.insert(turn.id.clone(), option.label.clone());
                cx.notify();
            }
            return;
        }
        // On a go-ahead to file, the digit is that outcome's press.
        let Some(offer) = digit.checked_sub(1).and_then(|at| turn.then.get(at)) else { return };
        self.outcome(offer.then, cx);
    }

    /// One outcome of a go-ahead to file, pressed: Yes with that Then.
    pub fn outcome(&mut self, then: OperatorThen, cx: &mut Context<Self>) {
        let Some(turn) = self.open_item().cloned() else { return };
        self.then.insert(turn.id.clone(), then);
        self.answer_now(OperatorAnswer::Yes, cx);
    }

    pub fn pick_option(&mut self, id: &str, label: &str, cx: &mut Context<Self>) {
        self.option.insert(id.to_string(), label.to_string());
        cx.notify();
    }

    /// One press on the open item. It leaves Your turn at once for Answered today, with a pending mark,
    /// and the next item opens.
    pub fn answer(&mut self, answer: OperatorAnswer, _window: &mut Window, cx: &mut Context<Self>) {
        self.answer_now(answer, cx);
    }

    fn answer_now(&mut self, answer: OperatorAnswer, cx: &mut Context<Self>) {
        let Some(turn) = self.open_item().cloned() else { return };
        if !face::answers_of(&turn).contains(&answer) {
            return;
        }
        // A pick of Later is done by Yes: the item goes Later, nothing is answered.
        if answer == OperatorAnswer::Yes && face::picks_later(&turn) && self.option.get(&turn.id).is_none() {
            return self.later(None, cx);
        }
        let then = (!turn.then.is_empty()).then(|| self.then_of(&turn)).flatten();
        // The option goes out only where it is not the Operator's pick: a press that names none takes the pick.
        let option = self.option_of(&turn).filter(|label| answer != OperatorAnswer::No && face::pick_of(&turn).as_ref() != Some(label)).map(|label| label.to_string());
        let press = OperatorAnswerPress { id: turn.id.clone(), answer, then: then.filter(|_| answer != OperatorAnswer::No), option, project: None };
        self.send(turn, press, cx);
    }

    /// The press goes to Fleet, and its item to Answered today: once, however often it is sent.
    fn send(&mut self, turn: OperatorTurnItem, press: OperatorAnswerPress, cx: &mut Context<Self>) {
        let at = self.index_of(&turn.id).unwrap_or(0);
        let was_open = self.open.as_deref() == Some(turn.id.as_str());
        self.take_out(&turn.id);
        self.done.retain(|done| done.id != turn.id);
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        self.done.push(Done { id: turn.id.clone(), kind: face::kind_of(&turn), title: face::title_of(&turn), said: None, at: now });
        self.pending.insert(turn.id.clone());
        self.refused = None;
        if was_open || self.open.is_none() {
            self.open_near(at, turn.group.as_deref());
        }
        cx.notify();
        let answer = press.answer;
        let task = self.wire.answer(press.clone(), cx);
        let task = cx.spawn(async move |this, cx| {
            let answered = task.await;
            let _ = this.update(cx, |this, cx| this.answered(&turn, &press, answer, answered, cx));
        });
        self.track(task);
    }

    fn answered(&mut self, turn: &OperatorTurnItem, press: &OperatorAnswerPress, answer: OperatorAnswer, answered: Result<OperatorAnswered, Failed>, cx: &mut Context<Self>) {
        self.pending.remove(&turn.id);
        match answered {
            Ok(answered) => {
                let chosen = self.option_of(turn);
                if let Some(done) = self.done.iter_mut().find(|done| done.id == turn.id) {
                    done.said = Some(said_of(turn, answer, chosen, &answered));
                }
                if let Some(act) = answered.open.clone() {
                    cx.emit(Opened::Act(act, turn.session_id.clone()));
                }
                self.set_page(answered.page, cx);
            }
            Err(failed) => {
                // Back where it was, open: with Fleet's why under it, or a toast to send it again.
                self.done.retain(|done| done.id != turn.id);
                if let Some(page) = self.page.as_mut() {
                    if !page.turn.iter().any(|one| one.id == turn.id) {
                        page.turn.insert(0, turn.clone());
                    }
                }
                self.open = Some(turn.id.clone());
                match failed {
                    Failed::Refused(why) => self.refused = Some((turn.id.clone(), why.into())),
                    Failed::Lost(why) => {
                        let page = cx.entity().downgrade();
                        let (turn, press) = (turn.clone(), press.clone());
                        let sentence = format!("Your answer to {} did not reach Fleet: {why}", face::card_title(&turn));
                        let retry: crate::toasts::Retry = std::rc::Rc::new(move |cx: &mut App| {
                            let (turn, press) = (turn.clone(), press.clone());
                            let _ = page.update(cx, |page, cx| page.send(turn, press, cx));
                        });
                        cx.defer(move |cx| crate::toasts::tell_retry(sentence, retry, cx));
                    }
                }
                cx.notify();
            }
        }
    }

    /// What he answered on his local day, oldest first, for Answered today.
    pub fn answered_today(&self) -> Vec<&Done> {
        let today = self.clock_now.unwrap_or_else(chrono::Local::now).date_naive();
        self.done.iter().filter(|done| done.at.date_naive() == today).collect()
    }

    fn take_out(&mut self, id: &str) {
        if let Some(page) = self.page.as_mut() {
            page.turn.retain(|turn| turn.id != id);
        }
    }

    /// Open the item now at `at`, or the last one. An ask that leaves a Job's card leaves it open on the
    /// Job's ask beside it.
    fn open_near(&mut self, at: usize, group: Option<&str>) {
        let items = self.items();
        let mate = group.and_then(|group| [Some(at), at.checked_sub(1)].into_iter().flatten().filter_map(|at| items.get(at)).find(|turn| turn.group.as_deref() == Some(group)));
        self.open = mate.or(items.get(at)).or(items.last()).map(|turn| turn.id.clone());
        self.reveal = true;
    }

    /// Later's menu, under the open card's Later: when the item comes back. From the keys, they rest on
    /// its first choice; picked, the item goes Later until then.
    pub fn open_later(&mut self, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(turn) = self.open_item().cloned() else { return };
        let Some(bounds) = self.later_bounds.borrow().get(&turn.id).copied() else { return };
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let choices = face::later_choices(now);
        let entries = choices.iter().map(|(id, label, at)| Entry::Row(Row::new(id, *label).detail(face::back_at(*at, now)))).collect();
        let gap = cx.look().length(crate::look::token::SPACE_1);
        let at = gpui::point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height + gap);
        let menu = cx.new(|cx| Dropdown::new("Later", entries, at, gpui::Anchor::TopRight, from_keys, cx));
        let id = turn.id.clone();
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id: choice, .. } = picked;
            this.later_menu = None;
            // The menu goes with the keys on it: they come back to the page, as Esc brings them.
            window.focus(&this.focus, cx);
            let until = choices.iter().find(|(id, ..)| id == choice).map(|(_, _, at)| *at);
            if until.is_some() && this.open.as_deref() == Some(id.as_str()) {
                this.later(until, cx);
            }
            cx.notify();
        });
        let shut = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            this.later_menu = None;
            window.focus(&this.focus, cx);
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.later_menu = Some((menu, [picked, shut]));
        cx.notify();
    }

    /// Later: the open item goes last until `until`, or for the rest of the day, and the next one opens.
    pub fn later(&mut self, until: Option<chrono::DateTime<chrono::Local>>, cx: &mut Context<Self>) {
        let Some(turn) = self.open_item().cloned() else { return };
        let at = self.index_of(&turn.id).unwrap_or(0);
        let until = until.map(|until| until.to_rfc3339());
        if let Some(page) = self.page.as_mut() {
            page.turn.retain(|one| one.id != turn.id);
            let mut moved = turn.clone();
            moved.later_until = Some(ipc::Instant::carried(until.clone().unwrap_or_default()));
            page.turn.push(moved);
        }
        if self.items().len() > 1 {
            self.open_near(at, turn.group.as_deref());
        }
        self.refused = None;
        cx.notify();
        let task = self.wire.later(&turn.id, until, cx);
        // Refused, as a time already passed is, the item opens again where Fleet keeps it, saying why.
        let task = cx.spawn(async move |this, cx| {
            let settled = task.await;
            let _ = this.update(cx, |this, cx| match settled {
                Ok(page) => this.set_page(page, cx),
                Err(why) => {
                    this.refused = Some((turn.id.clone(), why.into()));
                    this.open = Some(turn.id.clone());
                    this.reveal = true;
                    this.read(cx);
                }
            });
        });
        self.track(task);
    }

    /// ⌥↑ and ⌥↓: the open item one place up or down, a Job's card whole. The keys stay on it.
    pub fn shove(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(id) = self.open.clone() else { return };
        let Some(at) = self.index_of(&id) else { return };
        let groups: Vec<Option<&str>> = self.items().iter().map(|turn| turn.group.as_deref()).collect();
        let run = run_at(&groups, at);
        let to = if by < 0 { run.start.checked_sub(1) } else { Some(run.end).filter(|end| *end < groups.len()) };
        let Some(to) = to else { return };
        self.move_to(&id, to, cx);
    }

    /// A drag, or a shove: the item goes to `to`, with the rest of its Job's card, and Fleet keeps the order.
    pub fn move_to(&mut self, id: &str, to: usize, cx: &mut Context<Self>) {
        let pairs: Vec<(&str, Option<&str>)> = self.items().iter().map(|turn| (turn.id.as_str(), turn.group.as_deref())).collect();
        let Some(ids) = moved(&pairs, id, to) else { return };
        let Some(page) = self.page.as_mut() else { return };
        page.turn.sort_by_key(|turn| ids.iter().position(|id| *id == turn.id));
        cx.notify();
        let task = self.wire.order(ids, cx);
        self.settle(task, cx);
    }

    /// Fleet's page after a Later or an order, or why it was refused, which reads the page again.
    fn settle(&mut self, task: Task<Result<OperatorPage, String>>, cx: &mut Context<Self>) {
        let task = cx.spawn(async move |this, cx| {
            let settled = task.await;
            let _ = this.update(cx, |this, cx| match settled {
                Ok(page) => this.set_page(page, cx),
                Err(why) => {
                    Note::warn("the Operator's page did not take the move").field("why", why).emit();
                    this.read(cx);
                }
            });
        });
        self.track(task);
    }

    // -- the chat ----------------------------------------------------------------------------------

    /// Ask: the chat about the open item, inside its card, with the keys in its box.
    pub fn ask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let turn = self.open_item().cloned();
        let about = turn.as_ref().map(|turn| turn.id.clone());
        let title = turn.as_ref().map(face::title_of);
        let chat = self.chat.clone().filter(|chat| chat.read(cx).about == about).unwrap_or_else(|| self.new_chat(about, title, window, cx));
        let keys = chat.read(cx).focus_handle(cx);
        self.chat = Some(chat);
        window.focus(&keys, cx);
        cx.notify();
    }

    fn new_chat(&mut self, about: Option<String>, title: Option<SharedString>, window: &mut Window, cx: &mut Context<Self>) -> Entity<OperatorChat> {
        let chat = cx.new(|cx| OperatorChat::new(about, title, window, cx));
        self.subs.push(cx.subscribe_in(&chat, window, |this, chat, asked: &ChatAsked, window, cx| match asked {
            ChatAsked::Leave => this.leave_chat(window, cx),
            ChatAsked::Send(words) => this.say(chat.clone(), words.clone(), window, cx),
        }));
        let task = self.wire.talk(cx);
        let reading = chat.downgrade();
        let task = cx.spawn(async move |_, cx| {
            let talk = task.await;
            let _ = reading.update(cx, |chat, cx| match talk {
                Ok(talk) => chat.set_lines(talk.lines, cx),
                Err(why) => {
                    Note::warn("the Operator's talk could not be read").field("why", why).emit();
                    chat.set_lines(Vec::new(), cx);
                }
            });
        });
        self.track(task);
        chat
    }

    fn say(&mut self, chat: Entity<OperatorChat>, words: String, window: &mut Window, cx: &mut Context<Self>) {
        let about = chat.read(cx).about.clone();
        let Some(task) = self.wire.say(words, about, cx) else { return };
        let task = cx.spawn_in(window, async move |_, cx| {
            let said = task.await;
            let _ = chat.update_in(cx, |chat, window, cx| match said {
                Ok(said) => chat.answered(said.said, String::new(), cx),
                Err(why) => chat.refused(why, window, cx),
            });
        });
        self.track(task);
    }

    /// Esc in the chat: the digest is back and the keys are on the page.
    pub fn leave_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pane {
            window.focus(&self.focus, cx);
            return;
        }
        self.chat = None;
        window.focus(&self.focus, cx);
        // The keys are back on the open item: the page shows it.
        self.main_scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// The pane holds its chat from the start, about nothing until a row is pressed.
    fn open_pane_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.chat.is_none() {
            let chat = self.new_chat(None, None, window, cx);
            self.chat = Some(chat);
        }
    }

    // -- in front ----------------------------------------------------------------------------------

    /// `operator_in_front`, sent each time the window gains focus or is shown. Where Fleet says he came
    /// back after time away, the head says when.
    pub fn in_front(&mut self, cx: &mut Context<Self>) {
        let task = self.wire.in_front(cx);
        let live = self.wire.is_live();
        let task = cx.spawn(async move |this, cx| {
            let front = task.await;
            let _ = this.update(cx, |this, cx| match front {
                Ok(front) if live && (front.rewriting || this.back_at.is_none()) => {
                    let now = chrono::Local::now();
                    this.day = Some(face::day(&now));
                    this.back_at = Some(now.to_rfc3339());
                    cx.notify();
                }
                Ok(_) => {}
                Err(why) => Note::info("Fleet was not told the window came to the front").field("why", why).emit(),
            });
        });
        self.track(task);
    }

    // -- the box and the ideas ---------------------------------------------------------------------

    /// The Jobs `@` names in the box.
    pub fn set_jobs(&mut self, jobs: Vec<(String, String)>, cx: &mut Context<Self>) {
        self.composer.update(cx, |composer, cx| composer.set_jobs(jobs, cx));
    }

    /// What the box asks: its dispatch goes to Fleet, or the keys come back to the page.
    fn composed(&mut self, event: &Composed, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            Composed::Leave => window.focus(&self.focus, cx),
            Composed::Send(out) => self.dispatch(out.clone(), window, cx),
        }
    }

    /// The box's send, as `dispatch_job`. The box empties on the answer, and keeps what it held on a refusal.
    fn dispatch(&mut self, out: Outgoing, window: &mut Window, cx: &mut Context<Self>) {
        let task = self.wire.dispatch(out.dispatch, cx);
        let words = out.words;
        cx.spawn_in(window, async move |this, cx| {
            let sent = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match sent {
                    Ok((done, events)) => {
                        this.composer.update(cx, |composer, cx| composer.answered(Ok(&done), window, cx));
                        this.told(&done, events, &words, cx);
                    }
                    Err(why) => this.composer.update(cx, |composer, cx| composer.answered(Err(why), window, cx)),
                }
            });
        })
        .detach();
    }

    /// Fleet took a dispatch. With the Operator holding it, or keeping it as an idea, he stays where he is and the toast
    /// that `operator.dispatched` raises says what became of it; the mock sends no event, so its answer brings them.
    /// A Job started with the Operator off opens, as a start from the sheet does.
    fn told(&mut self, done: &ipc::JobDispatched, events: Vec<OperatorDispatched>, words: &str, cx: &mut Context<Self>) {
        if let Some(id) = done.operator.as_deref().or(done.parked.as_deref()) {
            crate::toasts::dispatch_held(id, words, cx);
        }
        let kept = events.iter().any(|one| one.way == DispatchWay::Parked);
        for one in events {
            crate::toasts::dispatched(one, cx);
        }
        if kept || done.parked.is_some() {
            self.read_parked(cx);
        }
        if let Some(record) = &done.session {
            if self.page.as_ref().is_some_and(|page| !page.on) {
                cx.emit(Opened::Job(record.id.clone()));
            }
        }
    }

    /// The ideas Fleet keeps, read again.
    pub fn read_parked(&mut self, cx: &mut Context<Self>) {
        self.parked_reading = true;
        let task = self.wire.parked(cx);
        let task = cx.spawn(async move |this, cx| {
            let read = task.await;
            let _ = this.update(cx, |this, cx| {
                this.parked_reading = false;
                match read {
                    Ok(list) => this.parked.update(cx, |parked, cx| parked.set(list.ideas, cx)),
                    Err(why) => Note::warn("the parked ideas could not be read").field("why", why).emit(),
                }
            });
        });
        self.track(task);
    }

    /// A press on an idea: Remove lets it go; Start sends it as a dispatch, unheld, and lets it go once Fleet has it.
    fn idea_asked(&mut self, asked: &ParkedAsked, cx: &mut Context<Self>) {
        match asked {
            ParkedAsked::Remove(id) => {
                self.parked.update(cx, |parked, cx| parked.busy(id, true, cx));
                self.remove_idea(id.clone(), cx);
            }
            ParkedAsked::Start(idea) => {
                self.parked.update(cx, |parked, cx| parked.busy(&idea.id, true, cx));
                // An idea started is no longer one to keep: asked for as an idea it would be kept again.
                let mut dispatch = idea.dispatch.clone();
                dispatch.start.held = false;
                if dispatch.kind == Some(ipc::DispatchKind::Idea) {
                    dispatch.kind = None;
                }
                let words = dispatch.start.request.clone().unwrap_or_default();
                let id = idea.id.clone();
                let task = self.wire.dispatch(dispatch, cx);
                let task = cx.spawn(async move |this, cx| {
                    let sent = task.await;
                    let _ = this.update(cx, |this, cx| match sent {
                        Ok((done, events)) => {
                            this.told(&done, events, &words, cx);
                            this.remove_idea(id, cx);
                        }
                        Err(why) => this.parked.update(cx, |parked, cx| {
                            parked.busy(&id, false, cx);
                            parked.refuse(why, cx);
                        }),
                    });
                });
                self.track(task);
            }
        }
    }

    fn remove_idea(&mut self, id: String, cx: &mut Context<Self>) {
        let task = self.wire.remove_parked(id.clone(), cx);
        let task = cx.spawn(async move |this, cx| {
            let removed = task.await;
            let _ = this.update(cx, |this, cx| match removed {
                Ok(list) => this.parked.update(cx, |parked, cx| parked.set(list.ideas, cx)),
                Err(why) => {
                    this.parked.update(cx, |parked, cx| {
                        parked.busy(&id, false, cx);
                        parked.refuse(why, cx);
                    });
                    this.read_parked(cx);
                }
            });
        });
        self.track(task);
    }

    // -- the walk's scenes -------------------------------------------------------------------------

    /// Move the mock Fleet to a named scene: the Operator answers the chat (`said`), a new item joins
    /// the queue (`arrives`), three join at once (`raised`), the next answer is refused (`refuse`) or its
    /// reply lost (`lose`), or it is switched off (`off`). Two scenes only scroll the page, where a
    /// step looks at what a short window holds below the open card: `rest` to the rows under it, `top`
    /// back to the card.
    pub fn scene(&mut self, name: &str, cx: &mut Context<Self>) -> Result<(), String> {
        match name {
            "rest" => {
                self.main_scroll.scroll_to_bottom();
                cx.notify();
                return Ok(());
            }
            "top" => {
                self.main_scroll.set_offset(gpui::Point::default());
                cx.notify();
                return Ok(());
            }
            _ => {}
        }
        let mock = self.mock_fleet().ok_or("the page is on a live Fleet")?;
        match name {
            "said" => {
                let line = mock.said().ok_or("nothing was asked of the Operator")?;
                let chat = self.chat.clone().ok_or("the chat is not open")?;
                chat.update(cx, |chat, cx| chat.answered(line.text, line.at, cx));
            }
            "arrives" => {
                mock.arrives();
                let page = mock.page.clone();
                self.set_page(page, cx);
                // The step looks at the row that joined, at the queue's end.
                self.main_scroll.scroll_to_bottom();
            }
            // Work asks to pair, and a press on its banner brings the page and its card to the front.
            "pairing" => {
                mock.pairings = vec![super::pairing::waiting("0a0b", "Work", "4821", "2026-11-11T14:08:00-05:00")];
                cx.defer(|cx| super::screen::open_call("mac:0a0b".into(), cx));
            }
            // The Gateway refuses the next Confirm or Decline.
            "pairing-refused" => mock.refuse_pairing = Some("Work's ask ran out. Ask again from Work's Settings.".into()),
            "raised" => {
                mock.raised();
                let page = mock.page.clone();
                self.set_page(page, cx);
            }
            // Fleet refuses the next answer, or its reply never arrives.
            "refuse" => mock.fail_answer = Some(Failed::Refused("The tracker did not take the new issue.".into())),
            "lose" => mock.fail_answer = Some(Failed::Lost("no answer in 30 seconds".into())),
            // Fleet refuses the next Later: the time it names has passed by the time it reads it.
            "late" => mock.refuse_later = Some("That time has passed. Pick a later one.".into()),
            "off" => {
                mock.switch(false);
                let page = mock.page.clone();
                self.done.clear();
                self.set_page(page, cx);
            }
            other => return Err(format!("no Operator scene named {other}")),
        }
        Ok(())
    }
}

/// The run of items around `at` that share its group: one Job's asks, which Your turn draws as one entry.
/// Fleet can list a Job's asks put Later as a second run under the same group, so a run is never more
/// than the neighbours.
pub(super) fn run_at(groups: &[Option<&str>], at: usize) -> std::ops::Range<usize> {
    let Some(group) = groups.get(at).copied().flatten() else { return at..at + 1 };
    let same = |one: &usize| groups[*one] == Some(group);
    let start = (0..at).rev().take_while(same).last().unwrap_or(at);
    let end = (at + 1..groups.len()).take_while(same).last().map_or(at + 1, |last| last + 1);
    start..end
}

/// Your turn's ids once the entry holding `id` goes where `to` stands: after that entry going down and
/// before it going up, a Job's asks moving whole and passed whole. `None` where nothing moves.
pub(super) fn moved(items: &[(&str, Option<&str>)], id: &str, to: usize) -> Option<Vec<String>> {
    let at = items.iter().position(|(one, _)| *one == id)?;
    let to = to.min(items.len() - 1);
    let groups: Vec<Option<&str>> = items.iter().map(|(_, group)| *group).collect();
    let moving = run_at(&groups, at);
    if moving.contains(&to) {
        return None;
    }
    let target = run_at(&groups, to);
    let down = to > at;
    let anchor = items[if down { target.end - 1 } else { target.start }].0;
    let mut ids: Vec<&str> = items.iter().enumerate().filter(|(one, _)| !moving.contains(one)).map(|(_, (one, _))| *one).collect();
    let place = ids.iter().position(|one| *one == anchor)? + usize::from(down);
    ids.splice(place..place, items[moving].iter().map(|(one, _)| *one));
    Some(ids.into_iter().map(String::from).collect())
}

/// What came of a press, in a few words, from Fleet's answer.
fn said_of(turn: &OperatorTurnItem, answer: OperatorAnswer, chosen: Option<SharedString>, answered: &OperatorAnswered) -> SharedString {
    if answer == OperatorAnswer::No {
        return "Dropped".into();
    }
    let mut said = match (&answered.filed, &answered.handed_to, &answered.started) {
        (Some(filed), Some(_), _) => {
            let to = turn.then.iter().find_map(|offer| offer.hand_to.as_ref()).map_or("the Job".to_string(), |to| to.job.clone());
            format!("Filed {filed} and handed to {to}")
        }
        (Some(filed), None, Some(_)) => format!("Filed {filed} and started a Job"),
        (Some(filed), None, None) => format!("Filed {filed}"),
        (None, _, Some(_)) => "Started a Job".to_string(),
        (None, _, None) if answered.open.is_some() => "Opened".to_string(),
        // Fleet settled it with the pick and gives nothing more back.
        (None, _, None) => format!("Answered {}", chosen.or_else(|| face::pick_of(turn)).map_or("Yes".to_string(), |pick| pick.to_string())),
    };
    match answer {
        OperatorAnswer::AlwaysHere => said.push_str(". Kept for this project"),
        OperatorAnswer::Always => said.push_str(". Kept everywhere"),
        _ => {}
    }
    said.into()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui::{TestAppContext, VisualTestContext};

    use super::*;

    fn open(cx: &mut TestAppContext) -> (Entity<OperatorPageView>, &mut VisualTestContext) {
        open_as(cx, false)
    }

    fn open_as(cx: &mut TestAppContext, pane: bool) -> (Entity<OperatorPageView>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (page, cx) = cx.add_window_view(|window, cx| OperatorPageView::mock(pane, window, cx));
        cx.update(|window, cx| {
            let keys = page.read(cx).focus.clone();
            window.focus(&keys, cx);
        });
        cx.run_until_parked();
        (page, cx)
    }

    fn pressed(page: &Entity<OperatorPageView>, cx: &mut VisualTestContext) -> Vec<OperatorAnswerPress> {
        page.update(cx, |page, _| page.mock_fleet().map(|mock| mock.pressed.clone()).unwrap_or_default())
    }

    fn ids(page: &Entity<OperatorPageView>, cx: &mut VisualTestContext) -> Vec<String> {
        page.read_with(cx, |page, _| page.items().iter().map(|turn| turn.id.clone()).collect())
    }

    fn open_id(page: &Entity<OperatorPageView>, cx: &mut VisualTestContext) -> Option<String> {
        page.read_with(cx, |page, _| page.open.clone())
    }

    #[gpui::test]
    fn a_says_yes_to_the_pick_and_the_next_item_opens_struck_through_below(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:1"));
        cx.simulate_keystrokes("a");
        assert_eq!(pressed(&page, cx), vec![OperatorAnswerPress { id: "turn:1".into(), answer: OperatorAnswer::Yes, then: None, option: None, project: None }]);
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:2"), "the next item opens");
        page.read_with(cx, |page, _| {
            assert_eq!(page.done.iter().map(|done| done.id.as_str()).collect::<Vec<_>>(), ["turn:1"]);
            assert_eq!(page.done[0].said.as_deref(), Some("Answered Approve"), "what came of it, once Fleet answered");
            assert!(page.pending.is_empty());
        });
        assert!(!ids(&page, cx).contains(&"turn:1".to_string()));
    }

    #[gpui::test]
    fn an_option_that_is_not_the_pick_goes_out_by_its_label(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("down down down down");
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:9"));
        cx.simulate_keystrokes("1 h");
        assert_eq!(
            pressed(&page, cx),
            vec![OperatorAnswerPress { id: "turn:9".into(), answer: OperatorAnswer::AlwaysHere, then: None, option: Some("Start a fix".into()), project: None }]
        );
        page.read_with(cx, |page, _| assert_eq!(page.done[0].said.as_deref(), Some("Answered Start a fix. Kept for this project")));
    }

    #[gpui::test]
    fn yes_on_the_pick_names_no_option(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("down down down down a");
        assert_eq!(pressed(&page, cx), vec![OperatorAnswerPress { id: "turn:9".into(), answer: OperatorAnswer::Yes, then: None, option: None, project: None }]);
    }

    #[gpui::test]
    fn a_digit_presses_that_outcome_with_yes(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("down");
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:2"));
        page.read_with(cx, |page, _| assert_eq!(page.open_item().and_then(|turn| page.then_of(turn)), Some(OperatorThen::FileAndHand), "the Operator's pick stands first"));
        cx.simulate_keystrokes("2");
        assert_eq!(pressed(&page, cx), vec![OperatorAnswerPress { id: "turn:2".into(), answer: OperatorAnswer::Yes, then: Some(OperatorThen::FileAndStart), option: None, project: None }]);
    }

    #[gpui::test]
    fn always_is_offered_only_where_fleet_keeps_a_rule(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.update(|window, cx| page.update(cx, |page, cx| page.answer(OperatorAnswer::Always, window, cx)));
        assert!(pressed(&page, cx).is_empty(), "a decision offers no Always");
        // A go-ahead to file offers its rule only after he answers; turn:9 keeps one and has no outcomes.
        cx.simulate_keystrokes("j");
        cx.update(|window, cx| page.update(cx, |page, cx| page.answer(OperatorAnswer::AlwaysHere, window, cx)));
        assert!(pressed(&page, cx).is_empty(), "a go-ahead to file offers no Always before its answer");
        cx.simulate_keystrokes("down down down");
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:9"));
        cx.update(|window, cx| page.update(cx, |page, cx| page.answer(OperatorAnswer::AlwaysHere, window, cx)));
        assert_eq!(pressed(&page, cx).iter().map(|press| (press.id.as_str(), press.answer)).collect::<Vec<_>>(), [("turn:9", OperatorAnswer::AlwaysHere)]);
    }

    #[gpui::test]
    fn later_puts_the_item_last_until_its_pick_and_opens_the_next(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        // L opens Later's menu with the keys on its first choice, and Enter takes it.
        cx.simulate_keystrokes("l");
        assert!(page.read_with(cx, |page, _| page.later_menu.is_some()));
        cx.simulate_keystrokes("enter");
        assert_eq!(ids(&page, cx), ["turn:2", "turn:3", "turn:4", "turn:9", "turn:10", "turn:11", "turn:1"]);
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:2"));
        page.read_with(cx, |page, _| assert!(page.items().last().is_some_and(|turn| turn.later_until.is_some())));
    }

    #[gpui::test]
    fn a_shove_moves_the_open_item_and_fleet_keeps_the_order(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("alt-down");
        assert_eq!(ids(&page, cx), ["turn:2", "turn:1", "turn:3", "turn:4", "turn:9", "turn:10", "turn:11"]);
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:1"), "the keys stay on the item it moved");
        let kept: Vec<String> = page.update(cx, |page, _| page.mock_fleet().map(|mock| mock.page.turn.iter().map(|turn| turn.id.clone()).collect()).unwrap_or_default());
        assert_eq!(kept, ["turn:2", "turn:1", "turn:3", "turn:4", "turn:9", "turn:10", "turn:11"]);
        cx.update(|_, cx| page.update(cx, |page, cx| page.move_to("turn:4", 0, cx)));
        assert_eq!(ids(&page, cx), ["turn:4", "turn:2", "turn:1", "turn:3", "turn:9", "turn:10", "turn:11"], "a drop moves it as a drag would");
    }

    #[test]
    fn a_jobs_asks_move_as_one_entry_and_are_passed_whole() {
        let items = [("a", None), ("g1", Some("job")), ("g2", Some("job")), ("b", None), ("l1", Some("job")), ("l2", Some("job"))];
        let order = |id: &str, to: usize| moved(&items, id, to);
        assert_eq!(run_at(&items.map(|(_, group)| group), 2), 1..3);
        assert_eq!(run_at(&items.map(|(_, group)| group), 4), 4..6, "the same Job's asks put Later are a run of their own");
        assert_eq!(order("g2", 0).unwrap(), ["g1", "g2", "a", "b", "l1", "l2"], "up: the whole card goes before the item");
        assert_eq!(order("g1", 3).unwrap(), ["a", "b", "g1", "g2", "l1", "l2"], "down: the whole card goes after the item");
        assert_eq!(order("a", 1).unwrap(), ["g1", "g2", "a", "b", "l1", "l2"], "down onto a card: after its last ask");
        assert_eq!(order("b", 2).unwrap(), ["a", "b", "g1", "g2", "l1", "l2"], "up onto a card: before its first ask");
        assert_eq!(order("b", 5).unwrap(), ["a", "g1", "g2", "l1", "l2", "b"]);
        assert_eq!(order("g1", 2), None, "onto itself nothing moves");
        assert_eq!(order("a", 0), None);
        let single = [("1", None), ("2", None), ("3", None)];
        assert_eq!(moved(&single, "1", 1).unwrap(), ["2", "1", "3"], "a single item moves as it did");
        assert_eq!(moved(&single, "3", 0).unwrap(), ["3", "1", "2"]);
    }

    #[gpui::test]
    fn a_jobs_card_shoves_whole_and_an_answered_ask_leaves_the_rest_open(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.open_id("turn:11", cx));
        cx.simulate_keystrokes("alt-up");
        assert_eq!(ids(&page, cx), ["turn:1", "turn:2", "turn:3", "turn:4", "turn:10", "turn:11", "turn:9"]);
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:11"), "the keys stay on the ask");
        cx.simulate_keystrokes("a");
        assert_eq!(pressed(&page, cx).iter().map(|press| press.id.as_str()).collect::<Vec<_>>(), ["turn:11"]);
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:10"), "the Job's other ask stays open");
        page.read_with(cx, |page, _| assert_eq!(page.open_item().and_then(|turn| turn.group.clone()), None, "one ask left is a card of its own"));
        cx.simulate_keystrokes("alt-down");
        assert_eq!(ids(&page, cx), ["turn:1", "turn:2", "turn:3", "turn:4", "turn:9", "turn:10"]);
    }

    #[gpui::test]
    fn a_refused_press_puts_the_item_back_open_with_why(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        // Fleet no longer holds it, so it refuses.
        page.update(cx, |page, _| page.mock_fleet().map(|mock| mock.page.turn.retain(|turn| turn.id != "turn:1")));
        cx.simulate_keystrokes("a");
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:1"));
        page.read_with(cx, |page, _| {
            assert!(page.done.is_empty());
            assert_eq!(page.refused.as_ref().map(|(id, _)| id.as_str()), Some("turn:1"));
            assert!(page.items().iter().any(|turn| turn.id == "turn:1"));
        });
    }

    #[gpui::test]
    fn a_new_item_is_told_once(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let told: Rc<RefCell<Vec<String>>> = Rc::default();
        let heard = told.clone();
        let _keep = cx.update(|_, cx| cx.subscribe(&page, move |_, arrived: &Arrived, _| heard.borrow_mut().extend(arrived.0.iter().map(|turn| turn.id.clone()))));
        cx.update(|_, cx| page.update(cx, |page, cx| page.scene("arrives", cx))).expect("the scene plays");
        cx.update(|_, cx| page.update(cx, |page, cx| page.read(cx)));
        cx.run_until_parked();
        assert_eq!(*told.borrow(), ["turn:5"]);
    }

    #[gpui::test]
    fn slash_asks_about_the_open_item_and_esc_gives_the_keys_back(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("/");
        let chat = page.read_with(cx, |page, _| page.chat.clone()).expect("the chat is open");
        assert_eq!(chat.read_with(cx, |chat, _| chat.about.clone()).as_deref(), Some("turn:1"));
        cx.update(|window, cx| assert!(chat.read(cx).focus_handle(cx).is_focused(window), "the keys are in its box"));
        cx.simulate_keystrokes("a");
        assert!(pressed(&page, cx).is_empty(), "a letter in the box is typing, not Yes");
        cx.simulate_keystrokes("escape");
        assert!(page.read_with(cx, |page, _| page.chat.is_none()));
        cx.update(|window, cx| assert!(page.read(cx).focus.is_focused(window)));
    }

    fn type_in_box(page: &Entity<OperatorPageView>, words: &str, cx: &mut VisualTestContext) {
        let composer = page.read_with(cx, |page, _| page.composer.clone());
        cx.update(|window, cx| composer.update(cx, |composer, cx| composer.focus(window, cx)));
        cx.simulate_input(words);
    }

    #[gpui::test]
    fn alt_m_puts_the_caret_in_the_box_and_a_letter_there_is_typing_not_an_answer(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("alt-m");
        let composer = page.read_with(cx, |page, _| page.composer.clone());
        cx.update(|window, cx| assert!(composer.read(cx).has_keys(window, cx), "the caret is in the box"));
        cx.simulate_keystrokes("a");
        assert!(pressed(&page, cx).is_empty(), "a letter in the box is not Yes");
        assert_eq!(composer.read_with(cx, |composer, cx| composer.words(cx)), "a");
        cx.simulate_keystrokes("escape");
        cx.update(|window, cx| assert!(page.read(cx).focus.is_focused(window), "Esc gives the keys back to the page"));
    }

    #[gpui::test]
    fn a_send_is_an_unheld_dispatch_that_carries_the_kind_picked_and_none_when_none_is(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let composer = page.read_with(cx, |page, _| page.composer.clone());
        type_in_box(&page, "Keep the palette's last search", cx);
        cx.simulate_keystrokes("cmd-enter");
        cx.run_until_parked();
        type_in_box(&page, "The meter jumps", cx);
        composer.update(cx, |composer, cx| composer.pick_kind(ipc::DispatchKind::Bug, cx));
        cx.simulate_keystrokes("cmd-enter");
        cx.run_until_parked();
        let sent = page.update(cx, |page, _| page.mock_fleet().map(|mock| mock.dispatched.clone()).unwrap_or_default());
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0].kind, None, "no chip sends no kind");
        assert_eq!(sent[1].kind, Some(ipc::DispatchKind::Bug));
        assert!(sent.iter().all(|one| !one.start.held), "his send is never a draft");
        assert_eq!(composer.read_with(cx, |composer, cx| composer.words(cx)), "", "the answer emptied the box");
        assert_eq!(composer.read_with(cx, |composer, _| composer.kind()), None, "and the kind he picked went with it");
    }

    #[gpui::test]
    fn what_the_operator_kept_is_listed_newest_first_and_a_kept_send_joins_it(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let parked = page.read_with(cx, |page, _| page.parked.clone());
        let ids = |cx: &mut VisualTestContext| parked.read_with(cx, |parked, _| parked.ideas().iter().map(|idea| idea.id.clone()).collect::<Vec<_>>());
        assert_eq!(ids(cx), ["idea:3", "idea:2", "idea:1"]);
        type_in_box(&page, "A thought for later", cx);
        cx.simulate_keystrokes("cmd-enter");
        cx.run_until_parked();
        assert_eq!(ids(cx).len(), 4);
        assert_eq!(ids(cx)[0], "idea:101", "the idea the Operator just kept is the newest");
    }

    #[gpui::test]
    fn start_sends_a_kept_idea_unheld_and_without_the_kind_idea_then_lets_it_go(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let parked = page.read_with(cx, |page, _| page.parked.clone());
        let idea = parked.read_with(cx, |parked, _| parked.ideas().into_iter().find(|idea| idea.id == "idea:2").cloned()).expect("a kept idea");
        parked.update(cx, |_, cx| cx.emit(ParkedAsked::Start(idea)));
        cx.run_until_parked();
        let sent = page.update(cx, |page, _| page.mock_fleet().map(|mock| mock.dispatched.clone()).unwrap_or_default());
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].kind, None, "an idea started is no longer one to keep");
        assert!(!sent[0].start.held);
        let left = parked.read_with(cx, |parked, _| parked.ideas().iter().map(|idea| idea.id.clone()).collect::<Vec<_>>());
        assert_eq!(left, ["idea:3", "idea:1"]);
        let (kept, bug) = (parked.read_with(cx, |parked, _| parked.ideas().len()), left.contains(&"idea:3".to_string()));
        assert!(bug && kept == 2);
    }

    #[gpui::test]
    fn remove_lets_an_idea_go_and_an_empty_list_draws_nothing(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let parked = page.read_with(cx, |page, _| page.parked.clone());
        for id in ["idea:1", "idea:2", "idea:3"] {
            parked.update(cx, |_, cx| cx.emit(ParkedAsked::Remove(id.to_string())));
            cx.run_until_parked();
        }
        assert!(parked.read_with(cx, |parked, _| parked.is_read() && !parked.draws()));
        assert!(page.update(cx, |page, _| page.mock_fleet().map(|mock| mock.dispatched.is_empty()).unwrap_or(false)), "removing starts nothing");
    }

    #[gpui::test]
    fn coming_to_the_front_tells_fleet(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let before = page.update(cx, |page, _| page.mock_fleet().map_or(0, |mock| mock.fronted));
        cx.update(|_, cx| page.update(cx, |page, cx| page.in_front(cx)));
        cx.run_until_parked();
        assert_eq!(page.update(cx, |page, _| page.mock_fleet().map_or(0, |mock| mock.fronted)), before + 1);
    }

    fn opened(page: &Entity<OperatorPageView>, cx: &mut VisualTestContext) -> (Rc<RefCell<Vec<Opened>>>, Subscription) {
        let told: Rc<RefCell<Vec<Opened>>> = Rc::default();
        let heard = told.clone();
        let keep = cx.update(|_, cx| cx.subscribe(page, move |_, opened: &Opened, _| heard.borrow_mut().push(opened.clone())));
        (told, keep)
    }

    #[gpui::test]
    fn h_shift_a_and_d_answer_a_kept_item(cx: &mut TestAppContext) {
        for (keys, answer) in [("h", OperatorAnswer::AlwaysHere), ("shift-a", OperatorAnswer::Always), ("d", OperatorAnswer::No)] {
            let (page, cx) = open(cx);
            cx.simulate_keystrokes("down down down down");
            cx.simulate_keystrokes(keys);
            assert_eq!(pressed(&page, cx).iter().map(|press| (press.id.as_str(), press.answer)).collect::<Vec<_>>(), [("turn:9", answer)], "{keys}");
        }
    }

    #[gpui::test]
    fn a_plan_offers_no_no(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("d h");
        assert!(pressed(&page, cx).is_empty(), "a plan takes Approve or Send changes");
    }

    #[gpui::test]
    fn c_sends_changes_in_the_plans_job_and_answers_nothing(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let (told, _keep) = opened(&page, cx);
        let job = page.read_with(cx, |page, _| page.open_item().and_then(|turn| turn.session_id.clone())).expect("the plan has its Job");
        cx.simulate_keystrokes("c");
        assert_eq!(*told.borrow(), [Opened::Job(job)]);
        assert!(pressed(&page, cx).is_empty());
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:1"), "the plan stays open");
    }

    #[gpui::test]
    fn typing_in_the_chat_presses_and_moves_nothing(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("/");
        cx.simulate_keystrokes("a l j k h d c 2 alt-up alt-down");
        assert!(pressed(&page, cx).is_empty());
        assert_eq!(ids(&page, cx), ["turn:1", "turn:2", "turn:3", "turn:4", "turn:9", "turn:10", "turn:11"]);
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:1"));
        assert!(page.read_with(cx, |page, _| page.then.is_empty()), "a digit in the box picks no Then");
    }

    #[gpui::test]
    fn a_digit_picks_only_a_then(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("2");
        assert!(page.read_with(cx, |page, _| page.then.is_empty()), "a plan has no Then");
        assert!(pressed(&page, cx).is_empty());
    }

    #[gpui::test]
    fn switch_on_reads_on_at_once_and_a_refusal_puts_it_back_with_why(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.scene("off", cx)).expect("the scene plays");
        assert!(page.read_with(cx, |page, _| page.page.as_ref().is_some_and(|page| !page.on && page.turn.is_empty())));

        page.update(cx, |page, _| page.mock_fleet().map(|mock| mock.refuse_switch = Some("Fleet could not save the setting".into())));
        page.update(cx, |page, cx| page.switch_on(cx));
        cx.run_until_parked();
        page.read_with(cx, |page, _| {
            assert!(page.page.as_ref().is_some_and(|page| !page.on), "back off");
            assert_eq!(page.switch_refused.as_deref(), Some("Fleet could not save the setting"));
            assert!(!page.switching);
        });

        page.update(cx, |page, cx| page.switch_on(cx));
        assert!(page.read_with(cx, |page, _| page.page.as_ref().is_some_and(|page| page.on)), "on before Fleet answers");
        cx.run_until_parked();
        page.read_with(cx, |page, _| {
            assert!(page.switch_refused.is_none());
            assert!(!page.items().is_empty(), "Fleet's turn is read again");
        });
    }

    #[gpui::test]
    fn a_press_on_a_time_flips_every_time(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        assert!(!page.read_with(cx, |page, _| page.aged));
        page.update(cx, |page, cx| page.flip_stamps(cx));
        assert!(page.read_with(cx, |page, _| page.aged));
        page.update(cx, |page, cx| page.flip_stamps(cx));
        assert!(!page.read_with(cx, |page, _| page.aged));
    }

    #[gpui::test]
    fn enter_and_space_press_the_button_with_the_keys_and_say_no_yes(cx: &mut TestAppContext) {
        for key in ["enter", "space"] {
            let (page, cx) = open(cx);
            let later = page.read_with(cx, |page, _| page.later_focus.clone());
            cx.update(|window, cx| window.focus(&later, cx));
            cx.simulate_keystrokes(key);
            assert!(pressed(&page, cx).is_empty(), "{key} on Later is not Yes");
            assert!(page.read_with(cx, |page, _| page.later_menu.is_some()), "{key} on Later opens its menu of when");
        }
    }

    #[gpui::test]
    fn tab_from_the_page_starts_at_its_first_pill(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("tab");
        let first = page.read_with(cx, |page, _| page.first_stop.clone());
        cx.update(|window, _| assert!(first.is_focused(window)));
    }

    #[gpui::test]
    fn enter_on_a_tile_head_opens_and_folds_it(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let keys = page.read_with(cx, |page, _| page.stops.get("Landed").cloned()).expect("Landed is a tab stop");
        cx.update(|window, cx| window.focus(&keys, cx));
        cx.simulate_keystrokes("enter");
        assert_eq!(page.read_with(cx, |page, _| page.folded.get("Landed").copied()), Some(true), "open");
        assert!(pressed(&page, cx).is_empty(), "Enter on a tile is not Yes");
        cx.simulate_keystrokes("enter");
        assert_eq!(page.read_with(cx, |page, _| page.folded.get("Landed").copied()), Some(false), "folded");
    }

    #[gpui::test]
    fn every_rule_lists_everywhere_first_and_esc_gives_the_keys_back_to_its_link(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let order: Vec<Option<String>> = page.read_with(cx, |page, _| page.rules_in_order().iter().map(|rule| rule.repository.clone()).collect());
        assert_eq!(order, [None, None, Some("armada".into()), Some("armada".into()), Some("storefront".into()), Some("storefront".into())]);

        let link = page.read_with(cx, |page, _| page.stops.get("Every rule").cloned()).expect("Every rule is a tab stop");
        cx.update(|window, cx| window.focus(&link, cx));
        cx.simulate_keystrokes("enter");
        assert_eq!(page.read_with(cx, |page, _| page.rules), Some(0));
        cx.simulate_keystrokes("down down up");
        assert_eq!(page.read_with(cx, |page, _| page.rules), Some(1));
        cx.simulate_keystrokes("a l alt-up");
        assert!(pressed(&page, cx).is_empty(), "no key reaches Your turn behind the sheet");
        assert_eq!(ids(&page, cx), ["turn:1", "turn:2", "turn:3", "turn:4", "turn:9", "turn:10", "turn:11"]);
        cx.simulate_keystrokes("escape");
        assert_eq!(page.read_with(cx, |page, _| page.rules), None);
        cx.update(|window, _| assert!(link.is_focused(window), "the keys are back on Every rule"));
    }

    #[gpui::test]
    fn the_whole_log_opens_on_a_sheet_with_the_weeks_days_and_esc_gives_the_keys_back_to_its_link(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let link = page.read_with(cx, |page, _| page.stops.get("The whole Log").cloned()).expect("The whole Log is a tab stop");
        cx.update(|window, cx| window.focus(&link, cx));
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        let log = page.read_with(cx, |page, _| page.log.clone()).expect("the Log is open");
        assert!(log.contains("## Wednesday 2026-11-11"), "the week's journal, as Fleet writes it");
        cx.simulate_keystrokes("a l alt-up");
        assert!(pressed(&page, cx).is_empty(), "no key reaches Your turn behind the sheet");
        cx.simulate_keystrokes("escape");
        assert_eq!(page.read_with(cx, |page, _| page.log.clone()), None);
        cx.update(|window, _| assert!(link.is_focused(window), "the keys are back on The whole Log"));
    }

    fn standing(page: &Entity<OperatorPageView>, cx: &mut VisualTestContext) -> (Vec<i64>, Vec<i64>) {
        page.read_with(cx, |page, _| {
            let digest = page.page.as_ref().map(|page| &page.digest);
            (digest.map_or(Vec::new(), |digest| digest.learned.iter().map(|rule| rule.id).collect()), digest.map_or(Vec::new(), |digest| digest.holds.iter().map(|hold| hold.id).collect()))
        })
    }

    #[gpui::test]
    fn f_on_every_rule_forgets_the_rule_it_has_on_and_the_next_one_takes_its_place(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let link = page.read_with(cx, |page, _| page.stops.get("Every rule").cloned()).expect("Every rule is a tab stop");
        cx.update(|window, cx| window.focus(&link, cx));
        cx.simulate_keystrokes("enter down");
        let on = page.read_with(cx, |page, _| page.rules_in_order()[1].id);
        cx.simulate_keystrokes("f");
        cx.run_until_parked();
        let (rules, _) = standing(&page, cx);
        assert!(!rules.contains(&on), "forgotten");
        assert_eq!(rules.len(), 5);
        assert!(page.update(cx, |page, _| page.mock_fleet().is_some_and(|mock| mock.page.digest.learned.iter().all(|rule| rule.id != on))), "Fleet was told");
        assert_eq!(page.read_with(cx, |page, _| page.rules), Some(1), "the next rule has the sheet");
        assert!(pressed(&page, cx).is_empty(), "f answers nothing in Your turn");
    }

    #[gpui::test]
    fn forget_lifts_a_hold_at_once_and_a_refusal_reads_the_page_again(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.forget(12, cx));
        assert_eq!(standing(&page, cx).1, [11], "gone before Fleet answers");
        cx.run_until_parked();
        assert_eq!(standing(&page, cx).1, [11]);

        // An id nothing has: the mock refuses, and the page is read again as it was.
        page.update(cx, |page, cx| page.forget(99, cx));
        cx.run_until_parked();
        assert_eq!(standing(&page, cx), (vec![1, 2, 3, 4, 5, 6], vec![11]));
    }

    #[gpui::test]
    fn the_pane_from_a_job_holds_its_chat_from_the_start(cx: &mut TestAppContext) {
        let (page, cx) = open_as(cx, true);
        let chat = page.read_with(cx, |page, _| page.chat.clone()).expect("the pane opens with its chat");
        assert_eq!(chat.read_with(cx, |chat, _| chat.about.clone()), None, "about nothing until a row is pressed");
        assert!(chat.read_with(cx, |chat, _| chat.read), "the conversation so far is read");
    }
}
