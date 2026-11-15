//! The page's state and acts: the card in front, the cards waiting behind it, the talk about a card, and what the
//! Operator is allowed to do unasked. Every press answers at once: the page moves first and takes Fleet's page when it
//! comes. A refusal puts the card back and says why on it.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use gpui::{App, AppContext as _, Bounds, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable as _, Pixels, ScrollHandle, SharedString, Subscription, Task, Window};
use ipc::operator_board::{DispatchWay, OperatorDispatched};
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorAnswered, OperatorPage, OperatorThen, OperatorTurnItem};
use ipc::{OperatorActs, SessionId, WaitingAct};

use super::composer::{Composed, OperatorComposer, Outgoing};
use super::face;
use super::parked::{ParkedAsked, ParkedIdeas};
use super::scenes::{self, MockFleet};
use super::wire::{Failed, Wire};
use crate::dropdown::{Dropdown, Entry, Picked, Row};
use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::log::Note;
use crate::look::ActiveLook as _;

/// What the page asks of the window around it: a place it does not draw.
#[derive(Clone, Debug, PartialEq)]
pub enum Opened {
    /// The act of an item Fleet cannot do itself, such as a walk to open or a pull request, from the
    /// Job that holds it.
    Act(WaitingAct, Option<SessionId>),
    /// A Job, from a row of the feed.
    Job(SessionId),
}

/// Items that joined the queue since the page was last read, oldest first: what raises a toast. Jobs
/// screens own toasts (`banner.rs`, `notify.rs`) and subscribe to this on [`super::live`].
#[derive(Clone, Debug, PartialEq)]
pub struct Arrived(pub Vec<OperatorTurnItem>);

impl EventEmitter<Opened> for OperatorPageView {}
impl EventEmitter<Arrived> for OperatorPageView {}

/// A talk about a card: the card folds to one line and the Operator's thread runs under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Talk {
    /// The card's id.
    pub about: String,
    /// The card's title, for the tag in the box and the folded line.
    pub title: SharedString,
    /// The folded card is open again, beside the talk.
    pub unfolded: bool,
    /// What he sent that the thread has not echoed yet, drawn as his own message.
    pub sent: Option<SharedString>,
}

/// Where each press that opens a menu was drawn, by name.
pub(super) type Bounded = Rc<RefCell<HashMap<&'static str, Bounds<Pixels>>>>;

pub struct OperatorPageView {
    pub(super) wire: Wire,
    /// Fleet's page, with this sitting's presses already on it. `None` until it is read: a skeleton.
    pub(super) page: Option<OperatorPage>,
    /// The card in front, by id: where he brought one from Waiting, else the first.
    pub(super) open: Option<String>,
    /// The cards settled here, which stay out of the queue though Fleet keeps one whose act it left to him to open.
    pub(super) settled: BTreeSet<String>,
    /// The cards whose press is in flight.
    pub(super) pending: BTreeSet<String>,
    /// A press Fleet refused, on that card, with why.
    pub(super) refused: Option<(String, SharedString)>,
    /// The Jobs' titles, by id, which a card's chips read.
    pub(super) titles: HashMap<String, String>,
    pub(super) talk: Option<Talk>,
    /// The box under the card, whose send is a dispatch, or a talk where it holds a tag.
    pub(super) composer: Entity<OperatorComposer>,
    /// The ideas the Operator kept for later.
    pub(super) parked: Entity<ParkedIdeas>,
    parked_reading: bool,
    pub focus: FocusHandle,
    /// Every time on the page read as its age, after a press on one: they flip together, as the thread's do.
    pub(super) aged: bool,
    /// The clock the page reads its times against: the walk's 13:05, else this Mac's.
    pub(super) clock_now: Option<chrono::DateTime<chrono::Local>>,
    /// Switch on or Switch off pressed, and Fleet has not answered; or why it refused.
    pub(super) switching: bool,
    /// The press in flight is Switch off.
    pub(super) switching_off: bool,
    pub(super) switch_refused: Option<SharedString>,
    pub(super) switch_off_refused: Option<SharedString>,
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
    pub(super) main_scroll: ScrollHandle,
    pub(super) talk_scroll: ScrollHandle,
    /// The page's first tab stop, its first pill: where Tab goes from the page itself.
    pub(super) first_stop: FocusHandle,
    /// The open card's Discuss later, so a test can put the keys on a press.
    pub(super) later_focus: FocusHandle,
    /// Where each press that opens a menu was drawn, by name, so the menu opens under it.
    pub(super) drawn: Bounded,
    /// Discuss later's menu of when the card comes back, while it is open.
    pub(super) later_menu: Option<(Entity<Dropdown>, [Subscription; 2])>,
    /// The menu of what the Operator may do unasked, while it is open.
    pub(super) acts_menu: Option<(Entity<Dropdown>, [Subscription; 2])>,
    /// Fleet's sentence where it refused the last change of that value.
    pub(super) acts_refused: Option<SharedString>,
    /// The list of the cards waiting behind the one in front, while it is open, with the row the keys are on.
    pub(super) waiting_open: bool,
    pub(super) waiting_focus: FocusHandle,
    /// The turn ids already seen, so a new one is told once. `None` before the first read.
    seen: Option<BTreeSet<String>>,
    read_at: u64,
    reading: bool,
    subs: Vec<Subscription>,
}

impl OperatorPageView {
    fn new(wire: Wire, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subs = Vec::new();
        // Whenever the window gains focus or is shown: Fleet keeps when he was last in front.
        subs.push(cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.in_front(cx);
            }
        }));
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
            open: None,
            settled: BTreeSet::new(),
            pending: BTreeSet::new(),
            refused: None,
            titles: HashMap::new(),
            talk: None,
            composer,
            parked,
            parked_reading: false,
            focus: cx.focus_handle(),
            aged: false,
            clock_now: None,
            switching: false,
            switching_off: false,
            switch_refused: None,
            switch_off_refused: None,
            pairing_pending: BTreeSet::new(),
            pairing_refused: None,
            pairing_front: None,
            pairing_focus: cx.focus_handle(),
            pairing_reveal: false,
            main_scroll: ScrollHandle::new(),
            talk_scroll: ScrollHandle::new(),
            first_stop: cx.focus_handle(),
            later_focus: cx.focus_handle(),
            drawn: Default::default(),
            later_menu: None,
            acts_menu: None,
            acts_refused: None,
            waiting_open: false,
            waiting_focus: cx.focus_handle(),
            seen: None,
            read_at: 0,
            reading: false,
            subs,
        }
    }

    /// The page on the owner's Fleet: read when the port is known, and again on each `operator.page_changed`.
    pub fn live(fleet: &Entity<FleetConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Wire::Live(fleet.clone()), window, cx);
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
    pub fn mock(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Wire::Mock(Box::new(MockFleet::new())), window, cx);
        this.clock_now = Some(scenes::now());
        this.titles = scenes::titles();
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

    /// Whether Fleet's page has been read once: before it, the queue is not known to be empty.
    pub fn is_read(&self) -> bool {
        self.page.is_some()
    }

    /// Everything Fleet queues: the card in front, then the cards waiting, and the cards raised in a talk.
    pub fn items(&self) -> &[OperatorTurnItem] {
        self.page.as_ref().map_or(&[], |page| page.turn.as_slice())
    }

    /// A card the Operator raised in the talk now open: it carries on from the card he is talking about.
    pub(super) fn is_talk_card(&self, turn: &OperatorTurnItem) -> bool {
        let (Some(talk), Some(card)) = (&self.talk, &turn.card) else { return false };
        card.talk_of.as_deref() == Some(talk.about.as_str())
    }

    /// The cards in line, the talk's own left out.
    pub(super) fn queue(&self) -> Vec<&OperatorTurnItem> {
        self.items().iter().filter(|turn| !self.is_talk_card(turn)).collect()
    }

    /// The card in front: the one he brought there, else the first in line.
    pub fn open_item(&self) -> Option<&OperatorTurnItem> {
        let queue = self.queue();
        self.open.as_deref().and_then(|open| queue.iter().copied().find(|turn| turn.id == open)).or_else(|| queue.first().copied())
    }

    /// The cards waiting behind the one in front, in the order Fleet queues them.
    pub fn waiting(&self) -> Vec<&OperatorTurnItem> {
        let front = self.open_item().map(|turn| turn.id.clone());
        self.queue().into_iter().filter(|turn| Some(&turn.id) != front.as_ref()).collect()
    }

    /// The cards the Operator raised in the talk, oldest first.
    pub(super) fn talk_cards(&self) -> Vec<&OperatorTurnItem> {
        self.items().iter().filter(|turn| self.is_talk_card(turn)).collect()
    }

    /// The card the keys act on: the newest the Operator raised in the talk, else the one in front.
    pub fn target(&self) -> Option<&OperatorTurnItem> {
        self.talk_cards().last().copied().or_else(|| self.open_item())
    }

    /// A Job's title, as the card's chip says it.
    pub(super) fn title_of_job(&self, id: &SessionId) -> SharedString {
        self.titles.get(id.as_str()).cloned().map_or_else(|| id.as_str().to_string().into(), SharedString::from)
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

    /// Fleet's page replaces the one held. The card in front stays where it is still there, and a card not seen before
    /// is told once. A card he settled here stays out of the queue, though Fleet keeps one whose act it left to him
    /// to open until he does it.
    pub fn set_page(&mut self, mut page: OperatorPage, cx: &mut Context<Self>) {
        let ids: BTreeSet<String> = page.turn.iter().map(|turn| turn.id.clone()).collect();
        if let Some(seen) = &self.seen {
            let new: Vec<OperatorTurnItem> = page.turn.iter().filter(|turn| !seen.contains(&turn.id)).cloned().collect();
            if !new.is_empty() {
                cx.emit(Arrived(new));
            }
        }
        self.seen = Some(ids);
        page.turn.retain(|turn| !self.settled.contains(&turn.id));
        let ids: BTreeSet<String> = page.turn.iter().map(|turn| turn.id.clone()).collect();
        self.settled.retain(|id| ids.contains(id) || self.pending.contains(id));
        self.page = Some(page);
        // A talk lasts until its card is settled or put back.
        if self.talk.as_ref().is_some_and(|talk| !ids.contains(&talk.about)) {
            self.talk = None;
        }
        let keep = self.open.as_ref().filter(|open| self.queue().iter().any(|turn| &turn.id == *open)).cloned();
        self.open = keep.or_else(|| self.queue().first().map(|turn| turn.id.clone()));
        if self.talk.is_some() {
            self.read_thread(cx);
        }
        cx.notify();
    }

    // -- the keys and presses ----------------------------------------------------------------------

    pub fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        // ⌥M puts the caret in the box from anywhere on the page, as it does in a Job's view.
        if act.is("message_job") {
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
        if self.waiting_open {
            return cx.propagate();
        }
        match act.id {
            // The registry's slots are j, k, ↓, ↑: even slots go down. They go through the cards waiting.
            "call_move_focus" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "call_yes" => self.answer(OperatorAnswer::Yes, window, cx),
            "call_send" => self.enter(window, cx),
            "call_later" => self.open_later(true, window, cx),
            // `/` is Discuss, which the registry knows as Ask the Operator.
            "call_ask" => self.discuss(window, cx),
            "call_pick" => self.digit(act.digit.unwrap_or(0) as usize, cx),
            _ => cx.propagate(),
        }
    }

    /// Enter where the page itself has the keys: the Operator's pick. A press with the keys, such as Later or a row, takes
    /// Enter itself and does what a click does: gpui runs the binding first, so it is handed on.
    fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus.is_focused(window) {
            self.answer(OperatorAnswer::Yes, window, cx);
        } else {
            cx.propagate();
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
        self.set_switch(true, cx);
    }

    /// Switch off, from the page's head: the same setting the other way. The page reads off at once, and
    /// goes back on with why if Fleet refuses.
    pub fn switch_off(&mut self, cx: &mut Context<Self>) {
        self.set_switch(false, cx);
    }

    fn set_switch(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.switching {
            return;
        }
        self.switching = true;
        self.switching_off = !on;
        self.switch_refused = None;
        self.switch_off_refused = None;
        if let Some(page) = self.page.as_mut() {
            page.on = on;
        }
        cx.notify();
        let task = self.wire.switch(on, cx);
        let task = cx.spawn(async move |this, cx| {
            let saved = task.await;
            let _ = this.update(cx, |this, cx| {
                this.switching = false;
                match saved {
                    Ok(()) if !this.wire.is_live() => this.read(cx),
                    Ok(()) => cx.notify(),
                    Err(why) => {
                        if let Some(page) = this.page.as_mut() {
                            page.on = !on;
                        }
                        if on {
                            this.switch_refused = Some(why.into());
                        } else {
                            this.switch_off_refused = Some(why.into());
                        }
                        cx.notify();
                    }
                }
            });
        });
        self.track(task);
    }

    // -- the card in front and the cards waiting ---------------------------------------------------

    /// ↓ and ↑: the next or the previous card in line comes to the front.
    pub fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let queue: Vec<String> = self.queue().iter().map(|turn| turn.id.clone()).collect();
        if queue.is_empty() {
            return;
        }
        let at = self.open_item().and_then(|front| queue.iter().position(|id| *id == front.id)).unwrap_or(0) as isize;
        let to = (at + by).clamp(0, queue.len() as isize - 1) as usize;
        self.bring(&queue[to], cx);
    }

    /// A card comes to the front. A talk about the card that was there ends: its card is still waiting.
    pub fn bring(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.queue().iter().all(|turn| turn.id != id) {
            return;
        }
        if self.open.as_deref() != Some(id) {
            self.talk = None;
            self.composer.update(cx, |composer, cx| composer.untag(cx));
        }
        self.open = Some(id.to_string());
        self.waiting_open = false;
        self.main_scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// The old name of a press on a card that is not the open one.
    pub fn open_id(&mut self, id: &str, cx: &mut Context<Self>) {
        self.bring(id, cx);
    }

    /// Waiting opens its list, or shuts it.
    pub fn toggle_waiting(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.waiting_open && self.waiting().is_empty() {
            return;
        }
        self.waiting_open = !self.waiting_open;
        if self.waiting_open {
            let keys = self.waiting_focus.clone();
            window.defer(cx, move |window, cx| window.focus(&keys, cx));
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    pub fn close_waiting(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.waiting_open) {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    // -- settling ----------------------------------------------------------------------------------

    /// A digit: the option in that place settles the card; on a go-ahead to file, the Then in that place.
    pub fn digit(&mut self, digit: usize, cx: &mut Context<Self>) {
        let Some(turn) = self.target().cloned() else { return };
        let Some(at) = digit.checked_sub(1) else { return };
        if face::chooses(&turn) {
            if let Some(option) = turn.item.options.get(at) {
                self.settle_option(&turn.id, &option.label.clone(), cx);
            }
        } else if let Some(offer) = turn.then.get(at) {
            self.settle_then(&turn.id, offer.then, cx);
        } else if at == 0 && turn.then.is_empty() {
            self.settle(&turn.id, None, None, cx);
        }
    }

    /// The card settles at once on the option named.
    pub fn settle_option(&mut self, id: &str, label: &str, cx: &mut Context<Self>) {
        self.settle(id, Some(label.to_string()), None, cx);
    }

    /// The card settles at once on a Then, on a go-ahead to file.
    pub fn settle_then(&mut self, id: &str, then: OperatorThen, cx: &mut Context<Self>) {
        self.settle(id, None, Some(then), cx);
    }

    /// Yes: the Operator's pick, on the card the keys act on. A walk is his to look at, so it opens its Job and answers nothing.
    pub fn answer(&mut self, answer: OperatorAnswer, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(turn) = self.target().cloned() else { return };
        if answer != OperatorAnswer::Yes {
            return;
        }
        // A walk is his to look at, not to say Yes to: the press opens its Job, where the walk is, and nothing
        // is answered. The item leaves the queue when he approves the walk there.
        if face::goes_to_look(&turn) {
            if let Some(job) = turn.session_id.clone() {
                cx.emit(Opened::Job(job));
            }
            return;
        }
        // A pick of Later is done by Yes: the card goes Later, nothing is answered.
        if face::picks_later(&turn) {
            return self.later(None, cx);
        }
        let option = face::chooses(&turn).then(|| face::pick_of(&turn).filter(|pick| turn.item.options.iter().any(|one| one.label == pick.as_ref())).or_else(|| turn.item.options.first().map(|one| SharedString::from(one.label.clone())))).flatten();
        let then = turn.then.iter().find(|offer| offer.pick).or(turn.then.first()).map(|offer| offer.then);
        self.settle(&turn.id, option.map(|label| label.to_string()), then, cx);
    }

    /// The press goes to Fleet and its card leaves the queue at once, once however often it is sent. The next card comes
    /// to the front.
    fn settle(&mut self, id: &str, option: Option<String>, then: Option<OperatorThen>, cx: &mut Context<Self>) {
        let Some(turn) = self.items().iter().find(|turn| turn.id == id).cloned() else { return };
        if self.pending.contains(id) {
            return;
        }
        // A card the Operator raised in a talk carries the talk on: settling it ends the talk and the card it was about.
        let base = turn.card.as_ref().and_then(|card| card.talk_of.clone()).filter(|about| self.talk.as_ref().is_some_and(|talk| &talk.about == about)).unwrap_or_else(|| id.to_string());
        let at = self.queue().iter().position(|one| one.id == base).unwrap_or(0);
        let was_front = self.open_item().is_some_and(|front| front.id == base);
        self.settled.insert(id.to_string());
        self.pending.insert(id.to_string());
        if self.talk.as_ref().is_some_and(|talk| talk.about == base) {
            self.talk = None;
            self.composer.update(cx, |composer, cx| composer.untag(cx));
        }
        if let Some(page) = self.page.as_mut() {
            page.turn.retain(|one| one.id != id && one.id != base);
        }
        self.refused = None;
        if was_front || self.open.is_none() {
            let queue: Vec<String> = self.queue().iter().map(|one| one.id.clone()).collect();
            self.open = queue.get(at).or(queue.last()).cloned();
            self.main_scroll.set_offset(gpui::Point::default());
        }
        let press = OperatorAnswerPress { id: id.to_string(), answer: OperatorAnswer::Yes, then, option, project: None };
        self.send(turn, press, cx);
        cx.notify();
    }

    fn send(&mut self, turn: OperatorTurnItem, press: OperatorAnswerPress, cx: &mut Context<Self>) {
        self.settled.insert(turn.id.clone());
        self.pending.insert(turn.id.clone());
        let task = self.wire.answer(press.clone(), cx);
        let task = cx.spawn(async move |this, cx| {
            let answered = task.await;
            let _ = this.update(cx, |this, cx| this.answered(&turn, &press, answered, cx));
        });
        self.track(task);
    }

    fn answered(&mut self, turn: &OperatorTurnItem, press: &OperatorAnswerPress, answered: Result<OperatorAnswered, Failed>, cx: &mut Context<Self>) {
        self.pending.remove(&turn.id);
        match answered {
            Ok(answered) => {
                if let Some(act) = answered.open.clone() {
                    cx.emit(Opened::Act(act, turn.session_id.clone()));
                }
                self.set_page(answered.page, cx);
            }
            Err(failed) => {
                // Back where it was, in front: with Fleet's why under it, or a toast to send it again.
                self.settled.remove(&turn.id);
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
                            let _ = page.update(cx, |page, cx| {
                                page.settled.insert(turn.id.clone());
                                page.pending.insert(turn.id.clone());
                                if let Some(page) = page.page.as_mut() {
                                    page.turn.retain(|one| one.id != turn.id);
                                }
                                page.send(turn, press, cx);
                                cx.notify();
                            });
                        });
                        cx.defer(move |cx| crate::toasts::tell_retry(sentence, retry, cx));
                    }
                }
                cx.notify();
            }
        }
    }

    // -- Discuss later -----------------------------------------------------------------------------

    /// Discuss later's menu, under its press: when the card comes back. From the keys, they rest on its first choice;
    /// picked, the card goes back until then.
    pub fn open_later(&mut self, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(turn) = self.target().cloned() else { return };
        let Some(bounds) = self.drawn.borrow().get("later").copied() else { return };
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let choices = face::later_choices(now);
        let entries = choices.iter().map(|(id, label, at)| Entry::Row(Row::new(id, *label).detail(face::back_at(*at, now)))).collect();
        let gap = cx.look().length(crate::look::token::SPACE_1);
        let at = gpui::point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height + gap);
        let menu = cx.new(|cx| Dropdown::new("Discuss later", entries, at, gpui::Anchor::TopRight, from_keys, cx));
        let id = turn.id.clone();
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id: choice, .. } = picked;
            this.later_menu = None;
            // The menu goes with the keys on it: they come back to the page, as Esc brings them.
            window.focus(&this.focus, cx);
            let until = choices.iter().find(|(id, ..)| id == choice).map(|(_, _, at)| *at);
            if until.is_some() && this.target().is_some_and(|turn| turn.id == id) {
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

    /// Discuss later: the card goes back until `until`, or for the rest of the day, and the next one comes to the front.
    pub fn later(&mut self, until: Option<chrono::DateTime<chrono::Local>>, cx: &mut Context<Self>) {
        let Some(turn) = self.target().cloned() else { return };
        let at = self.queue().iter().position(|one| one.id == turn.id).unwrap_or(0);
        let until = until.map(|until| until.to_rfc3339());
        // Fleet leaves a card put back out of the queue until its time.
        if let Some(page) = self.page.as_mut() {
            page.turn.retain(|one| one.id != turn.id);
        }
        self.settled.insert(turn.id.clone());
        if self.talk.as_ref().is_some_and(|talk| talk.about == turn.id) {
            self.talk = None;
            self.composer.update(cx, |composer, cx| composer.untag(cx));
        }
        if self.open.as_deref() == Some(turn.id.as_str()) || self.open.is_none() {
            let queue: Vec<String> = self.queue().iter().map(|one| one.id.clone()).collect();
            self.open = queue.get(at).or(queue.last()).cloned();
        }
        self.refused = None;
        cx.notify();
        let task = self.wire.later(&turn.id, until, cx);
        // Refused, as a time already passed is, the card comes back in front where Fleet keeps it, saying why.
        let task = cx.spawn(async move |this, cx| {
            let later = task.await;
            let _ = this.update(cx, |this, cx| match later {
                Ok(page) => {
                    this.settled.remove(&turn.id);
                    this.set_page(page, cx)
                }
                Err(why) => {
                    this.settled.remove(&turn.id);
                    this.refused = Some((turn.id.clone(), why.into()));
                    this.open = Some(turn.id.clone());
                    this.read(cx);
                }
            });
        });
        self.track(task);
    }

    // -- Discuss -----------------------------------------------------------------------------------

    /// Discuss: the box under the card takes the keys, with a tag naming the card in it. The card stays in sight.
    pub fn discuss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(turn) = self.target().cloned() else { return self.composer.update(cx, |composer, cx| composer.focus(window, cx)) };
        let title = face::card_title(&turn);
        self.composer.update(cx, |composer, cx| {
            composer.tag(turn.id.clone(), title, cx);
            composer.focus(window, cx);
        });
        cx.notify();
    }

    /// The talk about a card, as he sent words with its tag: the card folds and the Operator's thread runs under it.
    fn talk_about(&mut self, words: String, about: String, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.items().iter().find(|turn| turn.id == about).map(face::card_title).unwrap_or_else(|| about.clone().into());
        // He is talking about the front card, or one the Operator raised in the talk.
        if self.talk.as_ref().map(|talk| talk.about.as_str()) != Some(about.as_str()) && self.is_front(&about) {
            self.talk = Some(Talk { about: about.clone(), title, unfolded: false, sent: None });
        }
        if let Some(talk) = self.talk.as_mut() {
            talk.sent = Some(words.clone().into());
        }
        self.read_thread(cx);
        self.talk_scroll.scroll_to_bottom();
        cx.notify();
        let task = self.wire.say(words, Some(about), cx);
        let Some(task) = task else {
            // The mock holds the Operator's answer until a scene gives one.
            self.composer.update(cx, |composer, cx| composer.sent(window, cx));
            return;
        };
        let composer = self.composer.clone();
        cx.spawn_in(window, async move |this, cx| {
            let said = task.await;
            let _ = this.update_in(cx, |this, window, cx| match said {
                Ok(_) => composer.update(cx, |composer, cx| composer.sent(window, cx)),
                Err(why) => {
                    if let Some(talk) = this.talk.as_mut() {
                        talk.sent = None;
                    }
                    composer.update(cx, |composer, cx| composer.refuse(why, cx));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn is_front(&self, id: &str) -> bool {
        self.open_item().is_some_and(|turn| turn.id == id) || self.items().iter().any(|turn| turn.id == id && self.is_talk_card(turn))
    }

    /// The folded card opens beside the talk, or folds again.
    pub fn toggle_fold(&mut self, cx: &mut Context<Self>) {
        if let Some(talk) = self.talk.as_mut() {
            talk.unfolded = !talk.unfolded;
            cx.notify();
        }
    }

    /// The Operator session's thread, read from Fleet: the talk's rows are drawn from it.
    fn read_thread(&mut self, cx: &mut Context<Self>) {
        let (Some(session), Wire::Live(fleet)) = (self.page.as_ref().and_then(|page| page.session_id.clone()), &self.wire) else { return };
        fleet.update(cx, |fleet, cx| fleet.read_thread(session.as_str(), false, cx));
    }

    /// The rows of the Operator's thread this talk draws: from his last words about the card on, as Fleet tags them.
    pub(super) fn talk_rows(&self, cx: &App) -> Vec<ipc::ThreadItem> {
        let (Some(talk), Some(session)) = (&self.talk, self.page.as_ref().and_then(|page| page.session_id.clone())) else { return Vec::new() };
        let items: Vec<ipc::ThreadItem> = match &self.wire {
            Wire::Live(fleet) => fleet.read(cx).held().threads.get(session.as_str()).map(|held| held.items.clone()).unwrap_or_default(),
            Wire::Mock(mock) => mock.thread.items.clone(),
        };
        let tag = format!("[About card {} ", talk.about);
        let start = items.iter().rposition(|item| matches!(item, ipc::ThreadItem::Row { row: ipc::SessionRow::Message { from: ipc::SessionVoice::You, text, .. } } if text.starts_with(&tag)));
        match start {
            Some(start) => items[start..].to_vec(),
            None => Vec::new(),
        }
    }

    // -- what the Operator may do unasked -----------------------------------------------------------

    /// The project the head's value is for: the one the window is scoped to, else the first Fleet lists.
    pub fn acts_project(&self, cx: &App) -> Option<ipc::ProjectOperatorActs> {
        let list = &self.page.as_ref()?.operator_acts;
        let scoped = crate::dispatch::screen::scope(cx).and_then(|(id, _)| id);
        scoped.and_then(|id| list.iter().find(|one| one.project == id.as_ref())).or_else(|| list.first()).cloned()
    }

    /// The menu of values, under the chip.
    pub fn open_acts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.acts_project(cx) else { return };
        let Some(bounds) = self.drawn.borrow().get("acts").copied() else { return };
        let entries = ACTS.iter().map(|(id, acts, label, _)| Entry::Choice { id, label: (*label).into(), on: project.acts == Some(*acts) }).collect();
        let gap = cx.look().length(crate::look::token::SPACE_1);
        let at = gpui::point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height + gap);
        let menu = cx.new(|cx| Dropdown::new("What the Operator does unasked", entries, at, gpui::Anchor::TopRight, true, cx));
        let id = project.project.clone();
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id: choice, .. } = picked;
            this.acts_menu = None;
            window.focus(&this.focus, cx);
            if let Some((_, acts, ..)) = ACTS.iter().find(|(word, ..)| word == choice) {
                this.set_acts(id.clone(), Some(*acts), cx);
            }
            cx.notify();
        });
        let shut = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            this.acts_menu = None;
            window.focus(&this.focus, cx);
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.acts_menu = Some((menu, [picked, shut]));
        cx.notify();
    }

    /// The value is set at once, and goes back with Fleet's sentence where it refuses.
    pub fn set_acts(&mut self, project: String, acts: Option<OperatorActs>, cx: &mut Context<Self>) {
        let was = self.page.as_ref().and_then(|page| page.operator_acts.iter().find(|one| one.project == project)).map(|one| one.acts);
        self.put_acts(&project, acts);
        self.acts_refused = None;
        cx.notify();
        let task = self.wire.set_acts(project.clone(), acts, cx);
        let task = cx.spawn(async move |this, cx| {
            let set = task.await;
            let _ = this.update(cx, |this, cx| {
                match set {
                    Ok(list) => {
                        if let Some(page) = this.page.as_mut() {
                            page.operator_acts = list.projects;
                        }
                    }
                    Err(why) => {
                        if let Some(was) = was {
                            this.put_acts(&project, was);
                        }
                        this.acts_refused = Some(why.into());
                    }
                }
                cx.notify();
            });
        });
        self.track(task);
    }

    fn put_acts(&mut self, project: &str, acts: Option<OperatorActs>) {
        if let Some(one) = self.page.as_mut().and_then(|page| page.operator_acts.iter_mut().find(|one| one.project == project)) {
            one.acts = acts;
        }
    }

    // -- in front ----------------------------------------------------------------------------------

    /// `operator_in_front`, sent each time the window gains focus or is shown.
    pub fn in_front(&mut self, cx: &mut Context<Self>) {
        let task = self.wire.in_front(cx);
        let task = cx.spawn(async move |_, _| {
            if let Err(why) = task.await {
                Note::info("Fleet was not told the window came to the front").field("why", why).emit();
            }
        });
        self.track(task);
    }

    /// A feed row's press: its Job opens.
    pub fn open_job(&mut self, id: SessionId, cx: &mut Context<Self>) {
        cx.emit(Opened::Job(id));
    }

    // -- the box and the ideas ---------------------------------------------------------------------

    /// The Jobs `@` names in the box.
    pub fn set_jobs(&mut self, jobs: Vec<(String, String)>, cx: &mut Context<Self>) {
        self.composer.update(cx, |composer, cx| composer.set_jobs(jobs, cx));
    }

    /// Every Job's title, ended ones too: the chips on a card read them.
    pub fn set_titles(&mut self, titles: HashMap<String, String>, cx: &mut Context<Self>) {
        if self.titles != titles {
            self.titles = titles;
            cx.notify();
        }
    }

    /// What the box asks: its dispatch goes to Fleet, its talk to the Operator, or the keys come back to the page.
    fn composed(&mut self, event: &Composed, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            Composed::Leave => window.focus(&self.focus, cx),
            Composed::Send(out) => self.dispatch(out.clone(), window, cx),
            Composed::Talk { words, about } => self.talk_about(words.clone(), about.clone(), window, cx),
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

    /// Move the mock Fleet to a named scene: the Operator answers the talk and raises a card in it (`said`), a new card
    /// joins the queue (`arrives`), three join at once (`raised`), the next answer is refused (`refuse`) or its
    /// reply lost (`lose`), the next change of what it does unasked is refused (`acts-refused`), or it is switched off
    /// (`off`). Two scenes only scroll the page: `rest` to what is under the card, `top` back to it.
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
                mock.said().ok_or("nothing was said to the Operator")?;
                let page = mock.page.clone();
                if let Some(talk) = self.talk.as_mut() {
                    talk.sent = None;
                }
                // The card it raised arrives with the page, once, and raises no toast.
                if let Some(seen) = self.seen.as_mut() {
                    seen.extend(page.turn.iter().map(|turn| turn.id.clone()));
                }
                self.set_page(page, cx);
                self.talk_scroll.scroll_to_bottom();
            }
            "arrives" => {
                mock.arrives();
                let page = mock.page.clone();
                self.set_page(page, cx);
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
            "refuse" => mock.fail_answer = Some(Failed::Refused("The Operator could not settle the card.".into())),
            "lose" => mock.fail_answer = Some(Failed::Lost("no answer in 30 seconds".into())),
            // Fleet refuses the next Discuss later: the time it names has passed by the time it reads it.
            "late" => mock.refuse_later = Some("That time has passed. Pick a later one.".into()),
            "acts-refused" => mock.refuse_acts = Some("No project has that id".into()),
            "off" => {
                mock.switch(false);
                let page = mock.page.clone();
                self.set_page(page, cx);
            }
            other => return Err(format!("no Operator scene named {other}")),
        }
        Ok(())
    }
}

/// What the Operator may do unasked, as the menu words it: its wire word, its value, its label and what it means.
pub(super) const ACTS: [(&str, OperatorActs, &str, &str); 3] = [
    ("act_then_tell", OperatorActs::ActThenTell, "Act then tell", "It acts on what it sees, then tells you"),
    ("ask_before_starting", OperatorActs::AskBeforeStarting, "Ask before starting", "It asks before it starts a Job"),
    ("only_when_told", OperatorActs::OnlyWhenTold, "Only when told", "It does nothing until you say"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};

    fn open(cx: &mut TestAppContext) -> (Entity<OperatorPageView>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (page, cx) = cx.add_window_view(|window, cx| OperatorPageView::mock(window, cx));
        cx.run_until_parked();
        (page, cx)
    }

    fn front(page: &Entity<OperatorPageView>, cx: &mut VisualTestContext) -> String {
        page.read_with(cx, |page, _| page.open_item().map(|turn| turn.id.clone()).unwrap_or_default())
    }

    #[gpui::test]
    fn one_card_is_in_front_and_four_wait_behind_it(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.read_with(cx, |page, _| {
            assert_eq!(page.open_item().map(|turn| turn.id.as_str()), Some("turn:31"));
            assert_eq!(page.waiting().iter().map(|turn| turn.id.as_str()).collect::<Vec<_>>(), ["turn:32", "turn:33", "turn:34", "turn:35"]);
        });
    }

    #[gpui::test]
    fn a_press_on_an_option_settles_the_card_at_once_with_yes_and_that_option(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| {
            page.settle_option("turn:31", "Let both run", cx);
            assert_eq!(page.open_item().map(|turn| turn.id.as_str()), Some("turn:32"), "the next card is in front before Fleet answers");
        });
        cx.run_until_parked();
        page.update(cx, |page, _| {
            let pressed = page.mock_fleet().expect("the walk's Fleet").pressed.clone();
            assert_eq!(pressed.len(), 1);
            assert_eq!((pressed[0].id.as_str(), pressed[0].answer, pressed[0].option.as_deref()), ("turn:31", OperatorAnswer::Yes, Some("Let both run")));
            assert_eq!(page.waiting().len(), 3);
        });
    }

    #[gpui::test]
    fn a_number_settles_the_option_in_that_place_and_yes_takes_the_operators_pick(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.digit(3, cx));
        cx.run_until_parked();
        page.update(cx, |page, _| {
            let pressed = page.mock_fleet().expect("the walk's Fleet").pressed.clone();
            assert_eq!(pressed[0].option.as_deref(), Some("Hand Pocket's change to Jobs screens"));
        });
        page.update_in(cx, |page, window, cx| page.answer(OperatorAnswer::Yes, window, cx));
        cx.run_until_parked();
        page.update(cx, |page, _| {
            let pressed = page.mock_fleet().expect("the walk's Fleet").pressed.clone();
            assert_eq!(pressed[1].id, "turn:32");
            assert!(pressed[1].option.is_some(), "Yes sends the card's pick");
        });
    }

    #[gpui::test]
    fn a_refused_press_puts_the_card_back_in_front_with_fleets_why(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| {
            page.mock_fleet().expect("the walk's Fleet").fail_answer = Some(Failed::Refused("The Operator could not settle the card.".into()));
            page.digit(1, cx);
        });
        cx.run_until_parked();
        assert_eq!(front(&page, cx), "turn:31");
        page.read_with(cx, |page, _| assert_eq!(page.refused.as_ref().map(|(id, why)| (id.as_str(), why.as_ref())), Some(("turn:31", "The Operator could not settle the card."))));
    }

    #[gpui::test]
    fn a_waiting_card_comes_to_the_front_and_the_one_it_replaced_waits(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.bring("turn:34", cx));
        assert_eq!(front(&page, cx), "turn:34");
        page.read_with(cx, |page, _| assert!(page.waiting().iter().any(|turn| turn.id == "turn:31")));
    }

    #[gpui::test]
    fn discuss_tags_the_box_with_the_card_and_leaves_the_card_in_front(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update_in(cx, |page, window, cx| page.discuss(window, cx));
        let tagged = page.read_with(cx, |page, cx| page.composer.read(cx).tagged());
        assert_eq!(tagged.map(|(id, title)| (id, title.to_string())), Some(("turn:31".to_string(), "Two Jobs are changing paging.rs".to_string())));
        assert_eq!(front(&page, cx), "turn:31");
    }

    #[gpui::test]
    fn a_talk_about_a_card_goes_to_the_operator_with_its_id_and_folds_the_card(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update_in(cx, |page, window, cx| page.talk_about("Why hold Pocket?".into(), "turn:31".into(), window, cx));
        page.update(cx, |page, _| {
            assert_eq!(page.talk.as_ref().map(|talk| talk.about.as_str()), Some("turn:31"));
            assert_eq!(page.mock_fleet().expect("the walk's Fleet").heard, vec![("Why hold Pocket?".to_string(), Some("turn:31".to_string()))]);
        });
    }

    #[gpui::test]
    fn a_card_the_operator_raises_in_the_talk_is_drawn_in_it_and_settling_it_ends_the_talk(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update_in(cx, |page, window, cx| page.talk_about("Why hold Pocket?".into(), "turn:31".into(), window, cx));
        page.update(cx, |page, cx| page.scene("said", cx).expect("the Operator answers"));
        page.read_with(cx, |page, cx| {
            assert_eq!(page.talk_cards().iter().map(|turn| turn.id.as_str()).collect::<Vec<_>>(), ["turn:36"]);
            assert_eq!(page.waiting().len(), 4, "the raised card does not wait in line");
            let rows = page.talk_rows(cx);
            assert_eq!(rows.len(), 2, "his words, then the Operator's");
        });
        page.update(cx, |page, cx| page.digit(2, cx));
        cx.run_until_parked();
        page.update(cx, |page, _| {
            assert!(page.talk.is_none(), "the talk ends with its card");
            assert_eq!(page.mock_fleet().expect("the walk's Fleet").pressed[0].id, "turn:36");
            assert_eq!(page.open_item().map(|turn| turn.id.as_str()), Some("turn:32"), "the card it was about is settled too, and the next is in front");
        });
    }

    #[gpui::test]
    fn discuss_later_takes_the_card_out_of_line_and_a_refusal_brings_it_back(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.later(None, cx));
        cx.run_until_parked();
        assert_eq!(front(&page, cx), "turn:32");
        page.update(cx, |page, cx| {
            page.mock_fleet().expect("the walk's Fleet").refuse_later = Some("That time has passed. Pick a later one.".into());
            page.later(None, cx);
        });
        cx.run_until_parked();
        assert_eq!(front(&page, cx), "turn:32");
        page.read_with(cx, |page, _| assert_eq!(page.refused.as_ref().map(|(id, _)| id.as_str()), Some("turn:32")));
    }

    #[gpui::test]
    fn a_value_for_what_the_operator_does_unasked_is_set_and_a_refusal_says_why(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.set_acts("armada".into(), Some(OperatorActs::OnlyWhenTold), cx));
        cx.run_until_parked();
        page.read_with(cx, |page, cx| assert_eq!(page.acts_project(cx).and_then(|one| one.acts), Some(OperatorActs::OnlyWhenTold)));
        page.update(cx, |page, cx| {
            page.mock_fleet().expect("the walk's Fleet").refuse_acts = Some("No project has that id".into());
            page.set_acts("armada".into(), Some(OperatorActs::ActThenTell), cx);
        });
        cx.run_until_parked();
        page.read_with(cx, |page, cx| {
            assert_eq!(page.acts_project(cx).and_then(|one| one.acts), Some(OperatorActs::OnlyWhenTold), "the value stays");
            assert!(page.acts_refused.is_some());
        });
    }
}
