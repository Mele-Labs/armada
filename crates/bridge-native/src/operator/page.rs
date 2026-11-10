//! The page's state and acts: which item is open, what he answered this sitting, the chat on Ask. Every
//! press answers at once: the page moves first, says so with a pending mark, and takes Fleet's page when
//! it comes. A refusal puts the item back and says why on it.

use std::collections::{BTreeSet, HashMap};

use gpui::{App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, SharedString, Subscription, Task, Window};
use ipc::operator_page::{OperatorAnswer, OperatorAnswerPress, OperatorAnswered, OperatorPage, OperatorThen, OperatorTurnItem};
use ipc::{SessionId, WaitingAct};

use super::chat::{ChatAsked, OperatorChat};
use super::face::{self, Kind, Standing};
use super::scenes::{self, MockFleet};
use super::wire::Wire;
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
}

/// What the page asks of the window around it: a place it does not draw.
#[derive(Clone, Debug, PartialEq)]
pub enum Opened {
    /// The act of an item Fleet cannot do itself, such as a walk to open or a pull request, from the
    /// Job that holds it.
    Act(WaitingAct, Option<SessionId>),
    /// The week's Log, a markdown file on this machine.
    Log(String),
    /// A Job, from a row of the digest.
    Job(SessionId),
}

/// Items that joined Your turn since the page was last read, oldest first: what raises a toast. Jobs
/// screens own toasts (`banner.rs`, `notify.rs`) and subscribe to this on [`super::live`].
#[derive(Clone, Debug, PartialEq)]
pub struct Arrived(pub Vec<OperatorTurnItem>);

impl EventEmitter<Opened> for OperatorPageView {}
impl EventEmitter<Arrived> for OperatorPageView {}

pub struct OperatorPageView {
    pub(super) wire: Wire,
    /// Fleet's page, with this sitting's presses already on it. `None` until it is read: a skeleton.
    pub(super) page: Option<OperatorPage>,
    pub(super) standing: Standing,
    /// The date, and when he came back.
    pub(super) day: Option<SharedString>,
    pub(super) back_at: Option<SharedString>,
    /// The item open, by id: one at a time.
    pub(super) open: Option<String>,
    pub(super) done: Vec<Done>,
    /// The items whose press is in flight.
    pub(super) pending: BTreeSet<String>,
    /// A press Fleet refused, on that item, with why.
    pub(super) refused: Option<(String, SharedString)>,
    /// The Then he picked on a go-ahead to file, where he picked one; else the Operator's.
    pub(super) then: HashMap<String, OperatorThen>,
    pub(super) chat: Option<Entity<OperatorChat>>,
    /// The digest's tiles he has folded or opened, by name, against how they open.
    pub(super) folded: HashMap<&'static str, bool>,
    pub focus: FocusHandle,
    /// The pane from a Job: the chat and a short list.
    pub(super) pane: bool,
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
        OperatorPageView {
            wire,
            page: None,
            standing: Standing::default(),
            day: None,
            back_at: None,
            open: None,
            done: Vec::new(),
            pending: BTreeSet::new(),
            refused: None,
            then: HashMap::new(),
            chat: None,
            folded: HashMap::new(),
            focus: cx.focus_handle(),
            pane,
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
            let moved = held.operator_page_moved;
            let ready = held.port().is_some();
            if ready && (moved != this.read_at || (this.page.is_none() && !this.reading)) {
                this.read_at = moved;
                this.read(cx);
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
        this.standing = scenes::standing();
        let (day, back) = scenes::when();
        this.day = Some(day);
        this.back_at = Some(back);
        this.read(cx);
        this
    }

    pub fn mock_fleet(&mut self) -> Option<&mut MockFleet> {
        match &mut self.wire {
            Wire::Mock(mock) => Some(mock),
            Wire::Live(_) => None,
        }
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
    pub fn set_page(&mut self, page: OperatorPage, cx: &mut Context<Self>) {
        let ids: BTreeSet<String> = page.turn.iter().map(|turn| turn.id.clone()).collect();
        if let Some(seen) = &self.seen {
            let new: Vec<OperatorTurnItem> = page.turn.iter().filter(|turn| !seen.contains(&turn.id)).cloned().collect();
            if !new.is_empty() {
                cx.emit(Arrived(new));
            }
        }
        self.seen = Some(ids.clone());
        self.then.retain(|id, _| ids.contains(id));
        let keep = self.open.as_ref().filter(|open| ids.contains(*open)).cloned();
        self.open = keep.or_else(|| page.turn.first().map(|turn| turn.id.clone()));
        self.page = Some(page);
        cx.notify();
    }

    // -- the keys and presses ----------------------------------------------------------------------

    pub fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if self.chat_has_keys(window, cx) {
            return cx.propagate();
        }
        match act.id {
            // The registry's slots are j, k, ↓, ↑: even slots go down.
            "call_move_focus" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "call_yes" | "call_send" => self.answer(OperatorAnswer::Yes, window, cx),
            "call_dismiss" => self.answer(OperatorAnswer::No, window, cx),
            "call_later" => self.later(cx),
            "call_ask" => self.ask(window, cx),
            // ⌥↑ then ⌥↓.
            "call_reorder" => self.shove(if act.slot == 0 { -1 } else { 1 }, cx),
            "call_pick" => self.digit(act.digit.unwrap_or(0) as usize, window, cx),
            _ => cx.propagate(),
        }
    }

    fn chat_has_keys(&self, window: &Window, cx: &App) -> bool {
        self.chat.as_ref().is_some_and(|chat| chat.read(cx).focus_handle(cx).contains_focused(window, cx))
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
        cx.notify();
    }

    pub fn open_id(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.index_of(id).is_some() {
            self.open = Some(id.to_string());
            cx.notify();
        }
    }

    /// A digit: on a go-ahead to file, the Then in that place; elsewhere the answer in that place.
    pub fn digit(&mut self, digit: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(turn) = self.open_item().cloned() else { return };
        let Some(at) = digit.checked_sub(1) else { return };
        if !turn.then.is_empty() {
            if let Some(offer) = turn.then.get(at) {
                self.then.insert(turn.id.clone(), offer.then);
                cx.notify();
            }
            return;
        }
        if let Some(answer) = face::answers_of(&turn).get(at).copied() {
            self.answer(answer, window, cx);
        }
    }

    pub fn pick_then(&mut self, id: &str, then: OperatorThen, cx: &mut Context<Self>) {
        self.then.insert(id.to_string(), then);
        cx.notify();
    }

    /// One press on the open item. It leaves Your turn at once, struck through below with a pending mark,
    /// and the next item opens.
    pub fn answer(&mut self, answer: OperatorAnswer, window: &mut Window, cx: &mut Context<Self>) {
        let Some(turn) = self.open_item().cloned() else { return };
        if !face::answers_of(&turn).contains(&answer) {
            return;
        }
        let then = (!turn.then.is_empty()).then(|| self.then_of(&turn)).flatten();
        let press = OperatorAnswerPress { id: turn.id.clone(), answer, then: then.filter(|_| answer != OperatorAnswer::No) };
        let at = self.index_of(&turn.id).unwrap_or(0);
        self.take_out(&turn.id);
        self.done.push(Done { id: turn.id.clone(), kind: face::kind_of(&turn), title: face::title_of(&turn), said: None });
        self.pending.insert(turn.id.clone());
        self.refused = None;
        self.open_near(at);
        cx.notify();
        let task = self.wire.answer(press, cx);
        let task = cx.spawn_in(window, async move |this, cx| {
            let answered = task.await;
            let _ = this.update(cx, |this, cx| this.answered(&turn, answer, answered, cx));
        });
        self.track(task);
    }

    fn answered(&mut self, turn: &OperatorTurnItem, answer: OperatorAnswer, answered: Result<OperatorAnswered, String>, cx: &mut Context<Self>) {
        self.pending.remove(&turn.id);
        match answered {
            Ok(answered) => {
                if let Some(done) = self.done.iter_mut().find(|done| done.id == turn.id) {
                    done.said = Some(said_of(turn, answer, &answered));
                }
                if let Some(act) = answered.open.clone() {
                    cx.emit(Opened::Act(act, turn.session_id.clone()));
                }
                self.set_page(answered.page, cx);
            }
            Err(why) => {
                // Back where it was, open, with why.
                self.done.retain(|done| done.id != turn.id);
                if let Some(page) = self.page.as_mut() {
                    if !page.turn.iter().any(|one| one.id == turn.id) {
                        page.turn.insert(0, turn.clone());
                    }
                }
                self.open = Some(turn.id.clone());
                self.refused = Some((turn.id.clone(), why.into()));
                cx.notify();
            }
        }
    }

    fn take_out(&mut self, id: &str) {
        if let Some(page) = self.page.as_mut() {
            page.turn.retain(|turn| turn.id != id);
        }
    }

    /// Open the item now at `at`, or the last one.
    fn open_near(&mut self, at: usize) {
        let items = self.items();
        self.open = items.get(at).or(items.last()).map(|turn| turn.id.clone());
    }

    /// Later: the open item goes last for the rest of the day, and the next one opens.
    pub fn later(&mut self, cx: &mut Context<Self>) {
        let Some(turn) = self.open_item().cloned() else { return };
        let at = self.index_of(&turn.id).unwrap_or(0);
        if let Some(page) = self.page.as_mut() {
            page.turn.retain(|one| one.id != turn.id);
            let mut moved = turn.clone();
            moved.later_until.get_or_insert_with(|| ipc::Instant::carried(""));
            page.turn.push(moved);
        }
        if self.items().len() > 1 {
            self.open_near(at);
        }
        cx.notify();
        let task = self.wire.later(&turn.id, cx);
        self.settle(task, cx);
    }

    /// ⌥↑ and ⌥↓: the open item one place up or down. The keys stay on it.
    pub fn shove(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(id) = self.open.clone() else { return };
        let Some(at) = self.index_of(&id) else { return };
        let to = (at as isize + by).clamp(0, self.items().len() as isize - 1) as usize;
        self.move_to(&id, to, cx);
    }

    /// A drag, or a shove: the item goes to `to`, and Fleet keeps the order.
    pub fn move_to(&mut self, id: &str, to: usize, cx: &mut Context<Self>) {
        let Some(at) = self.index_of(id) else { return };
        if at == to {
            return;
        }
        let Some(page) = self.page.as_mut() else { return };
        let turn = page.turn.remove(at);
        page.turn.insert(to.min(page.turn.len()), turn);
        let ids: Vec<String> = page.turn.iter().map(|turn| turn.id.clone()).collect();
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

    /// Ask: the chat about the open item, in place of the digest, with the keys in its box.
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
                    this.back_at = Some(now.format("%H:%M").to_string().into());
                    cx.notify();
                }
                Ok(_) => {}
                Err(why) => Note::info("Fleet was not told the window came to the front").field("why", why).emit(),
            });
        });
        self.track(task);
    }

    // -- the walk's scenes -------------------------------------------------------------------------

    /// Move the mock Fleet to a named scene: the Operator answers the chat (`said`), or a new item joins
    /// the queue (`arrives`).
    pub fn scene(&mut self, name: &str, cx: &mut Context<Self>) -> Result<(), String> {
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
            }
            other => return Err(format!("no Operator scene named {other}")),
        }
        Ok(())
    }
}

/// What came of a press, in a few words, from Fleet's answer.
fn said_of(turn: &OperatorTurnItem, answer: OperatorAnswer, answered: &OperatorAnswered) -> SharedString {
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
        (None, _, None) => face::pick_of(turn).map_or("Yes".to_string(), |pick| pick.to_string()),
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
        assert_eq!(pressed(&page, cx), vec![OperatorAnswerPress { id: "turn:1".into(), answer: OperatorAnswer::Yes, then: None }]);
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:2"), "the next item opens");
        page.read_with(cx, |page, _| {
            assert_eq!(page.done.iter().map(|done| done.id.as_str()).collect::<Vec<_>>(), ["turn:1"]);
            assert_eq!(page.done[0].said.as_deref(), Some("Approve"), "what came of it, once Fleet answered");
            assert!(page.pending.is_empty());
        });
        assert!(!ids(&page, cx).contains(&"turn:1".to_string()));
    }

    #[gpui::test]
    fn a_digit_picks_a_then_and_yes_carries_it(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("down");
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:2"));
        page.read_with(cx, |page, _| assert_eq!(page.open_item().and_then(|turn| page.then_of(turn)), Some(OperatorThen::FileAndHand), "the Operator's pick stands first"));
        cx.simulate_keystrokes("2 a");
        assert_eq!(pressed(&page, cx), vec![OperatorAnswerPress { id: "turn:2".into(), answer: OperatorAnswer::Yes, then: Some(OperatorThen::FileAndStart) }]);
    }

    #[gpui::test]
    fn always_is_offered_only_where_fleet_keeps_a_rule(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.update(|window, cx| page.update(cx, |page, cx| page.answer(OperatorAnswer::Always, window, cx)));
        assert!(pressed(&page, cx).is_empty(), "a decision offers no Always");
        cx.simulate_keystrokes("j");
        cx.update(|window, cx| page.update(cx, |page, cx| page.answer(OperatorAnswer::AlwaysHere, window, cx)));
        assert_eq!(pressed(&page, cx).iter().map(|press| (press.id.as_str(), press.answer)).collect::<Vec<_>>(), [("turn:2", OperatorAnswer::AlwaysHere)]);
    }

    #[gpui::test]
    fn later_puts_the_item_last_for_today_and_opens_the_next(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("l");
        assert_eq!(ids(&page, cx), ["turn:2", "turn:3", "turn:4", "turn:1"]);
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:2"));
        page.read_with(cx, |page, _| assert!(page.items().last().is_some_and(|turn| turn.later_until.is_some())));
    }

    #[gpui::test]
    fn a_shove_moves_the_open_item_and_fleet_keeps_the_order(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        cx.simulate_keystrokes("alt-down");
        assert_eq!(ids(&page, cx), ["turn:2", "turn:1", "turn:3", "turn:4"]);
        assert_eq!(open_id(&page, cx).as_deref(), Some("turn:1"), "the keys stay on the item it moved");
        let kept: Vec<String> = page.update(cx, |page, _| page.mock_fleet().map(|mock| mock.page.turn.iter().map(|turn| turn.id.clone()).collect()).unwrap_or_default());
        assert_eq!(kept, ["turn:2", "turn:1", "turn:3", "turn:4"]);
        cx.update(|_, cx| page.update(cx, |page, cx| page.move_to("turn:4", 0, cx)));
        assert_eq!(ids(&page, cx), ["turn:4", "turn:2", "turn:1", "turn:3"], "a drop moves it as a drag would");
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

    #[gpui::test]
    fn coming_to_the_front_tells_fleet(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let before = page.update(cx, |page, _| page.mock_fleet().map_or(0, |mock| mock.fronted));
        cx.update(|_, cx| page.update(cx, |page, cx| page.in_front(cx)));
        cx.run_until_parked();
        assert_eq!(page.update(cx, |page, _| page.mock_fleet().map_or(0, |mock| mock.fronted)), before + 1);
    }

    #[gpui::test]
    fn the_pane_from_a_job_holds_its_chat_from_the_start(cx: &mut TestAppContext) {
        let (page, cx) = open_as(cx, true);
        let chat = page.read_with(cx, |page, _| page.chat.clone()).expect("the pane opens with its chat");
        assert_eq!(chat.read_with(cx, |chat, _| chat.about.clone()), None, "about nothing until a row is pressed");
        assert!(chat.read_with(cx, |chat, _| chat.read), "the conversation so far is read");
    }
}
