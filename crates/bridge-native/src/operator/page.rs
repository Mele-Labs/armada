//! The page's state and acts: the card in front, the cards queued behind it in Next up, the talk about a card, and what the
//! Operator is allowed to do unasked. Every press answers at once: the page moves first and takes Fleet's page when it
//! comes. A refusal puts the card back and says why on it.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use gpui::{App, AppContext as _, Bounds, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable as _, FollowMode, ListAlignment, ListState, Pixels, ScrollHandle, SharedString, Subscription, Task, Window};
use ipc::operator_board::{DispatchWay, OperatorDispatched};
use ipc::operator_talk::TalkToOperator;
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorAnswered, OperatorPage, OperatorThen, OperatorTurnItem};
use ipc::{OperatorActs, ParkedIdea, SessionId, WaitingAct};

use super::composer::{Composed, OperatorComposer};
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
    /// The Operator's own session, from the status card's button: it opens as a Job's view does, and the Operator's page
    /// is where Esc comes back to.
    Session(SessionId),
    /// A plan, sketch, page or draft a Job holds, from its card: the Review panel opens over the page with it chosen, the
    /// Operator's reading and pick beside it.
    Review(ipc::ItemRef, Option<(SharedString, Option<SharedString>)>),
}

/// Items that joined the queue since the page was last read, oldest first: what raises a toast. Jobs
/// screens own toasts (`banner.rs`, `notify.rs`) and subscribe to this on [`super::live`].
#[derive(Clone, Debug, PartialEq)]
pub struct Arrived(pub Vec<OperatorTurnItem>);

impl EventEmitter<Opened> for OperatorPageView {}
impl EventEmitter<Arrived> for OperatorPageView {}

/// A talk about a card: his words with its tag, and the cards the Operator raises about it, in the talk under the card.
#[derive(Clone, Debug, PartialEq)]
pub struct Talk {
    /// The card's id.
    pub about: String,
    /// The card's title, for the tag in the box and the talk's About row.
    pub title: SharedString,
    /// What he sent that the thread has not echoed yet, drawn as his own message.
    pub sent: Option<SharedString>,
}

/// Where each press that opens a menu was drawn, by name.
pub(super) type Bounded = Rc<RefCell<HashMap<&'static str, Bounds<Pixels>>>>;

pub struct OperatorPageView {
    pub(super) wire: Wire,
    /// Fleet's page, with this sitting's presses already on it. `None` until it is read: a skeleton.
    pub(super) page: Option<OperatorPage>,
    /// The card in front, by id: where he brought one from Next up, else the first.
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
    /// What he sent to the Operator about no card that the thread has not echoed yet, drawn as his own message.
    pub(super) loose: Option<SharedString>,
    /// The box under the card, whose send is a talk, about the card its tag names, or a dispatch with a kind chip picked.
    pub(super) composer: Entity<OperatorComposer>,
    /// The ideas the Operator kept for later: Next up lists them after the cards.
    pub(super) parked: Entity<ParkedIdeas>,
    /// The kept idea brought to the front from Next up, by id, drawn as a card in place of the Operator's own.
    pub(super) idea_front: Option<String>,
    /// The card in the feed's working entries brought to the front to be read, by id: it is picked, so it takes no press.
    pub(super) working_front: Option<String>,
    parked_reading: bool,
    /// The Operator's own session as Fleet holds it, which the status card reads. `None` is a session not started.
    pub(super) session: Option<ipc::SessionRecord>,
    /// Restart is in flight: the switch goes off and on again, and the card reads Starting until it is done.
    pub(super) restarting: bool,
    /// The session that was running when Restart was pressed.
    restart_from: Option<SessionId>,
    /// Why Fleet refused the switch that restarts the Operator.
    pub(super) restart_refused: Option<SharedString>,
    pub focus: FocusHandle,
    /// Every time on the page read as its age, with the clock in its tooltip; a press on one flips them together, as the thread's do.
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
    /// The talk under the card, a list that follows its tail, and the rows it draws.
    pub(super) talk_list: ListState,
    pub(super) talk_said: Vec<super::talk::Said>,
    /// The pictures under his words in the talk, by the file's id.
    pub(super) talk_pictures: HashMap<String, super::talk::TalkPicture>,
    /// The talk has overflowed its space: the card in front folds to one line from then on.
    pub(super) talk_long: Rc<Cell<bool>>,
    /// How tall the talk's rows are together, as last drawn.
    pub(super) talk_tall: Rc<Cell<gpui::Pixels>>,
    /// How tall the open card's scrolling words are, and the room the front needs with them scrolled away, as last drawn.
    pub(super) said_tall: Rc<Cell<gpui::Pixels>>,
    pub(super) front_floor: Rc<Cell<gpui::Pixels>>,
    /// How much of the open card's answers a short window scrolled away, as last drawn.
    pub(super) answers_hidden: Rc<Cell<gpui::Pixels>>,
    /// Where the open card's foot ends, the front's height and the room it needs, where the box ends and how tall the talk
    /// is, as last drawn.
    pub(super) foot_low: Rc<Cell<gpui::Pixels>>,
    pub(super) front_need: Rc<Cell<(gpui::Pixels, gpui::Pixels)>>,
    pub(super) box_low: Rc<Cell<gpui::Pixels>>,
    pub(super) talk_room: Rc<Cell<gpui::Pixels>>,
    /// How much of the talk's top row its top edge cuts while it follows its tail, as last drawn.
    pub(super) talk_cut: Rc<Cell<gpui::Pixels>>,
    /// He opened the folded card. It folds again when the talk grows or he sends.
    pub(super) unfolded: bool,
    /// The cards that would have taken the front while it was folded or the talk in use, oldest first: the new-card strip.
    pub(super) fresh: Vec<String>,
    /// How long the live thread was when the talk last read it, so a change elsewhere in Fleet does not read it again.
    talk_heard: Option<(usize, Option<String>)>,
    /// The page's first tab stop, its first pill: where Tab goes from the page itself.
    pub(super) first_stop: FocusHandle,
    /// The open card's Snooze, so a test can put the keys on a press.
    pub(super) later_focus: FocusHandle,
    /// The option rows' focus handles, by card and place: a row lifts while it has the keys.
    pub(super) option_focus: RefCell<HashMap<String, FocusHandle>>,
    /// The Ready to try card in front, drawn from the Job view this frame for the card's body to take.
    pub(super) ready_drawn: RefCell<Option<gpui::AnyElement>>,
    /// Where each press that opens a menu was drawn, by name, so the menu opens under it.
    pub(super) drawn: Bounded,
    /// Snooze's menu of when the card comes back, while it is open.
    pub(super) later_menu: Option<(Entity<Dropdown>, [Subscription; 2])>,
    /// The menu of what the Operator may do unasked, while it is open.
    pub(super) acts_menu: Option<(Entity<Dropdown>, [Subscription; 2])>,
    /// Fleet's sentence where it refused the last change of that value.
    pub(super) acts_refused: Option<SharedString>,
    /// Next up, the cards queued behind the one in front and the kept ideas, while it is open.
    pub(super) next_open: bool,
    /// Next up's head, where the keys come back to when it folds.
    pub(super) next_head: FocusHandle,
    /// Next up's first row, where the keys go when it opens.
    pub(super) next_first: FocusHandle,
    /// Everything Next up draws, so the page can tell the keys are inside it.
    pub(super) next_group: FocusHandle,
    /// A host screen draws the page's chrome: it draws as the right side of that screen, with its own header row, the card and
    /// the talk in one centred column, and no padding or ground of its own.
    pub(super) embedded: bool,
    /// Whether the side column (Next up and the feed) was drawn last frame, so a pane that turns out narrower than the first
    /// guess folds it at once.
    pub(super) side_drawn: Rc<Cell<bool>>,
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
        let parked = cx.new(|_| ParkedIdeas::new());
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
            loose: None,
            composer,
            parked,
            idea_front: None,
            working_front: None,
            parked_reading: false,
            session: None,
            restarting: false,
            restart_from: None,
            restart_refused: None,
            focus: cx.focus_handle(),
            aged: true,
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
            talk_list: {
                let list = ListState::new(0, ListAlignment::Top, gpui::px(600.));
                list.set_follow_mode(FollowMode::Tail);
                list
            },
            talk_said: Vec::new(),
            talk_pictures: HashMap::new(),
            talk_long: Rc::new(Cell::new(false)),
            talk_tall: Rc::new(Cell::new(gpui::px(0.))),
            said_tall: Rc::new(Cell::new(gpui::px(0.))),
            front_floor: Rc::new(Cell::new(gpui::px(0.))),
            answers_hidden: Rc::new(Cell::new(gpui::px(0.))),
            foot_low: Rc::new(Cell::new(gpui::px(0.))),
            front_need: Rc::new(Cell::new((gpui::px(0.), gpui::px(0.)))),
            box_low: Rc::new(Cell::new(gpui::px(0.))),
            talk_room: Rc::new(Cell::new(gpui::px(0.))),
            talk_cut: Rc::new(Cell::new(gpui::px(0.))),
            unfolded: false,
            fresh: Vec::new(),
            talk_heard: None,
            first_stop: cx.focus_handle(),
            later_focus: cx.focus_handle(),
            option_focus: RefCell::new(HashMap::new()),
            ready_drawn: RefCell::new(None),
            drawn: Default::default(),
            later_menu: None,
            acts_menu: None,
            acts_refused: None,
            next_open: false,
            next_head: cx.focus_handle(),
            next_first: cx.focus_handle(),
            next_group: cx.focus_handle(),
            embedded: false,
            side_drawn: Rc::new(Cell::new(true)),
            seen: None,
            read_at: 0,
            reading: false,
            subs,
        }
    }

    /// The page as the right side of a host screen: the host draws the title and the chrome around it.
    pub fn embedded(mut self) -> Self {
        self.embedded = true;
        self
    }

    #[cfg(test)]
    pub(super) fn is_embedded(&self) -> bool {
        self.embedded
    }

    /// The caret goes to the box under the card: the Cockpit's ⌘J with no Job open.
    pub fn focus_box(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.composer.update(cx, |composer, cx| composer.focus(window, cx));
    }

    /// Where the host turns embedding on or off after the page is built.
    pub fn set_embedded(&mut self, embedded: bool, cx: &mut Context<Self>) {
        if self.embedded != embedded {
            self.embedded = embedded;
            cx.notify();
        }
    }

    /// The page on the owner's Fleet: read when the port is known, and again on each `operator.page_changed`.
    pub fn live(fleet: &Entity<FleetConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Wire::Live(fleet.clone()), window, cx);
        this.session = super::status::session_of(&fleet.read(cx).held().jobs).cloned();
        this.subs.push(cx.observe(fleet, |this, fleet, cx| {
            let held = fleet.read(cx).held();
            // A Mac confirmed or declined stays pressed until Fleet's calls drop it.
            let waiting: Vec<String> = held.calls.iter().flat_map(|calls| calls.calls.iter()).filter(|call| call.kind == ipc::StandingCallKind::MacPairing).map(|call| call.key.clone()).collect();
            this.pairing_pending.retain(|key| waiting.contains(key));
            let moved = held.operator_page_moved;
            let ready = held.port().is_some();
            let session = super::status::session_of(&held.jobs).cloned();
            if session != this.session {
                // A restart is done once the session is a new one.
                if this.restarting && session.as_ref().is_some_and(|new| this.restart_from.as_ref() != Some(&new.id)) {
                    this.restarting = false;
                }
                this.session = session;
                cx.notify();
            }
            if ready && (moved != this.read_at || (this.page.is_none() && !this.reading)) {
                this.read_at = moved;
                this.read(cx);
            }
            if ready && !this.parked.read(cx).is_read() && !this.parked_reading {
                this.read_parked(cx);
            }
            this.heard_thread(cx);
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
        this.session = this.mock_fleet().and_then(|mock| mock.session.clone());
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
        // A card waiting in the new-card strip is not in front until he shows it.
        self.open.as_deref().and_then(|open| queue.iter().copied().find(|turn| turn.id == open)).or_else(|| queue.iter().copied().find(|turn| !self.fresh.contains(&turn.id)))
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

    /// The Job whose Ready to try is the card in front, drawn whole with its own answers: the window opens that Job in the Job
    /// view without a pick, which holds the card's state.
    pub fn ready_in_front(&self) -> Option<SessionId> {
        if self.idea_front.is_some() || self.working_front.is_some() {
            return None;
        }
        self.open_item().filter(|turn| face::answers_on_card(turn)).and_then(|turn| turn.session_id.clone())
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

    /// The cards waiting on him as Fleet's page last said them, for the Project place's Needs you block.
    pub fn turn(&self) -> &[OperatorTurnItem] {
        self.page.as_ref().map_or(&[], |page| page.turn.as_slice())
    }

    /// Fleet's page replaces the one held. The card in front stays where it is still there, and a card not seen before
    /// is told once. A card he settled here stays out of the queue, though Fleet keeps one whose act it left to him
    /// to open until he does it.
    pub fn set_page(&mut self, mut page: OperatorPage, cx: &mut Context<Self>) {
        // While the card is folded or the talk in use, a card that would take the front waits in the strip instead.
        let holds = self.page.is_some() && self.holds_front(cx);
        let was_front = self.open_item().map(|turn| turn.id.clone());
        // A card whose press is in flight stays in the feed's working entries, though Fleet's page may be older than the press.
        let local: Vec<OperatorTurnItem> = self
            .page
            .as_ref()
            .map(|old| old.working.iter().filter(|held| self.pending.contains(&held.id) && !page.working.iter().any(|one| one.id == held.id)).cloned().collect())
            .unwrap_or_default();
        page.working.splice(0..0, local);
        let was_back: BTreeSet<String> = self.page.as_ref().map(|old| old.turn.iter().filter(|turn| face::returned(turn)).map(|turn| turn.id.clone()).collect()).unwrap_or_default();
        let came_back = page.turn.iter().find(|turn| face::returned(turn) && !was_back.contains(&turn.id) && !self.settled.contains(&turn.id)).map(|turn| turn.id.clone());
        let ids: BTreeSet<String> = page.turn.iter().map(|turn| turn.id.clone()).collect();
        let mut new_ids: Vec<String> = Vec::new();
        if let Some(seen) = &self.seen {
            let new: Vec<OperatorTurnItem> = page.turn.iter().filter(|turn| !seen.contains(&turn.id)).cloned().collect();
            new_ids = new.iter().map(|turn| turn.id.clone()).collect();
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
        let queued: Vec<String> = self.queue().iter().map(|turn| turn.id.clone()).collect();
        self.fresh.retain(|id| queued.contains(id));
        if holds {
            // What would have come in front: a card that came back, or the first new one where nothing was in front.
            let would = came_back.clone().or_else(|| was_front.is_none().then(|| queued.iter().find(|id| new_ids.contains(id)).cloned()).flatten());
            if let Some(id) = would.filter(|id| !self.fresh.contains(id) && Some(id) != was_front.as_ref()) {
                self.fresh.push(id);
            }
        }
        let keep = self.open.as_ref().filter(|open| queued.contains(open)).cloned();
        self.open = keep.or_else(|| queued.iter().find(|id| !self.fresh.contains(id)).cloned());
        // A card that came back stands ahead of the rest, and what went wrong needs his eyes: it takes the front, unless the
        // card there is folded or the talk in use, where it waits in the strip.
        if let Some(id) = came_back.filter(|_| !holds) {
            self.open = Some(id);
            self.idea_front = None;
            self.working_front = None;
            self.main_scroll.set_offset(gpui::Point::default());
        }
        let working = self.page.as_ref().map_or(&[][..], |page| page.working.as_slice());
        self.working_front = self.working_front.take().filter(|id| working.iter().any(|one| one.id == *id));
        self.read_thread(cx);
        self.sync_talk(cx);
        cx.notify();
    }

    // -- the keys and presses ----------------------------------------------------------------------

    pub fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        // ⌥M puts the caret in the box from anywhere on the page, as it does in a Job's view.
        if act.is("message_job") {
            return self.composer.update(cx, |composer, cx| composer.focus(window, cx));
        }
        // ⌘Enter is Show where the new-card strip stands and the box holds no words to send.
        if act.is("send_message") && !self.fresh.is_empty() && self.composer.read(cx).words(cx).trim().is_empty() {
            return self.show_fresh(window, cx);
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
        // Inside Next up the arrows, j, k, Enter and Esc are its own: they go on to its rows.
        if self.next_open && self.next_group.contains_focused(window, cx) {
            return cx.propagate();
        }
        // Esc puts the Operator's own card back where a kept idea, or a card he picked, is in front.
        if act.is("close") && (self.idea_front.take().is_some() || self.working_front.take().is_some()) {
            cx.notify();
            return;
        }
        // A stopped Operator stands in front of the queued card: its keys wait until it is back.
        if self.stopped_in_front() && matches!(act.id, "call_yes" | "call_send" | "call_snooze" | "call_ask" | "call_pick") {
            return;
        }
        match act.id {
            // The registry's slots are j, k, ↓, ↑: even slots go down. They go through the cards waiting.
            "call_move_focus" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "call_yes" => self.answer(OperatorAnswer::Yes, window, cx),
            "call_send" => self.enter(window, cx),
            "call_snooze" => self.open_later(true, window, cx),
            // `/` is Discuss, which the registry knows as Ask the Operator.
            "call_ask" => self.discuss(window, cx),
            "call_pick" => {
                let digit = act.digit.unwrap_or(0) as usize;
                if !self.ready_key(digit, window, cx) {
                    self.digit(digit, cx);
                }
            }
            _ => cx.propagate(),
        }
    }

    /// 1, 2 and 3 on a Job's build to try in front answer it on the Job's own card, as on the Job's thread. Whether they did.
    fn ready_key(&mut self, digit: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.idea_front.is_some() || self.working_front.is_some() {
            return false;
        }
        let Some(turn) = self.target().filter(|turn| face::answers_on_card(turn)) else { return false };
        let (Some(job), item) = (turn.session_id.clone(), turn.item.id.clone()) else { return false };
        crate::job_view::ready_key_on_page(job.as_str(), &item, &digit.to_string(), window, cx)
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
        if to as isize != at {
            crate::ui_sounds::moved();
        }
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
        self.fresh.retain(|one| one != id);
        self.idea_front = None;
        self.working_front = None;
        self.main_scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// Show, on the new-card strip, or ⌘Enter while it stands: the newest card comes in front, unfolded, and the strip goes.
    pub fn show_fresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.fresh.last().cloned() else { return };
        self.bring(&id, cx);
        self.fresh.clear();
        self.unfolded = true;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// The card he picked that the Operator is working on, where it is brought to the front to be read.
    pub(super) fn front_working(&self) -> Option<&OperatorTurnItem> {
        let id = self.working_front.as_deref()?;
        self.page.as_ref()?.working.iter().find(|turn| turn.id == id)
    }

    /// The cards he picked that the Operator has not settled, newest pick first.
    pub fn working(&self) -> &[OperatorTurnItem] {
        self.page.as_ref().map_or(&[], |page| page.working.as_slice())
    }

    /// A card from the feed's working entries comes to the front, read only: its pick and its steps with it. A talk about the
    /// card that was there ends: its card is still waiting. Esc puts that card back.
    pub fn bring_working(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.working().iter().all(|turn| turn.id != id) {
            return;
        }
        if self.talk.take().is_some() {
            self.composer.update(cx, |composer, cx| composer.untag(cx));
        }
        self.idea_front = None;
        self.working_front = Some(id.to_string());
        self.main_scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// The old name of a press on a card that is not the open one.
    pub fn open_id(&mut self, id: &str, cx: &mut Context<Self>) {
        self.bring(id, cx);
    }

    /// Next up opens, with the keys on its first row, or folds, with the keys back on its head.
    pub fn toggle_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.next_open && self.next_count(cx) == 0 {
            return;
        }
        self.next_open = !self.next_open;
        if self.next_open {
            let keys = self.next_first.clone();
            window.defer(cx, move |window, cx| window.focus(&keys, cx));
        } else {
            window.focus(&self.next_head, cx);
        }
        cx.notify();
    }

    /// Esc inside Next up folds it.
    pub fn close_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.next_open) {
            window.focus(&self.next_head, cx);
            cx.notify();
        }
    }

    /// The kept ideas, newest first.
    pub(super) fn ideas(&self, cx: &App) -> Vec<ParkedIdea> {
        self.parked.read(cx).ideas().into_iter().cloned().collect()
    }

    /// What Next up counts: the cards queued behind the one in front, and the kept ideas.
    pub fn next_count(&self, cx: &App) -> usize {
        self.waiting().len() + self.parked.read(cx).ideas().len()
    }

    /// The kept idea in front, while it is still kept.
    pub(super) fn front_idea(&self, cx: &App) -> Option<ParkedIdea> {
        let id = self.idea_front.as_deref()?;
        self.parked.read(cx).ideas().into_iter().find(|idea| idea.id == id).cloned()
    }

    /// A kept idea comes to the front as a card. A talk about the card that was there ends: its card is still waiting.
    pub fn bring_idea(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.parked.read(cx).ideas().iter().all(|idea| idea.id != id) {
            return;
        }
        if self.talk.take().is_some() {
            self.composer.update(cx, |composer, cx| composer.untag(cx));
        }
        self.idea_front = Some(id.to_string());
        self.working_front = None;
        self.main_scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    /// The front idea's press: Start sends it as a dispatch, Remove lets it go. The page answers both as it answers
    /// a press on the idea's own row, once Fleet has the dispatch or the removal.
    pub fn ask_idea(&mut self, asked: ParkedAsked, cx: &mut Context<Self>) {
        self.parked.update(cx, |_, cx| cx.emit(asked));
    }

    // -- settling ----------------------------------------------------------------------------------

    /// A digit: the option in that place settles the card; on a go-ahead to file, the Then in that place.
    pub fn digit(&mut self, digit: usize, cx: &mut Context<Self>) {
        // A kept idea in front has two options: 1 starts it and 2 lets it go.
        if let Some(idea) = self.front_idea(cx) {
            match digit {
                1 => self.ask_idea(ParkedAsked::Start(idea), cx),
                2 => self.ask_idea(ParkedAsked::Remove(idea.id), cx),
                _ => {}
            }
            return;
        }
        // A card he picked is read only: it takes no press.
        if self.working_front.is_some() {
            return;
        }
        let Some(turn) = self.target().cloned() else { return };
        let Some(at) = digit.checked_sub(1) else { return };
        if face::chooses(&turn) {
            if let Some(option) = turn.item.options.get(at) {
                self.settle_option(&turn.id, &option.label.clone(), cx);
            }
        } else if let Some(offer) = turn.then.get(at) {
            self.settle_then(&turn.id, offer.then, cx);
        } else if at == 0 && face::goes_to_review(&turn) {
            self.review(&turn, cx);
        } else if at == 0 && turn.then.is_empty() && !face::answers_on_card(&turn) {
            self.settle(&turn.id, None, None, cx);
        }
    }

    /// The card's item opens in the Review panel over the page, with what the Operator read of it.
    fn review(&mut self, turn: &OperatorTurnItem, cx: &mut Context<Self>) {
        let Some(session_id) = turn.session_id.clone() else { return };
        let read = turn.read.as_ref().map(|read| (SharedString::from(read.read.clone()), Some(read.pick.trim()).filter(|pick| !pick.is_empty()).map(|pick| SharedString::from(pick.to_string()))));
        cx.emit(Opened::Review(ipc::ItemRef { session_id, item: turn.item.id.clone() }, read));
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
        if self.idea_front.is_some() || self.working_front.is_some() {
            return;
        }
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
        // A plan, sketch, page or draft is read and answered in the Review panel; a Ready to try on its own card's buttons.
        if face::goes_to_review(&turn) {
            return self.review(&turn, cx);
        }
        if face::answers_on_card(&turn) {
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
        // A card moves to the feed's working entries at once, as Fleet puts it there: the next card is in front before it answers.
        self.start_working(&turn, option.as_deref());
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

    /// His press on a card picks it: it shows at the top of the feed's working entries, with no steps yet, until Fleet's page says what
    /// became of it.
    fn start_working(&mut self, turn: &OperatorTurnItem, option: Option<&str>) {
        let Some(option) = option.map(str::to_string).or_else(|| face::pick_of(turn).map(|pick| pick.to_string())) else { return };
        let at = self.clock_now.unwrap_or_else(chrono::Local::now).to_rfc3339();
        let Some(picked) = face::picked_card(turn, &option, &at) else { return };
        if let Some(page) = self.page.as_mut() {
            page.working.retain(|one| one.id != turn.id);
            page.working.insert(0, picked);
        }
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
                    page.working.retain(|one| one.id != turn.id);
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
                                page.start_working(&turn, press.option.as_deref());
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

    // -- Snooze -----------------------------------------------------------------------------

    /// Snooze's menu, under its press: when the card comes back. From the keys, they rest on its first choice;
    /// picked, the card goes back until then.
    pub fn open_later(&mut self, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.idea_front.is_some() || self.working_front.is_some() {
            return;
        }
        let Some(turn) = self.target().cloned() else { return };
        let Some(bounds) = self.drawn.borrow().get("later").copied() else { return };
        let now = self.clock_now.unwrap_or_else(chrono::Local::now);
        let choices = face::later_choices(now);
        let entries = choices.iter().map(|(id, label, at)| Entry::Row(Row::new(id, *label).detail(face::back_at(*at, now)))).collect();
        let gap = cx.look().length(crate::look::token::SPACE_1);
        let at = gpui::point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height + gap);
        let menu = cx.new(|cx| Dropdown::new("Snooze", entries, at, gpui::Anchor::TopRight, from_keys, cx));
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

    /// Snooze: the card goes back until `until`, or for the rest of the day, and the next one comes to the front.
    pub fn later(&mut self, until: Option<chrono::DateTime<chrono::Local>>, cx: &mut Context<Self>) {
        if self.idea_front.is_some() || self.working_front.is_some() {
            return;
        }
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
        // A kept idea has no Discuss: the keys go to the box, untagged.
        let target = if self.idea_front.is_some() || self.working_front.is_some() { None } else { self.target().cloned() };
        let Some(turn) = target else { return self.composer.update(cx, |composer, cx| composer.focus(window, cx)) };
        let title = face::card_title(&turn);
        self.composer.update(cx, |composer, cx| {
            composer.tag(turn.id.clone(), title, window, cx);
            composer.focus(window, cx);
        });
        cx.notify();
    }

    /// The talk about a card, as he sent words with its tag: the card folds and the Operator's thread runs under it.
    fn talk_about(&mut self, talk: TalkToOperator, about: String, window: &mut Window, cx: &mut Context<Self>) {
        let words = talk.words.clone();
        let title = self.items().iter().find(|turn| turn.id == about).map(face::card_title).unwrap_or_else(|| about.clone().into());
        // He is talking about the front card, or one the Operator raised in the talk.
        if self.talk.as_ref().map(|talk| talk.about.as_str()) != Some(about.as_str()) && self.is_front(&about) {
            self.talk = Some(Talk { about: about.clone(), title, sent: None });
        }
        if let Some(talk) = self.talk.as_mut() {
            talk.sent = Some(words.clone().into());
        }
        // Only his last send waits to be echoed.
        self.loose = None;
        // A send folds the card he had opened, and the talk shows its end.
        self.unfolded = false;
        self.read_thread(cx);
        self.sync_talk(cx);
        self.talk_list.set_follow_mode(FollowMode::Tail);
        cx.notify();
        let task = self.wire.say(talk, cx);
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

    /// A talk to the Operator about no card, as he sent words with no tag: they go in the talk under the card with no
    /// About row, and no card folds or settles for them.
    fn talk_loose(&mut self, talk: TalkToOperator, window: &mut Window, cx: &mut Context<Self>) {
        let words = talk.words.clone();
        self.loose = Some(words.clone().into());
        if let Some(talk) = self.talk.as_mut() {
            talk.sent = None;
        }
        let unfolded = self.unfolded;
        self.read_thread(cx);
        self.sync_talk(cx);
        self.unfolded = unfolded;
        self.talk_list.set_follow_mode(FollowMode::Tail);
        cx.notify();
        let task = self.wire.say(talk, cx);
        let Some(task) = task else {
            self.composer.update(cx, |composer, cx| composer.sent(window, cx));
            return;
        };
        let composer = self.composer.clone();
        cx.spawn_in(window, async move |this, cx| {
            let said = task.await;
            let _ = this.update_in(cx, |this, window, cx| match said {
                Ok(_) => composer.update(cx, |composer, cx| composer.sent(window, cx)),
                Err(why) => {
                    this.loose = None;
                    this.sync_talk(cx);
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

    /// A press on the folded card: it opens above the talk until the talk grows or he sends.
    pub fn unfold(&mut self, cx: &mut Context<Self>) {
        self.unfolded = true;
        cx.notify();
    }

    /// The card in front is folded to one line: the talk has overflowed its space and he has not opened it since. A card
    /// that says the Operator is off or stopped, or a Mac asking to pair, never folds.
    pub fn is_folded(&self) -> bool {
        self.talk_long.get() && !self.unfolded && !matches!(self.status(), super::status::Status::Off) && !self.stopped_in_front() && self.pairing_front.is_none()
    }

    /// A stopped Operator stands in front, in red, ahead of any queued card. A kept idea or picked card he brought is his own.
    pub(super) fn stopped_in_front(&self) -> bool {
        matches!(self.status(), super::status::Status::Stopped { .. }) && self.idea_front.is_none() && self.working_front.is_none()
    }

    /// The card in front is folded, or the talk is in use: his words are about a card, or wait in the box.
    pub(super) fn holds_front(&self, cx: &App) -> bool {
        !matches!(self.status(), super::status::Status::Off) && !self.stopped_in_front() && (self.is_folded() || self.talk.is_some() || !self.composer.read(cx).words(cx).trim().is_empty())
    }

    /// The Operator session's thread, read from Fleet once: the talk's rows are drawn from it, and Fleet keeps it fresh.
    fn read_thread(&mut self, cx: &mut Context<Self>) {
        let (Some(session), Wire::Live(fleet)) = (self.page.as_ref().and_then(|page| page.session_id.clone()), &self.wire) else { return };
        if fleet.read(cx).held().threads.contains_key(session.as_str()) {
            return;
        }
        fleet.update(cx, |fleet, cx| fleet.read_thread(session.as_str(), false, cx));
    }

    /// Fleet changed: where the Operator's thread did, the talk reads it again.
    fn heard_thread(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.page.as_ref().and_then(|page| page.session_id.clone()) else { return };
        let heard = {
            let Wire::Live(fleet) = &self.wire else { return };
            let Some(held) = fleet.read(cx).held().threads.get(session.as_str()) else { return };
            let last = held.items.last().and_then(|item| match item {
                ipc::ThreadItem::Row { row: ipc::SessionRow::Message { id, text, .. } } => Some(format!("{id}:{}", text.len())),
                _ => None,
            });
            (held.items.len(), last)
        };
        if self.talk_heard.as_ref() != Some(&heard) {
            self.talk_heard = Some(heard);
            let was = self.talk_said.len();
            self.sync_talk(cx);
            if self.talk_said.len() != was {
                cx.notify();
            }
        }
    }

    // -- what the Operator may do unasked -----------------------------------------------------------

    /// The project the head's value is for: the one the window is scoped to, else the first Fleet lists. On All there is
    /// none: the value is each project's own, and the chip says so.
    pub fn acts_project(&self, cx: &App) -> Option<ipc::ProjectOperatorActs> {
        let list = &self.page.as_ref()?.operator_acts;
        let (scoped, _) = crate::dispatch::screen::scope(cx)?;
        scoped.and_then(|id| list.iter().find(|one| one.project == id.as_ref())).or_else(|| list.first()).cloned()
    }

    /// The window is on All and Fleet lists projects: the chip is for every project at once.
    pub fn acts_on_all(&self, cx: &App) -> bool {
        self.page.as_ref().is_some_and(|page| !page.operator_acts.is_empty()) && crate::dispatch::screen::scope(cx).is_none()
    }

    /// The menu of values, under the chip. On All each project is a header with its three values under it.
    pub fn open_acts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let on_all = self.acts_on_all(cx);
        let Some(bounds) = self.drawn.borrow().get("acts").copied() else { return };
        let gap = cx.look().length(crate::look::token::SPACE_1);
        let at = gpui::point(bounds.origin.x + bounds.size.width, bounds.origin.y + bounds.size.height + gap);
        // Each pick by its id: the project it is for, and the value.
        let mut picks: Vec<(&'static str, String, OperatorActs)> = Vec::new();
        let entries: Vec<Entry> = if on_all {
            let list = self.page.as_ref().map(|page| page.operator_acts.clone()).unwrap_or_default();
            list.iter()
                .flat_map(|project| {
                    let head = Entry::Label(interned(project.name.clone()));
                    let values: Vec<Entry> = ACTS
                        .iter()
                        .map(|(word, acts, label, _)| {
                            let id = interned(format!("{}:{word}", project.project));
                            picks.push((id, project.project.clone(), *acts));
                            Entry::Row(Row::new(id, *label).member().ticked(project.acts == Some(*acts)))
                        })
                        .collect();
                    std::iter::once(head).chain(values)
                })
                .collect()
        } else {
            let Some(project) = self.acts_project(cx) else { return };
            ACTS.iter()
                .map(|(id, acts, label, _)| {
                    picks.push((id, project.project.clone(), *acts));
                    Entry::Choice { id, label: (*label).into(), on: project.acts == Some(*acts) }
                })
                .collect()
        };
        let menu = cx.new(|cx| Dropdown::new("What the Operator does unasked", entries, at, gpui::Anchor::TopRight, true, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id: choice, .. } = picked;
            this.acts_menu = None;
            window.focus(&this.focus, cx);
            if let Some((_, project, acts)) = picks.iter().find(|(word, ..)| word == choice) {
                this.set_acts(project.clone(), Some(*acts), cx);
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

    // -- the Operator's own session ----------------------------------------------------------------

    /// Whether the page reads the Operator switched off: the Cockpit draws its floor plan in the page's place.
    pub fn is_off(&self) -> bool {
        self.page.as_ref().is_some_and(|page| !page.on)
    }

    /// A switch is in flight, and which way it goes.
    pub fn switching(&self) -> Option<bool> {
        self.switching.then_some(!self.switching_off)
    }

    /// Why Fleet refused the last Switch on, where it did.
    pub fn switch_refused(&self) -> Option<&str> {
        self.switch_refused.as_deref()
    }

    /// The Operator's session, where Fleet holds one.
    pub fn session_id(&self) -> Option<SessionId> {
        self.session.as_ref().map(|session| session.id.clone())
    }

    /// Where the Operator's session stands, as its record reads. While Restart is in flight it reads Starting; with the
    /// switch off, Off.
    pub fn status(&self) -> super::status::Status {
        if self.restarting {
            return super::status::Status::Starting;
        }
        if self.page.as_ref().is_some_and(|page| !page.on) {
            return super::status::Status::Off;
        }
        super::status::status_of(self.session.as_ref())
    }

    /// "See what it is doing": the Operator's session opens as a Job's view does.
    pub fn open_session(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.session_id() {
            cx.emit(Opened::Session(id));
        }
    }

    /// Restart, on a stopped Operator. Fleet has no act that starts a stopped Operator's session again: it ends the session when
    /// `features.operator` goes off and starts one on the next tick once it is on, so the switch goes off and on.
    /// The card reads Starting until the new session is held, or for half a minute where none comes.
    pub fn restart(&mut self, cx: &mut Context<Self>) {
        if self.restarting || self.switching {
            return;
        }
        self.restarting = true;
        self.restart_from = self.session_id();
        self.restart_refused = None;
        cx.notify();
        let off = self.wire.switch(false, cx);
        let task = cx.spawn(async move |this, cx| {
            let on = match off.await {
                Ok(()) => this.update(cx, |this, cx| this.wire.switch(true, cx)).ok(),
                Err(why) => {
                    let _ = this.update(cx, |this, cx| this.restart_refused_by(why, cx));
                    return;
                }
            };
            let Some(on) = on else { return };
            match on.await {
                Ok(()) => {
                    let live = this
                        .update(cx, |this, cx| {
                            // The walk's Fleet sends no event, so it is read in its place and holds no session yet.
                            if !this.wire.is_live() {
                                this.restarting = false;
                                this.session = this.mock_fleet().and_then(|mock| mock.session.clone());
                                this.read(cx);
                            }
                            cx.notify();
                            this.wire.is_live()
                        })
                        .unwrap_or(false);
                    if !live {
                        return;
                    }
                    cx.background_executor().timer(std::time::Duration::from_secs(30)).await;
                    let _ = this.update(cx, |this, cx| {
                        if std::mem::take(&mut this.restarting) {
                            cx.notify();
                        }
                    });
                }
                Err(why) => {
                    let _ = this.update(cx, |this, cx| this.restart_refused_by(format!("switched off but not on again: {why}"), cx));
                }
            }
        });
        self.track(task);
    }

    fn restart_refused_by(&mut self, why: String, cx: &mut Context<Self>) {
        self.restarting = false;
        self.restart_refused = Some(why.into());
        cx.notify();
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

    /// What the box asks: its talk goes to the Operator, or the keys come back to the page.
    fn composed(&mut self, event: &Composed, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            Composed::Leave => window.focus(&self.focus, cx),
            Composed::Talk { words, about, attachments, mentions } => {
                let talk = TalkToOperator { words: words.clone(), about: about.clone(), attachments: attachments.clone(), mentions: mentions.clone() };
                match about {
                    Some(about) => self.talk_about(talk.clone(), about.clone(), window, cx),
                    None => self.talk_loose(talk, window, cx),
                }
            }
        }
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
                Ok(list) => {
                    // The idea is gone, so the Operator's own card is in front again: its keys and presses are live.
                    if this.idea_front.as_deref() == Some(id.as_str()) {
                        this.idea_front = None;
                    }
                    this.parked.update(cx, |parked, cx| parked.set(list.ideas, cx));
                    cx.notify();
                }
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

    /// Move the mock Fleet to a named scene: the Operator answers the talk and raises a card in it (`said`), says more in
    /// it (`replies`), a new card joins the queue (`arrives`) or comes where nothing was in front (`front-arrives`), three join at once (`raised`), the next answer is refused (`refuse`) or its
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
            // He scrolls the talk up to the Operator's last words, and it stops following its end.
            "talk-up" => {
                let ix = self.talk_said.iter().rposition(|said| matches!(said, super::talk::Said::Operator { .. })).ok_or("the Operator has said nothing")?;
                self.talk_list.scroll_to(gpui::ListOffset { item_ix: ix, offset_in_item: gpui::px(0.) });
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
                self.loose = None;
                // The card it raised arrives with the page, once, and raises no toast.
                if let Some(seen) = self.seen.as_mut() {
                    seen.extend(page.turn.iter().map(|turn| turn.id.clone()));
                }
                self.set_page(page, cx);
            }
            // The Operator says more in the talk, with no card raised: the talk grows, and a card he opened folds again.
            "replies" => {
                mock.replies();
                self.sync_talk(cx);
                cx.notify();
            }
            // A card arrives where nothing was in front, while the card is folded: it waits in the new-card strip.
            "front-arrives" => {
                mock.comes();
                let page = mock.page.clone();
                self.set_page(page, cx);
            }
            // A Job's plan, then its build to try, comes in front unfolded, as a press on the strip's Show brings a card.
            "plan-comes" | "build-comes" => {
                let id = if name == "plan-comes" { mock.plan_comes() } else { mock.build_comes() }.ok_or("the card did not decode")?;
                let (page, session) = (mock.page.clone(), mock.session.clone());
                if let Some(seen) = self.seen.as_mut() {
                    seen.insert(id.clone());
                }
                self.session = session;
                self.set_page(page, cx);
                self.bring(&id, cx);
                self.fresh.clear();
                self.unfolded = true;
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
            // The Operator reports the oldest card it is working on failed: it comes back first in line, with why.
            "returns" => {
                mock.returns().ok_or("no card is being worked on")?;
                let page = mock.page.clone();
                self.set_page(page, cx);
            }
            "raised" => {
                mock.raised();
                let page = mock.page.clone();
                self.set_page(page, cx);
            }
            // Fleet refuses the next answer, or its reply never arrives.
            "refuse" => mock.fail_answer = Some(Failed::Refused("The Operator could not settle the card.".into())),
            "lose" => mock.fail_answer = Some(Failed::Lost("no answer in 30 seconds".into())),
            // Fleet refuses the next Snooze: the time it names has passed by the time it reads it.
            "late" => mock.refuse_later = Some("That time has passed. Pick a later one.".into()),
            "acts-refused" => mock.refuse_acts = Some("No project has that id".into()),
            // Nothing needs him: the page empties, and the Operator is in a turn, between turns, or stopped.
            "quiet" | "working" | "watching" | "stopped" => {
                if name == "quiet" {
                    mock.quiet();
                } else {
                    mock.session = Some(scenes::session(name));
                }
                let (page, session) = (mock.page.clone(), mock.session.clone());
                self.session = session;
                self.set_page(page, cx);
            }
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

/// A menu entry's id lives as long as the app: one per project and value, kept once however often the menu opens.
fn interned(text: String) -> &'static str {
    static HELD: std::sync::OnceLock<std::sync::Mutex<HashMap<String, &'static str>>> = std::sync::OnceLock::new();
    let mut held = HELD.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    held.entry(text.clone()).or_insert_with(|| Box::leak(text.into_boxed_str()))
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
    use gpui::{IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext};

    /// His words as a talk, with nothing sent beside them.
    fn said(words: &str, about: Option<&str>) -> TalkToOperator {
        TalkToOperator { words: words.into(), about: about.map(str::to_string), attachments: Vec::new(), mentions: Vec::new() }
    }

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

    fn acts_of(page: &OperatorPageView, project: &str) -> Option<OperatorActs> {
        page.page.as_ref()?.operator_acts.iter().find(|one| one.project == project)?.acts
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
    fn a_click_on_the_operators_pick_settles_the_card_with_it(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let (label, name) = page.read_with(cx, |page, _| {
            let turn = page.open_item().expect("a card in front");
            let pick = face::pick_of(turn).map(|pick| pick.to_string()).unwrap_or_else(|| turn.item.options[0].label.clone());
            let option = turn.item.options.iter().find(|one| one.label == pick).expect("the pick is an option");
            let cost = option.description.clone().filter(|said| !said.is_empty()).map(SharedString::from);
            (pick.clone(), super::super::card::option_name(&pick.into(), cost.as_ref()))
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let at = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), gpui::Role::Button, &name).map(|drawn| drawn.bounds)).unwrap_or_else(|| panic!("{name} is drawn"));
        cx.simulate_click(at.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        page.update(cx, |page, _| {
            let pressed = page.mock_fleet().expect("the walk's Fleet").pressed.clone();
            assert_eq!(pressed.first().and_then(|one| one.option.as_deref()), Some(label.as_str()), "the pick's press goes out with the pick");
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
        page.update_in(cx, |page, window, cx| page.talk_about(said("Why hold Pocket?", Some("turn:31")), "turn:31".into(), window, cx));
        page.update(cx, |page, _| {
            assert_eq!(page.talk.as_ref().map(|talk| talk.about.as_str()), Some("turn:31"));
            assert_eq!(page.mock_fleet().expect("the walk's Fleet").heard, vec![("Why hold Pocket?".to_string(), Some("turn:31".to_string()))]);
        });
    }

    #[gpui::test]
    fn a_talk_about_no_card_goes_with_no_id_settles_nothing_and_its_answer_follows_in_the_talk(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update_in(cx, |page, window, cx| page.talk_loose(said("Which Jobs are running now?", None), window, cx));
        page.update(cx, |page, _| {
            assert!(page.talk.is_none(), "no card is talked about");
            assert_eq!(page.mock_fleet().expect("the walk's Fleet").heard, vec![("Which Jobs are running now?".to_string(), None)]);
            use super::super::talk::Said;
            assert!(matches!(&page.talk_said[..], [Said::You { at: None, .. }]), "his words with no About row: {:?}", page.talk_said);
        });
        page.update(cx, |page, cx| page.scene("said", cx).expect("the Operator answers"));
        page.update(cx, |page, _| {
            use super::super::talk::Said;
            assert!(matches!(&page.talk_said[..], [Said::You { at: Some(_), .. }, Said::Operator { .. }]), "his words once, then the answer: {:?}", page.talk_said);
            assert!(page.talk_cards().is_empty(), "no card is raised");
            assert_eq!(page.open_item().map(|turn| turn.id.as_str()), Some("turn:31"), "the card in front stands");
            assert!(page.mock_fleet().expect("the walk's Fleet").pressed.is_empty());
        });
    }

    #[gpui::test]
    fn a_picture_he_sends_the_operator_is_drawn_under_his_words_in_the_talk(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let mut talk = said("This toast is wrong", None);
        talk.attachments = vec![ipc::SessionUpload { name: "Pasted file.png".into(), media_type: "image/png".into(), data: String::new() }];
        page.update_in(cx, |page, window, cx| page.talk_loose(talk, window, cx));
        page.update(cx, |page, cx| page.scene("said", cx).expect("the Operator answers"));
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        page.read_with(cx, |page, _| {
            use super::super::talk::Said;
            assert!(matches!(&page.talk_said[..], [Said::You { files, .. }, ..] if files.len() == 1), "the thread echoes his picture on his row: {:?}", page.talk_said);
        });
        assert!(drawn(cx, gpui::Role::Image, "Pasted file.png"), "the picture is drawn under his words");
    }

    #[gpui::test]
    fn a_card_the_operator_raises_in_the_talk_is_drawn_in_it_and_settling_it_ends_the_talk(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update_in(cx, |page, window, cx| page.talk_about(said("Why hold Pocket?", Some("turn:31")), "turn:31".into(), window, cx));
        page.update(cx, |page, cx| page.scene("said", cx).expect("the Operator answers"));
        page.read_with(cx, |page, _| {
            assert_eq!(page.talk_cards().iter().map(|turn| turn.id.as_str()).collect::<Vec<_>>(), ["turn:36"]);
            assert_eq!(page.waiting().len(), 4, "the raised card does not wait in line");
            use super::super::talk::Said;
            assert!(
                matches!(&page.talk_said[..], [Said::About { .. }, Said::You { .. }, Said::Operator { .. }, Said::Raised(id)] if id == "turn:36"),
                "the About row, his words, the Operator's, then the card it raised: {:?}",
                page.talk_said
            );
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
    fn snooze_takes_the_card_out_of_line_and_a_refusal_brings_it_back(cx: &mut TestAppContext) {
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
        page.read_with(cx, |page, _| assert_eq!(acts_of(page, "armada"), Some(OperatorActs::OnlyWhenTold)));
        page.update(cx, |page, cx| {
            page.mock_fleet().expect("the walk's Fleet").refuse_acts = Some("No project has that id".into());
            page.set_acts("armada".into(), Some(OperatorActs::ActThenTell), cx);
        });
        cx.run_until_parked();
        page.read_with(cx, |page, _| {
            assert_eq!(acts_of(page, "armada"), Some(OperatorActs::OnlyWhenTold), "the value stays");
            assert!(page.acts_refused.is_some());
        });
    }

    #[gpui::test]
    fn on_all_the_chip_is_for_every_project_and_its_menu_sets_the_project_a_pick_is_under(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.read_with(cx, |page, cx| {
            assert!(page.acts_on_all(cx), "the walk's window is on All");
            assert!(page.acts_project(cx).is_none(), "no project stands for All");
        });
        page.update_in(cx, |page, window, cx| {
            page.drawn.borrow_mut().insert("acts", Bounds::default());
            page.open_acts(window, cx);
        });
        let menu = page.read_with(cx, |page, cx| {
            let (menu, _) = page.acts_menu.as_ref().expect("the menu is open");
            // Each project is a header with its three values under it.
            let entries = &menu.read(cx).entries;
            assert_eq!(entries.len(), 8);
            assert!(matches!(entries[0], Entry::Label("armada")) && matches!(entries[4], Entry::Label("storefront")));
            assert!(matches!(&entries[1], Entry::Row(row) if row.ticked == Some(true)), "armada's Act then tell is ticked where it is set");
            assert!(matches!(&entries[5], Entry::Row(row) if row.ticked == Some(false)), "storefront has none set");
            menu.clone()
        });
        menu.update(cx, |_, cx| cx.emit(Picked::Entry { id: "storefront:only_when_told", keep: false }));
        cx.run_until_parked();
        page.read_with(cx, |page, _| {
            assert_eq!(acts_of(page, "storefront"), Some(OperatorActs::OnlyWhenTold));
            assert_eq!(acts_of(page, "armada"), Some(OperatorActs::ActThenTell), "the other project is not touched");
        });
    }

    #[gpui::test]
    fn next_up_counts_the_queue_and_the_kept_ideas_and_an_idea_comes_to_the_front(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.read_with(cx, |page, cx| {
            assert_eq!(page.next_count(cx), 7, "four queued cards and three kept ideas");
            assert_eq!(page.ideas(cx).iter().map(|idea| idea.id.as_str()).collect::<Vec<_>>(), ["idea:3", "idea:2", "idea:1"], "newest first");
        });
        page.update(cx, |page, cx| page.bring_idea("idea:2", cx));
        page.read_with(cx, |page, cx| assert_eq!(page.front_idea(cx).map(|idea| idea.id), Some("idea:2".to_string())));
        // An idea has no Snooze: the card behind it stays where it is.
        page.update(cx, |page, cx| page.later(None, cx));
        assert_eq!(front(&page, cx), "turn:31");
        // Esc puts the Operator's own card back.
        page.update_in(cx, |page, window, cx| page.on_act(&Act { id: "close", slot: 0, digit: None }, window, cx));
        page.read_with(cx, |page, cx| assert!(page.front_idea(cx).is_none()));
        // A press on a queued card brings it in place of the idea.
        page.update(cx, |page, cx| page.bring_idea("idea:1", cx));
        page.update(cx, |page, cx| page.bring("turn:34", cx));
        page.read_with(cx, |page, cx| assert!(page.front_idea(cx).is_none()));
        assert_eq!(front(&page, cx), "turn:34");
    }

    #[gpui::test]
    fn an_ideas_first_option_starts_it_and_its_second_removes_it(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let parked = page.read_with(cx, |page, _| page.parked.clone());
        let asked = std::rc::Rc::new(RefCell::new(Vec::new()));
        let seen = asked.clone();
        let _held = cx.update(|_, cx| cx.subscribe(&parked, move |_, event: &ParkedAsked, _| seen.borrow_mut().push(event.clone())));
        page.update(cx, |page, cx| page.bring_idea("idea:2", cx));
        page.update(cx, |page, cx| page.digit(1, cx));
        assert!(matches!(asked.borrow().as_slice(), [ParkedAsked::Start(idea)] if idea.id == "idea:2"));
        page.update(cx, |page, cx| page.digit(2, cx));
        cx.run_until_parked();
        assert!(matches!(asked.borrow().as_slice(), [ParkedAsked::Start(_), ParkedAsked::Remove(id)] if id == "idea:2"));
        page.read_with(cx, |page, cx| {
            assert!(page.ideas(cx).iter().all(|idea| idea.id != "idea:2"), "Fleet let it go");
            assert!(page.front_idea(cx).is_none(), "so the Operator's card is back in front");
            assert!(page.idea_front.is_none(), "and its keys, Snooze and Discuss are live again, not held off by the idea that went");
        });
        assert_eq!(front(&page, cx), "turn:31");
    }

    #[gpui::test]
    fn next_up_opens_and_folds_and_takes_the_keys_in(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.read_with(cx, |page, _| assert!(!page.next_open, "folded until he opens it"));
        page.update_in(cx, |page, window, cx| page.toggle_next(window, cx));
        cx.run_until_parked();
        page.read_with(cx, |page, _| assert!(page.next_open));
        let inside = page.update_in(cx, |page, window, cx| page.next_group.contains_focused(window, cx));
        assert!(inside, "the keys are in it");
        page.update_in(cx, |page, window, cx| page.close_next(window, cx));
        page.read_with(cx, |page, _| assert!(!page.next_open));
    }

    /// Whether an element of that role and name was drawn in the window.
    fn drawn(cx: &mut VisualTestContext, role: gpui::Role, name: &str) -> bool {
        cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
    }

    fn working_ids(page: &Entity<OperatorPageView>, cx: &mut VisualTestContext) -> Vec<String> {
        page.read_with(cx, |page, _| page.working().iter().map(|turn| turn.id.clone()).collect())
    }

    #[gpui::test]
    fn a_press_moves_the_card_to_working_at_once_with_his_option_and_no_steps(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        assert_eq!(working_ids(&page, cx), ["turn:30"], "the Operator is working on one card already");
        page.update(cx, |page, cx| {
            page.settle_option("turn:31", "Let both run", cx);
            // Before Fleet has answered: the card is in the feed, out of the queue, and the next is in front.
            let held = page.working().first().expect("the card is working");
            assert_eq!(held.id, "turn:31");
            let card = held.card.as_ref().expect("a card");
            assert_eq!(card.picked.as_ref().map(|picked| picked.option.as_str()), Some("Let both run"));
            assert!(card.steps.is_empty() && card.returned.is_none());
            assert!(page.items().iter().all(|turn| turn.id != "turn:31"));
            assert_eq!(page.open_item().map(|turn| turn.id.as_str()), Some("turn:32"));
        });
        cx.run_until_parked();
        assert_eq!(working_ids(&page, cx), ["turn:31", "turn:30"], "Fleet's page keeps it, newest pick first");
        assert_eq!(front(&page, cx), "turn:32");
    }

    #[gpui::test]
    fn a_refused_press_takes_the_card_out_of_working_and_puts_it_back_in_front(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| {
            page.mock_fleet().expect("the walk's Fleet").fail_answer = Some(Failed::Refused("The Operator could not settle the card.".into()));
            page.digit(1, cx);
            assert_eq!(page.working().first().map(|turn| turn.id.as_str()), Some("turn:31"), "working until Fleet says no");
        });
        cx.run_until_parked();
        assert_eq!(working_ids(&page, cx), ["turn:30"]);
        assert_eq!(front(&page, cx), "turn:31");
        page.read_with(cx, |page, _| assert_eq!(page.refused.as_ref().map(|(id, why)| (id.as_str(), why.as_ref())), Some(("turn:31", "The Operator could not settle the card."))));
    }

    #[gpui::test]
    fn a_card_that_came_back_is_in_front_with_a_did_not_settle_chip_and_its_why_first_and_a_press_picks_it_anew(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.scene("returns", cx).expect("the Operator reports it failed"));
        cx.run_until_parked();
        assert_eq!(front(&page, cx), "turn:30", "a card that came back takes the front");
        assert!(drawn(cx, gpui::Role::Article, "Retry the flaky login test or skip it"));
        assert!(drawn(cx, gpui::Role::Status, "You picked Retry it once. The retry Job stopped before it finished"), "its why leads the read");
        assert!(working_ids(&page, cx).is_empty());
        page.read_with(cx, |page, _| {
            let turn = page.open_item().expect("a card in front");
            assert_eq!(face::kind_of(turn).word, "Did not settle");
            assert_eq!(face::kind_of(turn).hue, crate::look::token::STATUS_COMPLETED_FAILED, "in the danger hue");
            assert_eq!(face::returned_lead(turn).as_deref(), Some("You picked Retry it once. The retry Job stopped before it finished"));
            assert!(page.waiting().iter().any(|turn| turn.id == "turn:31"), "the card it replaced waits");
        });
        // Its options are live: a press picks it anew, with the steps and the reason cleared.
        page.update(cx, |page, cx| {
            page.digit(2, cx);
            let card = page.working().first().and_then(|turn| turn.card.as_ref()).expect("working again");
            assert_eq!(card.picked.as_ref().map(|picked| picked.option.as_str()), Some("Skip it for now"));
            assert!(card.steps.is_empty() && card.returned.is_none());
        });
        cx.run_until_parked();
        page.update(cx, |page, _| assert_eq!(page.mock_fleet().expect("the walk's Fleet").pressed[0].id, "turn:30"));
    }

    #[gpui::test]
    fn a_press_on_a_working_row_brings_the_card_to_the_front_read_only_and_esc_puts_the_front_card_back(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        assert!(drawn(cx, gpui::Role::Button, "Retry it once, Retry the flaky login test or skip it"), "the working row is in the feed");
        assert!(!drawn(cx, gpui::Role::Article, "Retry the flaky login test or skip it"));
        page.update(cx, |page, cx| page.bring_working("turn:30", cx));
        cx.run_until_parked();
        assert!(drawn(cx, gpui::Role::Article, "Retry the flaky login test or skip it"), "its card is drawn in front");
        assert!(drawn(cx, gpui::Role::ListItem, "Retry it once, About 5 min of a slot, picked"), "his pick is marked");
        assert!(drawn(cx, gpui::Role::ListItem, "The retry Job is running"), "its steps are drawn");
        page.read_with(cx, |page, _| {
            let turn = page.front_working().expect("the working card is in front");
            assert_eq!((turn.id.as_str(), turn.card.as_ref().map(|card| card.steps.len())), ("turn:30", Some(2)));
        });
        // Picked already: no press settles it.
        page.update(cx, |page, cx| page.digit(1, cx));
        page.update_in(cx, |page, window, cx| page.answer(OperatorAnswer::Yes, window, cx));
        cx.run_until_parked();
        page.update(cx, |page, _| assert!(page.mock_fleet().expect("the walk's Fleet").pressed.is_empty()));
        assert_eq!(working_ids(&page, cx), ["turn:30"]);
        page.update_in(cx, |page, window, cx| page.on_act(&Act { id: "close", slot: 0, digit: None }, window, cx));
        page.read_with(cx, |page, _| assert!(page.front_working().is_none()));
        assert_eq!(front(&page, cx), "turn:31");
    }

    // -- the status card ---------------------------------------------------------------------------

    fn record(wire: &str) -> ipc::SessionRecord {
        let mut all = ipc::decode::<std::collections::BTreeMap<String, ipc::SessionRecord>>("the Operator's sessions", include_bytes!("../../fixtures/operator-session.json")).expect("the fixture decodes");
        all.remove(wire).expect("a record in that state")
    }

    #[test]
    fn the_state_is_read_from_the_shape_of_the_operators_session_record() {
        use super::super::status::{session_of, status_of, Status};
        let now = scenes::now();
        // A turn running is Working, and the line says what Fleet saw it doing with how long the turn has run.
        let working = record("working");
        assert_eq!(status_of(Some(&working)), Status::Working { since: "2026-11-11T12:58:00-05:00".into(), doing: Some("Reading the live Jobs".into()) });
        assert_eq!(status_of(Some(&working)).line(now), "Reading the live Jobs, for 7m");
        assert_eq!(status_of(Some(&working)).chip().0, "Working");
        // With no activity it says it is in a turn.
        let mut quiet = working.clone();
        quiet.activity = None;
        assert_eq!(status_of(Some(&quiet)).line(now), "In a turn, for 7m");
        // Between turns is Watching, from when Fleet last heard, else the last turn, else the last time it saw the session.
        let watching = record("watching");
        assert_eq!(status_of(Some(&watching)).line(now), "Between turns. Last heard from 14m ago");
        let mut heard = watching.clone();
        heard.last_heard_at = Some(ipc::Instant::carried("2026-11-11T13:00:00-05:00"));
        assert_eq!(status_of(Some(&heard)).line(now), "Between turns. Last heard from 5m ago", "a word heard counts first");
        let mut seen = watching.clone();
        seen.last_turn_at = None;
        assert_eq!(status_of(Some(&seen)).line(now), "Between turns. Last heard from 13m ago", "the last time Fleet saw it is the last resort");
        assert_eq!(status_of(Some(&watching)).chip().0, "Watching");
        // Stopped by its standing, by a stop Fleet recorded, or by its session having ended.
        let stopped = record("stopped");
        assert_eq!(status_of(Some(&stopped)).title(now), "The Operator stopped 25m ago");
        assert_eq!(status_of(Some(&stopped)).line(now), "Its process ended");
        assert_eq!(status_of(Some(&stopped)).chip().0, "Stopped");
        let mut by_standing = watching.clone();
        by_standing.standing = Some(ipc::SessionStanding::Stopped);
        assert!(matches!(status_of(Some(&by_standing)), Status::Stopped { .. }));
        let mut by_record = watching.clone();
        by_record.stopped = stopped.stopped.clone();
        assert!(matches!(status_of(Some(&by_record)), Status::Stopped { .. }));
        let mut ended = watching.clone();
        ended.state = ipc::SessionState::Ended;
        assert_eq!(status_of(Some(&ended)).line(now), "Its session ended");
        // Stopped wins over a turn that is still set.
        let mut both = working.clone();
        both.stopped = stopped.stopped.clone();
        assert!(matches!(status_of(Some(&both)), Status::Stopped { .. }));
        // No session yet is Starting, with a neutral chip and no button.
        assert_eq!(status_of(None), Status::Starting);
        assert_eq!(status_of(None).line(now), "Fleet starts its session on the next tick");
        // Among Fleet's Jobs the Operator's live session is the one read, over an ended one that was seen later.
        let mut old = watching.clone();
        old.id = ipc::SessionId::carried("old-op");
        old.state = ipc::SessionState::Ended;
        old.last_seen_at = ipc::Instant::carried("2026-11-11T13:00:00-05:00");
        let mut other = watching.clone();
        other.id = ipc::SessionId::carried("a-job");
        other.operator = false;
        let held = [other, old, watching];
        assert_eq!(session_of(&held).map(|job| job.id.as_str()), Some("mock-op"));
        assert_eq!(session_of(&held[..2]).map(|job| job.id.as_str()), Some("old-op"), "an ended session is the one left to read");
        assert_eq!(session_of(&held[..1]), None);
    }

    #[gpui::test]
    fn with_nothing_in_front_a_status_card_stands_in_its_place_in_each_state(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        assert!(!drawn(cx, gpui::Role::Region, "Nothing needs you"), "a card in front leaves no status card");
        for (scene, line, title) in [
            ("quiet", "Reading the live Jobs, for 7m", "Nothing needs you"),
            ("watching", "Between turns. Last heard from 14m ago", "Nothing needs you"),
            ("stopped", "Its process ended", "The Operator stopped 25m ago"),
        ] {
            page.update(cx, |page, cx| page.scene(scene, cx).expect("a scene"));
            cx.run_until_parked();
            assert!(page.read_with(cx, |page, _| page.open_item().is_none()), "{scene}");
            assert!(drawn(cx, gpui::Role::Region, title), "{scene}: {title}");
            assert!(drawn(cx, gpui::Role::Status, line), "{scene}: {line}");
        }
        assert!(drawn(cx, gpui::Role::Button, "Restart the Operator"), "the stopped card offers Restart");
        // Each card has the one press its state offers: Restart on the stopped one, the button that opens the session on the rest.
        page.update(cx, |page, cx| page.scene("watching", cx).expect("a scene"));
        cx.run_until_parked();
        assert!(drawn(cx, gpui::Role::Button, "See what it is doing"));
        // (The test window keeps every frame's labels, so what is no longer drawn is read from the page's own state.)
        page.read_with(cx, |page, _| assert!(matches!(page.status(), super::super::status::Status::Watching { .. }) && page.session_id().is_some()));
    }

    #[gpui::test]
    fn the_last_act_is_the_newest_feed_entry_and_the_line_is_left_out_when_the_feed_is_empty(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.scene("quiet", cx).expect("a scene"));
        cx.run_until_parked();
        assert!(drawn(cx, gpui::Role::Status, "Last act 1m ago: Fix desktop_unit after #412"), "the feed's first entry, with its age");
        // A newer entry replaces it, and a merged pull request says its number.
        page.update(cx, |page, cx| {
            let mut read = page.page.clone().expect("the page is read");
            read.feed.insert(0, ipc::operator_page::OperatorFeedEntry { at: ipc::Instant::carried("2026-11-11T13:05:00-05:00"), kind: ipc::operator_page::OperatorFeedKind::Merged, title: "Slot meter".into(), session_id: None, number: Some("3414".into()), said: None });
            page.set_page(read, cx);
        });
        cx.run_until_parked();
        assert!(drawn(cx, gpui::Role::Status, "Last act <1m ago: #3414 Slot meter"));
        page.update(cx, |page, cx| {
            let mut read = page.page.clone().expect("the page is read");
            read.feed.clear();
            page.set_page(read, cx);
        });
        cx.run_until_parked();
        // With no entry there is nothing to say: the card stands without the line.
        page.read_with(cx, |page, _| assert!(page.page.as_ref().and_then(|page| page.feed.first()).is_none()));
        assert!(drawn(cx, gpui::Role::Region, "Nothing needs you"));
    }

    #[gpui::test]
    fn a_press_on_see_what_it_is_doing_asks_to_open_the_operators_session_by_its_id(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let asked = std::rc::Rc::new(RefCell::new(Vec::new()));
        let heard = asked.clone();
        let _hearing = cx.update(|_, cx| cx.subscribe(&page, move |_, event: &Opened, _| heard.borrow_mut().push(event.clone())));
        page.update(cx, |page, cx| page.scene("quiet", cx).expect("a scene"));
        cx.run_until_parked();
        let button = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), gpui::Role::Button, "See what it is doing").map(|drawn| drawn.bounds)).expect("the button is drawn");
        cx.simulate_click(button.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        assert_eq!(*asked.borrow(), [Opened::Session(SessionId::carried("mock-op"))]);
        // Starting has no session to open and no button.
        page.update(cx, |page, cx| {
            page.session = None;
            cx.notify();
        });
        cx.run_until_parked();
        // The button is drawn only where a session can be opened.
        assert!(page.read_with(cx, |page, _| page.session_id().is_none()));
        page.update(cx, |page, cx| page.open_session(cx));
        assert_eq!(asked.borrow().len(), 1, "with no session nothing is asked");
    }

    #[gpui::test]
    fn a_plans_review_and_its_number_ask_for_the_review_panel_and_a_build_to_try_takes_no_key(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let asked = std::rc::Rc::new(RefCell::new(Vec::new()));
        let heard = asked.clone();
        let _hearing = cx.update(|_, cx| cx.subscribe(&page, move |_, event: &Opened, _| heard.borrow_mut().push(event.clone())));
        page.update(cx, |page, cx| page.scene("plan-comes", cx).expect("a scene"));
        cx.run_until_parked();
        let review = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), gpui::Role::Button, "Review").map(|drawn| drawn.bounds)).expect("Review is drawn");
        cx.simulate_click(review.center(), gpui::Modifiers::none());
        page.update(cx, |page, cx| page.digit(1, cx));
        cx.run_until_parked();
        let read = Some(("It keeps the chip's colour in one place and tests it on a stale branch.".into(), Some("Approve".into())));
        let plan = Opened::Review(ipc::ItemRef { session_id: SessionId::carried("mock-01"), item: "plan:2".into() }, read);
        assert_eq!(*asked.borrow(), [plan.clone(), plan], "the press and 1 each ask for the panel");
        page.read_with(cx, |page, _| assert!(page.pending.is_empty() && page.open_item().is_some_and(|turn| turn.id == "turn:50"), "nothing is answered: the plan stays in front"));
        page.update(cx, |page, cx| page.scene("build-comes", cx).expect("a scene"));
        cx.update(|window, cx| page.update(cx, |page, cx| page.answer(OperatorAnswer::Yes, window, cx)));
        page.update(cx, |page, cx| page.digit(1, cx));
        cx.run_until_parked();
        assert_eq!(asked.borrow().len(), 2, "a build to try asks for nothing");
        page.read_with(cx, |page, _| {
            assert!(page.pending.is_empty(), "nor is it answered: its answers are its own card's");
            assert_eq!(page.ready_in_front().map(|job| job.as_str().to_string()).as_deref(), Some("mock-02"));
        });
    }

    #[gpui::test]
    fn restart_switches_the_operator_off_and_on_and_the_card_reads_starting(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.scene("quiet", cx).expect("a scene"));
        page.update(cx, |page, cx| page.scene("stopped", cx).expect("a scene"));
        cx.run_until_parked();
        page.update(cx, |page, cx| page.restart(cx));
        assert!(page.read_with(cx, |page, _| page.restarting), "it reads Starting while the switch goes round");
        cx.run_until_parked();
        page.read_with(cx, |page, _| {
            assert!(!page.restarting && page.session.is_none());
            assert_eq!(page.status(), super::super::status::Status::Starting);
            assert!(page.page.as_ref().is_some_and(|page| page.on), "the Operator is on again");
            assert!(page.items().is_empty(), "the page is as quiet as it was");
        });
        assert!(drawn(cx, gpui::Role::Status, "Fleet starts its session on the next tick"));
        // A refused switch is said on the card, which keeps its Restart.
        page.update(cx, |page, cx| page.scene("stopped", cx).expect("a scene"));
        page.update(cx, |page, cx| {
            page.mock_fleet().expect("the walk's Fleet").refuse_switch = Some("Settings are locked".into());
            page.restart(cx);
        });
        cx.run_until_parked();
        assert!(drawn(cx, gpui::Role::Alert, "Not restarted: Settings are locked"));
        assert!(drawn(cx, gpui::Role::Button, "Restart the Operator"));
    }

    /// A press on a labelled element, found where it was drawn.
    fn press_on(cx: &mut VisualTestContext, role: gpui::Role, name: &str) {
        let at = cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|drawn| drawn.bounds)).unwrap_or_else(|| panic!("{name} is drawn"));
        cx.simulate_click(at.center(), gpui::Modifiers::none());
        cx.run_until_parked();
    }

    fn act(page: &Entity<OperatorPageView>, cx: &mut VisualTestContext, id: &'static str) {
        page.update_in(cx, |page, window, cx| page.on_act(&crate::keymap::Act { id, slot: 0, digit: None }, window, cx));
        cx.run_until_parked();
    }

    /// The Operator says more, until the talk overflows its room and the frame that measured it is drawn.
    fn talk_overflows(page: &Entity<OperatorPageView>, cx: &mut VisualTestContext) {
        page.update(cx, |page, cx| {
            for _ in 0..12 {
                page.scene("replies", cx).expect("the Operator replies");
            }
        });
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }

    #[gpui::test]
    fn switched_off_its_card_stands_in_the_front_cards_place_and_switch_on_turns_it_on(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.scene("off", cx).expect("a scene"));
        cx.run_until_parked();
        page.read_with(cx, |page, _| assert_eq!(page.status(), super::super::status::Status::Off));
        assert!(drawn(cx, gpui::Role::Region, "The Operator is off"));
        assert!(drawn(cx, gpui::Role::Status, "Every Job asks you directly"));
        press_on(cx, gpui::Role::Button, "Switch the Operator on");
        page.read_with(cx, |page, _| {
            assert!(page.page.as_ref().is_some_and(|page| page.on), "the press switches it on");
            assert_ne!(page.status(), super::super::status::Status::Off);
        });
    }

    #[gpui::test]
    fn a_stopped_operator_comes_in_front_of_the_queued_card_and_the_cards_keys_wait(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.scene("stopped", cx).expect("a scene"));
        cx.run_until_parked();
        page.read_with(cx, |page, _| {
            assert_eq!(page.open_item().map(|turn| turn.id.as_str()), Some("turn:31"), "the card still waits");
            assert!(page.stopped_in_front());
        });
        assert!(drawn(cx, gpui::Role::Region, "The Operator stopped 25m ago"));
        assert!(drawn(cx, gpui::Role::Button, "Restart the Operator"));
        act(&page, cx, "call_yes");
        page.update(cx, |page, _| assert!(page.mock_fleet().expect("the walk's Fleet").pressed.is_empty(), "Y settles nothing while it is stopped"));
        // Restarted, the card is in front again and its keys act.
        page.update(cx, |page, cx| page.restart(cx));
        cx.run_until_parked();
        page.read_with(cx, |page, _| assert!(!page.stopped_in_front()));
    }

    #[gpui::test]
    fn the_card_folds_once_the_talk_overflows_and_a_press_opens_it_until_the_talk_grows(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.read_with(cx, |page, _| assert!(!page.is_folded(), "with no talk the card is open"));
        talk_overflows(&page, cx);
        page.read_with(cx, |page, _| assert!(page.is_folded(), "a talk longer than its room folds the card"));
        assert!(drawn(cx, gpui::Role::Button, "Unfold Two Jobs are changing paging.rs"));
        press_on(cx, gpui::Role::Button, "Unfold Two Jobs are changing paging.rs");
        page.read_with(cx, |page, _| assert!(!page.is_folded(), "the press opens it"));
        page.update(cx, |page, cx| page.scene("replies", cx).expect("the Operator replies"));
        page.read_with(cx, |page, _| assert!(page.is_folded(), "the talk grew, and the card folds again"));
    }

    #[gpui::test]
    fn a_card_that_comes_while_the_card_is_folded_waits_in_the_strip_and_show_brings_it_unfolded(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.scene("quiet", cx).expect("a scene"));
        talk_overflows(&page, cx);
        page.update(cx, |page, cx| page.scene("front-arrives", cx).expect("a scene"));
        cx.run_until_parked();
        page.read_with(cx, |page, _| {
            assert_eq!(page.fresh, ["turn:41"], "the card waits in the strip");
            assert!(page.open_item().is_none(), "and does not take the front");
        });
        assert!(drawn(cx, gpui::Role::Region, "New card: Checkout's coupon fix has a walk for you"));
        assert!(drawn(cx, gpui::Role::Status, "A new card"));
        // ⌘Enter with nothing in the box is Show.
        act(&page, cx, "send_message");
        page.read_with(cx, |page, _| {
            assert_eq!(page.open_item().map(|turn| turn.id.as_str()), Some("turn:41"));
            assert!(page.fresh.is_empty() && !page.is_folded(), "it comes in front, open");
        });
    }

    #[gpui::test]
    fn a_card_that_comes_while_the_card_is_open_and_the_talk_idle_takes_the_front(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        page.update(cx, |page, cx| page.scene("quiet", cx).expect("a scene"));
        cx.run_until_parked();
        page.update(cx, |page, cx| page.scene("front-arrives", cx).expect("a scene"));
        cx.run_until_parked();
        page.read_with(cx, |page, _| {
            assert!(page.fresh.is_empty(), "no strip");
            assert_eq!(page.open_item().map(|turn| turn.id.as_str()), Some("turn:41"));
        });
    }
    // -- the page in a pane that is part of a screen ---------------------------------------------

    /// A host screen's pane: the page in a box `pane` wide, in a window much wider than it.
    struct Pane {
        page: Entity<OperatorPageView>,
        /// Its width, or the window's own where None.
        pane: Option<f32>,
    }

    impl Render for Pane {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let pane = gpui::div().h_full().child(self.page.clone());
            let pane = match self.pane {
                Some(wide) => pane.w(gpui::px(wide)),
                None => pane.w_full(),
            };
            // The window's own root, as the app draws it: it clears the label book before each frame.
            crate::labels::label_root(gpui::div().size_full().flex().child(pane))
        }
    }

    fn pane_of(pane: Option<f32>, embedded: bool, cx: &mut TestAppContext) -> (Entity<OperatorPageView>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (host, cx) = cx.add_window_view(|window, cx| {
            let page = cx.new(|cx| {
                let page = OperatorPageView::mock(window, cx);
                if embedded { page.embedded() } else { page }
            });
            Pane { page, pane }
        });
        let page = host.read_with(cx, |host, _| host.page.clone());
        cx.simulate_resize(gpui::size(gpui::px(1800.), gpui::px(900.)));
        settle(cx);
        (page, cx)
    }

    /// Draws until the page has measured its pane and drawn again for it.
    fn settle(cx: &mut VisualTestContext) {
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
    }

    fn bounds_named(cx: &mut VisualTestContext, role: gpui::Role, name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
        cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|drawn| drawn.bounds))
    }

    fn side_drawn(cx: &mut VisualTestContext) -> bool {
        cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().any(|one| one.role == gpui::Role::Heading && one.name.starts_with("Since you last looked")))
    }

    #[gpui::test]
    fn a_pane_too_narrow_for_two_columns_folds_the_side_column(cx: &mut TestAppContext) {
        let (_, cx) = pane_of(None, false, cx);
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(900.)));
        settle(cx);
        assert!(side_drawn(cx), "a roomy window keeps Next up and the feed beside the card");
        cx.simulate_resize(gpui::size(gpui::px(640.), gpui::px(900.)));
        settle(cx);
        assert!(!side_drawn(cx), "a 640 window folds the side column away");
        assert!(bounds_named(cx, gpui::Role::Heading, "Operator").is_some(), "the head stays");
    }

    #[gpui::test]
    fn widths_follow_the_panes_own_bounds_not_the_windows(cx: &mut TestAppContext) {
        // The window is 1800 wide; the pane is 640. The card's room, the side column and the box all go by the 640.
        let (page, cx) = pane_of(Some(640.), true, cx);
        let room = cx.update(|window, cx| page.read(cx).ready_room(window, &cx.look().clone()));
        assert!(room <= 640., "the Ready to try card has {room} points to draw in, in a 640 pane");
        let sent = bounds_named(cx, gpui::Role::Group, "Message box").expect("the box is drawn");
        assert!(sent.right() <= gpui::px(640.) && sent.left() >= gpui::px(0.), "the box {sent:?} stays inside the pane");
    }

    #[gpui::test]
    fn an_embedded_page_draws_the_header_row_and_one_column_centred_at_a_readable_width(cx: &mut TestAppContext) {
        let (page, cx) = pane_of(Some(1800.), true, cx);
        let (word, readable, edge) = cx.update(|_, cx| {
            let look = cx.look().clone();
            (page.read(cx).status().chip().0, f32::from(super::super::view::readable(&look)), f32::from(look.length(crate::look::token::SPACE_12) * 7.5 + look.length(crate::look::token::GAP_PANEL)))
        });
        for (role, name) in [(gpui::Role::Heading, "Operator"), (gpui::Role::Status, word)] {
            assert!(bounds_named(cx, role, name).is_some(), "the header row draws {name}");
        }
        let column = bounds_named(cx, gpui::Role::Group, "Message box").expect("the box is drawn");
        assert!(f32::from(column.size.width) <= readable, "the column is {:?} wide, over {readable}", column.size.width);
        // The main panel is the pane less the side column; the column stands in its middle.
        let middle = (1800. - edge) / 2.;
        assert!((f32::from(column.center().x) - middle).abs() < 2., "the column's middle {:?} is the main panel's {middle}", column.center().x);
    }

    #[gpui::test]
    fn the_standalone_page_keeps_drawing_its_column_as_wide_as_the_panel(cx: &mut TestAppContext) {
        let (_, cx) = pane_of(None, false, cx);
        let column = bounds_named(cx, gpui::Role::Group, "Message box").expect("the box is drawn");
        assert!(f32::from(column.size.width) > 1000., "unchanged: {:?} wide", column.size.width);
    }

    fn drawn_count(cx: &mut VisualTestContext, role: gpui::Role, name: &str) -> usize {
        cx.update(|window, cx| cx.default_global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().filter(|one| one.role == role && one.name == name).count())
    }

    #[gpui::test]
    fn an_embedded_status_card_leaves_out_the_chip_and_the_line_the_header_row_already_says(cx: &mut TestAppContext) {
        let (page, cx) = pane_of(Some(1800.), true, cx);
        page.update(cx, |page, cx| page.scene("quiet", cx).expect("a scene"));
        settle(cx);
        assert!(bounds_named(cx, gpui::Role::Region, "Nothing needs you").is_some(), "the card stands");
        assert_eq!(drawn_count(cx, gpui::Role::Status, "Working"), 1, "the state chip is the header's alone");
        assert_eq!(drawn_count(cx, gpui::Role::Status, "Reading the live Jobs, for 7m"), 1, "and so is the status line");
        assert_eq!(drawn_count(cx, gpui::Role::Status, "Last act 1m ago: Fix desktop_unit after #412"), 1, "the last act stays in the card");
    }

    #[gpui::test]
    fn the_standalone_status_card_keeps_its_chip_and_its_line(cx: &mut TestAppContext) {
        let (page, cx) = pane_of(None, false, cx);
        page.update(cx, |page, cx| page.scene("quiet", cx).expect("a scene"));
        settle(cx);
        assert_eq!(drawn_count(cx, gpui::Role::Status, "Working"), 1);
        assert_eq!(drawn_count(cx, gpui::Role::Status, "Reading the live Jobs, for 7m"), 1);
    }
}
