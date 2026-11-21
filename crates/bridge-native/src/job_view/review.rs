//! The review sheet: every item with an Approve he reads, across Jobs, on one sheet (the look "Several at
//! once: Grouped", 14 Nov). A plan, a sketch, a document and a draft stand in a list grouped under each Job's
//! name and state, the chosen one is drawn beside it with its notes and its own answers in its band, and the list's foot holds Approve all
//! and, under its caret, Dismiss all, away from the item's own buttons.
//!
//! **Every answer is `answer_items` or `dismiss_items`**, one item or all still asking, and each outcome
//! lands on its own row: a refusal stands there as Fleet's sentence and the rest settle
//! (`docs/practices/protocol/approve-all-and-dismiss-all.md`). A question, a command and a Job's start
//! stay on their cards.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::{
    anchored, canvas, deferred, div, prelude::*, px, Anchor, AnyElement, Bounds, Context, Entity, FocusHandle, Focusable as _, FontWeight,
    KeyDownEvent, MouseButton, Pixels, Role, SharedString, Subscription, Window,
};
use gpui_component::input::{Enter, InputEvent, Textarea, TextareaState};
use ipc::{
    AnswerItems, DismissItems, ItemAnswer, ItemHow, ItemNote, ItemOutcome, ItemRef, ItemSettled, ItemsSettled, JobState, PlanCardState, SessionId,
    ReadyState, SessionRow, ThreadDocument, ThreadHead, ThreadItem, ThreadSketch, WaitingItem, WaitingSource,
};

use super::acts::{button, kbd, Tone, SHEET};
use super::cards::glass;
use super::rows::{glyph, tip, Size};
use super::{JobView, Page};
use crate::dropdown::{under, Entry};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};

/// What an item is. The sheet draws a plan, a sketch, a document and a draft and nothing else: a
/// question, a command and a pull request stay on the Needs you card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Plan,
    Sketch,
    Document,
    Draft,
    /// An artifact of parts, answered part by part in its own detail (`reader_sheet.rs`).
    Artifact,
    /// A build the Job offered to try, one item for each still asking: drawn as the Ready to try card draws it, with what he approved
    /// and what was built side by side, and answered there (`ready_card.rs`), so the sheet offers no Approve or Dismiss of its own.
    Build,
}

impl Kind {
    /// The kind's glyph, the word its tooltip names it by, and the hue its tile takes (the look's kind-glyph).
    pub(super) fn mark(self) -> (&'static str, &'static str, Token) {
        match self {
            Kind::Plan => ("list-todo", "Plan", token::KIND_DELIVERY),
            Kind::Sketch => ("layers", "Sketch", token::KIND_STEP),
            Kind::Draft => ("file-text", "Draft", token::KIND_SETUP),
            Kind::Document => ("scroll-text", "Document", token::KIND_GATE),
            Kind::Artifact => ("layers", "Artifact", token::KIND_STEP),
            Kind::Build => ("circle-dot", "Ready to try", token::STATUS_COMPLETED_SUCCESS),
        }
    }

    /// Whether the item is answered in its own detail, by its parts or its build's four answers, and never by the sheet's
    /// Approve, Ask for changes and Dismiss.
    pub(super) fn answers_inside(self) -> bool {
        matches!(self, Kind::Artifact | Kind::Build)
    }
}

/// One thing waiting on the sheet.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Item {
    /// The Job it waits in.
    pub job: String,
    /// The waiting item's id: `plan:2`, `sketch:<title>`, `document:<page>`, `draft:<row>`.
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub since: String,
    /// What it is for, from its card's For line. Absent where the card has none.
    pub for_line: Option<String>,
    /// Its newest revision, as `r3`. Absent for a plan and a draft.
    pub revision: Option<String>,
    /// Whether Ask for changes is offered. A proposed workflow has no note to send back with, and a plan
    /// with no group has nowhere to put one.
    pub send_back: bool,
    /// A page he is only to read (`read:<page>`): it has no Approve, Ask for changes or Dismiss, only Open.
    pub read: bool,
    /// The artifact of parts an item of kind Artifact stands for, whatever its own id is.
    pub artifact: Option<String>,
}

/// Whose items the panel lists: one Job's, where it was opened from a Job, or every Job's grouped by Job, where it was opened from
/// a screen that is no Job's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum Scope {
    Job(String),
    #[default]
    All,
}

/// The page a Document item is about: `document:<page>` waits on his answer, `read:<page>` on his reading it.
pub(super) fn page_of_item(item: &str) -> Option<&str> {
    item.strip_prefix("document:").or_else(|| item.strip_prefix("read:"))
}

impl Item {
    pub(super) fn key(&self) -> String {
        key_of(&self.job, &self.id)
    }
}

/// The sheet's key for an item: its Job and its id, which alone is not unique across Jobs.
pub(super) fn key_of(job: &str, id: &str) -> String {
    format!("{job}\u{1f}{id}")
}

/// The key of the item that stands for artifact `artifact` of Job `job` in `groups`. A waiting row naming the artifact is
/// listed in its place, so a press on the artifact's card picks that row.
fn standing_for(groups: &[Group], job: &str, artifact: &str) -> Option<String> {
    groups.iter().filter(|group| group.job == job).flat_map(|group| group.items.iter()).find(|item| super::reader_sheet::artifact_in(item) == Some(artifact)).map(Item::key)
}

/// The items of one Job, under its name and state.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Group {
    pub job: String,
    pub title: String,
    pub state: Option<JobState>,
    pub items: Vec<Item>,
}

/// How an item was answered. None of these is an open item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Verdict {
    Approved,
    ChangesAsked,
    Dismissed,
    /// Fleet stopped listing it, answered on its card or by another of his Bridges.
    Answered,
}

impl Verdict {
    /// The glyph, the words and the hue its row shows.
    fn mark(self) -> (&'static str, &'static str, Token) {
        match self {
            Verdict::Approved => ("badge-check", "Approved", token::STATUS_COMPLETED_SUCCESS),
            Verdict::ChangesAsked => ("pencil", "Changes asked", token::STATUS_REJECTED),
            Verdict::Dismissed => ("x", "Dismissed", token::STATUS_KILLED),
            Verdict::Answered => ("check", "Answered", token::FG_MUTED),
        }
    }
}

/// Where one row stands: its verdict, or none while it still asks, and Fleet's sentence where it refused one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Row {
    pub verdict: Option<Verdict>,
    pub refused: Option<String>,
}

/// The notes of the chosen item; what is typed is kept under the item.
pub(super) struct NoteBox {
    pub key: String,
    pub area: Entity<TextareaState>,
    _typed: Subscription,
}

/// What the sheet remembers while it is open. Held on the page of the Job it was opened from.
#[derive(Default)]
pub(super) struct ReviewState {
    pub open: bool,
    /// The chosen item's key.
    pub at: Option<String>,
    /// The groups as they were when the sheet opened and as they came since, so one answered stays in its
    /// place until the sheet shuts.
    pub groups: Vec<Group>,
    /// The keys Fleet still lists as waiting.
    pub live: HashSet<String>,
    pub rows: HashMap<String, Row>,
    pub back_to: Option<FocusHandle>,
    pub note: Option<NoteBox>,
    /// The artifact he opened the sheet on, kept in its list while it is up though it asks no more.
    pub opened: Option<String>,
    pub scope: Scope,
    /// The live pages of the chosen artifact's page parts, by the page each names: they go with the panel (`page_tab.rs`).
    pub pages: HashMap<String, (Entity<super::page_tab::PageTab>, Subscription)>,
}

impl ReviewState {
    fn items(&self) -> impl Iterator<Item = &Item> {
        self.groups.iter().flat_map(|group| group.items.iter())
    }

    fn item(&self, key: &str) -> Option<&Item> {
        self.items().find(|item| item.key() == key)
    }

    fn row(&self, key: &str) -> Row {
        self.rows.get(key).cloned().unwrap_or_default()
    }
}

/// The sheet's focus: one handle for each thing a key can reach.
pub(super) struct Focuses {
    pub list: FocusHandle,
    pub approve: FocusHandle,
    pub changes: FocusHandle,
    pub dismiss: FocusHandle,
    pub close: FocusHandle,
    pub approve_all: FocusHandle,
    pub more_all: FocusHandle,
}

impl Focuses {
    pub(super) fn new(cx: &mut gpui::App) -> Self {
        Focuses {
            list: cx.focus_handle(),
            approve: cx.focus_handle(),
            changes: cx.focus_handle(),
            dismiss: cx.focus_handle(),
            close: cx.focus_handle(),
            approve_all: cx.focus_handle(),
            more_all: cx.focus_handle(),
        }
    }
}

/// Where the list foot's caret was drawn, among the rows' bounds, for the menu that opens above it.
const MORE_ALL: &str = "more-all";

/// Where each item's row was drawn, for a menu opened from the keys.
pub(super) type Rows = Rc<RefCell<HashMap<String, Bounds<Pixels>>>>;

/// What the Operator read of an item, for the panel to show above its notes: the reading, and the pick it would make.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorRead {
    pub reading: SharedString,
    pub pick: Option<SharedString>,
}

/// Opens the review panel on an item, as `open_over` asks. Set by the Job view, which draws the panel.
#[derive(Clone)]
struct OpenOver(Rc<dyn Fn(ItemRef, Option<OperatorRead>, &mut Window, &mut gpui::App)>);

impl gpui::Global for OpenOver {}

/// **Open the review panel over the screen with `item` chosen**, as OpenMod opens a mod's panel over Settings. It records where
/// the keys were, so one Esc gives them back to the card he pressed, and keeps `operator_read`, where the Operator page has one
/// for this item, to draw above its notes as a small block headed Operator. The Jobs screen passes None. It does nothing where
/// no Job view stands, or the item is not one the panel holds.
pub fn open_over(window: &mut Window, cx: &mut gpui::App, item: ItemRef, operator_read: Option<(SharedString, Option<SharedString>)>) {
    let Some(open) = cx.try_global::<OpenOver>().cloned() else { return };
    let read = operator_read.map(|(reading, pick)| OperatorRead { reading, pick: pick.filter(|pick| !pick.trim().is_empty()) });
    (open.0)(item, read, window, cx);
}

pub(super) fn offer_open_over(cx: &mut Context<JobView>) {
    super::on_page::offer(cx);
    let view = cx.entity().downgrade();
    cx.set_global(OpenOver(Rc::new(move |item, read, window, cx| {
        let _ = view.update(cx, |this, cx| this.open_review_for(item, read, window, cx));
    })));
}

/// The key context the sheet holds its keys under: the registry's `review sheet` scope.
const CONTEXT: &str = "review-sheet";

gpui::actions!(review, [ItemMenu]);

pub(super) fn bind(cx: &mut gpui::App) {
    cx.bind_keys([gpui::KeyBinding::new("shift-f10", ItemMenu, Some(CONTEXT)), gpui::KeyBinding::new("menu", ItemMenu, Some(CONTEXT))]);
}

fn instant(at: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(at).map_or(0, |then| then.timestamp_millis())
}

/// The sketch card the ledger's row `id` stands for: the pinned one, else the newest in the thread.
pub(super) fn sketch_card<'a>(head: &'a ThreadHead, thread: &'a [ThreadItem], id: &str) -> Option<&'a ThreadSketch> {
    head.sketch.as_ref().filter(|card| card.sketch == id).or_else(|| {
        thread.iter().rev().find_map(|item| match item {
            ThreadItem::Sketch(card) if card.sketch == id => Some(card),
            _ => None,
        })
    })
}

/// The newest revision card of a page.
pub(super) fn document_card<'a>(thread: &'a [ThreadItem], page: &str) -> Option<&'a ThreadDocument> {
    thread.iter().rev().find_map(|item| match item {
        ThreadItem::Document(card) if card.page == page => Some(card),
        _ => None,
    })
}

pub(super) fn draft_row<'a>(thread: &'a [ThreadItem], row: &str) -> Option<&'a SessionRow> {
    thread.iter().rev().find_map(|item| match item {
        ThreadItem::Row { row: found @ SessionRow::Draft { id, .. } } if id == row => Some(found),
        _ => None,
    })
}

fn for_text(line: Option<&ipc::ArtifactFor>) -> Option<String> {
    line.and_then(|line| line.text.clone()).filter(|text| !text.trim().is_empty())
}

/// Every item a Job has for the sheet, oldest first (`waitingOf`). **A waiting row the sheet has no body
/// for is not built** and stays with the Needs you card.
pub(super) fn build(job: &str, waiting: &[WaitingItem], head: &ThreadHead, thread: &[ThreadItem], has_groups: bool) -> Vec<Item> {
    let mut items: Vec<Item> = waiting
        .iter()
        .filter_map(|one| {
            let item = |kind, name: String, for_line, revision, send_back| Item {
                job: job.to_string(),
                id: one.id.clone(),
                kind,
                name,
                since: one.since.as_str().to_string(),
                for_line,
                revision,
                send_back,
                read: one.source == WaitingSource::Document && one.id.starts_with("read:"),
                artifact: None,
            };
            match one.source {
                WaitingSource::Plan => head
                    .plan
                    .as_ref()
                    .filter(|plan| plan.state == PlanCardState::Waiting)
                    .map(|plan| item(Kind::Plan, format!("Plan v{}", plan.version), None, None, has_groups)),
                WaitingSource::Sketch => sketch_card(head, thread, &one.id).map(|card| {
                    let revision = (!card.revisions.is_empty()).then(|| format!("r{}", card.revisions.len()));
                    item(Kind::Sketch, card.title.clone(), for_text(card.for_line.as_ref()), revision, true)
                }),
                WaitingSource::Document => document_card(thread, page_of_item(&one.id).unwrap_or(&one.id))
                    .map(|card| item(Kind::Document, card.title.clone(), for_text(card.for_line.as_ref()), Some(format!("r{}", card.revision)), !one.id.starts_with("read:"))),
                WaitingSource::Draft => match draft_row(thread, one.id.strip_prefix("draft:").unwrap_or(&one.id)) {
                    Some(SessionRow::Draft { title, change, for_line, .. }) => Some(item(Kind::Draft, title.clone(), for_text(for_line.as_ref()), None, change.is_none())),
                    _ => None,
                },
                // A Job's one Needs you row stands for each build still to try, which the loop after this one lists.
                _ => None,
            }
        })
        .collect();
    if waiting.iter().any(|one| one.source == WaitingSource::ReadyToTry) {
        items.extend(builds_to_try(job, thread));
    }
    items.sort_by_key(|item| instant(&item.since));
    items
}

/// An item for each build of the Job still to answer, open or put off, each drawn as the Ready to try card draws it.
fn builds_to_try(job: &str, thread: &[ThreadItem]) -> Vec<Item> {
    thread
        .iter()
        .filter_map(|one| match one {
            ThreadItem::ReadyToTry(card) if matches!(card.state, ReadyState::Open | ReadyState::Later) => Some(Item {
                job: job.to_string(),
                id: card.id.clone(),
                kind: Kind::Build,
                name: card.about.clone(),
                since: card.offered_at.as_str().to_string(),
                for_line: None,
                revision: None,
                send_back: false,
                read: false,
                artifact: None,
            }),
            _ => None,
        })
        .collect()
}

/// Whether a waiting item is one the sheet could hold, before its card is read.
fn reviewable(source: WaitingSource) -> bool {
    matches!(source, WaitingSource::Plan | WaitingSource::Sketch | WaitingSource::Document | WaitingSource::Draft | WaitingSource::ReadyToTry)
}

/// One owed document and nothing else for the sheet: it opens on its own and the sheet has nothing to
/// add (`lonePage`).
pub(super) fn lone_page(items: &[Item]) -> Option<&Item> {
    match items {
        [only] if only.kind == Kind::Document => Some(only),
        _ => None,
    }
}

/// The groups in the order the sheet stands them: the Job it was opened from first, then each other Job by
/// its oldest item. A Job with nothing for the sheet has no group.
pub(super) fn grouped(first: &str, mut groups: Vec<Group>) -> Vec<Group> {
    groups.retain(|group| !group.items.is_empty());
    groups.sort_by_key(|group| (group.job != first, group.items.iter().map(|item| instant(&item.since)).min().unwrap_or(i64::MAX)));
    groups
}

/// The keys of every item, in the order the list stands them.
pub(super) fn order(groups: &[Group]) -> Vec<String> {
    groups.iter().flat_map(|group| group.items.iter().map(Item::key)).collect()
}

/// The first open item after `from`, else the first open one. None once every item is answered.
pub(super) fn next_open(order: &[String], rows: &HashMap<String, Row>, from: &str) -> Option<String> {
    let open = |key: &&String| rows.get(key.as_str()).is_none_or(|row| row.verdict.is_none());
    let here = order.iter().position(|key| key == from).map_or(0, |at| at + 1);
    order[here.min(order.len())..].iter().chain(order.iter()).find(open).cloned()
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

/// What his notes box holds, as the notes of a verdict: one note, or none where it is blank.
pub(super) fn notes_of(text: &str) -> Vec<ItemNote> {
    let text = text.trim();
    if text.is_empty() {
        Vec::new()
    } else {
        vec![ItemNote { text: text.to_string(), on: None }]
    }
}

/// `answer_items` for these items, each with its own notes, in the order given.
pub(super) fn answer_batch(items: &[&Item], how: ItemHow, notes: impl Fn(&Item) -> String) -> AnswerItems {
    AnswerItems {
        items: items
            .iter()
            .map(|item| ItemAnswer { session_id: SessionId::carried(item.job.clone()), item: item.id.clone(), how, notes: notes_of(&notes(item)), extras: Default::default() })
            .collect(),
    }
}

/// `dismiss_items` for these items, in the order given.
pub(super) fn dismiss_batch(items: &[&Item]) -> DismissItems {
    DismissItems { items: items.iter().map(|item| ItemRef { session_id: SessionId::carried(item.job.clone()), item: item.id.clone() }).collect() }
}

/// Fleet's outcomes put on the rows: a settled item keeps `verdict`, a refused one asks again with Fleet's
/// sentence on its row.
pub(super) fn settle_rows(rows: &mut HashMap<String, Row>, outcomes: &[ItemOutcome], verdict: Verdict) {
    for outcome in outcomes {
        let row = rows.entry(key_of(outcome.session_id.as_str(), &outcome.item)).or_default();
        match &outcome.outcome {
            ItemSettled::Settled => *row = Row { verdict: Some(verdict), refused: None },
            ItemSettled::Refused { message, .. } => *row = Row { verdict: None, refused: Some(message.clone()) },
        }
    }
}

fn note_key(job: &str, item: &str) -> String {
    format!("review:{job}:{item}:note")
}

/// Whether a press answers the chosen item or every item still asking.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Send {
    One,
    All,
}

impl JobView {
    pub(super) fn review(&self) -> Option<&ReviewState> {
        self.shown.as_ref().and_then(|id| self.pages.get(id)).map(|page| &page.review)
    }

    pub(super) fn review_mut(&mut self) -> Option<&mut ReviewState> {
        let id = self.shown.clone()?;
        self.pages.get_mut(&id).map(|page| &mut page.review)
    }

    pub(super) fn review_open(&self) -> bool {
        self.review().is_some_and(|review| review.open)
    }

    /// Whether the Review panel is up, for a screen that draws this view under its own to host it.
    pub(crate) fn review_up(&self) -> bool {
        self.review_open()
    }

    /// The card whose open box holds the keys, by the key its ⌘↵ is kept under.
    fn focused_card_box(&self, window: &Window, cx: &gpui::App) -> Option<String> {
        let page = self.shown.as_ref().and_then(|id| self.pages.get(id))?;
        page.cards.boxes.iter().find(|(_, area)| area.read(cx).focus_handle(cx).contains_focused(window, cx)).and_then(|(key, _)| key.rsplit_once(':').map(|(card, _)| card.to_string()))
    }

    fn review_noting(&self, window: &Window, cx: &gpui::App) -> bool {
        self.review().and_then(|review| review.note.as_ref()).is_some_and(|note| note.area.read(cx).focus_handle(cx).contains_focused(window, cx))
    }

    /// The items Job `id` has for the sheet now, oldest first.
    pub(super) fn review_items(&self, id: &str, cx: &gpui::App) -> Vec<Item> {
        let (Some(record), Some(held)) = (self.record(id, cx), self.thread(id, cx)) else { return Vec::new() };
        let has_groups = record.plan.as_ref().is_some_and(|plan| !plan.groups.is_empty());
        let mut items = build(id, &record.waiting_for, &held.head, &held.items, has_groups);
        // A waiting row the thread holds no card for, a takes part of a sketch among them, stands for the artifact it names.
        for one in record.waiting_for.iter().filter(|one| matches!(one.source, WaitingSource::Sketch | WaitingSource::Document | WaitingSource::Draft)) {
            if items.iter().any(|item| item.id == one.id) {
                continue;
            }
            if let Some(mut item) = self.waiting_artifact_item(id, one) {
                item.id = one.id.clone();
                item.since = one.since.as_str().to_string();
                items.push(item);
            }
        }
        items.sort_by_key(|item| instant(&item.since));
        items
    }

    /// Whether the sheet has items of this Job to put to the person.
    pub(super) fn review_applies(&self, id: &str, cx: &gpui::App) -> bool {
        !self.review_items(id, cx).is_empty()
    }

    /// Every Job's items for the sheet, grouped, the Job `first` heading them.
    fn review_groups(&self, first: &str, opened: Option<&str>, scope: &Scope, cx: &gpui::App) -> Vec<Group> {
        let list = self.list.read(cx);
        // The Operator is no Job, but the pages it keeps for him are read here as a Job's are.
        let operator = list.operator_record().map(|record| record.id.as_str());
        let groups = list
            .records()
            .iter()
            .chain(list.operator_record())
            .filter(|record| match scope {
                Scope::Job(job) => record.id.as_str() == job,
                Scope::All => true,
            })
            .map(|record| {
                let job = record.id.as_str();
                let mut items = self.review_items(job, cx);
                if operator == Some(job) {
                    items.retain(|item| page_of_item(&item.id).is_some());
                    return Group { job: job.to_string(), title: "Operator".into(), state: None, items };
                }
                // The shown Job's artifacts of parts that still ask stand with what else waits on him, and the one he opened.
                if job == first {
                    items.extend(self.artifact_items(job, opened));
                    // A document he opened is read in the panel, with the item that waits on him for it, else one of its own.
                    if let Some(page) = opened.filter(|page| !items.iter().any(|item| page_of_item(&item.id) == Some(*page))) {
                        items.extend(self.document_item(job, page, cx));
                    }
                }
                // One item per artifact: a waiting row naming it already stands for it, so a second is dropped.
                let mut seen = std::collections::HashSet::new();
                items.retain(|item| super::reader_sheet::artifact_in(item).is_none_or(|artifact| seen.insert(artifact.to_string())));
                // A sketch, page or draft card and the artifact of parts it is are one entry too, and the artifact stands:
                // the card holds only its drawing, the artifact every part, a pick among them.
                let twins: Vec<String> = items
                    .iter()
                    .filter_map(|item| {
                        let artifact = super::reader_sheet::artifact_in(item)?;
                        let answered_by = self.artifact_of(job, artifact).and_then(|one| one.item.as_deref());
                        let card = items.iter().find(|other| other.kind != Kind::Artifact && (other.id == artifact || Some(other.id.as_str()) == answered_by))?;
                        Some(card.id.clone())
                    })
                    .collect();
                items.retain(|item| !twins.contains(&item.id));
                items.sort_by_key(|item| instant(&item.since));
                Group { job: job.to_string(), title: record.title.clone().unwrap_or_else(|| "New Job".into()), state: record.mark.map(|mark| mark.state), items }
            })
            .collect();
        grouped(first, groups)
    }

    /// The Jobs with something the sheet could hold whose thread is not read yet: their cards are what
    /// the sheet draws, so each is read, and its group comes in when it lands.
    fn review_read_threads(&mut self, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else { return };
        let list = self.list.read(cx);
        let wanted: Vec<String> = list
            .records()
            .iter()
            .chain(list.operator_record())
            .filter(|record| record.waiting_for.iter().any(|one| reviewable(one.source)))
            .map(|record| record.id.as_str().to_string())
            .filter(|id| self.thread(id, cx).is_none())
            .collect();
        fleet.update(cx, |fleet, cx| {
            for id in &wanted {
                fleet.read_thread(id, false, cx);
            }
        });
    }

    // ---- opening and closing ---------------------------------------------------------------

    /// The Review row, the band with nothing else on it and `⇧⌘Y`: a page that is all the Job owes is
    /// read in its own tab, one beside other owed things on its card, and any other set opens the sheet.
    pub(super) fn review_press(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.review_press_item(None, window, cx);
    }

    /// The same, from a row of the To review section: the sheet opens with that row's item chosen.
    pub(super) fn review_press_item(&mut self, item: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let items = self.review_items(&id, cx);
        if let Some(one) = lone_page(&items).map(|one| one.id.clone()) {
            self.set_needs(false, None, window, cx);
            match self.sole_document(&id, cx) {
                Some(page) if page_of_item(&one) == Some(page.as_str()) => self.open_document(&page, window, cx),
                _ => self.go_to_card(&id, &one, cx),
            }
            return;
        }
        match item {
            Some(item) => self.open_review_on(item, window, cx),
            None => self.open_review(window, cx),
        }
    }

    /// The thread comes to the card of the page owed, where the page is not all the Job owes.
    pub(super) fn go_to_card(&mut self, id: &SharedString, waiting: &str, cx: &mut Context<Self>) {
        let at = self.thread(id, cx).and_then(|held| {
            let page = page_of_item(waiting)?;
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

    /// The sheet's choice moves to the shown Job's item `item`, as a press on its row does.
    pub(super) fn review_at(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        self.review_choose(&key_of(&id, item), window, cx);
    }

    /// `open_over`: the Job the item waits in is shown, the panel opens, and the item is chosen with its read kept beside it.
    fn open_review_for(&mut self, item: ItemRef, read: Option<OperatorRead>, window: &mut Window, cx: &mut Context<Self>) {
        let job = SharedString::from(item.session_id.as_str().to_string());
        self.review_from = window.focused(cx);
        // The Job comes on screen now, not when the list's change is heard, so the panel opens on it; and as no pick, so its box
        // does not take the keys from the screen the press came from.
        if self.shown.as_ref() != Some(&job) {
            self.list.update(cx, |list, cx| list.open_quietly(Some(job.clone()), cx));
            self.followed(window, cx);
        }
        let key = key_of(&job, &item.item);
        self.operator_reads.clear();
        // From a screen that is no Job's the list is every Job's, grouped by Job.
        self.open_review_with(Scope::All, None, Some(&item.item), window, cx);
        if let Some(read) = read.filter(|_| self.review().is_some_and(|review| review.item(&key).is_some())) {
            self.operator_reads.insert(key, read);
        }
        cx.notify();
    }

    pub(super) fn open_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_review_at(None, window, cx);
    }

    /// A press on one of the Job's To review rows: the panel lists this Job's items alone, with the pressed one chosen.
    pub(super) fn open_review_on(&mut self, item: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        self.open_review_with(Scope::Job(id.to_string()), None, Some(item), window, cx);
    }

    /// The sheet over the window, with artifact `artifact` of the shown Job picked where one is named. Where it is up already
    /// the choice moves and nothing else does: the place he came from is kept.
    pub(super) fn open_review_at(&mut self, artifact: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        self.open_review_with(Scope::Job(id.to_string()), artifact, None, window, cx);
    }

    /// The panel with `scope`'s items, and either an artifact or the Job's waiting item `pick` chosen. Where it is up already
    /// the item it holds is replaced and nothing else moves: it never opens a second, and the place he came from is kept. An
    /// item `pick` the list lacks opens the artifact it names instead, so the list never lacks what he pressed.
    pub(super) fn open_review_with(&mut self, scope: Scope, artifact: Option<&str>, pick: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.review_open() {
            if let Some(review) = self.review_mut() {
                review.opened = artifact.map(str::to_string).or(review.opened.take());
            }
            self.review_sync(window, cx);
            let standing = artifact.and_then(|artifact| self.review().and_then(|review| standing_for(&review.groups, &id, artifact)));
            if let Some(key) = standing.or_else(|| artifact.map(|artifact| self.opened_key(&id, artifact, cx))).or_else(|| pick.map(|pick| key_of(&id, pick))) {
                self.review_choose(&key, window, cx);
            }
            return;
        }
        let groups = self.review_groups(&id, artifact, &scope, cx);
        let missing = pick.filter(|pick| !groups.iter().any(|group| group.job == id.as_ref() && group.items.iter().any(|item| item.id == *pick)));
        if let Some(pick) = missing {
            // The item is in no list: its artifact is read in the panel, if it names one the Job holds.
            let named = self.record(&id, cx).and_then(|record| record.waiting_for.iter().find(|one| one.id == pick).and_then(|one| one.artifact.clone()));
            if let Some(named) = named {
                if self.artifact_of(&id, &named).is_some() {
                    self.open_review_with(scope, Some(&named), None, window, cx);
                    return;
                }
            }
        }
        if groups.is_empty() {
            return;
        }
        self.review_read_threads(cx);
        // The Needs you panel shuts as the sheet opens, so the keys do not go back to it: the thread has them.
        let back_to = self.review_from.take().or_else(|| window.focused(cx)).filter(|held| !self.dock_focus.contains_focused(window, cx) && *held != self.dock_focus);
        if let Some(dock) = self.dock_mut() {
            dock.open = Some(false);
            dock.detail = None;
        }
        // Each Job drawn on the sheet has a page, which its plan and cards read their state from.
        for group in &groups {
            self.pages.entry(SharedString::from(group.job.clone())).or_insert_with(Page::new);
        }
        let first = match (artifact, pick) {
            (Some(artifact), _) => Some(standing_for(&groups, &id, artifact).unwrap_or_else(|| self.opened_key(&id, artifact, cx))),
            (None, Some(pick)) => Some(key_of(&id, pick)),
            (None, None) => groups.first().and_then(|group| group.items.first()).map(Item::key),
        };
        let Some(page) = self.pages.get_mut(&id) else { return };
        let review = &mut page.review;
        *review = ReviewState::default();
        review.open = true;
        review.opened = artifact.map(str::to_string);
        review.scope = scope;
        review.live = order(&groups).into_iter().collect();
        review.groups = groups;
        review.back_to = back_to;
        if let Some(first) = first.as_deref() {
            self.review_choose(first, window, cx);
        }
        window.focus(&self.review_focus.list, cx);
        // An artifact's parts hold the keys of their own, a number for a take, once they are drawn.
        if let Some(item) = first.as_deref().and_then(|key| self.review().and_then(|review| review.item(key)).cloned()).filter(|item| item.kind == Kind::Artifact) {
            cx.spawn_in(window, async move |this, cx| {
                this.update_in(cx, |this, window, cx| this.review_parts_focus(&item, window, cx)).ok();
            })
            .detach();
        }
        cx.notify();
    }

    pub(crate) fn close_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        self.detail_zoom = None;
        self.operator_reads.clear();
        match back {
            Some(handle) => window.focus(&handle, cx),
            None => window.focus(&self.focus, cx),
        }
        cx.notify();
    }

    /// Fleet moved: the items it lists now replace what the sheet drew, a Job whose thread came in adds its
    /// group, and an item that went without his answer here reads Answered.
    pub(super) fn review_sync(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if !self.review_open() {
            return;
        }
        let opened = self.review().and_then(|review| review.opened.clone());
        let scope = self.review().map(|review| review.scope.clone()).unwrap_or_default();
        let now = self.review_groups(&id, opened.as_deref(), &scope, cx);
        for group in &now {
            self.pages.entry(SharedString::from(group.job.clone())).or_insert_with(Page::new);
        }
        let Some(review) = self.review_mut() else { return };
        review.live = order(&now).into_iter().collect();
        for fresh in now {
            match review.groups.iter_mut().find(|held| held.job == fresh.job) {
                Some(held) => {
                    held.title = fresh.title;
                    held.state = fresh.state;
                    for item in fresh.items {
                        match held.items.iter().position(|one| one.id == item.id) {
                            Some(at) => held.items[at] = item,
                            None => held.items.push(item),
                        }
                    }
                }
                None => review.groups.push(fresh),
            }
        }
        let gone: Vec<String> = review.items().filter(|item| !item.kind.answers_inside()).map(Item::key).filter(|key| !review.live.contains(key)).collect();
        for key in gone {
            let row = review.rows.entry(key).or_default();
            if row.verdict.is_none() {
                *row = Row { verdict: Some(Verdict::Answered), refused: None };
            }
        }
        cx.notify();
    }

    /// Choose item `key`: its notes come up with what was typed in them before.
    fn review_choose(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.review().and_then(|review| review.item(key)).cloned() else { return };
        let held = self.review().and_then(|review| review.note.as_ref()).is_some_and(|note| note.key == key);
        if !held && !item.kind.answers_inside() {
            let kept_key = note_key(&item.job, &item.id);
            let kept = self.draft_words(&kept_key);
            let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 10));
            area.update(cx, |area, cx| area.set_value(kept, window, cx));
            let typed = cx.subscribe_in(&area, window, move |this, area, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    let text = area.read(cx).value().to_string();
                    this.keep_words(&kept_key, &text, cx);
                    cx.notify();
                }
            });
            if let Some(review) = self.review_mut() {
                review.note = Some(NoteBox { key: key.to_string(), area, _typed: typed });
            }
        }
        if let Some(review) = self.review_mut() {
            review.at = Some(key.to_string());
        }
        // A drawing in the detail starts at fit for each item chosen.
        self.detail_zoom = Some(super::reader_sheet::DetailZoom::new());
        cx.notify();
    }

    /// Where an item stands as its row reads it: an artifact of parts stands where its parts put it, and anything else where the
    /// sheet's own answers did.
    fn item_row(&self, item: &Item, review: &ReviewState, cx: &gpui::App) -> Row {
        if item.kind == Kind::Build {
            return Row { verdict: self.build_verdict(item, cx), refused: None };
        }
        match self.artifact_verdict(item) {
            Some(verdict) => Row { verdict, refused: None },
            None => review.row(&item.key()),
        }
    }

    /// Where a build stands as its card does: asking while it is open or put off, else what he said or what Fleet did to it. His
    /// press counts in the frame it is made, before Fleet's card says so.
    fn build_verdict(&self, item: &Item, cx: &gpui::App) -> Option<Verdict> {
        let page = self.pages.get(item.job.as_str())?;
        let held = self.thread(&item.job, cx)?;
        let card = held.items.iter().find_map(|one| match one {
            ThreadItem::ReadyToTry(card) if card.id == item.id => Some(card),
            _ => None,
        })?;
        match super::ready_card::Standing::of(card, page.cards.answers.get(&card.id)) {
            super::ready_card::Standing::Open | super::ready_card::Standing::Later => None,
            super::ready_card::Standing::Good => Some(Verdict::Approved),
            super::ready_card::Standing::Rework => Some(Verdict::ChangesAsked),
            super::ready_card::Standing::Withdrawn | super::ready_card::Standing::Replaced => Some(Verdict::Answered),
        }
    }

    /// The chosen item, where the sheet is up.
    pub(super) fn review_chosen(&self) -> Option<Item> {
        self.review_current().map(|(item, _)| item)
    }

    fn review_current(&self) -> Option<(Item, Row)> {
        let review = self.review()?;
        let at = review.at.as_ref()?;
        Some((review.item(at)?.clone(), review.row(at)))
    }

    fn review_notes_text(&self, item: &Item, cx: &gpui::App) -> String {
        let held = self.review().and_then(|review| review.note.as_ref()).filter(|note| note.key == item.key());
        match held {
            Some(note) => note.area.read(cx).value().to_string(),
            None => self.draft_words(&note_key(&item.job, &item.id)),
        }
    }

    /// The items still asking, in the list's order.
    fn review_asking(&self) -> Vec<Item> {
        let Some(review) = self.review() else { return Vec::new() };
        review.items().filter(|item| !item.read && !item.kind.answers_inside() && review.row(&item.key()).verdict.is_none()).cloned().collect()
    }

    // ---- answers ---------------------------------------------------------------------------

    /// `A`: Approve the chosen item, with its notes.
    fn review_approve(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((item, row)) = self.review_current() else { return };
        if row.verdict.is_some() || item.read || item.kind.answers_inside() || !self.item_asks(&item, cx) {
            return;
        }
        let body = answer_batch(&[&item], ItemHow::Approve, |one| self.review_notes_text(one, cx));
        self.review_send(Send::One, vec![item], Verdict::Approved, "/sessions/items/answer", ipc::encode(&body), window, cx);
    }

    /// `C`: send the chosen item back with its notes. With none typed the caret goes to its notes, since a
    /// send back with no note is refused.
    fn review_changes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((item, row)) = self.review_current() else { return };
        if row.verdict.is_some() || !item.send_back || item.read || !self.item_asks(&item, cx) {
            return;
        }
        if notes_of(&self.review_notes_text(&item, cx)).is_empty() {
            self.review_focus_notes(window, cx);
            return;
        }
        let body = answer_batch(&[&item], ItemHow::SendBack, |one| self.review_notes_text(one, cx));
        self.review_send(Send::One, vec![item], Verdict::ChangesAsked, "/sessions/items/answer", ipc::encode(&body), window, cx);
    }

    /// `D`: the chosen item off his list, with no verdict.
    fn review_dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((item, row)) = self.review_current() else { return };
        if item.kind == Kind::Artifact {
            self.review_parts_dismiss(&item, window, cx);
            return;
        }
        if row.verdict.is_some() || item.read || item.kind == Kind::Build {
            return;
        }
        let body = dismiss_batch(&[&item]);
        self.review_send(Send::One, vec![item], Verdict::Dismissed, "/sessions/items/dismiss", ipc::encode(&body), window, cx);
    }

    /// `⇧A`: every item still asking, approved in one call, each with its own notes.
    fn review_approve_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let asking = self.review_asking();
        if asking.is_empty() {
            return;
        }
        let refs: Vec<&Item> = asking.iter().collect();
        let body = answer_batch(&refs, ItemHow::Approve, |one| self.review_notes_text(one, cx));
        self.review_send(Send::All, asking, Verdict::Approved, "/sessions/items/answer", ipc::encode(&body), window, cx);
    }

    /// `⇧D`: every item still asking, dismissed in one call.
    fn review_dismiss_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let asking = self.review_asking();
        if asking.is_empty() {
            return;
        }
        let refs: Vec<&Item> = asking.iter().collect();
        let body = dismiss_batch(&refs);
        self.review_send(Send::All, asking, Verdict::Dismissed, "/sessions/items/dismiss", ipc::encode(&body), window, cx);
    }

    /// The rows take the verdict in the frame of the press, and Fleet's outcomes put right any it refused.
    #[allow(clippy::too_many_arguments)]
    fn review_send(
        &mut self,
        send: Send,
        items: Vec<Item>,
        verdict: Verdict,
        route: &'static str,
        body: Result<String, ipc::Unencodable>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keys: Vec<String> = items.iter().map(Item::key).collect();
        let from = self.review().and_then(|review| review.at.clone());
        if let Some(review) = self.review_mut() {
            for key in &keys {
                review.rows.insert(key.clone(), Row { verdict: Some(verdict), refused: None });
            }
        }
        // A sent verdict's notes are spent: the box is empty for the next time.
        if verdict != Verdict::Dismissed {
            for item in &items {
                self.keep_words(&note_key(&item.job, &item.id), "", cx);
            }
            if let Some(review) = self.review_mut() {
                if review.note.as_ref().is_some_and(|note| keys.contains(&note.key)) {
                    review.note = None;
                }
            }
        }
        if send == Send::One {
            self.review_advance(from.as_deref(), window, cx);
        } else if let Some(at) = from {
            self.review_choose(&at, window, cx);
        }
        if self.review_open() {
            window.focus(&self.review_focus.list, cx);
        }
        cx.notify();
        let landed = move |this: &mut JobView, answer: Result<Vec<u8>, String>, _window: &mut Window, cx: &mut Context<JobView>| {
            let outcomes = answer.and_then(|bytes| ipc::decode::<ItemsSettled>("ItemsSettled", &bytes).map(|settled| settled.outcomes).map_err(|why| why.to_string()));
            this.review_landed(&keys, verdict, outcomes, cx);
        };
        if self.fleet.is_none() {
            // Mock data: Fleet's outcomes as the scenario has them.
            let settled = body.map_err(|why| format!("{why:?}")).and_then(|body| crate::mock::items_settled(route, &body));
            let bytes = settled.and_then(|settled| ipc::encode(&settled).map(String::into_bytes).map_err(|why| format!("{why:?}")));
            landed(self, bytes, window, cx);
            return;
        }
        self.post(route.to_string(), body, window, cx, Box::new(landed));
    }

    /// Fleet answered: each row as its outcome says, or every row of the press back to asking with the
    /// reason where the call itself failed.
    fn review_landed(&mut self, keys: &[String], verdict: Verdict, outcomes: Result<Vec<ItemOutcome>, String>, cx: &mut Context<Self>) {
        let Some(review) = self.review_mut() else { return };
        match outcomes {
            Ok(outcomes) => settle_rows(&mut review.rows, &outcomes, verdict),
            Err(why) => {
                for key in keys {
                    review.rows.insert(key.clone(), Row { verdict: None, refused: Some(why.clone()) });
                }
            }
        }
        cx.notify();
    }

    /// After one item is answered the next open one comes up, and the last answer closes the sheet.
    fn review_advance(&mut self, from: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = self.review() else { return };
        // An artifact is answered by its parts and never by this: it is no next to come up.
        let keys: Vec<String> = order(&review.groups).into_iter().filter(|key| review.item(key).is_some_and(|item| !item.kind.answers_inside())).collect();
        match next_open(&keys, &review.rows, from.unwrap_or_default()) {
            Some(to) => self.review_choose(&to, window, cx),
            None => self.close_review(window, cx),
        }
    }

    fn review_focus_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(area) = self.review().and_then(|review| review.note.as_ref()).map(|note| note.area.clone()) {
            area.update(cx, |area, cx| area.focus(window, cx));
        }
        cx.notify();
    }

    // ---- keys ------------------------------------------------------------------------------

    /// What the sheet does with the thread's acts while it is up. True where it took the act.
    pub(super) fn review_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.review_open() {
            return false;
        }
        // ⌘↵ reaches the sheet as Send: in an item's notes it asks for changes with them.
        if act.is("send_message") {
            if self.review_noting(window, cx) {
                self.review_changes(window, cx);
            } else if let Some(key) = self.focused_card_box(window, cx) {
                // A part's Ask for changes box in the panel sends as its Send does.
                self.commit_box(&key, window, cx);
            }
            return true;
        }
        // The cards behind take none of the thread's keys while the sheet is up.
        act.is("send_message") || act.is("thread_send_back") || act.is("thread_yes") || act.is("thread_change") || act.is("thread_no") || act.is("needs_you") || act.is("needs_you_fold")
    }

    /// The registry's `review sheet` keys.
    fn review_sheet_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) {
        match act.id {
            "items_approve" => self.review_approve(window, cx),
            "items_ask_changes" => self.review_changes(window, cx),
            "items_dismiss" => self.review_dismiss(window, cx),
            "items_approve_all" => self.review_approve_all(window, cx),
            "items_dismiss_all" => self.review_dismiss_all(window, cx),
            // j, k, ↓, ↑ in the registry's order: the even slots move down.
            "items_move" => {
                self.review_step(if act.slot % 2 == 0 { "down" } else { "up" }, window, cx);
                window.focus(&self.review_focus.list, cx);
            }
            _ => cx.propagate(),
        }
    }

    fn review_step(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = self.review() else { return };
        let keys = order(&review.groups);
        let here = review.at.as_ref().and_then(|at| keys.iter().position(|key| key == at));
        if let Some(to) = step_to(keys.len(), here, key) {
            if matches!(key, "up" | "down") && Some(to) != here {
                crate::ui_sounds::moved();
            }
            self.review_choose(&keys[to], window, cx);
        }
    }

    /// Esc and Tab, caught before anything inside the sheet hears them.
    fn review_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            // One Esc leaves from any depth, and the keys go back to what opened the sheet. The picture zoomer, drawn over it
            // with its own focus, hears its Esc first.
            "escape" => {
                // A box he is typing in closes alone, its words kept: the panel stays for the next Esc.
                if !self.review_close_box(window, cx) {
                    self.close_review(window, cx);
                }
            }
            "tab" => self.review_tab(event.keystroke.modifiers.shift, window, cx),
            _ => {
                let noting = self.review_noting(window, cx);
                if self.review_zoom_key(event, noting, window, cx) {
                    cx.stop_propagation();
                }
                return;
            }
        }
        cx.stop_propagation();
    }

    /// Esc in the Notes box or in a box of the chosen artifact's parts: the box gives up the keys, the words stay kept in it, and
    /// where an Ask for changes box is open it closes. False where he is in no box.
    fn review_close_box(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.review_noting(window, cx) {
            window.focus(&self.review_focus.list, cx);
            return true;
        }
        let Some(artifact) = self.review_chosen().and_then(|item| super::reader_sheet::artifact_in(&item).map(str::to_string)) else { return false };
        if !self.parts_typing(&artifact, window, cx) {
            return false;
        }
        self.close_parts_boxes(&artifact, window, cx);
        true
    }

    /// Tab stays inside the sheet: it goes round its stops and never out of them.
    fn review_tab(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let focus = &self.review_focus;
        let current = self.review_current();
        let open = current.as_ref().is_some_and(|(item, row)| row.verdict.is_none() && !item.kind.answers_inside());
        let mut stops: Vec<FocusHandle> = vec![focus.list.clone()];
        if open && !current.as_ref().is_some_and(|(item, _)| item.read) {
            stops.push(focus.dismiss.clone());
            if current.as_ref().is_some_and(|(item, _)| item.send_back) {
                stops.push(focus.changes.clone());
            }
            stops.push(focus.approve.clone());
            if let Some(note) = self.review().and_then(|review| review.note.as_ref()) {
                stops.push(note.area.read(cx).focus_handle(cx));
            }
        }
        stops.push(focus.close.clone());
        if !self.review_asking().is_empty() {
            stops.push(focus.approve_all.clone());
            stops.push(focus.more_all.clone());
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

    /// The list's own keys: Home and End go to an end, and ↑ and ↓ a step where the registry's keys are not heard.
    fn review_list_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let mods = event.keystroke.modifiers;
        if mods.alt || mods.platform || mods.control || mods.shift {
            return;
        }
        let key = event.keystroke.key.as_str();
        if matches!(key, "home" | "end" | "up" | "down") {
            self.review_step(key, window, cx);
            cx.stop_propagation();
        }
    }

    /// Shift+F10 and the menu key, on the list: the chosen item's menu, under its row.
    pub(super) fn review_menu_key(&mut self, _: &ItemMenu, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = self.review().and_then(|review| review.at.clone()) else { return };
        let Some(bounds) = self.review_rows.borrow().get(&at).copied() else { return };
        self.review_menu(&at, under(bounds), true, window, cx);
    }

    /// The item's menu at `at`: its answers, as the band's buttons.
    fn review_menu(&mut self, key: &str, at: gpui::Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = self.review() else { return };
        let Some(item) = review.item(key).cloned() else { return };
        if review.row(key).verdict.is_some() || item.kind.answers_inside() || item.read {
            return;
        }
        let mut entries = vec![Entry::Act { id: "approve", label: "Approve".into(), icon: Some("check"), danger: false }];
        if item.send_back {
            entries.push(Entry::Act { id: "changes", label: "Ask for changes".into(), icon: Some("pencil"), danger: false });
        }
        if !item.read {
            entries.push(Entry::Act { id: "dismiss", label: "Dismiss".into(), icon: Some("x"), danger: false });
        }
        let back = Some(self.review_focus.list.clone());
        self.open_pop_back(
            "Item",
            entries,
            at,
            from_keys,
            back,
            move |this, id, window, cx| match id {
                "approve" => this.review_approve(window, cx),
                "changes" => this.review_changes(window, cx),
                "dismiss" => this.review_dismiss(window, cx),
                _ => {}
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
        let room = crate::room::of(window, cx);
        let viewport = room.size;
        // The sheet takes the whole window, `--space-6` from each edge over the scrim.
        let margin = look.length(token::SPACE_6);
        let wide = (viewport.width - margin * 2.).max(px(0.));
        let tall = (viewport.height - look.length(token::H_TITLE_BAR) - margin * 2.).max(px(0.));
        // The list keeps the look's width where the sheet has room, and gives some up where it has not.
        let roomy = wide >= look.length(token::SPACE_12) * 19.;
        let list_w = if roomy { look.length(token::SPACE_12) * 7. } else { look.length(token::SPACE_12) * 5. + look.length(token::SPACE_6) };
        let current = review.at.as_ref().and_then(|at| review.item(at)).cloned();

        let mut rows: Vec<AnyElement> = Vec::new();
        for group in &review.groups {
            rows.push(self.review_group_head(group, look));
            rows.extend(group.items.iter().map(|item| self.review_row(item, review, look, cx)));
        }
        let list = labelled(
            div()
                .id("review-list")
                .track_focus(&self.review_focus.list)
                .on_action(cx.listener(Self::review_menu_key))
                .on_key_down(cx.listener(Self::review_list_key))
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_3))
                .overflow_y_scroll()
                .children(rows),
            Role::Group,
            "Waiting on you",
        );
        // The rows scroll above the all-items foot, which stays put under them.
        let column = div()
            .id("review-column")
            .border_r(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .flex_none()
            .w(list_w)
            .flex()
            .flex_col()
            .bg(look.color(token::BG_SUNKEN))
            .child(list)
            .child(self.review_list_foot(review, look, cx));

        // What a picture and a row of takes are sized to: the detail's own room.
        self.sheet.set(gpui::size((wide - list_w - look.length(token::BORDER_WIDTH) * 2.).max(px(1.)), tall));
        let detail = current.as_ref().map(|item| self.review_detail(item, &self.item_row(item, review, cx), review, wide - list_w, look, cx));
        let foot = self.review_foot(current.as_ref(), look, cx);
        let sheet = labelled(
            div()
                .id("review-sheet")
                .key_context(CONTEXT)
                .on_action(cx.listener(Self::review_sheet_act))
                .capture_key_down(cx.listener(Self::review_key))
                .occlude()
                .w(wide)
                .h(tall)
                .mt(look.length(token::H_TITLE_BAR) + margin)
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG) * 1.5)
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .child(div().flex_1().min_h_0().flex().child(column).children(detail))
                .child(foot),
            Role::Dialog,
            "Review",
        );
        Some(
            deferred(
                anchored().position(room.origin).anchor(Anchor::TopLeft).child(
                    // A press on the ground behind takes nothing: Close and Esc are the two exits.
                    div().id("review-scrim").occlude().w(viewport.width).h(viewport.height).flex().justify_center().bg(look.color(token::SCRIM)).child(sheet),
                ),
            )
            .with_priority(0)
            .into_any_element(),
        )
    }

    /// A Job's head in the list: its state's glyph and its name.
    fn review_group_head(&self, group: &Group, look: &Look) -> AnyElement {
        let state = group.state.map(|state| {
            let (icon, said) = crate::jobs::glyph_of(state, None);
            labelled(
                div()
                    .id(SharedString::from(format!("review-job-state:{}", group.job)))
                    .flex_none()
                    .tooltip(tip(said.clone()))
                    .child(glyph(look, icon, Size::Mark, look.color(crate::jobs::hue_of(state)))),
                Role::Image,
                said,
            )
        });
        labelled(
            div()
                .id(SharedString::from(format!("review-job:{}", group.job)))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_2))
                .pt(look.length(token::SPACE_3))
                .pb(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_SUBTLE))
                .children(state)
                .child(div().flex_1().min_w_0().truncate().child(SharedString::from(group.title.clone()))),
            Role::Heading,
            group.title.clone(),
        )
        .into_any_element()
    }

    /// The kind's tile: its glyph on its hue, as the look's kind-glyph.
    fn review_kind_tile(&self, key: &str, kind: Kind, look: &Look) -> AnyElement {
        let (icon, said, hue) = kind.mark();
        labelled(
            div()
                .id(SharedString::from(format!("review-kind:{key}")))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM) + look.length(token::SPACE_1) / 2.)
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(hue).opacity(0.18))
                .tooltip(tip(said))
                .child(glyph(look, icon, Size::Small, look.color(hue))),
            Role::Image,
            said,
        )
        .into_any_element()
    }

    /// The chip that says where an item stands: Asking, or its verdict.
    fn review_chip(&self, key: &str, row: &Row, look: &Look) -> AnyElement {
        let (icon, said, hue) = match row.verdict {
            Some(verdict) => verdict.mark(),
            None => ("circle-dot", "Asking", token::STATUS_AWAITING_REVIEW),
        };
        labelled(
            div()
                .id(SharedString::from(format!("review-chip:{key}")))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_2))
                .py(px(2.))
                .rounded_full()
                .bg(look.color(hue).opacity(0.16))
                .text_color(look.color(hue))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .tooltip(tip(said))
                .child(glyph(look, icon, Size::Mark, look.color(hue)))
                .child(said),
            Role::Image,
            said,
        )
        .into_any_element()
    }

    /// One item of the list: its kind, title, For line and where it stands; an answered one dims.
    fn review_row(&self, item: &Item, review: &ReviewState, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = item.key();
        let row = self.item_row(item, review, cx);
        let on = review.at.as_deref() == Some(key.as_str());
        let (pick, menu_item, kept) = (key.clone(), key.clone(), key.clone());
        let rows = Rc::clone(&self.review_rows);
        let body = div()
            .id(SharedString::from(format!("review-item:{key}")))
            .relative()
            .flex_none()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if on { look.color(token::BORDER_STRONG) } else { gpui::transparent_black() })
            .when(on, |row| row.bg(look.color(token::BG_RAISED)).shadow(look.shadows(token::SHADOW_CARD)))
            .when(!on, |row| row.hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER))))
            .when(row.verdict.is_some(), |row| row.opacity(0.72))
            .cursor_pointer()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(self.review_kind_tile(&key, item.kind, look))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(item.name.clone())),
                    )
                    .child(self.review_chip(&key, &row, look)),
            )
            .children(item.for_line.clone().map(|line| {
                div()
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_MUTED))
                    .line_clamp(2)
                    .child(SharedString::from(line))
            }))
            .children(row.refused.clone().map(|why| {
                labelled(
                    div()
                        .id(SharedString::from(format!("review-refused:{key}")))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                        .child(SharedString::from(why.clone())),
                    Role::Status,
                    why,
                )
            }))
            .child(canvas(move |bounds, _, _| { rows.borrow_mut().insert(kept.clone(), bounds); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            .on_click(cx.listener(move |this, _, window, cx| {
                this.review_choose(&pick, window, cx);
                window.focus(&this.review_focus.list, cx);
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                    this.review_choose(&menu_item, window, cx);
                    this.review_menu(&menu_item, event.position, false, window, cx);
                }),
            );
        labelled(body, Role::Button, item.name.clone()).into_any_element()
    }

    /// A button of the band or the foot, with its key, its focus and its press. `fill` paints it in a
    /// state's hue, and `danger` writes a ghost's words in the killed hue.
    #[allow(clippy::too_many_arguments)]
    fn review_button(
        &self,
        look: &Look,
        id: &'static str,
        said: &'static str,
        tone: Tone,
        fill: Option<Token>,
        danger: bool,
        chip: &'static str,
        focus: &FocusHandle,
        on: bool,
        press: fn(&mut JobView, &mut Window, &mut Context<JobView>),
        cx: &Context<Self>,
    ) -> AnyElement {
        let base = button(look, id, said, tone, false, None, on)
            .track_focus(focus)
            .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
            .when_some(fill, |b, hue| b.bg(look.color(hue)).border_color(look.color(hue)).text_color(look.color(token::FG_INVERSE)))
            .when(danger, |b| b.text_color(look.color(token::STATUS_KILLED)))
            .child(kbd(look, chip))
            .when(on, |b| {
                b.on_click(cx.listener(move |this, _, window, cx| press(this, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.platform {
                        press(this, window, cx);
                        cx.stop_propagation();
                    }
                }))
            });
        labelled(base, Role::Button, said).into_any_element()
    }

    /// The chosen item: the reader's band, its body, and its notes beside it, or under it where the sheet is narrow.
    fn review_detail(&self, item: &Item, row: &Row, review: &ReviewState, room: Pixels, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = item.key();
        let open = row.verdict.is_none();
        let hue = match row.verdict {
            None => token::STATUS_AWAITING_REVIEW,
            Some(Verdict::Approved) => token::STATUS_COMPLETED_SUCCESS,
            Some(Verdict::ChangesAsked) => token::STATUS_REJECTED,
            Some(Verdict::Dismissed) => token::STATUS_KILLED,
            Some(Verdict::Answered) => token::STATUS_NOT_STARTED,
        };
        let focus = &self.review_focus;
        let parts = item.kind == Kind::Artifact;
        let close = self.review_button(look, "review-close", "Close", Tone::Ghost, None, false, "Esc", &focus.close, true, Self::close_review, cx);
        let acts = self.review_item_acts(item, row, look, cx);
        let for_line = item.for_line.clone().map(|line| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .gap(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child("For:"))
                .child(div().flex_1().min_w_0().child(SharedString::from(line)))
        });
        let band = labelled(
            div()
                .id(SharedString::from(format!("review-band:{key}")))
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .bg(look.color(hue).opacity(0.18))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(hue).opacity(0.4))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .child(self.review_kind_tile(&format!("band:{key}"), item.kind, look))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(look.length(token::TEXT_BASE))
                                .line_height(look.length(token::LEADING_BASE))
                                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                .text_color(look.color(token::FG_DEFAULT))
                                // A build's own title is drawn in its card below, so the band names what it is.
                                .child(SharedString::from(if item.kind == Kind::Build { item.kind.mark().1.to_string() } else { item.name.clone() })),
                        )
                        .children(item.revision.clone().map(|revision| {
                            div()
                                .id(SharedString::from(format!("review-revision:{key}")))
                                .flex_none()
                                .px(look.length(token::SPACE_1) * 1.5)
                                .py(px(2.))
                                .rounded(look.length(token::RADIUS_SM))
                                .border(look.length(token::BORDER_WIDTH))
                                .border_color(look.color(token::BORDER_DEFAULT))
                                .font_family(super::rows::mono(look))
                                .text_size(look.length(token::TEXT_2XS))
                                .line_height(look.length(token::LEADING_2XS))
                                .text_color(look.color(token::FG_MUTED))
                                .tooltip(tip(format!("Revision {}", revision.trim_start_matches('r'))))
                                .child(SharedString::from(revision))
                        }))
                        .child(self.review_chip(&format!("band:{key}"), row, look))
                        .children(acts)
                        .child(close),
                )
                .children(for_line)
                .children(row.refused.clone().map(|why| {
                    div()
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                        .child(SharedString::from(why))
                })),
            Role::Group,
            "Answers",
        );

        let staged = self.review_stage(item, look, cx).or_else(|| self.review_page_stage(item, cx));
        let make_body = || {
            div()
            .id(SharedString::from(format!("review-body:{key}")))
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .when(parts, |body| body.track_scroll(&self.body_scroll))
            .child(self.review_body(item, room, look, cx))
        };
        let side = room >= look.length(token::SPACE_12) * 11.;
        let notes = (!item.kind.answers_inside()).then(|| self.review_notes(item, review, open, side, look, cx));
        let under = div().flex_1().min_h_0().flex().when(!side, |d| d.flex_col()).child(staged.unwrap_or_else(|| make_body().into_any_element())).children(notes);
        let foot = self.review_parts_foot(item, look, cx);
        labelled(div().id(SharedString::from(format!("review-detail:{key}"))).flex_1().min_w_0().min_h_0().flex().flex_col().child(band).child(under).children(foot), Role::Region, item.name.clone())
            .into_any_element()
    }

    /// Whether the item puts a question to him. A sketch the Job showed with `asks: false` arrives with `asked` false and has no
    /// answer bar, here as on its card (`sketch_card.rs`) and in its reading (`sketch_read.rs`).
    fn item_asks(&self, item: &Item, cx: &gpui::App) -> bool {
        if item.kind != Kind::Sketch {
            return true;
        }
        self.thread(&item.job, cx).and_then(|held| sketch_card(&held.head, &held.items, &item.id).map(|card| card.asked)).unwrap_or(true)
    }

    /// The chosen item's own answers, for its band: none where it is answered, an artifact or a build (each answers in its own
    /// detail) or an item he is only to read, which has Open.
    fn review_item_acts(&self, item: &Item, row: &Row, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if row.verdict.is_some() || item.kind.answers_inside() || item.read || !self.item_asks(item, cx) {
            return None;
        }
        let focus = &self.review_focus;
        let acts = div().flex_none().flex().items_center().gap(look.length(token::SPACE_2));
        Some(
            acts.child(self.review_button(look, "review-dismiss", "Dismiss", Tone::Ghost, None, false, "D", &focus.dismiss, true, Self::review_dismiss, cx))
                .when(item.send_back, |acts| acts.child(self.review_button(look, "review-changes", "Ask for changes", Tone::Secondary, None, false, "C", &focus.changes, true, Self::review_changes, cx)))
                .child(self.review_button(look, "review-approve", "Approve", Tone::Primary, Some(token::STATUS_COMPLETED_SUCCESS), false, "A", &focus.approve, true, Self::review_approve, cx))
                .into_any_element(),
        )
    }

    /// The notes rail: what goes with this item's verdict, typed here and kept under the item.
    fn review_notes(&self, item: &Item, review: &ReviewState, open: bool, side: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let area = review.note.as_ref().filter(|note| note.key == item.key() && open).map(|note| note.area.clone());
        let field = area.map(|area| {
            labelled(
                div()
                    .id("review-note")
                    .key_context(crate::keymap::TEXT_BOX)
                    // ⌘↩ sends it back, taken before the text area puts a line break in.
                    .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                        if action.secondary {
                            cx.stop_propagation();
                            this.review_changes(window, cx);
                        }
                    }))
                    .w_full()
                    .px(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border_l(look.length(token::BORDER_WIDTH) * 3.)
                    .border_color(look.color(token::YOU))
                    .bg(look.color(token::YOU).opacity(0.09))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(Textarea::new(&area).appearance(false).aria_label("Notes")),
                Role::TextInput,
                "Notes",
            )
        });
        div()
            .flex_none()
            .when(side, |d| d.w(look.length(token::SPACE_12) * 4. + look.length(token::SPACE_2)).h_full().border_l(look.length(token::BORDER_WIDTH)))
            .when(!side, |d| d.w_full().border_t(look.length(token::BORDER_WIDTH)))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_SUNKEN))
            .p(look.length(token::SPACE_3))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .children(self.operator_reads.get(&item.key()).map(|read| self.review_operator(read, look)))
            .child(
                div()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child("Notes"),
            )
            .children(field)
            .into_any_element()
    }

    /// What the Operator read of the item, for reading only: its words, and the pick it would make.
    fn review_operator(&self, read: &OperatorRead, look: &Look) -> AnyElement {
        labelled(
            div()
                .id("review-operator")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .pb(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                // Its label and box stand as the Notes under it do.
                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Operator"))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .px(look.length(token::SPACE_2))
                        .py(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .border_l(look.length(token::BORDER_WIDTH) * 3.)
                        .border_color(look.color(token::OPERATOR))
                        .bg(look.color(token::OPERATOR).opacity(0.09))
                        .child(div().child(read.reading.clone()))
                        .children(read.pick.clone().map(|pick| div().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(format!("Would pick: {pick}"))))),
                ),
            Role::Group,
            "Operator",
        )
        .into_any_element()
    }

    /// The body of the chosen item: what the thread's card shows, without its answers.
    fn review_body(&self, item: &Item, room: Pixels, look: &Look, cx: &Context<Self>) -> AnyElement {
        let job = item.job.as_str();
        let (Some(held), Some(page)) = (self.thread(job, cx), self.pages.get(job)) else { return self.review_skeleton(look) };
        match item.kind {
            Kind::Artifact => self.review_parts(item, look, cx),
            // A build is drawn and answered as its card is: head, pictures, his words, where to look, and the four answers.
            Kind::Build => {
                let card = held.items.iter().find_map(|one| match one {
                    ThreadItem::ReadyToTry(card) if card.id == item.id => Some(card),
                    _ => None,
                });
                match card {
                    Some(card) => self.ready_detail(card, SHEET, f32::from(room - look.length(token::SPACE_6) * 2.), page, chrono::Local::now(), look, cx),
                    None => self.review_skeleton(look),
                }
            }
            Kind::Plan => {
                let card = held.head.plan.as_ref();
                match card.and_then(|card| self.record(job, cx).and_then(|record| record.plan.as_ref()).filter(|plan| plan.version == card.version).map(|plan| (card, plan))) {
                    Some((card, plan)) => self.plan_body(SHEET, &card.id, plan, false, &[], page, look, cx),
                    None => self.review_skeleton(look),
                }
            }
            Kind::Sketch => match sketch_card(&held.head, &held.items, &item.id) {
                Some(card) => glass(
                    look,
                    card.replaced,
                    match super::cards::shown_scene(card, None) {
                        (key, Some(scene)) => Some(crate::scene::view(key.into(), scene, crate::scene::Fit::Preview, crate::scene::Clock::default(), crate::scene::Lens::default()).into_any_element()),
                        _ => None,
                    },
                ),
                None => self.review_skeleton(look),
            },
            // The page itself is a web page, drawn by the embedded browser; the sheet holds its frame.
            Kind::Document => labelled(
                div()
                    .id(SharedString::from(format!("review-page:{}", item.key())))
                    .w_full()
                    .min_h(look.length(token::H_ROW_UNIT) * 8.)
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_SUNKEN)),
                Role::Region,
                "Page",
            )
            .into_any_element(),
            Kind::Draft => match draft_row(&held.items, item.id.strip_prefix("draft:").unwrap_or(&item.id)) {
                Some(SessionRow::Draft { id, text, .. }) => div()
                    .w_full()
                    .p(look.length(token::SPACE_4))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_SUNKEN))
                    .child(self.prose(format!("review-draft:{id}"), text, look, cx))
                    .into_any_element(),
                _ => self.review_skeleton(look),
            },
        }
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

    /// The foot of the sheet: the move keys, and nothing else.
    fn review_foot(&self, chosen: Option<&Item>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let enlarge = chosen.is_some_and(|item| self.review_enlarge(item, cx).is_some());
        let hint = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .text_color(look.color(token::FG_SUBTLE))
            .child(kbd(look, "J"))
            .child(kbd(look, "K"))
            .child("move")
            .when(enlarge, |hint| hint.child(div().px(look.length(token::SPACE_1)).child("\u{b7}")).child(kbd(look, "F")).child("enlarge"))
            .child(div().px(look.length(token::SPACE_1)).child("\u{b7}"))
            .child(kbd(look, "Esc"))
            .child("back");
        labelled(
            div()
                .id("review-foot")
                .flex_none()
                .flex()
                .items_center()
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_2))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .child(hint),
            Role::Group,
            "Keys",
        )
        .into_any_element()
    }

    /// The foot of the list: Approve all, and the caret that holds Dismiss all, in one split button under the rows.
    fn review_list_foot(&self, review: &ReviewState, look: &Look, cx: &Context<Self>) -> AnyElement {
        let asking = review.items().any(|item| !item.kind.answers_inside() && !item.read && review.row(&item.key()).verdict.is_none());
        let focus = &self.review_focus;
        let ink = look.color(token::FG_INVERSE);
        let green = look.color(token::STATUS_COMPLETED_SUCCESS);
        let rows = Rc::clone(&self.review_rows);
        // Two parts of one green control, a hairline apart; a part is off with nothing left asking.
        let part = |id: &'static str, focus: &FocusHandle| {
            div()
                .id(id)
                .track_focus(focus)
                .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .h(look.length(token::H_CONTROL))
                .border(look.length(token::BORDER_WIDTH))
                .bg(green)
                .border_color(green)
                .text_color(ink)
                .when(asking, |part| part.cursor_pointer().hover(|style| style.opacity(0.9)))
                .when(!asking, |part| part.opacity(0.4))
        };
        let main = part("review-approve-all", &focus.approve_all)
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .rounded_l(look.length(token::RADIUS_MD))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .child("Approve all")
            .child(kbd(look, "\u{21e7}A"))
            .when(asking, |part| {
                part.on_click(cx.listener(|this, _, window, cx| this.review_approve_all(window, cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.platform {
                        this.review_approve_all(window, cx);
                        cx.stop_propagation();
                    }
                }))
            });
        let caret = part("review-more-all", &focus.more_all)
            .relative()
            .w(look.length(token::H_CONTROL))
            .rounded_r(look.length(token::RADIUS_MD))
            .child(glyph(look, "chevron-down", Size::Small, ink))
            .child(canvas(move |bounds, _, _| { rows.borrow_mut().insert(MORE_ALL.to_string(), bounds); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            .when(asking, |part| {
                part.on_click(cx.listener(|this, _, window, cx| this.review_more_all(false, window, cx))).on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.platform {
                        this.review_more_all(true, window, cx);
                        cx.stop_propagation();
                    }
                }))
            });
        labelled(
            div()
                .id("review-list-foot")
                .flex_none()
                .flex()
                .justify_end()
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_3))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .gap(look.length(token::BORDER_WIDTH))
                        .child(labelled(main, Role::Button, "Approve all"))
                        .child(labelled(caret, Role::Button, "More for every item")),
                ),
            Role::Toolbar,
            "Every item",
        )
        .into_any_element()
    }

    /// The caret's menu, over the foot and opening upward: Dismiss all, as `⇧D` does.
    fn review_more_all(&mut self, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(bounds) = self.review_rows.borrow().get(MORE_ALL).copied() else { return };
        let at = gpui::point(bounds.origin.x, bounds.origin.y - px(4.));
        let entries = vec![Entry::Item { id: "dismiss-all", label: "Dismiss all".into(), icon: Some("x"), danger: false, key: Some("\u{21e7}D".into()), off: None }];
        let back = Some(self.review_focus.more_all.clone());
        self.open_button_menu_from(
            Anchor::BottomLeft,
            "More for every item",
            entries,
            at,
            from_keys,
            back,
            |this, id, window, cx| {
                if id == "dismiss-all" {
                    this.review_dismiss_all(window, cx);
                }
            },
            window,
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Modifiers;

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
            r#"{"kind":"sketch","id":"sk-1","at":"2026-11-09T14:01:00Z","sketch":"sketch:Tail","title":"Tail layout","drawing":{},"asked":true,"for_line":{"text":"Picks the tail the thread keeps."}},
               {"kind":"document","id":"document:design:r2","at":"2026-11-09T14:03:00Z","page":"design","title":"Stale branch design","revision":2,"asked":true},
               {"kind":"row","row":{"kind":"draft","id":"d1","at":"2026-11-09T14:10:00Z","title":"Comment","text":"words"}}"#,
        )
    }

    #[test]
    fn each_build_still_to_try_is_an_item_of_the_sheet_answered_in_its_own_detail() {
        let (head, mut items) = held();
        items.push(crate::job_view::ready_card::tests::thread_card(r#","state":"open""#));
        let mut with_card = owed();
        with_card.extend(waiting(r#"[{"id":"try:j","text":"Ready to try","since":"2026-11-09T14:20:00Z","source":"ready_to_try","artifact":"try:j"}]"#));
        let built = build("j", &with_card, &head, &items, true);
        let ready: Vec<&Item> = built.iter().filter(|item| item.kind == Kind::Build).collect();
        assert_eq!(ready.len(), 1, "one item for the one build");
        assert_eq!((ready[0].id.as_str(), ready[0].name.as_str(), ready[0].send_back, ready[0].read), ("try:native/chip@aaaa1111", "Sketch boxes drawn as cards", false, false));
        assert!(ready[0].kind.answers_inside());
        assert!(reviewable(WaitingSource::ReadyToTry));
        let none = build("j", &with_card, &head, &held().1, true);
        assert!(none.iter().all(|item| item.kind != Kind::Build), "a Job whose thread holds no build has nothing to show");
        let mut settled = held().1;
        settled.push(crate::job_view::ready_card::tests::thread_card(r#","state":"answered","verdict":"good""#));
        assert!(build("j", &with_card, &head, &settled, true).iter().all(|item| item.kind != Kind::Build), "a build he answered is no longer one to try");
    }

    #[test]
    fn the_items_stand_oldest_first_and_a_question_is_not_one_of_them() {
        let (head, items) = held();
        let built = build("j", &owed(), &head, &items, true);
        let order: Vec<&str> = built.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(order, ["plan:2", "sketch:Tail", "document:design", "draft:d1"]);
        assert_eq!(built.iter().map(|item| item.kind).collect::<Vec<_>>(), [Kind::Plan, Kind::Sketch, Kind::Document, Kind::Draft]);
        assert_eq!(built[0].name, "Plan v2");
        assert_eq!(built[1].for_line.as_deref(), Some("Picks the tail the thread keeps."));
        assert_eq!(built[2].revision.as_deref(), Some("r2"));
        assert!(built.iter().all(|item| item.job == "j"));
    }

    #[test]
    fn a_read_item_stands_on_the_sheet_with_only_open_to_do() {
        let (head, items) = held();
        let wire = waiting(r#"[{"id":"read:design","text":"Read: Stale branch design","since":"2026-11-09T14:03:00Z","source":"document","artifact":"design"}]"#);
        let built = build("j", &wire, &head, &items, true);
        assert_eq!(built.len(), 1, "a Read item is matched as a document is");
        assert_eq!((built[0].kind, built[0].read, built[0].send_back), (Kind::Document, true, false), "no Ask for changes, and only Open");
        assert_eq!(page_of_item("read:design"), Some("design"));
        assert_eq!(page_of_item("document:design"), Some("design"));
        assert_eq!(page_of_item("sketch:Tail"), None);
        assert!(!build("j", &owed(), &head, &items, true).iter().any(|item| item.read), "an item that waits on his answer is not a read one");
    }

    #[test]
    fn a_row_with_no_card_to_draw_stays_with_the_needs_you_card() {
        let (head, _) = held();
        assert!(build("j", &owed(), &head, &[], true).iter().all(|item| item.kind == Kind::Plan));
    }

    #[test]
    fn a_plan_with_no_group_and_a_proposed_workflow_have_no_send_back() {
        let (head, items) = held();
        let plan = |groups| build("j", &owed(), &head, &items, groups).into_iter().find(|item| item.kind == Kind::Plan).expect("a plan");
        assert!(plan(true).send_back);
        assert!(!plan(false).send_back);
        let (_, workflow) = thread(
            &format!("{{{TRAIL}}}"),
            r#"{"kind":"row","row":{"kind":"draft","id":"d1","at":"2026-11-09T14:10:00Z","title":"Workflow quick","text":"steps","change":12}}"#,
        );
        let draft = build("j", &owed(), &head, &workflow, true).into_iter().find(|item| item.kind == Kind::Draft).expect("a draft");
        assert!(!draft.send_back);
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
        let all = build("j", &owed(), &head, &items, true);
        assert!(lone_page(&all).is_none());
        let doc = |ids: &str| build("j", &waiting(ids), &head, &items, true);
        let one = doc(r#"[{"id":"document:design","text":"Design","since":"2026-11-09T14:03:00Z","source":"document"}]"#);
        assert_eq!(lone_page(&one).map(|item| item.id.as_str()), Some("document:design"));
        let sketch = doc(r#"[{"id":"sketch:Tail","text":"Tail","since":"2026-11-09T14:03:00Z","source":"sketch"}]"#);
        assert!(lone_page(&sketch).is_none());
    }

    fn item(job: &str, id: &str, since: &str) -> Item {
        Item { job: job.into(), id: id.into(), kind: Kind::Sketch, name: id.into(), since: since.into(), for_line: None, revision: None, send_back: true, read: false, artifact: None }
    }

    fn group(job: &str, items: Vec<Item>) -> Group {
        Group { job: job.into(), title: format!("Job {job}"), state: Some(JobState::Waiting), items }
    }

    #[test]
    fn the_sheet_groups_by_job_with_the_opening_job_first_then_the_oldest_waiting() {
        let groups = grouped(
            "b",
            vec![
                group("a", vec![item("a", "sketch:One", "2026-11-09T10:00:00Z")]),
                group("b", vec![item("b", "plan:2", "2026-11-09T12:00:00Z"), item("b", "sketch:Two", "2026-11-09T12:05:00Z")]),
                group("c", vec![item("c", "draft:d1", "2026-11-09T09:00:00Z")]),
                group("d", Vec::new()),
            ],
        );
        assert_eq!(groups.iter().map(|group| group.job.as_str()).collect::<Vec<_>>(), ["b", "c", "a"], "the opener first, then oldest first; an empty Job has no group");
        assert_eq!(order(&groups), [key_of("b", "plan:2"), key_of("b", "sketch:Two"), key_of("c", "draft:d1"), key_of("a", "sketch:One")]);
        assert_ne!(key_of("a", "plan:2"), key_of("b", "plan:2"), "the same id in two Jobs is two rows");
    }

    #[test]
    fn approve_all_sends_every_item_still_asking_each_with_its_own_notes() {
        let (one, two, three) = (item("a", "sketch:One", "t"), item("b", "plan:2", "t"), item("b", "draft:d1", "t"));
        let notes = |it: &Item| if it.id == "plan:2" { "  Keep the second group.  ".to_string() } else { String::new() };
        let body = answer_batch(&[&one, &two, &three], ItemHow::Approve, notes);
        assert_eq!(body.items.len(), 3);
        assert_eq!(body.items.iter().map(|one| (one.session_id.as_str(), one.item.as_str())).collect::<Vec<_>>(), [("a", "sketch:One"), ("b", "plan:2"), ("b", "draft:d1")]);
        assert!(body.items.iter().all(|one| one.how == ItemHow::Approve));
        assert_eq!(body.items[1].notes, vec![ItemNote { text: "Keep the second group.".into(), on: None }]);
        assert!(body.items[0].notes.is_empty() && body.items[2].notes.is_empty(), "blank notes go as none");
        let wire = ipc::encode(&body).expect("encodes");
        assert!(wire.contains(r#""how":"approve""#) && !wire.contains(r#""notes":[]"#), "{wire}");
    }

    #[test]
    fn dismiss_all_sends_every_item_still_asking_by_job_and_id() {
        let (one, two) = (item("a", "sketch:One", "t"), item("b", "document:design", "t"));
        let body = dismiss_batch(&[&one, &two]);
        let sent: Vec<(&str, &str)> = body.items.iter().map(|one| (one.session_id.as_str(), one.item.as_str())).collect();
        assert_eq!(sent, [("a", "sketch:One"), ("b", "document:design")]);
    }

    #[test]
    fn a_send_back_carries_its_notes_and_blank_notes_are_none() {
        assert!(notes_of("   \n ").is_empty());
        let one = item("a", "sketch:One", "t");
        let body = answer_batch(&[&one], ItemHow::SendBack, |_| "Wider rows.".into());
        assert_eq!(body.items[0].how, ItemHow::SendBack);
        assert_eq!(body.items[0].notes, vec![ItemNote { text: "Wider rows.".into(), on: None }]);
    }

    #[test]
    fn each_outcome_lands_on_its_own_row_and_a_refusal_asks_again_with_fleets_sentence() {
        let settled = ipc::decode::<ItemsSettled>(
            "ItemsSettled",
            br#"{"outcomes":[
              {"session_id":"a","item":"sketch:One","outcome":{"kind":"settled"}},
              {"session_id":"b","item":"sketch:One","outcome":{"kind":"refused","code":"fleet.sketch_redrawn","message":"the sketch `One` is not the drawing that was read: the job has drawn it again since"}},
              {"session_id":"b","item":"draft:d1","outcome":{"kind":"settled"}}
            ]}"#,
        )
        .expect("decodes");
        let mut rows: HashMap<String, Row> = [key_of("a", "sketch:One"), key_of("b", "sketch:One"), key_of("b", "draft:d1")]
            .into_iter()
            .map(|key| (key, Row { verdict: Some(Verdict::Approved), refused: None }))
            .collect();
        settle_rows(&mut rows, &settled.outcomes, Verdict::Approved);
        assert_eq!(rows[&key_of("a", "sketch:One")], Row { verdict: Some(Verdict::Approved), refused: None });
        assert_eq!(rows[&key_of("b", "draft:d1")].verdict, Some(Verdict::Approved));
        let refused = &rows[&key_of("b", "sketch:One")];
        assert_eq!(refused.verdict, None, "a refused item still asks");
        assert_eq!(refused.refused.as_deref(), Some("the sketch `One` is not the drawing that was read: the job has drawn it again since"));
        let keys = vec![key_of("a", "sketch:One"), key_of("b", "sketch:One"), key_of("b", "draft:d1")];
        assert_eq!(next_open(&keys, &rows, &keys[0]).as_deref(), Some(keys[1].as_str()), "the refused row is the one left to answer");
    }

    #[test]
    fn the_next_open_item_is_the_one_after_else_the_first_open_one() {
        let keys: Vec<String> = (0..4).map(|at| format!("i{at}")).collect();
        let rows = |done: &[usize]| done.iter().map(|at| (format!("i{at}"), Row { verdict: Some(Verdict::Approved), refused: None })).collect::<HashMap<_, _>>();
        assert_eq!(next_open(&keys, &rows(&[1]), "i0").as_deref(), Some("i2"));
        assert_eq!(next_open(&keys, &rows(&[1, 2, 3]), "i1").as_deref(), Some("i0"));
        assert_eq!(next_open(&keys, &rows(&[0, 1, 2, 3]), "i1"), None);
        assert_eq!(next_open(&[], &HashMap::new(), "i0"), None);
    }

    const TWO: &str = r#"[
      {"id":"sketch:Newer","text":"Newer","since":"2026-11-09T14:05:00Z","source":"sketch"},
      {"id":"sketch:Older","text":"Older","since":"2026-11-09T14:01:00Z","source":"sketch"}
    ]"#;

    /// Two open sketches as Fleet folds them: the newest the head's pin and no item, the older its card in the thread.
    fn two_sketches() -> (ThreadHead, Vec<ThreadItem>) {
        thread(
            &format!(r#"{{{TRAIL},"sketch":{{"id":"sk-2","at":"2026-11-09T14:05:00Z","sketch":"sketch:Newer","title":"Newer","drawing":{{}},"asked":true}}}}"#),
            r#"{"kind":"sketch","id":"sk-1","at":"2026-11-09T14:01:00Z","sketch":"sketch:Older","title":"Older","drawing":{},"asked":true}"#,
        )
    }

    #[test]
    fn every_open_sketch_is_its_own_item_with_its_own_send_back_oldest_first() {
        let (head, items) = two_sketches();
        let built = build("j", &waiting(TWO), &head, &items, false);
        assert_eq!(built.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), ["sketch:Older", "sketch:Newer"]);
        assert_eq!(built.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(), ["Older", "Newer"]);
        assert!(built.iter().all(|item| item.kind == Kind::Sketch && item.send_back));
        assert_eq!(sketch_card(&head, &items, "sketch:Older").map(|card| card.id.as_str()), Some("sk-1"));
        assert_eq!(sketch_card(&head, &items, "sketch:Newer").map(|card| card.id.as_str()), Some("sk-2"));
    }

    #[test]
    fn answering_the_pinned_sketch_leaves_the_other_and_the_pin_moves_to_it() {
        let (mut head, mut items) = two_sketches();
        let how = ipc::SketchHow::Approved;
        crate::mock::sketch_answered(&mut head, &mut items, "sketch:Newer", how, "2026-11-09T14:10:00Z", None).expect("changed");
        assert_eq!(head.sketch.as_ref().map(|card| card.sketch.as_str()), Some("sketch:Older"));
        assert!(items.iter().all(|item| !matches!(item, ThreadItem::Sketch(card) if card.sketch == "sketch:Older")), "the pin's card is no item");
        assert!(items.iter().any(|item| matches!(item, ThreadItem::Sketch(card) if card.sketch == "sketch:Newer" && card.settled.is_some())), "the answered card is back in the thread, settled");
        let rest = waiting(r#"[{"id":"sketch:Older","text":"Older","since":"2026-11-09T14:01:00Z","source":"sketch"}]"#);
        let built = build("j", &rest, &head, &items, false);
        assert_eq!(built.iter().map(|item| (item.id.as_str(), item.send_back)).collect::<Vec<_>>(), [("sketch:Older", true)]);
        assert!(head.sketch.as_ref().is_some_and(|card| card.settled.is_none()));
    }

    #[test]
    fn answering_the_other_sketch_leaves_the_pin() {
        let (mut head, mut items) = two_sketches();
        crate::mock::sketch_answered(&mut head, &mut items, "sketch:Older", ipc::SketchHow::SentBack, "2026-11-09T14:10:00Z", Some("Wider".into())).expect("changed");
        assert_eq!(head.sketch.as_ref().map(|card| card.sketch.as_str()), Some("sketch:Newer"));
        assert!(items.iter().any(|item| matches!(item, ThreadItem::Sketch(card) if card.sketch == "sketch:Older" && card.settled.as_ref().is_some_and(|settled| settled.how == ipc::SketchHow::SentBack))));
        let rest = waiting(r#"[{"id":"sketch:Newer","text":"Newer","since":"2026-11-09T14:05:00Z","source":"sketch"}]"#);
        assert_eq!(build("j", &rest, &head, &items, false).len(), 1);
    }

    #[test]
    fn the_last_sketch_answered_leaves_no_pin() {
        let (mut head, mut items) = thread(
            &format!(r#"{{{TRAIL},"sketch":{{"id":"sk-2","at":"2026-11-09T14:05:00Z","sketch":"sketch:Only","title":"Only","drawing":{{}},"asked":true}}}}"#),
            "",
        );
        crate::mock::sketch_answered(&mut head, &mut items, "sketch:Only", ipc::SketchHow::Approved, "2026-11-09T14:10:00Z", None).expect("changed");
        assert!(head.sketch.is_none());
        assert_eq!(items.len(), 1);
    }

    /// What the last frame drew with this role and name, the last drawn.
    fn drawn_at(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> Bounds<Pixels> {
        cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds))
            .unwrap_or_else(|| panic!("{name} was not drawn"))
    }

    fn sheet_open<'a>(cx: &'a mut gpui::TestAppContext) -> (Entity<JobView>, &'a mut gpui::VisualTestContext) {
        let (view, cx) = super::super::parts_fold::rooted_job(cx, "review-grouped", "mock-01", gpui::size(px(1400.), px(900.)), |_| {});
        view.update_in(cx, |view, window, cx| view.open_review(window, cx));
        super::super::parts_fold::redraw(cx);
        (view, cx)
    }

    fn verdicts(view: &Entity<JobView>, cx: &mut gpui::VisualTestContext) -> Vec<Option<Verdict>> {
        view.read_with(cx, |view, _| view.review().map(|review| review.items().map(|item| review.row(&item.key()).verdict).collect()).unwrap_or_default())
    }

    #[gpui::test]
    fn the_item_acts_stand_in_its_band_and_the_all_acts_under_the_list_where_the_caret_holds_dismiss_all(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::{drawn, redraw};
        let (view, cx) = sheet_open(cx);
        let column = drawn_at(cx, Role::Group, "Waiting on you");
        let foot = drawn_at(cx, Role::Toolbar, "Every item");
        let all = drawn_at(cx, Role::Button, "Approve all");
        let band = drawn_at(cx, Role::Group, "Answers");
        let approve = drawn_at(cx, Role::Button, "Approve");
        assert!(foot.left() == column.left() && foot.right() <= band.left() + px(1.), "the foot stands under the list, not under the detail: {foot:?} {column:?}");
        assert!(all.left() >= foot.left() && all.right() <= foot.right() && all.top() >= foot.top() && all.bottom() <= foot.bottom(), "Approve all is in the foot: {all:?} {foot:?}");
        assert!(all.top() >= column.bottom(), "and under the rows, which scroll above it: {all:?} {column:?}");
        assert!(all.right() <= band.left() + px(1.), "and not under the detail: {all:?} {band:?}");
        assert!(approve.left() >= band.left() && approve.right() <= band.right() && approve.top() >= band.top() && approve.bottom() <= band.bottom(), "the item's Approve is in its band: {approve:?} {band:?}");
        assert!(!drawn(cx, Role::Button, "Dismiss all"), "Dismiss all waits under the caret");

        // The caret's menu holds Dismiss all, and Esc shuts the menu alone.
        let caret = drawn_at(cx, Role::Button, "More for every item").center();
        cx.simulate_click(caret, Modifiers::none());
        redraw(cx);
        assert!(drawn(cx, Role::MenuItem, "Dismiss all"), "the caret opens a menu with Dismiss all");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!drawn(cx, Role::MenuItem, "Dismiss all"), "Esc shuts the menu");
        assert!(view.read_with(cx, |view, _| view.review_open()), "and leaves the panel");

        // Pressing it dismisses what ⇧D does.
        let caret = drawn_at(cx, Role::Button, "More for every item").center();
        cx.simulate_click(caret, Modifiers::none());
        redraw(cx);
        let dismiss = drawn_at(cx, Role::MenuItem, "Dismiss all").center();
        cx.simulate_click(dismiss, Modifiers::none());
        redraw(cx);
        let by_menu = verdicts(&view, cx);
        assert!(by_menu.iter().any(|verdict| *verdict == Some(Verdict::Dismissed)), "the menu's Dismiss all dismissed: {by_menu:?}");
        let (view, cx) = sheet_open(cx);
        // The key's act, `items_dismiss_all`, is this call (`review_sheet_act`); keys are not bound in this window.
        view.update_in(cx, |view, window, cx| view.review_dismiss_all(window, cx));
        redraw(cx);
        assert_eq!(verdicts(&view, cx), by_menu, "the same as the key");
    }
}
