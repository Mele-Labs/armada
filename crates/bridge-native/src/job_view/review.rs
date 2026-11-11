//! The review sheet (`list-needs-palette.md`, section 3): what a Job has put to the person to read,
//! a plan, a sketch, a document and a draft, on one sheet. The items stand oldest first in a list,
//! the one chosen is drawn beside it, and the answers sit in the sheet's head in fixed places.
//!
//! **Nothing here is a second path to Fleet.** The bodies are the thread's own cards drawn without
//! their answers, and an answer given here settles the card in the thread as one given on the card
//! settles the sheet: both read `CardState::answers`. A press answers in the frame it lands, the next
//! open item comes up, and a refusal puts the item back with Fleet's reason.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use chrono::Local;
use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, Anchor, AnyElement, Bounds, Context, Entity, FocusHandle, Focusable as _,
    FontWeight, KeyDownEvent, MouseButton, Pixels, Role, SharedString, Subscription, Window,
};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use ipc::{
    AnswerDocument, AnswerSketch, AnswerWaiting, ApprovePlan, DismissWaiting, DocumentHow, PlanCardState, PlanMark, SendPlanChanges, SessionId, SessionRow,
    SketchHow, ThreadDocument, ThreadHead, ThreadItem, ThreadSketch, WaitingItem, WaitingSource,
};

use super::acts::{button, refusal, Answer, Tone, SHEET};
use super::cards::glass;
use super::rows::{glyph, tip, Size};
use super::{JobView, Page};
use crate::dropdown::{under, Entry};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};

/// What an item is. The sheet draws a plan, a sketch, a document and a draft and nothing else: a
/// question, a command and a pull request stay on the Needs you card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Plan,
    Sketch,
    Document,
    Draft,
}

impl Kind {
    /// The kind's glyph and the word its tooltip names it by (`REVIEW_MARKS`).
    pub(super) fn mark(self) -> (&'static str, &'static str) {
        match self {
            Kind::Plan => ("list-todo", "Plan"),
            Kind::Sketch => ("pencil-ruler", "Sketch"),
            Kind::Draft => ("message-square-text", "Draft"),
            Kind::Document => ("file-text", "Document"),
        }
    }
}

/// One thing waiting on the sheet.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Item {
    /// The waiting item's id: `plan:2`, `sketch:<title>`, `document:<page>`, `draft:<row>`.
    pub id: String,
    pub kind: Kind,
    /// What the row reads, and what the primary names after its verb.
    pub name: String,
    pub since: String,
    /// Whether Send back is offered. A proposed workflow has no note to send back with, and a plan
    /// with no group has nowhere to put one.
    pub send_back: bool,
    /// Whether the row has a menu to put it away.
    pub dismissable: bool,
}

/// Where an item stands on the sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Standing {
    Open,
    Answered,
    SentBack,
}

/// How an item stands, and the word its mark's tooltip says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Settled {
    pub standing: Standing,
    pub said: &'static str,
}

/// The note that sends an item back; what is typed is the Job's draft under the item.
pub(super) struct NoteBox {
    pub area: Entity<TextareaState>,
    _typed: Subscription,
}

/// What the sheet remembers of one Job while it is open.
#[derive(Default)]
pub(super) struct ReviewState {
    pub open: bool,
    pub at: Option<String>,
    /// The items as they were when the sheet opened and as they came since, so one answered stays
    /// in its place until the sheet shuts.
    pub seen: Vec<Item>,
    /// The ids Fleet still lists as waiting.
    pub live: HashSet<String>,
    /// How each item stood when the sheet last advanced.
    pub before: HashMap<String, Standing>,
    pub back_to: Option<FocusHandle>,
    pub note: Option<NoteBox>,
}

/// The sheet's focus: one handle for each thing a key can reach.
pub(super) struct Focuses {
    pub list: FocusHandle,
    pub send_back: FocusHandle,
    pub primary: FocusHandle,
    pub close: FocusHandle,
    pub note_send: FocusHandle,
    pub note_cancel: FocusHandle,
}

impl Focuses {
    pub(super) fn new(cx: &mut gpui::App) -> Self {
        Focuses {
            list: cx.focus_handle(),
            send_back: cx.focus_handle(),
            primary: cx.focus_handle(),
            close: cx.focus_handle(),
            note_send: cx.focus_handle(),
            note_cancel: cx.focus_handle(),
        }
    }
}

/// Where each item's row was drawn, for a menu opened from the keys.
pub(super) type Rows = Rc<RefCell<HashMap<String, Bounds<Pixels>>>>;

gpui::actions!(review, [ItemMenu]);

pub(super) fn bind(cx: &mut gpui::App) {
    cx.bind_keys([gpui::KeyBinding::new("shift-f10", ItemMenu, Some("review-sheet")), gpui::KeyBinding::new("menu", ItemMenu, Some("review-sheet"))]);
}

fn instant(at: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(at).map_or(0, |then| then.timestamp_millis())
}

/// The sketch card the ledger's row `id` stands for: the pinned one, else the newest in the thread.
fn sketch_card<'a>(head: &'a ThreadHead, thread: &'a [ThreadItem], id: &str) -> Option<&'a ThreadSketch> {
    head.sketch.as_ref().filter(|card| card.sketch == id).or_else(|| {
        thread.iter().rev().find_map(|item| match item {
            ThreadItem::Sketch(card) if card.sketch == id => Some(card),
            _ => None,
        })
    })
}

/// The newest revision card of a page.
fn document_card<'a>(thread: &'a [ThreadItem], page: &str) -> Option<&'a ThreadDocument> {
    thread.iter().rev().find_map(|item| match item {
        ThreadItem::Document(card) if card.page == page => Some(card),
        _ => None,
    })
}

fn draft_row<'a>(thread: &'a [ThreadItem], row: &str) -> Option<&'a SessionRow> {
    thread.iter().rev().find_map(|item| match item {
        ThreadItem::Row { row: found @ SessionRow::Draft { id, .. } } if id == row => Some(found),
        _ => None,
    })
}

/// Every item the Job has to read, oldest first (`waitingOf`). **A waiting row the sheet has no body
/// for is not built** and stays with the Needs you card.
pub(super) fn build(waiting: &[WaitingItem], head: &ThreadHead, thread: &[ThreadItem], has_groups: bool) -> Vec<Item> {
    let mut items: Vec<Item> = waiting
        .iter()
        .filter_map(|one| {
            let item = |kind, name: String, send_back, dismissable| Item { id: one.id.clone(), kind, name, since: one.since.as_str().to_string(), send_back, dismissable };
            match one.source {
                WaitingSource::Plan => head
                    .plan
                    .as_ref()
                    .filter(|plan| plan.state == PlanCardState::Waiting)
                    .map(|plan| item(Kind::Plan, format!("Plan v{}", plan.version), has_groups, false)),
                WaitingSource::Sketch => sketch_card(head, thread, &one.id).map(|card| item(Kind::Sketch, card.title.clone(), true, true)),
                WaitingSource::Document => {
                    document_card(thread, one.id.strip_prefix("document:").unwrap_or(&one.id)).map(|card| item(Kind::Document, card.title.clone(), true, true))
                }
                WaitingSource::Draft => match draft_row(thread, one.id.strip_prefix("draft:").unwrap_or(&one.id)) {
                    Some(SessionRow::Draft { title, change, .. }) => Some(item(Kind::Draft, title.clone(), change.is_none(), true)),
                    _ => None,
                },
                _ => None,
            }
        })
        .collect();
    items.sort_by_key(|item| instant(&item.since));
    items
}

/// One owed document and nothing else for the sheet: it opens on its own and the sheet has nothing to
/// add (`lonePage`).
pub(super) fn lone_page(items: &[Item]) -> Option<&Item> {
    match items {
        [only] if only.kind == Kind::Document => Some(only),
        _ => None,
    }
}

/// The first open item after `from`, else the first open one. None once every item is answered.
pub(super) fn next_open_item(items: &[(String, Standing)], from: &str) -> Option<String> {
    let here = items.iter().position(|(id, _)| id == from).map_or(0, |at| at + 1);
    items[here.min(items.len())..].iter().chain(items.iter()).find(|(_, standing)| *standing == Standing::Open).map(|(id, _)| id.clone())
}

/// Where a list key goes: one up or down, or an end. Clamped at both.
pub(super) fn step_to(len: usize, at: Option<usize>, key: &str) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let here = at.unwrap_or(0);
    Some(match key {
        "home" => 0,
        "end" => len - 1,
        "down" => (here + 1).min(len - 1),
        "up" => here.saturating_sub(1),
        _ => return None,
    })
}

fn note_key(job: &str, item: &str) -> String {
    format!("review:{job}:{item}:note")
}

impl JobView {
    fn review(&self) -> Option<&ReviewState> {
        self.shown.as_ref().and_then(|id| self.pages.get(id)).map(|page| &page.review)
    }

    fn review_mut(&mut self) -> Option<&mut ReviewState> {
        let id = self.shown.clone()?;
        self.pages.get_mut(&id).map(|page| &mut page.review)
    }

    pub(super) fn review_open(&self) -> bool {
        self.review().is_some_and(|review| review.open)
    }

    fn review_noting(&self) -> bool {
        self.review().is_some_and(|review| review.note.is_some())
    }

    /// The items Fleet says wait now, oldest first.
    pub(super) fn review_items(&self, id: &str, cx: &gpui::App) -> Vec<Item> {
        let (Some(record), Some(held)) = (self.record(id, cx), self.thread(id, cx)) else { return Vec::new() };
        let has_groups = record.plan.as_ref().is_some_and(|plan| !plan.groups.is_empty());
        build(&record.waiting_for, &held.head, &held.items, has_groups)
    }

    /// Whether the sheet has items to put to the person.
    pub(super) fn review_applies(&self, id: &str, cx: &gpui::App) -> bool {
        !self.review_items(id, cx).is_empty()
    }

    /// The thread's cards an item answers on, by the key `CardState::answers` holds them under.
    fn card_keys(&self, id: &str, item: &Item, cx: &gpui::App) -> Vec<String> {
        let Some(held) = self.thread(id, cx) else { return Vec::new() };
        match item.kind {
            Kind::Plan => held.head.plan.iter().map(|plan| plan.id.clone()).collect(),
            Kind::Sketch => sketch_card(&held.head, &held.items, &item.id).map(|card| vec![card.id.clone(), format!("pin:{}", card.id)]).unwrap_or_default(),
            Kind::Document => document_card(&held.items, item.id.strip_prefix("document:").unwrap_or(&item.id)).map(|card| vec![card.id.clone()]).unwrap_or_default(),
            Kind::Draft => vec![item.id.clone()],
        }
    }

    /// Where an item stands: how it was answered here or on its card, else whether Fleet still lists it.
    pub(super) fn review_standing(&self, id: &str, item: &Item, cx: &gpui::App) -> Settled {
        let Some(page) = self.pages.get(id) else { return Settled { standing: Standing::Open, said: "" } };
        for key in self.card_keys(id, item, cx) {
            match page.cards.answers.get(&key) {
                Some(Answer::SentBack(_)) => return Settled { standing: Standing::SentBack, said: "Sent back" },
                Some(Answer::Withdrawn | Answer::Dismissed) => return Settled { standing: Standing::Answered, said: "Dismissed" },
                Some(Answer::Sent) => return Settled { standing: Standing::Answered, said: "Sent" },
                Some(_) => return Settled { standing: Standing::Answered, said: "Approved" },
                None => {}
            }
        }
        if page.review.live.contains(&item.id) {
            Settled { standing: Standing::Open, said: "" }
        } else {
            Settled { standing: Standing::Answered, said: "Answered" }
        }
    }

    fn review_standings(&self, id: &str, cx: &gpui::App) -> Vec<(String, Standing)> {
        let Some(page) = self.pages.get(id) else { return Vec::new() };
        page.review.seen.iter().map(|item| (item.id.clone(), self.review_standing(id, item, cx).standing)).collect()
    }

    fn review_current(&self, cx: &gpui::App) -> Option<(Item, Settled)> {
        let id = self.shown.as_ref()?;
        let review = self.review()?;
        let at = review.at.as_ref()?;
        let item = review.seen.iter().find(|item| &item.id == at)?.clone();
        let standing = self.review_standing(id, &item, cx);
        Some((item, standing))
    }

    // ---- opening and closing ---------------------------------------------------------------

    /// The Review row, the band with nothing else on it and `⇧⌘Y`: a page that is all the Job owes is
    /// read on its own card, any other set opens the sheet.
    pub(super) fn review_press(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let items = self.review_items(&id, cx);
        if let Some(one) = lone_page(&items).map(|one| one.id.clone()) {
            self.set_needs(false, None, window, cx);
            self.go_to_card(&id, &one, cx);
            return;
        }
        self.open_review(window, cx);
    }

    /// The thread comes to the card of the page owed: a Job here has no tab for it, so its card is the page.
    pub(super) fn go_to_card(&mut self, id: &SharedString, waiting: &str, cx: &mut Context<Self>) {
        let at = self.thread(id, cx).and_then(|held| {
            let page = waiting.strip_prefix("document:")?;
            held.items.iter().position(|item| matches!(item, ThreadItem::Document(card) if card.page == page))
        });
        if let (Some(at), Some(page)) = (at, self.pages.get(id)) {
            page.list.scroll_to(gpui::ListOffset { item_ix: page.entry(at), offset_in_item: px(0.) });
        }
        cx.notify();
    }

    /// `⇧⌘Y`: the sheet where it has items, else the panel.
    pub(super) fn needs_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.review_open() {
            return;
        }
        if self.review_applies(&id, cx) {
            self.review_press(window, cx);
        } else {
            let (things, review) = self.things(&id, cx);
            if things.is_empty() && review.is_empty() {
                return;
            }
            let up = self.needs_up(&id, cx);
            self.set_needs(!up, None, window, cx);
        }
    }

    /// The sheet's choice moves to `item`, as a press on its row does.
    pub(super) fn review_at(&mut self, item: &str, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(review) = self.review_mut() {
            review.at = Some(item.to_string());
        }
        cx.notify();
    }

    pub(super) fn open_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let items = self.review_items(&id, cx);
        if items.is_empty() {
            return;
        }
        // The Needs you panel shuts as the sheet opens, so the keys do not go back to it: the thread has them.
        let back_to = window.focused(cx).filter(|held| !self.dock_focus.contains_focused(window, cx) && *held != self.dock_focus);
        if let Some(dock) = self.dock_mut() {
            dock.open = Some(false);
            dock.detail = None;
        }
        let Some(page) = self.pages.get_mut(&id) else { return };
        let review = &mut page.review;
        *review = ReviewState::default();
        review.open = true;
        review.live = items.iter().map(|item| item.id.clone()).collect();
        review.before = items.iter().map(|item| (item.id.clone(), Standing::Open)).collect();
        review.at = items.first().map(|item| item.id.clone());
        review.seen = items;
        review.back_to = back_to;
        // The close is where the keys start, as the Sheet focuses its close on opening.
        window.focus(&self.review_focus.close, cx);
        cx.notify();
    }

    pub(super) fn close_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = self.review_mut() else { return };
        if !review.open {
            return;
        }
        let back = review.back_to.take();
        if let Some(id) = self.shown.clone() {
            if let Some(page) = self.pages.get_mut(&id) {
                page.review = ReviewState::default();
            }
        }
        self.pop = None;
        match back {
            Some(handle) => window.focus(&handle, cx),
            None => window.focus(&self.focus, cx),
        }
        cx.notify();
    }

    /// Fleet moved: the items it lists now replace what the sheet drew, an item that went stays where it
    /// was, and one answered elsewhere brings the next open item up.
    pub(super) fn review_sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if !self.review_open() {
            return;
        }
        let items = self.review_items(&id, cx);
        if let Some(page) = self.pages.get_mut(&id) {
            let review = &mut page.review;
            review.live = items.iter().map(|item| item.id.clone()).collect();
            for item in items {
                match review.seen.iter().position(|held| held.id == item.id) {
                    Some(at) => review.seen[at] = item,
                    None => review.seen.push(item),
                }
            }
            review.seen.sort_by_key(|item| instant(&item.since));
        }
        self.review_advance(window, cx);
    }

    /// An item that was open and is not any more has been answered: the next open one comes up, and the
    /// last answer closes the sheet.
    fn review_advance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(review) = self.review().filter(|review| review.open) else { return };
        let (at, was) = (review.at.clone(), review.before.clone());
        let standings = self.review_standings(&id, cx);
        let mut next = None;
        let mut close = false;
        if let Some(at) = &at {
            let now = standings.iter().find(|(held, _)| held == at).map(|(_, standing)| *standing);
            if was.get(at) == Some(&Standing::Open) && now.is_some_and(|standing| standing != Standing::Open) {
                match next_open_item(&standings, at) {
                    Some(to) => next = Some(to),
                    None => close = true,
                }
            }
        }
        if let Some(review) = self.review_mut() {
            review.before = standings.into_iter().collect();
        }
        if close {
            self.close_review(window, cx);
        } else if let Some(to) = next {
            self.review_choose(&to, window, cx);
        }
        cx.notify();
    }

    /// Draw item `id`; a note that was open is put away, its words kept.
    fn review_choose(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let had_note = self.review_noting();
        if let Some(review) = self.review_mut() {
            review.at = Some(id.to_string());
            review.note = None;
        }
        if had_note {
            window.focus(&self.review_focus.list, cx);
        }
        cx.notify();
    }

    // ---- answers ---------------------------------------------------------------------------

    /// The route and body of an answer to an item.
    fn review_request(&self, job: &str, item: &Item, how: SketchHow, words: Option<String>, cx: &gpui::App) -> Option<(&'static str, Result<String, String>)> {
        let session_id = SessionId::carried(job.to_string());
        let encoded = |body: Result<String, ipc::Unencodable>| body.map_err(|why| format!("{why:?}"));
        let held = self.thread(job, cx)?;
        Some(match item.kind {
            Kind::Plan => {
                let version = held.head.plan.as_ref()?.version;
                let mut marks = self.plan_marks(job);
                if how == SketchHow::Approved && marks.is_empty() {
                    ("/sessions/plan/approve", encoded(ipc::encode(&ApprovePlan { session_id, version })))
                } else if how == SketchHow::Approved {
                    // Approve with what he marked: the marks go in the same press.
                    ("/sessions/plan/changes", encoded(ipc::encode(&SendPlanChanges { session_id, version, marks, approve: true })))
                } else {
                    // His note goes with what he marked. It is about the whole plan and a mark has no place for a whole: it goes with the first group, as the plan card's does.
                    let group = self.record(job, cx)?.plan.as_ref()?.groups.first()?.id.clone();
                    marks.push(PlanMark::Note { id: "note-new-sheet".into(), group, task: None, text: words.unwrap_or_default() });
                    ("/sessions/plan/changes", encoded(ipc::encode(&SendPlanChanges { session_id, version, marks, approve: false })))
                }
            }
            Kind::Sketch => {
                let card = sketch_card(&held.head, &held.items, &item.id)?;
                let drawn = self.drawn_of(&card.sketch, cx).unwrap_or_default();
                ("/sessions/sketch/answer", encoded(ipc::encode(&AnswerSketch { session_id, sketch: card.sketch.clone(), drawn, how, words })))
            }
            Kind::Document => {
                let card = document_card(&held.items, item.id.strip_prefix("document:").unwrap_or(&item.id))?;
                (
                    "/sessions/document/answer",
                    encoded(ipc::encode(&AnswerDocument {
                        session_id,
                        document: format!("document:{}", card.page),
                        revision: card.revision.to_string(),
                        how: if how == SketchHow::Approved { DocumentHow::Approved } else { DocumentHow::SentBack },
                        words,
                    })),
                )
            }
            Kind::Draft => {
                let (choice, text) = if how == SketchHow::Approved { (Some(0), None) } else { (None, words) };
                ("/sessions/waiting/answer", encoded(ipc::encode(&AnswerWaiting { session_id, item_id: item.id.clone(), choice, text, mode: None, fixed: None })))
            }
        })
    }

    /// The primary, `⌘↵`: Approve this item.
    pub(super) fn review_primary(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((item, settled)) = self.review_current(cx) else { return };
        if settled.standing != Standing::Open || self.review_noting() {
            return;
        }
        self.review_answer(item, SketchHow::Approved, None, window, cx);
    }

    /// The note's Send back, `⌘↵` in the note.
    fn review_send_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((item, settled)) = self.review_current(cx) else { return };
        let words = self.review().and_then(|review| review.note.as_ref()).map(|note| note.area.read(cx).value().trim().to_string()).unwrap_or_default();
        if settled.standing != Standing::Open || words.is_empty() {
            return;
        }
        self.review_answer(item, SketchHow::SentBack, Some(words), window, cx);
    }

    fn review_answer(&mut self, item: Item, how: SketchHow, words: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let keys = self.card_keys(&job, &item, cx);
        let Some(first) = keys.first().cloned() else { return };
        let Some((route, body)) = self.review_request(&job, &item, how, words.clone(), cx) else { return };
        if item.kind == Kind::Plan {
            self.take_marks(&job);
        }
        let answer =if how == SketchHow::Approved { Answer::Approved } else { Answer::SentBack(words.clone().unwrap_or_default()) };
        // The sketch pinned after the plan is the same card: it reads the answer as the card does.
        if let Some(page) = self.pages.get_mut(&job) {
            for key in keys.iter().skip(1) {
                page.cards.answers.insert(key.clone(), answer.clone());
            }
            page.review.note = None;
        }
        let kept = note_key(&job, &item.id);
        self.keep_words(&kept, "", cx);
        self.settle(SHEET, first, answer, route, body, window, cx);
        self.review_advance(window, cx);
        window.focus(&self.review_focus.list, cx);
    }

    /// Put an item away from its menu: Fleet is told, and the row reads Dismissed.
    fn review_dismiss(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(found) = self.review().and_then(|review| review.seen.iter().find(|one| one.id == item)).cloned() else { return };
        let Some(first) = self.card_keys(&job, &found, cx).into_iter().next() else { return };
        let body = ipc::encode(&DismissWaiting { session_id: SessionId::carried(job.to_string()), item_id: item.to_string(), reason: None, misread: false }).map_err(|why| format!("{why:?}"));
        self.settle(SHEET, first, Answer::Withdrawn, "/sessions/waiting/dismiss", body, window, cx);
        self.review_advance(window, cx);
    }

    // ---- the note --------------------------------------------------------------------------

    /// `⌥R` and Send back: the note opens in the body's foot with what was typed there before, and
    /// where it is already open the caret goes to it.
    pub(super) fn review_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((item, settled)) = self.review_current(cx) else { return };
        if settled.standing != Standing::Open || !item.send_back {
            return;
        }
        let Some(job) = self.shown.clone() else { return };
        if let Some(area) = self.review().and_then(|review| review.note.as_ref()).map(|note| note.area.clone()) {
            area.update(cx, |area, cx| area.focus(window, cx));
            return;
        }
        let key = note_key(&job, &item.id);
        let kept = self.draft_words(&key);
        let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 8));
        area.update(cx, |area, cx| area.set_value(kept, window, cx));
        let typed = cx.subscribe_in(&area, window, move |this, area, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let text = area.read(cx).value().to_string();
                this.keep_words(&key, &text, cx);
                cx.notify();
            }
            InputEvent::PressEnter { secondary: true, .. } => this.review_send_note(window, cx),
            _ => {}
        });
        if let Some(review) = self.review_mut() {
            review.note = Some(NoteBox { area: area.clone(), _typed: typed });
        }
        area.update(cx, |area, cx| area.focus(window, cx));
        cx.notify();
    }

    /// `Esc` and Cancel: the note is put away and what was typed stays for the next time.
    fn review_leave_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(review) = self.review_mut() {
            review.note = None;
        }
        window.focus(&self.review_focus.list, cx);
        cx.notify();
    }

    // ---- keys ------------------------------------------------------------------------------

    /// What the sheet does with the thread's acts while it is up. True where it took the act.
    pub(super) fn review_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.review_open() {
            return false;
        }
        if act.is("send_message") {
            if self.review_noting() {
                self.review_send_note(window, cx);
            } else {
                self.review_primary(window, cx);
            }
            return true;
        }
        if act.is("thread_send_back") {
            self.review_note(window, cx);
            return true;
        }
        // The cards behind take none of the thread's keys while the sheet is up.
        act.is("thread_yes") || act.is("thread_change") || act.is("thread_no") || act.is("needs_you") || act.is("needs_you_fold")
    }

    /// Esc and Tab, caught before anything inside the sheet hears them.
    fn review_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => {
                if self.plan_field_open() {
                    self.field_done(window, cx);
                } else if self.review_noting() {
                    self.review_leave_note(window, cx);
                } else {
                    self.close_review(window, cx);
                }
            }
            "tab" => self.review_tab(event.keystroke.modifiers.shift, window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    /// Tab stays inside the sheet: it goes round its stops and never out of them.
    fn review_tab(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let focus = &self.review_focus;
        let current = self.review_current(cx);
        let open = current.as_ref().is_some_and(|(_, settled)| settled.standing == Standing::Open);
        let noting = self.review_noting();
        let mut stops: Vec<FocusHandle> = Vec::new();
        if open && !noting {
            if current.as_ref().is_some_and(|(item, _)| item.send_back) {
                stops.push(focus.send_back.clone());
            }
            stops.push(focus.primary.clone());
        }
        stops.push(focus.close.clone());
        stops.push(focus.list.clone());
        if noting {
            if let Some(note) = self.review().and_then(|review| review.note.as_ref()) {
                stops.push(note.area.read(cx).focus_handle(cx));
            }
            stops.push(focus.note_send.clone());
            stops.push(focus.note_cancel.clone());
        }
        let here = stops.iter().position(|stop| stop.contains_focused(window, cx));
        let to = match (here, back) {
            (None, false) => 0,
            (None, true) => stops.len() - 1,
            (Some(at), false) => (at + 1) % stops.len(),
            (Some(at), true) => (at + stops.len() - 1) % stops.len(),
        };
        window.focus(&stops[to], cx);
    }

    /// The list's own keys: ↑ ↓ Home End draw the item at once.
    fn review_list_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let mods = event.keystroke.modifiers;
        if mods.alt || mods.platform || mods.control || mods.shift {
            return;
        }
        let Some(review) = self.review() else { return };
        let ids: Vec<String> = review.seen.iter().map(|item| item.id.clone()).collect();
        let here = review.at.as_ref().and_then(|at| ids.iter().position(|id| id == at));
        if let Some(to) = step_to(ids.len(), here, event.keystroke.key.as_str()) {
            self.review_choose(&ids[to], window, cx);
            cx.stop_propagation();
        }
    }

    /// Shift+F10 and the menu key, on the list: the chosen item's menu, under its row.
    pub(super) fn review_menu_key(&mut self, _: &ItemMenu, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = self.review().and_then(|review| review.at.clone()) else { return };
        let Some(bounds) = self.review_rows.borrow().get(&at).copied() else { return };
        self.review_menu(&at, under(bounds), true, window, cx);
    }

    /// The item's menu (`Item`) at `at`: what puts it away.
    fn review_menu(&mut self, item: &str, at: gpui::Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(found) = self.review().and_then(|review| review.seen.iter().find(|one| one.id == item)).cloned() else { return };
        let Some(job) = self.shown.clone() else { return };
        if !found.dismissable || self.review_standing(&job, &found, cx).standing != Standing::Open {
            return;
        }
        let entries = vec![Entry::Act { id: "dismiss", label: "Dismiss".into(), icon: Some("x"), danger: false }];
        let target = item.to_string();
        let back = Some(self.review_focus.list.clone());
        self.open_pop_back(
            "Item",
            entries,
            at,
            from_keys,
            back,
            move |this, id, window, cx| {
                if id == "dismiss" {
                    this.review_dismiss(&target, window, cx);
                }
            },
            window,
            cx,
        );
    }

    // ---- drawing ---------------------------------------------------------------------------

    /// The sheet over the window, or nothing while it is shut.
    pub(super) fn render_review(&self, id: &SharedString, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let page = self.pages.get(id)?;
        let review = Some(&page.review).filter(|review| review.open)?;
        let look = cx.look();
        let viewport = window.viewport_size();
        let title: SharedString = self.record(id, cx).and_then(|job| job.title.clone()).map_or_else(|| "Review".into(), Into::into);
        let current = review.at.as_ref().and_then(|at| review.seen.iter().find(|item| &item.id == at)).cloned();
        let settled = current.as_ref().map(|item| self.review_standing(id, item, cx));
        let answers = current.as_ref().filter(|_| settled.is_some_and(|settled| settled.standing == Standing::Open));
        let noting = review.note.is_some() && answers.is_some();
        let focus = &self.review_focus;

        let send_back = answers.filter(|item| item.send_back).map(|_| {
            labelled(
                button(look, "review-send-back", "Send back", Tone::Secondary, true, Some("\u{2325}R"), !noting)
                    .track_focus(&focus.send_back)
                    .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                    .when(!noting, |b| {
                        b.on_click(cx.listener(|this, _, window, cx| this.review_note(window, cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.review_note(window, cx);
                                cx.stop_propagation();
                            }
                        }))
                    }),
                Role::Button,
                "Send back",
            )
        });
        let primary = answers.map(|item| {
            let said: SharedString = format!("Approve {}", item.name).into();
            labelled(
                button(look, "review-primary", &said, Tone::Primary, true, Some("\u{2318}\u{21b5}"), !noting)
                    .track_focus(&focus.primary)
                    .focus_visible(|style| style.border_color(look.color(token::TEXT_ON_ACCENT)))
                    .when(!noting, |b| {
                        b.on_click(cx.listener(|this, _, window, cx| this.review_primary(window, cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.platform {
                                this.review_primary(window, cx);
                                cx.stop_propagation();
                            }
                        }))
                    }),
                Role::Button,
                said,
            )
        });
        let close = labelled(
            div()
                .id("review-close")
                .track_focus(&focus.close)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_LG))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Close \u{b7} Esc"))
                .child(glyph(look, "x", Size::Mark, look.color(token::FG_MUTED)))
                .on_click(cx.listener(|this, _, window, cx| this.close_review(window, cx)))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.close_review(window, cx);
                        cx.stop_propagation();
                    }
                })),
            Role::Button,
            "Close",
        );
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_6))
            .h(look.length(token::H_ROW_UNIT) + look.length(token::SPACE_6))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_LG))
                    .line_height(look.length(token::LEADING_LG))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(title.clone()),
            )
            .children(send_back)
            .children(primary)
            .child(close);

        let rows: Vec<AnyElement> = review.seen.iter().map(|item| self.review_row(id, item, review, look, cx)).collect();
        let list = labelled(
            div()
                .id("review-list")
                .track_focus(&focus.list)
                .key_context("review-sheet")
                .on_action(cx.listener(Self::review_menu_key))
                .on_key_down(cx.listener(Self::review_list_key))
                .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                .border_r(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .flex_none()
                .w(look.length(token::SPACE_12) * 5. + look.length(token::SPACE_6))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_3))
                .overflow_y_scroll()
                .bg(look.color(token::BG_SUNKEN))
                .children(rows),
            Role::Group,
            "Waiting on you",
        );

        let detail = current.as_ref().map(|item| {
            let said = self.card_keys(id, item, cx).iter().find_map(|key| page.cards.said.get(key).cloned());
            let body = self.review_body(id, item, page, look, cx);
            let foot = noting.then(|| self.review_foot(review, look, cx)).flatten();
            labelled(
                div()
                    .id(SharedString::from(format!("review-detail:{}", item.id)))
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id(SharedString::from(format!("review-body:{}", item.id)))
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .gap(look.length(token::SPACE_3))
                            .p(look.length(token::PAD_CARD) + look.length(token::SPACE_3))
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .child(body)
                            .children(said.map(|why| refusal(look, "review", why))),
                    )
                    .children(foot),
                Role::Region,
                item.name.clone(),
            )
        });

        let margin = look.length(token::SPACE_4);
        let wide = (viewport.width * 0.76).max(look.length(token::W_DIALOG_WIDE)).min(viewport.width - margin * 2.);
        let sheet = labelled(
            div()
                .id("review-sheet")
                .capture_key_down(cx.listener(Self::review_key))
                .occlude()
                .w(wide)
                .h(viewport.height - look.length(token::H_TITLE_BAR) - margin * 2.)
                .mt(look.length(token::H_TITLE_BAR) + margin)
                .mr(margin)
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .child(head)
                .child(div().flex_1().min_h_0().flex().child(list).children(detail)),
            Role::Dialog,
            title,
        );
        Some(
            deferred(
                anchored().position(point(px(0.), px(0.))).anchor(Anchor::TopLeft).child(
                    // A press on the ground behind takes nothing: the close and Esc are the two exits.
                    div().id("review-scrim").occlude().w(viewport.width).h(viewport.height).flex().justify_end().bg(look.color(token::SCRIM)).child(sheet),
                ),
            )
            .with_priority(0)
            .into_any_element(),
        )
    }

    /// One item of the list: its kind, its name, and how it was answered.
    fn review_row(&self, id: &SharedString, item: &Item, review: &ReviewState, look: &Look, cx: &Context<Self>) -> AnyElement {
        let settled = self.review_standing(id, item, cx);
        let on = review.at.as_deref() == Some(item.id.as_str());
        let (icon, kind_said) = item.kind.mark();
        let spent = settled.standing != Standing::Open;
        let mark = match settled.standing {
            Standing::Open => None,
            Standing::Answered => Some(("badge-check", settled.said, token::STATUS_COMPLETED_SUCCESS)),
            Standing::SentBack => Some(("reply", "Sent back", token::FG_MUTED)),
        };
        let (pick, menu_item) = (item.id.clone(), item.id.clone());
        let rows = Rc::clone(&self.review_rows);
        let kept = item.id.clone();
        let row = div()
            .id(SharedString::from(format!("review-item:{}", item.id)))
            .relative()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_ROW_UNIT))
            .px(look.length(token::SPACE_3))
            // The chosen item is filled and ringed thin, square, as Electron's focused `[data-on]` item draws.
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if on { look.color(token::ACCENT) } else { gpui::transparent_black() })
            .when(on, |row| row.bg(look.color(token::SURFACE_ROW_SELECTED)))
            .when(!on, |row| row.hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER))))
            .cursor_pointer()
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(if spent { token::FG_SUBTLE } else { token::FG_DEFAULT }))
            .child(labelled(
                div().id(SharedString::from(format!("review-kind:{}", item.id))).flex_none().tooltip(tip(kind_said)).child(glyph(
                    look,
                    icon,
                    Size::Small,
                    look.color(if spent { token::FG_SUBTLE } else { token::FG_MUTED }),
                )),
                Role::Image,
                kind_said,
            ))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(item.name.clone())))
            .children(mark.map(|(icon, said, hue)| {
                labelled(
                    div().id(SharedString::from(format!("review-mark:{}", item.id))).flex_none().tooltip(tip(said)).child(glyph(look, icon, Size::Small, look.color(hue))),
                    Role::Image,
                    said,
                )
            }))
            .child(canvas(move |bounds, _, _| { rows.borrow_mut().insert(kept.clone(), bounds); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            .on_click(cx.listener(move |this, _, window, cx| {
                this.review_choose(&pick, window, cx);
                window.focus(&this.review_focus.list, cx);
            }))
            .when(item.dismissable, |row| {
                row.on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                        this.review_choose(&menu_item, window, cx);
                        this.review_menu(&menu_item, event.position, false, window, cx);
                    }),
                )
            });
        labelled(row, Role::Button, item.name.clone()).into_any_element()
    }

    /// The body of the chosen item: the thread's own card, drawn without its answers.
    fn review_body(&self, id: &SharedString, item: &Item, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(held) = self.thread(id, cx) else { return self.review_skeleton(look) };
        let now = Local::now();
        match item.kind {
            Kind::Plan => {
                let card = held.head.plan.as_ref();
                match card.and_then(|card| self.record(id, cx).and_then(|record| record.plan.as_ref()).filter(|plan| plan.version == card.version).map(|plan| (card, plan))) {
                    Some((card, plan)) => {
                        // A plan recorded again reads first as the diff the thread's card draws.
                        let diff = plan
                            .changes
                            .as_ref()
                            .filter(|changes| plan.version >= 2 && !changes.is_empty())
                            .map(|changes| self.plan_revision(SHEET, &card.id, plan, changes, page, look, cx));
                        self.plan_editor(id, plan, diff, false, look, cx)
                    }
                    None => self.review_skeleton(look),
                }
            }
            Kind::Sketch => match sketch_card(&held.head, &held.items, &item.id) {
                Some(card) => {
                    let band = self.sketch_band(SHEET, &card.id, &card.title, card.replaced, card.against.as_deref(), Some((&card.at, card.revisions.len(), None)), look, now, cx);
                    div().flex().flex_col().min_h(look.length(token::H_ROW_UNIT) * 8.).child(band)
                        .child(glass(
                            look,
                            card.replaced,
                            match super::cards::shown_scene(card, None) {
                                (key, Some(scene)) => Some(crate::scene::view(key.into(), scene, crate::scene::Fit::Preview, crate::scene::Clock::default(), crate::scene::Lens::default()).into_any_element()),
                                _ => None,
                            },
                        ))
                        .into_any_element()
                }
                None => self.review_skeleton(look),
            },
            Kind::Document => match document_card(&held.items, item.id.strip_prefix("document:").unwrap_or(&item.id)) {
                Some(card) => self.review_document(card, look, now, cx),
                None => self.review_skeleton(look),
            },
            Kind::Draft => match draft_row(&held.items, item.id.strip_prefix("draft:").unwrap_or(&item.id)) {
                Some(SessionRow::Draft { id: row, at, title, text, before, settled, change }) => div()
                    .flex()
                    .flex_col()
                    .min_h(look.length(token::H_ROW_UNIT) * 8.)
                    .child(self.draft(SHEET, row, at, title, text, before.as_deref(), settled.as_ref(), *change, true, page, look, now, cx))
                    .into_any_element(),
                _ => self.review_skeleton(look),
            },
        }
    }

    /// A document revision: its band, and the pane the page is read in.
    fn review_document(&self, card: &ThreadDocument, look: &Look, now: chrono::DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let band = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .child(glyph(look, "file-text", Size::Small, look.color(token::FG_MUTED)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_BASE))
                    .line_height(look.length(token::LEADING_BASE))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(card.title.clone())),
            )
            .child(
                div()
                    .id(SharedString::from(format!("review-revision:{}", card.id)))
                    .flex_none()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .tooltip(tip(format!("Revision {}", card.revision)))
                    .child(format!("r{}", card.revision)),
            )
            .child(self.stamp(&card.at, look, now, true, cx));
        // The page itself is a web page, which the native app draws in its embedded browser; the card is the band.
        div().flex().flex_col().gap(look.length(token::SPACE_3)).child(band).into_any_element()
    }

    /// The shape of a body that has not arrived: four lines at 40%, full, 85% and 60%.
    fn review_skeleton(&self, look: &Look) -> AnyElement {
        use gpui::AnimationExt as _;
        let bar = |width: f32| div().h(look.length(token::SPACE_3)).w(gpui::relative(width)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::FG_DEFAULT).opacity(0.07));
        labelled(
            div().id("review-loading").w_full().child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .child(bar(0.4))
                    .child(bar(1.0))
                    .child(bar(0.85))
                    .child(bar(0.6))
                    .with_animation(
                        "review-loading-pulse",
                        gpui::Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                        |shapes, at| shapes.opacity(at),
                    ),
            ),
            Role::Status,
            "Loading",
        )
        .into_any_element()
    }

    /// The note that sends the item back, in the body's foot.
    fn review_foot(&self, review: &ReviewState, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let note = review.note.as_ref()?;
        let blank = note.area.read(cx).value().trim().is_empty();
        let focus = &self.review_focus;
        let send = labelled(
            button(look, "review-note-send", "Send back", Tone::Primary, true, Some("\u{2318}\u{21b5}"), !blank).track_focus(&focus.note_send).when(!blank, |b| {
                b.on_click(cx.listener(|this, _, window, cx| this.review_send_note(window, cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.platform {
                        this.review_send_note(window, cx);
                        cx.stop_propagation();
                    }
                }))
            }),
            Role::Button,
            "Send back",
        );
        let cancel = labelled(
            button(look, "review-note-cancel", "Cancel", Tone::Ghost, true, Some("Esc"), true)
                .track_focus(&focus.note_cancel)
                .on_click(cx.listener(|this, _, window, cx| this.review_leave_note(window, cx)))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.review_leave_note(window, cx);
                        cx.stop_propagation();
                    }
                })),
            Role::Button,
            "Cancel",
        );
        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_RAISED))
                .child(labelled(
                    div()
                        .id("review-note")
                        .key_context(crate::keymap::TEXT_BOX)
                        .w_full()
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .bg(look.color(token::BG_SUNKEN))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(Textarea::new(&note.area).appearance(false).aria_label("Note")),
                    Role::TextInput,
                    "Note",
                ))
                .child(div().flex().items_center().justify_end().gap(look.length(token::SPACE_2)).child(send).child(cancel))
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn waiting(wire: &str) -> Vec<WaitingItem> {
        ipc::decode::<Vec<WaitingItem>>("waiting", wire.as_bytes()).expect("waiting items")
    }

    fn thread(head: &str, items: &str) -> (ThreadHead, Vec<ThreadItem>) {
        let wire = format!(r#"{{"session_id":"j","head":{head},"items":[{items}]}}"#);
        let read = ipc::decode::<ipc::JobThread>("thread", wire.as_bytes()).expect("a thread");
        (read.head, read.items)
    }

    const TRAIL: &str = r#""trail":{"started":"2026-11-09T09:00:00Z"}"#;

    fn owed() -> Vec<WaitingItem> {
        waiting(
            r#"[
              {"id":"draft:d1","text":"Comment on 3290","since":"2026-11-09T14:10:00Z","source":"draft"},
              {"id":"plan:2","text":"Plan v2","since":"2026-11-09T13:41:00Z","source":"plan"},
              {"id":"document:design","text":"Stale branch design","since":"2026-11-09T14:03:00Z","source":"document"},
              {"id":"sketch:Tail","text":"Tail layout","since":"2026-11-09T14:01:00Z","source":"sketch"},
              {"id":"ask:c","text":"A question","since":"2026-11-09T14:02:00Z","source":"ask_card"}
            ]"#,
        )
    }

    fn held() -> (ThreadHead, Vec<ThreadItem>) {
        thread(
            &format!(r#"{{{TRAIL},"plan":{{"id":"plan:v2","version":2,"state":"waiting"}}}}"#),
            r#"{"kind":"sketch","id":"sk-1","at":"2026-11-09T14:01:00Z","sketch":"sketch:Tail","title":"Tail layout","drawing":{},"asked":true},
               {"kind":"document","id":"document:design:r2","at":"2026-11-09T14:03:00Z","page":"design","title":"Stale branch design","revision":2,"asked":true},
               {"kind":"row","row":{"kind":"draft","id":"d1","at":"2026-11-09T14:10:00Z","title":"Comment","text":"words"}}"#,
        )
    }

    #[test]
    fn the_items_stand_oldest_first_and_a_question_is_not_one_of_them() {
        let (head, items) = held();
        let built = build(&owed(), &head, &items, true);
        let order: Vec<&str> = built.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(order, ["plan:2", "sketch:Tail", "document:design", "draft:d1"]);
        assert_eq!(built.iter().map(|item| item.kind).collect::<Vec<_>>(), [Kind::Plan, Kind::Sketch, Kind::Document, Kind::Draft]);
        assert_eq!(built[0].name, "Plan v2");
    }

    #[test]
    fn a_row_with_no_card_to_draw_stays_with_the_needs_you_card() {
        let (head, _) = held();
        assert!(build(&owed(), &head, &[], true).iter().all(|item| item.kind == Kind::Plan));
    }

    #[test]
    fn a_plan_with_no_group_and_a_proposed_workflow_have_no_send_back() {
        let (head, items) = held();
        let plan = |groups| build(&owed(), &head, &items, groups).into_iter().find(|item| item.kind == Kind::Plan).expect("a plan");
        assert!(plan(true).send_back);
        assert!(!plan(false).send_back);
        let (_, workflow) = thread(
            &format!("{{{TRAIL}}}"),
            r#"{"kind":"row","row":{"kind":"draft","id":"d1","at":"2026-11-09T14:10:00Z","title":"Workflow quick","text":"steps","change":12}}"#,
        );
        let draft = build(&owed(), &head, &workflow, true).into_iter().find(|item| item.kind == Kind::Draft).expect("a draft");
        assert!(!draft.send_back);
        assert!(draft.dismissable);
    }

    #[test]
    fn the_next_open_item_is_the_one_after_else_the_first_open_one() {
        use Standing::{Answered, Open, SentBack};
        let rows = |states: &[Standing]| states.iter().enumerate().map(|(at, standing)| (format!("i{at}"), *standing)).collect::<Vec<_>>();
        assert_eq!(next_open_item(&rows(&[Open, Answered, Open, Open]), "i0").as_deref(), Some("i2"));
        assert_eq!(next_open_item(&rows(&[Open, Answered, SentBack]), "i1").as_deref(), Some("i0"));
        assert_eq!(next_open_item(&rows(&[Answered, SentBack]), "i1"), None);
        assert_eq!(next_open_item(&[], "i0"), None);
    }

    #[test]
    fn the_keys_move_the_list_and_stop_at_its_ends() {
        assert_eq!(step_to(4, Some(1), "down"), Some(2));
        assert_eq!(step_to(4, Some(3), "down"), Some(3));
        assert_eq!(step_to(4, Some(0), "up"), Some(0));
        assert_eq!(step_to(4, Some(2), "home"), Some(0));
        assert_eq!(step_to(4, Some(0), "end"), Some(3));
        assert_eq!(step_to(0, None, "down"), None);
        assert_eq!(step_to(4, Some(1), "x"), None);
    }

    #[test]
    fn a_page_that_is_all_the_job_owes_opens_on_its_own() {
        let (head, items) = held();
        let all = build(&owed(), &head, &items, true);
        assert!(lone_page(&all).is_none());
        let doc = |ids: &str| build(&waiting(ids), &head, &items, true);
        let one = doc(r#"[{"id":"document:design","text":"Design","since":"2026-11-09T14:03:00Z","source":"document"}]"#);
        assert_eq!(lone_page(&one).map(|item| item.id.as_str()), Some("document:design"));
        let sketch = doc(r#"[{"id":"sketch:Tail","text":"Tail","since":"2026-11-09T14:03:00Z","source":"sketch"}]"#);
        assert!(lone_page(&sketch).is_none());
    }
}
