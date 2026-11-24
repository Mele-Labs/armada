//! The review sheet: one ask at a time, beside the list of what still asks (the layout "List beside the chosen ask", 19 Nov).
//! Each ask is one row, titled by its question; the chosen one is drawn beside the list with its question as the head, one line
//! of background, its body by kind, and an answer bar. An answered ask leaves the list at once and the next comes up; the sheet
//! closes when none is left. There is no Approve all.
//!
//! A plan, a sketch, a document and a draft are answered with `answer_items` or `dismiss_items`, an artifact by its parts, a
//! build as its Ready to try card is, and a question with `answer_waiting`. A refusal puts the ask back with Fleet's sentence.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::{
    anchored, canvas, deferred, div, prelude::*, px, Anchor, AnyElement, Bounds, Context, Entity, FocusHandle, Focusable as _, FontWeight,
    KeyDownEvent, MouseButton, Pixels, Role, SharedString, Subscription, Window,
};
use gpui_component::input::{Enter, InputEvent, TextareaState};
use ipc::{
    AnswerItems, DismissItems, ItemAnswer, ItemHow, ItemNote, ItemOutcome, ItemRef, ItemSettled, ItemsSettled, JobState, PlanCardState, SessionId,
    ReadyState, SessionRow, ThreadDocument, ThreadHead, ThreadItem, ThreadSketch, WaitingItem, WaitingSource,
};

use super::acts::{button, kbd, Tone, SHEET};
use super::cards::glass;
use super::judge::{plan_artifact, plan_readings, Pinned};
use super::rows::{glyph, tip, Size};
use super::{JobView, Page};
use crate::dropdown::{under, Entry};
use crate::generated::actions::ActionScope;
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
    /// A question the Job put with options and nothing to read: answered by an option, or in words.
    Question,
}

impl Kind {
    /// Whether the item is answered in its own detail, by its parts or its build's four answers, and never by the sheet's
    /// Approve, Ask for changes and Dismiss.
    pub(super) fn answers_inside(self) -> bool {
        matches!(self, Kind::Artifact | Kind::Build)
    }
}

/// What an ask is as the sheet draws it: the tile's glyph and hue, the body, and the answer the bar offers. An artifact reads as
/// what its parts are, so a vote among takes is Takes whatever the artifact is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ask {
    Takes,
    Plan,
    Drawing,
    Page,
    Draft,
    Question,
    Build,
}

impl Ask {
    /// The glyph, the word its tooltip names it by, and the one hue its tile takes.
    pub(super) fn mark(self) -> (&'static str, &'static str, Token) {
        match self {
            Ask::Takes => ("scale", "Takes", token::KIND_STEP),
            Ask::Plan => ("list-todo", "Plan", token::KIND_DELIVERY),
            Ask::Drawing => ("shapes", "Drawing", token::ACCENT),
            Ask::Page => ("scroll-text", "Page", token::KIND_GATE),
            Ask::Draft => ("file-text", "Draft", token::KIND_SETUP),
            Ask::Question => ("message-circle-question-mark", "Question", token::STATUS_REJECTED),
            Ask::Build => ("flask-conical", "Build to try", token::STATUS_COMPLETED_SUCCESS),
        }
    }

    /// What the ask wants of him, as the row's second line says it.
    pub(super) fn wants(self) -> &'static str {
        match self {
            Ask::Takes => "Pick one",
            Ask::Question => "Answer",
            Ask::Build => "Try it",
            Ask::Plan | Ask::Drawing | Ask::Page | Ask::Draft => "Approve",
        }
    }

    /// The bar's second answer, which opens the box in the bar: words in place of an option for a question.
    pub(super) fn second(self) -> &'static str {
        match self {
            Ask::Question => "Answer in words",
            _ => "Ask for changes",
        }
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

/// Where one row stands: its verdict, or none while it still asks, and Fleet's sentence where it refused one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Row {
    pub verdict: Option<Verdict>,
    pub refused: Option<String>,
}

/// The notes of the chosen item; what is typed is kept under the item, with its chips. The words are a rich note's, so a
/// picture pasted or a file dropped goes with the answer, `@` tags a Job or a file, and the words are formatted as a message's.
pub(super) struct NoteBox {
    pub key: String,
    pub area: Entity<TextareaState>,
    pub note: Entity<super::rich_note::RichNote>,
    _typed: Subscription,
    _sized: Subscription,
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
    /// The answer bar's box is open, for Ask for changes or Answer in words on the chosen ask.
    pub changing: bool,
    /// The bar's Edit box on a draft, by the ask's key: the draft's words to fix before approving.
    pub edit: Option<(String, Entity<TextareaState>)>,
    /// The option chosen on each question, by key, until Send answer.
    pub choice: HashMap<String, usize>,
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
    pub cancel: FocusHandle,
    pub send: FocusHandle,
    pub edit: FocusHandle,
}

impl Focuses {
    pub(super) fn new(cx: &mut gpui::App) -> Self {
        Focuses {
            list: cx.focus_handle(),
            approve: cx.focus_handle(),
            changes: cx.focus_handle(),
            dismiss: cx.focus_handle(),
            close: cx.focus_handle(),
            cancel: cx.focus_handle(),
            send: cx.focus_handle(),
            edit: cx.focus_handle(),
        }
    }
}

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

/// A walk's scene of the review sheet, as a person cannot drive it from the keys alone: `sheet-paste-picture` pastes a
/// picture into the sheet's open box, as ⌘V with one on the clipboard does, without touching his clipboard;
/// `sheet-settled-artifact` opens the mock's settled artifact of parts in the sheet.
pub fn walk_scene(name: &str, window: &mut Window, cx: &mut gpui::App) -> Result<(), String> {
    let view = JobView::live(cx).ok_or_else(|| "no Job view is drawn yet".to_string())?;
    match name {
        "sheet-paste-picture" => {
            let note = view.read(cx).review().and_then(|review| review.note.as_ref()).map(|held| held.note.clone()).ok_or_else(|| "the sheet's box is not open".to_string())?;
            let bytes = crate::mock::shown_still("pick-actions-a.png").unwrap_or_default().to_vec();
            let picture = gpui::ClipboardItem::new_image(&gpui::Image::from_bytes(gpui::ImageFormat::Png, bytes));
            note.update(cx, |note, cx| note.paste(&picture, cx)).then_some(()).ok_or_else(|| "the box took no picture".to_string())
        }
        "sheet-settled-artifact" => {
            view.update(cx, |view, cx| view.open_artifact_review(crate::mock_parts::ID, window, cx));
            Ok(())
        }
        other => Err(format!("no review sheet scene named {other}")),
    }
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

/// What Approve with fixes sends: the draft approved, with his words in place of the Job's.
pub(super) fn fixed_body(job: &str, item: &str, words: String) -> Result<String, ipc::Unencodable> {
    ipc::encode(&ipc::AnswerWaiting { session_id: SessionId::carried(job.to_string()), item_id: item.to_string(), choice: None, text: None, mode: None, fixed: Some(words), extras: Default::default() })
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

/// Whether a question's options carry what the sheet does not draw yet, a still or a preview, so it stays on the Needs you
/// card where they are drawn.
fn shows_more(one: &WaitingItem, head: &ThreadHead) -> bool {
    if one.options.iter().any(|option| option.picture.is_some()) {
        return true;
    }
    let Some(asked) = head.asked.as_ref().filter(|asked| one.source == WaitingSource::AskCard && super::ask_line::of_card(&one.id, &asked.call.call)) else { return false };
    let at = one.id.rsplit_once('#').and_then(|(_, at)| at.parse::<usize>().ok()).unwrap_or(0);
    asked.call.questions.get(at).is_some_and(|question| question.options.iter().any(|option| !option.preview.trim().is_empty()))
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
                // A question with options and nothing to read is one ask; its title is the question itself.
                WaitingSource::Agent | WaitingSource::AskCard if !one.options.is_empty() && one.artifact.is_none() && !shows_more(one, head) => {
                    Some(item(Kind::Question, one.text.clone(), None, None, true))
                }
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

    /// Runs `act` with Job `job` as the shown one. A card's and a part's acts read and answer the shown Job, and the sheet from the
    /// Jobs list holds other Jobs' asks: so a build or takes of another Job is answered on that Job's page and sent to its session.
    /// The sheet's own state stays on the page it was opened on, which `act` does not reach.
    pub(super) fn as_job<R>(&mut self, job: &str, act: impl FnOnce(&mut Self) -> R) -> R {
        let held = self.shown.replace(SharedString::from(job.to_string()));
        let done = act(self);
        self.shown = held;
        done
    }

    /// The card whose open box holds the keys, by the key its ⌘↵ is kept under.
    fn focused_card_box(&self, window: &Window, cx: &gpui::App) -> Option<String> {
        let page = self.shown.as_ref().and_then(|id| self.pages.get(id))?;
        page.cards.boxes.iter().find(|(_, area)| area.read(cx).focus_handle(cx).contains_focused(window, cx)).and_then(|(key, _)| key.rsplit_once(':').map(|(card, _)| card.to_string()))
    }

    /// The caret is in the bar's Edit box on a draft.
    fn review_editing(&self, window: &Window, cx: &gpui::App) -> bool {
        self.review().and_then(|review| review.edit.as_ref()).is_some_and(|(_, area)| area.read(cx).focus_handle(cx).contains_focused(window, cx))
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
        // One he answered goes at once, wherever he answered it, before Fleet's next list says so.
        if let Some(dock) = self.dock_of(id) {
            items.retain(|item| !dock.hidden.contains(&item.id));
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
                // Each Job's artifacts of parts that still ask stand with what else waits on him, and the shown Job's one he
                // opened: a vote of another Job is picked from the every-Job sheet as from its own.
                items.extend(self.artifact_items(job, opened.filter(|_| job == first)));
                if job == first {
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
                self.ask_titles(job, &mut items, &record.waiting_for);
                items.sort_by_key(|item| instant(&item.since));
                Group { job: job.to_string(), title: record.title.clone().unwrap_or_else(|| "New Job".into()), state: record.mark.map(|mark| mark.state), items }
            })
            .collect();
        grouped(first, groups)
    }

    /// Each ask titled by its question, as the Job put it on his list: the waiting row's words, where one stands for the item.
    /// An artifact takes the words of the row that names it, and a build those of the Job's one Ready to try row while it is the
    /// only build. An ask with no row keeps its card's title.
    fn ask_titles(&self, job: &str, items: &mut [Item], waiting: &[WaitingItem]) {
        let builds = items.iter().filter(|item| item.kind == Kind::Build).count();
        for item in items.iter_mut() {
            let row = match item.kind {
                Kind::Build if builds == 1 => waiting.iter().find(|one| one.source == WaitingSource::ReadyToTry),
                Kind::Build => None,
                Kind::Artifact => {
                    let artifact = item.artifact.as_deref().unwrap_or_default();
                    let answered_by = self.artifact_of(job, artifact).and_then(|one| one.item.clone());
                    waiting.iter().find(|one| one.artifact.as_deref() == Some(artifact) || Some(&one.id) == answered_by.as_ref() || one.id == item.id)
                }
                _ => waiting.iter().find(|one| one.id == item.id),
            };
            let Some(row) = row else { continue };
            if let Some(text) = Some(row.text.trim()).filter(|text| !text.is_empty()) {
                item.name = text.to_string();
            }
            // The ask has waited since it was put to him, not since the thing it asks about was made.
            item.since = row.since.as_str().to_string();
        }
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
        // Where the keys were is kept before the Job changes, so Esc gives them back to the card the press came from.
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
            // A row whose id is the artifact's own names it too.
            let named = self.record(&id, cx).and_then(|record| record.waiting_for.iter().find(|one| one.id == pick).and_then(|one| one.artifact.clone())).or_else(|| Some(pick.to_string()));
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
        // An artifact's parts hold the keys of their own once they are drawn. A vote among takes leaves them with the list, which
        // chooses a take by its number.
        if let Some(item) = first.as_deref().and_then(|key| self.review().and_then(|review| review.item(key)).cloned()).filter(|item| item.kind == Kind::Artifact && self.ask_of(item) != Ask::Takes) {
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
        if !held {
            let kept_key = note_key(&item.job, &item.id);
            let kept = self.draft_of(&kept_key);
            let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 10));
            area.update(cx, |area, cx| area.set_value(kept.content(), window, cx));
            // Made before the box's own watcher, so what a keystroke draws is drawn before the draft is read.
            let scope = self.as_job(&item.job, |this| this.note_scope(cx));
            let note = cx.new(|cx| super::rich_note::RichNote::new(area.clone(), self.ask_of(&item).second(), scope, window, cx));
            let sized = cx.subscribe(&note, |_, _, _: &super::rich_note::Resized, cx| cx.notify());
            let typed = cx.subscribe_in(&area, window, move |this, area, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    this.keep_draft(&kept_key, super::rich_note::kept_of(area, cx), cx);
                    cx.notify();
                }
            });
            if let Some(review) = self.review_mut() {
                review.note = Some(NoteBox { key: key.to_string(), area, note, _typed: typed, _sized: sized });
                // The box opens for one ask: another chosen starts with its bar shut.
                review.changing = false;
                review.edit = None;
            }
        }
        // A vote among takes starts on the Job's pick, so its answer is one press.
        if let Some(vote) = self.sheet_vote(&item) {
            if self.takes_of_key_in(&item.job, &vote).is_some_and(|(_, open, chosen)| open && chosen.is_none()) {
                if let Some(pick) = self.takes_of_key_in(&item.job, &vote).and_then(|(takes, _, _)| takes.iter().position(|take| super::parts_card::pick_mark(&take.title).1)) {
                    self.as_job(&item.job, |this| this.choose_take(&vote, pick, cx));
                }
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

    /// The words of the item's box as a message sends them: trimmed, with a blank line where a quote or a list ends in words.
    fn review_notes_text(&self, item: &Item, cx: &gpui::App) -> String {
        let held = self.review().and_then(|review| review.note.as_ref()).filter(|note| note.key == item.key());
        match held {
            Some(held) => held.note.read(cx).sent(cx),
            None => super::format::sent_text(self.draft_words(&note_key(&item.job, &item.id)).trim()),
        }
    }

    /// The pictures, files and tags the item's box holds, which go with its words.
    fn review_extras(&self, item: &Item, cx: &gpui::App) -> ipc::AnswerExtras {
        self.review().and_then(|review| review.note.as_ref()).filter(|note| note.key == item.key()).map(|held| held.note.read(cx).extras(cx)).unwrap_or_default()
    }

    /// A list is open over the words of the bar's box, so Esc is the list's.
    fn review_note_offering(&self, cx: &gpui::App) -> bool {
        self.review().and_then(|review| review.note.as_ref()).is_some_and(|held| held.note.read(cx).offering())
    }

    /// The asks the list shows: every one still asking, and the chosen one in the frame it is answered. A page he is only to
    /// read informs and is no ask, unless it is the one he opened.
    fn review_shown(&self, review: &ReviewState, cx: &gpui::App) -> Vec<Item> {
        review
            .items()
            .filter(|item| !item.read || review.opened.as_deref().is_some_and(|page| page_of_item(&item.id) == Some(page)))
            .filter(|item| review.at.as_deref() == Some(item.key().as_str()) || self.item_row(item, review, cx).verdict.is_none())
            .cloned()
            .collect()
    }

    /// The chosen ask, where it still asks.
    fn review_open_ask(&self, cx: &gpui::App) -> Option<Item> {
        let review = self.review()?;
        let item = review.item(review.at.as_ref()?)?.clone();
        self.item_row(&item, review, cx).verdict.is_none().then_some(item)
    }

    /// What the sheet draws an ask as.
    pub(super) fn ask_of(&self, item: &Item) -> Ask {
        match item.kind {
            Kind::Plan => Ask::Plan,
            Kind::Sketch => Ask::Drawing,
            Kind::Document => Ask::Page,
            Kind::Draft => Ask::Draft,
            Kind::Build => Ask::Build,
            Kind::Question => Ask::Question,
            Kind::Artifact => self.artifact_ask(item),
        }
    }

    /// The build an item stands for, as its card holds it.
    fn build_card(&self, item: &Item, cx: &gpui::App) -> Option<ipc::ThreadReadyToTry> {
        self.thread(&item.job, cx)?.items.iter().find_map(|one| match one {
            ThreadItem::ReadyToTry(card) if card.id == item.id => Some(card.clone()),
            _ => None,
        })
    }

    /// The options a question offers, each with what choosing it means.
    fn question_options(&self, item: &Item, cx: &gpui::App) -> Vec<ipc::WaitingOption> {
        self.record(&item.job, cx).and_then(|record| record.waiting_for.iter().find(|one| one.id == item.id).map(|one| one.options.clone())).unwrap_or_default()
    }

    /// The bar's answer for the ask: its words, and whether it can be pressed yet. A vote names the take it sends, and waits for
    /// one; a question waits for an option; a build waits until the preview holds it.
    fn review_primary_label(&self, item: &Item, ask: Ask, cx: &gpui::App) -> (String, bool) {
        match ask {
            Ask::Takes => {
                let chosen = self.sheet_vote(item).and_then(|vote| self.takes_of_key_in(&item.job, &vote)).and_then(|(takes, _, chosen)| chosen.and_then(|at| takes.get(at).cloned()));
                match chosen {
                    Some(take) => (format!("Pick {}", super::parts_card::pick_mark(&take.title).0), true),
                    None => ("Pick one".to_string(), false),
                }
            }
            Ask::Question => ("Send answer".to_string(), self.review().is_some_and(|review| review.choice.contains_key(&item.key()))),
            Ask::Build => ("Looks good".to_string(), self.build_card(item, cx).is_some_and(|card| card.in_build)),
            _ => ("Approve".to_string(), !item.read && self.item_asks(item, cx)),
        }
    }

    /// Whether the bar offers its second answer on the ask: a plan with no group and a proposed workflow have nothing to send
    /// back with, and a page he is only reading asks nothing.
    fn review_offers_second(&self, item: &Item, ask: Ask, cx: &gpui::App) -> bool {
        match ask {
            Ask::Takes | Ask::Question | Ask::Build => true,
            // An artifact's parts take his words as a send back of each part that still asks.
            _ if item.kind == Kind::Artifact => true,
            _ => item.send_back && !item.read && self.item_asks(item, cx),
        }
    }

    // ---- answers ---------------------------------------------------------------------------

    /// ⌘↵ and the bar's first button: the ask's own answer, then the next ask.
    fn review_primary(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.review_open_ask(cx) else { return };
        let ask = self.ask_of(&item);
        if !self.review_primary_label(&item, ask, cx).1 {
            return;
        }
        match item.kind {
            Kind::Artifact => {
                if !self.as_job(&item.job, |this| this.sheet_answer_parts(&item, None, window, cx)) {
                    return;
                }
            }
            Kind::Build => {
                let Some(card) = self.build_card(&item, cx) else { return };
                self.as_job(&item.job, |this| this.answer_ready(SHEET, &card, ipc::ReadyAnswer::LooksGood, window, cx));
            }
            Kind::Question => {
                let choice = self.review().and_then(|review| review.choice.get(&item.key()).copied());
                self.review_answer_question(&item, choice, None, Default::default(), window, cx);
                return;
            }
            Kind::Plan | Kind::Sketch | Kind::Document | Kind::Draft => {
                self.review_approve(window, cx);
                return;
            }
        }
        self.review_after(&item.key(), window, cx);
    }

    /// `A`: Approve the chosen item.
    fn review_approve(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((item, row)) = self.review_current() else { return };
        if row.verdict.is_some() || item.read || item.kind.answers_inside() || item.kind == Kind::Question || !self.item_asks(&item, cx) {
            return;
        }
        let body = answer_batch(&[&item], ItemHow::Approve, |_| String::new());
        self.review_send(vec![item], Verdict::Approved, "/sessions/items/answer", ipc::encode(&body), window, cx);
    }

    /// `C` and the bar's second button: the box opens in the bar, holding what was typed in it before.
    fn review_open_changes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.review_open_ask(cx) else { return };
        if !self.review_offers_second(&item, self.ask_of(&item), cx) {
            return;
        }
        if let Some(review) = self.review_mut() {
            review.changing = true;
        }
        self.review_focus_notes(window, cx);
    }

    /// Cancel, and Esc in the box: the bar shuts, and the words stay kept under the ask for the next time.
    fn review_cancel_changes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(review) = self.review_mut() {
            review.changing = false;
        }
        window.focus(&self.review_focus.list, cx);
        cx.notify();
    }

    /// Send, and ⌘↵ in the box: the ask goes back with his words, or a question is answered in them. With none typed the caret
    /// stays in the box, since a send back with no note is refused.
    fn review_send_changes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.review_open_ask(cx) else { return };
        let words = self.review_notes_text(&item, cx).trim().to_string();
        if words.is_empty() {
            self.review_focus_notes(window, cx);
            return;
        }
        let key = item.key();
        let extras = self.review_extras(&item, cx);
        match item.kind {
            Kind::Artifact => {
                if !self.as_job(&item.job, |this| this.sheet_answer_parts(&item, Some((words, extras)), window, cx)) {
                    return;
                }
            }
            Kind::Build => {
                let Some(card) = self.build_card(&item, cx) else { return };
                self.as_job(&item.job, |this| this.answer_ready(SHEET, &card, ipc::ReadyAnswer::NeedsRework { words }, window, cx));
            }
            Kind::Question => {
                self.review_answer_question(&item, None, Some(words), extras, window, cx);
                return;
            }
            Kind::Plan | Kind::Sketch | Kind::Document | Kind::Draft => {
                if !item.send_back || item.read || !self.item_asks(&item, cx) {
                    return;
                }
                let mut body = answer_batch(&[&item], ItemHow::SendBack, |one| self.review_notes_text(one, cx));
                if let Some(one) = body.items.first_mut() {
                    one.extras = extras;
                }
                self.review_send(vec![item], Verdict::ChangesAsked, "/sessions/items/answer", ipc::encode(&body), window, cx);
                return;
            }
        }
        self.keep_words(&note_key(&item.job, &item.id), "", cx);
        self.review_after(&key, window, cx);
    }

    /// The draft an ask stands for, as its row holds it: its words.
    fn draft_text(&self, item: &Item, cx: &gpui::App) -> Option<String> {
        let held = self.thread(&item.job, cx)?;
        match draft_row(&held.items, item.id.strip_prefix("draft:").unwrap_or(&item.id)) {
            Some(SessionRow::Draft { text, .. }) => Some(text.clone()),
            _ => None,
        }
    }

    /// Whether the bar offers Edit on the ask: a draft still asking, whose words he may fix before approving it.
    fn review_offers_edit(&self, item: &Item, cx: &gpui::App) -> bool {
        item.kind == Kind::Draft && !item.read && self.draft_text(item, cx).is_some()
    }

    /// `E` and the bar's Edit on a draft: the box opens in the bar holding the draft's words, to fix and approve.
    fn review_open_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.review_open_ask(cx) else { return };
        if !self.review_offers_edit(&item, cx) {
            return;
        }
        let key = item.key();
        let held = self.review().and_then(|review| review.edit.as_ref()).filter(|(on, _)| *on == key).map(|(_, area)| area.clone());
        let area = match held {
            Some(area) => area,
            None => {
                let text = self.draft_text(&item, cx).unwrap_or_default();
                let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(4, 12));
                area.update(cx, |area, cx| area.set_value(text, window, cx));
                area
            }
        };
        if let Some(review) = self.review_mut() {
            review.changing = false;
            review.edit = Some((key, area.clone()));
        }
        area.update(cx, |area, cx| area.focus(window, cx));
        cx.notify();
    }

    /// Cancel, and Esc in the Edit box: the bar shuts and the draft stands as it was.
    fn review_cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(review) = self.review_mut() {
            review.edit = None;
        }
        window.focus(&self.review_focus.list, cx);
        cx.notify();
    }

    /// Approve with fixes, and ⌘↵ in the Edit box: the draft is approved with his words in place of the Job's, or as written
    /// where he changed nothing.
    fn review_send_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.review_open_ask(cx) else { return };
        let Some(area) = self.review().and_then(|review| review.edit.as_ref()).filter(|(on, _)| *on == item.key()).map(|(_, area)| area.clone()) else { return };
        let words = area.read(cx).value().to_string();
        if words.trim().is_empty() {
            area.update(cx, |area, cx| area.focus(window, cx));
            return;
        }
        if Some(words.as_str()) == self.draft_text(&item, cx).as_deref() {
            self.review_approve(window, cx);
            return;
        }
        let body = fixed_body(&item.job, &item.id, words.clone());
        self.sheet_settles(&item.job, &item.id, Some(super::acts::Answer::Fixed(words)), cx);
        self.review_waiting(&item, Verdict::Approved, "/sessions/waiting/answer", body, window, cx);
    }

    /// `D`: the chosen ask off his list, with no verdict. A build has no Dismiss: it is tried or sent back.
    fn review_dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((item, row)) = self.review_current() else { return };
        match item.kind {
            Kind::Artifact => {
                self.as_job(&item.job, |this| this.review_parts_dismiss(&item, window, cx));
                self.review_after(&item.key(), window, cx);
            }
            // A build is dismissed by the Ready to try row that waits on him for it, as on Needs you.
            Kind::Build => {
                if row.verdict.is_some() {
                    return;
                }
                let Some(waiting) = self.record(&item.job, cx).and_then(|record| record.waiting_for.iter().find(|one| one.source == WaitingSource::ReadyToTry).map(|one| one.id.clone())) else { return };
                let body = ipc::encode(&ipc::DismissWaiting { session_id: SessionId::carried(item.job.clone()), item_id: waiting, reason: None, misread: false });
                self.review_waiting(&item, Verdict::Dismissed, "/sessions/waiting/dismiss", body, window, cx);
            }
            Kind::Question => {
                if row.verdict.is_some() {
                    return;
                }
                let body = ipc::encode(&ipc::DismissWaiting { session_id: SessionId::carried(item.job.clone()), item_id: item.id.clone(), reason: None, misread: false });
                self.review_waiting(&item, Verdict::Dismissed, "/sessions/waiting/dismiss", body, window, cx);
            }
            Kind::Plan | Kind::Sketch | Kind::Document | Kind::Draft => {
                if row.verdict.is_some() || item.read {
                    return;
                }
                let body = dismiss_batch(&[&item]);
                self.review_send(vec![item], Verdict::Dismissed, "/sessions/items/dismiss", ipc::encode(&body), window, cx);
            }
        }
    }

    /// A question answered by its option `choice`, or in his words.
    fn review_answer_question(&mut self, item: &Item, choice: Option<usize>, words: Option<String>, extras: ipc::AnswerExtras, window: &mut Window, cx: &mut Context<Self>) {
        if choice.is_none() && words.is_none() {
            return;
        }
        let body = ipc::encode(&ipc::AnswerWaiting {
            session_id: SessionId::carried(item.job.clone()),
            item_id: item.id.clone(),
            choice: choice.map(|at| at as u32),
            text: words,
            mode: None,
            fixed: None,
            extras,
        });
        self.keep_words(&note_key(&item.job, &item.id), "", cx);
        self.review_waiting(item, Verdict::Answered, "/sessions/waiting/answer", body, window, cx);
    }

    /// A waiting row answered or dismissed on its own: the row takes the verdict in the frame of the press and the next ask comes
    /// up; a refusal puts it back with Fleet's sentence.
    #[allow(clippy::too_many_arguments)]
    fn review_waiting(&mut self, item: &Item, verdict: Verdict, route: &'static str, body: Result<String, ipc::Unencodable>, window: &mut Window, cx: &mut Context<Self>) {
        let key = item.key();
        if let Some(review) = self.review_mut() {
            review.rows.insert(key.clone(), Row { verdict: Some(verdict), refused: None });
        }
        // It goes from Needs you and the thread's foot at once, as an answer given there does (`settle_row`).
        let job = SharedString::from(item.job.clone());
        if let Some(page) = self.pages.get_mut(&job) {
            page.dock.hidden.insert(item.id.clone());
        }
        self.want_needs(&job, cx);
        self.sync(&job, cx);
        let (back, back_job, back_item) = (key.clone(), job.clone(), item.id.clone());
        let body = body.map_err(|why| format!("{why:?}"));
        self.post_with(
            job,
            route,
            body,
            move |this, why, cx| {
                if let Some(page) = this.pages.get_mut(&back_job) {
                    page.dock.hidden.remove(&back_item);
                }
                if let Some(review) = this.review_mut() {
                    review.rows.insert(back, Row { verdict: None, refused: Some(why.to_string()) });
                }
                cx.notify();
            },
            window,
            cx,
        );
        self.review_after(&key, window, cx);
    }

    /// The rows take the verdict in the frame of the press, the next ask comes up, and Fleet's outcomes put right any it refused.
    fn review_send(&mut self, items: Vec<Item>, verdict: Verdict, route: &'static str, body: Result<String, ipc::Unencodable>, window: &mut Window, cx: &mut Context<Self>) {
        let keys: Vec<String> = items.iter().map(Item::key).collect();
        let from = self.review().and_then(|review| review.at.clone());
        if let Some(review) = self.review_mut() {
            for key in &keys {
                review.rows.insert(key.clone(), Row { verdict: Some(verdict), refused: None });
            }
        }
        // The thread's cards of each item fold to the verdict at once, as a press on the card folds it.
        for item in &items {
            let answer = match verdict {
                Verdict::Approved => super::acts::Answer::Approved,
                Verdict::ChangesAsked => super::acts::Answer::SentBack(self.review_notes_text(item, cx).trim().to_string()),
                Verdict::Dismissed | Verdict::Answered => super::acts::Answer::Withdrawn,
            };
            self.sheet_settles(&item.job, &item.id, Some(answer), cx);
        }
        let pairs: Vec<(String, String)> = items.iter().map(|item| (item.job.clone(), item.id.clone())).collect();
        // A sent verdict's words are spent: the box is empty for the next time.
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
        if let Some(from) = from {
            self.review_after(&from, window, cx);
        }
        let landed = move |this: &mut JobView, answer: Result<Vec<u8>, String>, _window: &mut Window, cx: &mut Context<JobView>| {
            let outcomes = answer.and_then(|bytes| ipc::decode::<ItemsSettled>("ItemsSettled", &bytes).map(|settled| settled.outcomes).map_err(|why| why.to_string()));
            this.review_landed(&keys, verdict, outcomes, cx);
            // One Fleet refused stands asking on its cards again.
            for (job, item) in &pairs {
                if this.review().and_then(|review| review.rows.get(&key_of(job, item))).is_some_and(|row| row.verdict.is_none()) {
                    this.sheet_settles(job, item, None, cx);
                }
            }
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

    /// After an answer the bar shuts, the next ask comes up, and the keys go back to the list.
    fn review_after(&mut self, from: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(review) = self.review_mut() {
            review.changing = false;
            review.edit = None;
        }
        self.review_advance(Some(from), window, cx);
        if self.review_open() {
            window.focus(&self.review_focus.list, cx);
        }
        cx.notify();
    }

    /// The next ask still asking after `from` comes up, and the last answer closes the sheet.
    fn review_advance(&mut self, from: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(review) = self.review() else { return };
        let order = order(&review.groups);
        let rows: HashMap<String, Row> = review.items().map(|item| (item.key(), self.item_row(item, review, cx))).collect();
        let listed: Vec<String> = order.into_iter().filter(|key| review.item(key).is_some_and(|item| !item.read)).collect();
        match next_open(&listed, &rows, from.unwrap_or_default()) {
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

    /// A press or a digit on a question's option: it is chosen, and Send answer sends it.
    fn review_choose_option(&mut self, key: &str, at: usize, cx: &mut Context<Self>) {
        if let Some(review) = self.review_mut() {
            review.choice.insert(key.to_string(), at);
        }
        cx.notify();
    }

    // ---- keys ------------------------------------------------------------------------------

    /// What the sheet does with the thread's acts while it is up. True where it took the act.
    pub(super) fn review_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.review_open() {
            return false;
        }
        // ⌘↵ reaches the sheet as Send: in the bar's box it sends his words, and anywhere else the ask's own answer.
        if act.is("send_message") {
            if self.review_editing(window, cx) {
                self.review_send_edit(window, cx);
            } else if self.review_noting(window, cx) {
                self.review_send_changes(window, cx);
            } else if let Some(key) = self.focused_card_box(window, cx) {
                // A part's Ask for changes box in the panel sends as its Send does.
                self.commit_box(&key, window, cx);
            } else {
                self.review_primary(window, cx);
            }
            return true;
        }
        // The cards behind take none of the thread's keys while the sheet is up.
        act.is("thread_send_back") || act.is("thread_yes") || act.is("thread_change") || act.is("thread_no") || act.is("needs_you") || act.is("needs_you_fold")
    }

    /// The registry's `review sheet` keys.
    fn review_sheet_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) {
        if self.review_noting(window, cx) {
            cx.propagate();
            return;
        }
        match act.id {
            "items_approve" => self.review_primary(window, cx),
            "items_ask_changes" => self.review_open_changes(window, cx),
            "items_edit" => self.review_open_edit(window, cx),
            "items_dismiss" => self.review_dismiss(window, cx),
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
        let keys: Vec<String> = self.review_shown(review, cx).iter().map(Item::key).collect();
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
        // The Judge's box over the bar hears its own Esc and Tab: Esc shuts the box alone, and Tab goes round it.
        if matches!(event.keystroke.key.as_str(), "escape" | "tab") && self.judge_box_focused(window, cx) {
            return;
        }
        match event.keystroke.key.as_str() {
            // One Esc leaves from any depth, and the keys go back to what opened the sheet. The picture zoomer, drawn over it
            // with its own focus, hears its Esc first.
            "escape" => {
                // A list open over the bar's words goes first, then the box, then the sheet.
                if self.review_note_offering(cx) {
                    if let Some(note) = self.review().and_then(|review| review.note.as_ref()).map(|held| held.note.clone()) {
                        note.update(cx, |note, cx| note.close_offer(cx));
                    }
                } else if !self.review_close_box(window, cx) {
                    self.close_review(window, cx);
                }
            }
            "tab" => self.review_tab(event.keystroke.modifiers.shift, window, cx),
            _ => {
                let noting = self.review_noting(window, cx);
                if !noting && self.margin_sheet_key(event, window, cx) {
                    cx.stop_propagation();
                    return;
                }
                if self.review_zoom_key(event, noting, window, cx) {
                    cx.stop_propagation();
                }
                return;
            }
        }
        cx.stop_propagation();
    }

    /// Esc in the bar's box or in a box of the chosen artifact's parts: the box shuts, and its words stay kept in it. False where
    /// he is in no box.
    fn review_close_box(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.review().is_some_and(|review| review.edit.is_some()) {
            self.review_cancel_edit(window, cx);
            return true;
        }
        if self.review_noting(window, cx) || self.review().is_some_and(|review| review.changing) {
            self.review_cancel_changes(window, cx);
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
        let mut stops: Vec<FocusHandle> = vec![focus.list.clone(), focus.close.clone()];
        if let Some(item) = self.review_open_ask(cx) {
            let ask = self.ask_of(&item);
            let changing = self.review().is_some_and(|review| review.changing);
            let editing = self.review().and_then(|review| review.edit.as_ref()).map(|(_, area)| area.read(cx).focus_handle(cx));
            if let Some(edit) = editing {
                stops.push(edit);
                stops.push(focus.cancel.clone());
                stops.push(focus.send.clone());
            } else if changing {
                if let Some(note) = self.review().and_then(|review| review.note.as_ref()) {
                    stops.push(note.area.read(cx).focus_handle(cx));
                }
                stops.push(focus.cancel.clone());
                stops.push(focus.send.clone());
            } else {
                stops.push(focus.approve.clone());
                if self.review_offers_edit(&item, cx) {
                    stops.push(focus.edit.clone());
                }
                if self.review_offers_second(&item, ask, cx) {
                    stops.push(focus.changes.clone());
                }
                stops.push(focus.dismiss.clone());
                let press = self.sheet_plan(&item, cx).and_then(|(key, _)| self.pages.get(item.job.as_str())?.keys.spot_of(&format!("judge-press:{key}")));
                stops.extend(press.map(|spot| spot.focus));
            }
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

    /// The list's own keys: Home and End go to an end, ↑ and ↓ a step where the registry's keys are not heard, and a digit
    /// chooses that take or option of the chosen ask.
    fn review_list_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let mods = event.keystroke.modifiers;
        if mods.alt || mods.platform || mods.control || mods.shift {
            return;
        }
        let key = event.keystroke.key.as_str();
        if matches!(key, "home" | "end" | "up" | "down") {
            self.review_step(key, window, cx);
            cx.stop_propagation();
            return;
        }
        let Some(item) = self.review_open_ask(cx) else { return };
        match self.ask_of(&item) {
            Ask::Takes => {
                let Some(vote) = self.sheet_vote(&item) else { return };
                let Some((takes, true, _)) = self.takes_of_key_in(&item.job, &vote) else { return };
                if let Some(at) = super::parts_card::take_of_key(key, takes.len()) {
                    self.as_job(&item.job, |this| this.choose_take(&vote, at, cx));
                    cx.stop_propagation();
                }
            }
            Ask::Question => {
                if let Some(at) = super::parts_card::take_of_key(key, self.question_options(&item, cx).len()) {
                    self.review_choose_option(&item.key(), at, cx);
                    cx.stop_propagation();
                }
            }
            _ => {}
        }
    }

    /// Shift+F10 and the menu key, on the list: the chosen item's menu, under its row.
    pub(super) fn review_menu_key(&mut self, _: &ItemMenu, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = self.review().and_then(|review| review.at.clone()) else { return };
        let Some(bounds) = self.review_rows.borrow().get(&at).copied() else { return };
        self.review_menu(&at, under(bounds), true, window, cx);
    }

    /// The ask's menu at `at`: the bar's answers, in its order.
    fn review_menu(&mut self, key: &str, at: gpui::Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.review_open_ask(cx).filter(|item| item.key() == key) else { return };
        let ask = self.ask_of(&item);
        let (label, ready) = self.review_primary_label(&item, ask, cx);
        let mut entries = Vec::new();
        if ready {
            entries.push(Entry::Act { id: "approve", label: label.into(), icon: Some("check"), danger: false });
        }
        if self.review_offers_edit(&item, cx) {
            entries.push(Entry::Act { id: "edit", label: "Edit".into(), icon: Some("square-pen"), danger: false });
        }
        if self.review_offers_second(&item, ask, cx) {
            entries.push(Entry::Act { id: "changes", label: ask.second().into(), icon: Some("pencil"), danger: false });
        }
        entries.push(Entry::Act { id: "dismiss", label: "Dismiss".into(), icon: Some("x"), danger: false });
        let back = Some(self.review_focus.list.clone());
        self.open_pop_back(
            "Ask",
            entries,
            at,
            from_keys,
            back,
            move |this, id, window, cx| match id {
                "approve" => this.review_primary(window, cx),
                "changes" => this.review_open_changes(window, cx),
                "edit" => this.review_open_edit(window, cx),
                "dismiss" => this.review_dismiss(window, cx),
                _ => {}
            },
            window,
            cx,
        );
    }

    // ---- drawing ---------------------------------------------------------------------------

    /// The sheet over the window, or nothing while it is shut: the list at the left, the chosen ask beside it.
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
        let now = chrono::Local::now();

        let shown = self.review_shown(review, cx);
        let mut rows: Vec<AnyElement> = Vec::new();
        for group in &review.groups {
            let items: Vec<&Item> = shown.iter().filter(|item| item.job == group.job).collect();
            if items.is_empty() {
                continue;
            }
            // From a Job the list is that Job's alone, and needs no head.
            if review.scope == Scope::All {
                rows.push(self.review_group_head(group, rows.is_empty(), look));
            }
            rows.extend(items.into_iter().map(|item| self.review_row(item, review, now, look, cx)));
        }
        let list = labelled(
            div()
                .id("review-list")
                .track_focus(&self.review_focus.list)
                .on_action(cx.listener(Self::review_menu_key))
                .on_key_down(cx.listener(Self::review_list_key))
                .flex_none()
                .w(list_w)
                .h_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_3))
                .overflow_y_scroll()
                .bg(look.color(token::BG_SUNKEN))
                .border_r(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .children(rows),
            Role::List,
            "Asks",
        );

        // What a picture and a row of takes are sized to: the detail's own room.
        self.sheet.set(gpui::size((wide - list_w - look.length(token::BORDER_WIDTH) * 2.).max(px(1.)), tall));
        let detail = current.as_ref().map(|item| self.review_detail(item, review, look, cx));
        // On a plan the Judge's keys are heard anywhere in the sheet, as on the thread's card. Deeper than the sheet's own
        // scope, they hand a key on to it wherever no note has focus.
        let judged = current.as_ref().and_then(|item| self.sheet_plan(item, cx));
        let inner = div().size_full().flex().child(list).children(detail).when_some(judged, |inner, (key, version)| {
            inner
                .key_context(ActionScope::Plan.key_context().unwrap_or_default())
                .on_action(cx.listener(move |this, act: &crate::keymap::Act, window, cx| this.plan_act(act, SHEET, &key, version, true, window, cx)))
        });
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
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG) * 1.5)
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .child(inner),
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

    /// A Job's head in the list, where the list is every Job's: its state's glyph and its name.
    fn review_group_head(&self, group: &Group, first: bool, look: &Look) -> AnyElement {
        let state = group.state.map(|state| {
            let (icon, said) = crate::jobs::glyph_of(state, None);
            labelled(
                div()
                    .id(SharedString::from(format!("review-job-state:{}", group.job)))
                    .flex_none()
                    .tooltip(tip(said.clone()))
                    .child(glyph(look, icon, Size::Small, look.color(crate::jobs::hue_of(state)))),
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
                .px(look.length(token::SPACE_3))
                .pt(look.length(if first { token::SPACE_1 } else { token::SPACE_4 }))
                .pb(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_MUTED))
                .children(state)
                .child(div().flex_1().min_w_0().child(SharedString::from(group.title.clone()))),
            Role::Heading,
            group.title.clone(),
        )
        .into_any_element()
    }

    /// The kind's tile: its glyph on its one hue, tinted.
    fn review_kind_tile(&self, key: &str, ask: Ask, look: &Look) -> AnyElement {
        let (icon, said, hue) = ask.mark();
        labelled(
            div()
                .id(SharedString::from(format!("review-kind:{key}")))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM) + look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(hue).opacity(0.16))
                .tooltip(tip(said))
                .child(glyph(look, icon, Size::Large, look.color(hue))),
            Role::Image,
            said,
        )
        .into_any_element()
    }

    /// One ask of the list: its tile, its question wrapped whole, and what it wants with its age. The chosen one stands raised
    /// with a hairline round it.
    fn review_row(&self, item: &Item, review: &ReviewState, now: chrono::DateTime<chrono::Local>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = item.key();
        let row = self.item_row(item, review, cx);
        let ask = self.ask_of(item);
        let on = review.at.as_deref() == Some(key.as_str());
        let (pick, menu_item, kept) = (key.clone(), key.clone(), key.clone());
        let rows = Rc::clone(&self.review_rows);
        let wants = match super::rows::age(&item.since, now) {
            Some(age) => format!("{} \u{b7} {age}", ask.wants()),
            None => ask.wants().to_string(),
        };
        let body = div()
            .id(SharedString::from(format!("review-item:{key}")))
            .relative()
            .flex_none()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if on { look.color(token::BORDER_DEFAULT) } else { gpui::transparent_black() })
            .when(on, |row| row.bg(look.color(token::BG_RAISED)).shadow(look.shadows(token::SHADOW_CARD)))
            .when(!on, |row| row.hover(|style| style.bg(look.color(token::SURFACE_ROW_HOVER))))
            .cursor_pointer()
            .child(self.review_kind_tile(&key, ask, look))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1) / 2.)
                    .child(
                        div()
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .font_weight(FontWeight(look.number(if on { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(item.name.clone())),
                    )
                    .child(
                        div()
                            .text_size(look.length(token::TEXT_XS))
                            .line_height(look.length(token::LEADING_XS))
                            .text_color(look.color(token::FG_MUTED))
                            .child(SharedString::from(wants)),
                    )
                    .children(row.refused.clone().map(|why| {
                        labelled(
                            div()
                                .id(SharedString::from(format!("review-refused:{key}")))
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_XS))
                                .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                                .child(SharedString::from(why.clone())),
                            Role::Status,
                            why,
                        )
                    })),
            )
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
        labelled(body, Role::ListItem, item.name.clone()).into_any_element()
    }

    /// A button of the bar, with its focus and its press, and its key in a chip where `chip` names one. The key of a button with no
    /// chip is in its tooltip.
    #[allow(clippy::too_many_arguments)]
    fn review_button(
        &self,
        look: &Look,
        id: &'static str,
        said: SharedString,
        tone: Tone,
        chip: Option<&'static str>,
        key: Option<&'static str>,
        focus: &FocusHandle,
        on: bool,
        press: fn(&mut JobView, &mut Window, &mut Context<JobView>),
        cx: &Context<Self>,
    ) -> AnyElement {
        let primary = tone == Tone::Primary;
        let chip = chip.map(|chip| {
            let plate = kbd(look, chip);
            if primary {
                plate.bg(gpui::transparent_black()).border_color(look.color(token::TEXT_ON_ACCENT).opacity(0.45)).text_color(look.color(token::TEXT_ON_ACCENT))
            } else {
                plate
            }
        });
        let base = button(look, id, &said, tone, false, None, on)
            .track_focus(focus)
            .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
            .children(chip)
            .when_some(key, |b, key| b.tooltip(tip(format!("{said}, {key}"))))
            .when(!on, |b| b.opacity(0.5))
            .when(on, |b| {
                b.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| press(this, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") && !event.keystroke.modifiers.platform {
                        press(this, window, cx);
                        cx.stop_propagation();
                    }
                }))
            });
        labelled(base, Role::Button, said).into_any_element()
    }

    /// The chosen ask: its question as the head with Close, one line of background, its body, and the answer bar.
    fn review_detail(&self, item: &Item, review: &ReviewState, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = item.key();
        let ask = self.ask_of(item);
        let close = labelled(
            div()
                .id("review-close")
                .track_focus(&self.review_focus.close)
                .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Close, Esc"))
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
        let background = self.review_background(item, cx).map(|line| {
            div()
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_MUTED))
                .line_clamp(1)
                .child(SharedString::from(line))
        });
        // The Operator's read runs under the question and Close both, to the body's right edge.
        let head = labelled(
            div()
                .id(SharedString::from(format!("review-head:{key}")))
                .flex_none()
                .flex()
                .flex_col()
                .px(look.length(token::SPACE_6))
                .pt(look.length(token::SPACE_4))
                .pb(look.length(token::SPACE_4))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(look.length(token::SPACE_3))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(look.length(token::SPACE_1))
                                .child(
                                    div()
                                        .text_size(look.length(token::TEXT_XL))
                                        .line_height(look.length(token::LEADING_XL))
                                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                        .text_color(look.color(token::FG_DEFAULT))
                                        .child(SharedString::from(item.name.clone())),
                                )
                                .children(background),
                        )
                        .child(close),
                )
                .children(self.operator_reads.get(&key).map(|read| self.review_operator(read, look))),
            Role::Heading,
            item.name.clone(),
        );

        let staged = (ask != Ask::Takes).then(|| self.review_stage(item, look, cx).or_else(|| self.review_page_stage(item, cx))).flatten();
        let parts = item.kind == Kind::Artifact;
        let body = staged.unwrap_or_else(|| {
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
                .py(look.length(token::SPACE_6))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .when(parts, |body| body.track_scroll(&self.body_scroll))
                .child(self.review_body(item, ask, look, cx))
                .into_any_element()
        });
        let bar = self.review_bar(item, ask, review, look, cx);
        labelled(div().id(SharedString::from(format!("review-detail:{key}"))).flex_1().min_w_0().min_h_0().flex().flex_col().child(head).child(body).children(bar), Role::Region, item.name.clone())
            .into_any_element()
    }

    /// The one line under the question that says what it is about: the card's For line, or what a build holds. A plan has none.
    fn review_background(&self, item: &Item, cx: &gpui::App) -> Option<String> {
        let line = match item.kind {
            // A plan's body opens on its summary, so a line above it would say it twice.
            Kind::Plan => None,
            Kind::Build => self.build_card(item, cx).map(|card| card.about),
            _ => item.for_line.clone(),
        };
        line.filter(|line| !line.trim().is_empty() && line.trim() != item.name.trim())
    }

    /// Whether the item puts a question to him. A sketch the Job showed with `asks: false` arrives with `asked` false and has no
    /// answer bar, here as on its card (`sketch_card.rs`) and in its reading (`sketch_read.rs`).
    fn item_asks(&self, item: &Item, cx: &gpui::App) -> bool {
        if item.kind != Kind::Sketch {
            return true;
        }
        self.thread(&item.job, cx).and_then(|held| sketch_card(&held.head, &held.items, &item.id).map(|card| card.asked)).unwrap_or(true)
    }

    /// The answer bar: the ask's own answer, its second, and Dismiss at the far end; or, once the second is pressed, the box with
    /// Cancel and Send. None where the ask no longer asks.
    fn review_bar(&self, item: &Item, ask: Ask, review: &ReviewState, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if self.item_row(item, review, cx).verdict.is_some() || item.read {
            return None;
        }
        let focus = &self.review_focus;
        let changing = review.changing && review.note.as_ref().is_some_and(|note| note.key == item.key());
        let bar = div()
            .id("review-bar")
            .flex_none()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_OVERLAY));
        let row = div().flex().items_center().gap(look.length(token::SPACE_2));
        let editing = review.edit.as_ref().filter(|(on, _)| *on == item.key()).map(|(_, area)| area.clone());
        let bar = if let Some(area) = editing {
            let empty = area.read(cx).value().trim().is_empty();
            let field = labelled(
                div()
                    .id("review-edit")
                    .key_context(crate::keymap::TEXT_BOX)
                    // ⌘↩ approves with the fixes, taken before the text area puts a line break in.
                    .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                        if action.secondary {
                            cx.stop_propagation();
                            this.review_send_edit(window, cx);
                        }
                    }))
                    .w_full()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_STRONG))
                    .bg(look.color(token::BG_RAISED))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(gpui_component::input::Textarea::new(&area).appearance(false).aria_label("Draft text")),
                Role::TextInput,
                "Draft text",
            );
            bar.child(field).child(
                row.child(div().flex_1())
                    .child(self.review_button(look, "review-cancel", "Cancel".into(), Tone::Ghost, None, Some("Esc"), &focus.cancel, true, Self::review_cancel_edit, cx))
                    .child(self.review_button(look, "review-send", "Approve with fixes".into(), Tone::Primary, Some("\u{2318}\u{21b5}"), None, &focus.send, !empty, Self::review_send_edit, cx)),
            )
        } else if changing {
            let (area, note) = review.note.as_ref().map(|held| (held.area.clone(), held.note.clone()))?;
            let empty = area.read(cx).value().trim().is_empty();
            let field = labelled(
                div()
                    .id("review-note")
                    .key_context(crate::keymap::TEXT_BOX)
                    // ⌘↩ sends, taken before the text area puts a line break in.
                    .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                        if action.secondary {
                            cx.stop_propagation();
                            this.review_send_changes(window, cx);
                        }
                    }))
                    .w_full()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_STRONG))
                    .bg(look.color(token::BG_RAISED))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(note),
                Role::TextInput,
                ask.second(),
            );
            let said: SharedString = if ask == Ask::Question { "Send answer".into() } else { "Send".into() };
            bar.child(field).child(
                row.child(div().flex_1())
                    .child(self.review_button(look, "review-cancel", "Cancel".into(), Tone::Ghost, None, Some("Esc"), &focus.cancel, true, Self::review_cancel_changes, cx))
                    .child(self.review_button(look, "review-send", said, Tone::Primary, Some("\u{2318}\u{21b5}"), None, &focus.send, !empty, Self::review_send_changes, cx)),
            )
        } else {
            let (label, ready) = self.review_primary_label(item, ask, cx);
            let second = self.review_offers_second(item, ask, cx);
            bar.child(
                row.child(self.review_button(look, "review-approve", label.into(), Tone::Primary, Some("\u{2318}\u{21b5}"), None, &focus.approve, ready, Self::review_primary, cx))
                    .when(self.review_offers_edit(item, cx), |row| row.child(self.review_button(look, "review-edit-open", "Edit".into(), Tone::Secondary, None, Some("E"), &focus.edit, true, Self::review_open_edit, cx)))
                    .when(second, |row| row.child(self.review_button(look, "review-changes", ask.second().into(), Tone::Secondary, None, Some("C"), &focus.changes, true, Self::review_open_changes, cx)))
                    .child(div().flex_1())
                    .child(self.review_button(look, "review-dismiss", "Dismiss".into(), Tone::Ghost, None, Some("D"), &focus.dismiss, true, Self::review_dismiss, cx))
                    .children(self.review_judge(item, look, cx)),
            )
        };
        Some(labelled(bar, Role::Toolbar, "Answer").into_any_element())
    }

    /// The card key and version of the plan an ask puts to him, while it is the plan the Job has recorded.
    fn sheet_plan(&self, item: &Item, cx: &gpui::App) -> Option<(String, u32)> {
        let card = self.thread(&item.job, cx)?.head.plan.as_ref().filter(|_| item.kind == Kind::Plan)?;
        self.record(&item.job, cx)?.plan.as_ref().filter(|plan| plan.version == card.version).map(|plan| (card.id.clone(), plan.version))
    }

    /// The Judge's press at the bar's end, on a plan: its box opens over the bar, and Read the notes goes to them in the body.
    fn review_judge(&self, item: &Item, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let (key, version) = self.sheet_plan(item, cx)?;
        let page = self.pages.get(item.job.as_str())?;
        let (reading, earlier) = plan_readings(self.record(&item.job, cx)?, version);
        Some(self.judge_press(SHEET, &key, plan_artifact(version), reading, &earlier, page, look, cx))
    }

    /// The body of the chosen ask, by kind: what its card shows, without its answers.
    fn review_body(&self, item: &Item, ask: Ask, look: &Look, cx: &Context<Self>) -> AnyElement {
        let job = item.job.as_str();
        let (Some(held), Some(page)) = (self.thread(job, cx), self.pages.get(job)) else { return self.review_skeleton(look) };
        match item.kind {
            Kind::Artifact if ask == Ask::Takes => self.sheet_takes(item, look, cx).unwrap_or_else(|| self.review_parts(item, look, cx)),
            Kind::Artifact => self.review_parts(item, look, cx),
            Kind::Build => match self.build_card(item, cx) {
                Some(card) => self.review_build(&card, look),
                None => self.review_skeleton(look),
            },
            Kind::Question => self.review_question(item, look, cx),
            Kind::Plan => {
                let card = held.head.plan.as_ref();
                let record = self.record(job, cx);
                match card.and_then(|card| record.and_then(|record| record.plan.as_ref().filter(|plan| plan.version == card.version).map(|plan| (card, record, plan)))) {
                    // The Judge's notes stand where the thread's card pinned them, as the plan is answered here alone.
                    Some((card, record, plan)) => {
                        let pinned = Pinned::of_plan(SHEET, &card.id, record, plan);
                        // A plan being revised keeps the card's body, which marks what changed; else the sheet's own reading.
                        if plan.version >= 2 && plan.changes.as_ref().is_some_and(|changes| !changes.is_empty()) {
                            div().w_full().child(self.plan_body(SHEET, &card.id, plan, false, &pinned, page, look, cx)).into_any_element()
                        } else {
                            self.review_plan(&card.id, plan, &pinned, page, look, cx)
                        }
                    }
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
            // The page itself is a web page that fills the body once it has loaded (`review_page_stage`); till then, its skeleton.
            Kind::Document => self.review_skeleton(look),
            Kind::Draft => match draft_row(&held.items, item.id.strip_prefix("draft:").unwrap_or(&item.id)) {
                // The draft is the body itself, edge to edge between the head and the bar, as a page is.
                Some(SessionRow::Draft { id, text, .. }) => div().w_full().child(self.prose(format!("review-draft:{id}"), text, look, cx)).into_any_element(),
                _ => self.review_skeleton(look),
            },
        }
    }

    /// What the Operator read of the ask, for reading only, under the background line: its words, and the pick it would make, in
    /// the Operator's tint with a hairline all round.
    fn review_operator(&self, read: &OperatorRead, look: &Look) -> AnyElement {
        labelled(
            div()
                .id("review-operator")
                .mt(look.length(token::SPACE_3))
                .w_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::OPERATOR).opacity(0.45))
                .bg(look.color(token::OPERATOR).opacity(0.08))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_1))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::OPERATOR))
                        .child(glyph(look, "sparkles", Size::Small, look.color(token::OPERATOR)))
                        .child("Operator"),
                )
                .child(div().text_color(look.color(token::FG_DEFAULT)).child(read.reading.clone()))
                .children(read.pick.clone().map(|pick| div().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(format!("Would pick: {pick}"))))),
            Role::Group,
            "Operator",
        )
        .into_any_element()
    }

    /// A plan as the sheet reads it: its summary, then each group straight under it, its title, its tasks' marks and what it
    /// gives. Where a group's work lands and who does it are the thread card's to show. The Judge's notes on the whole plan
    /// stand under its summary, and those on a group or a task under what the group gives.
    #[allow(clippy::too_many_arguments)]
    fn review_plan(&self, key: &str, plan: &ipc::SessionPlan, pinned: &[Pinned], page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let groups = plan.groups.iter().map(|group| {
            let marks = group.tasks.iter().map(|task| {
                let (icon, ink, said) = super::plan::task_look(task.state);
                div().id(SharedString::from(format!("review-task:{}", task.id))).flex_none().child(glyph(look, icon, Size::Small, look.color(ink))).tooltip(tip(said))
            });
            let gives = group.gives.clone().filter(|gives| !gives.trim().is_empty() && gives.trim() != group.title.trim());
            labelled(
                div()
                    .id(SharedString::from(format!("review-group:{}", group.id)))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_3))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(look.length(token::TEXT_SM))
                                    .line_height(look.length(token::LEADING_SM))
                                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                                    .text_color(look.color(token::FG_DEFAULT))
                                    .child(SharedString::from(group.title.clone())),
                            )
                            .child(div().flex_none().flex().items_center().gap(look.length(token::SPACE_1)).children(marks)),
                    )
                    .children(gives.map(|gives| {
                        div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(gives))
                    }))
                    .children(self.group_findings(pinned, group, page, look, cx))
                    .children(self.plan_sketches(key, plan, Some((&group.id, &group.title)), page, look, cx)),
                Role::ListItem,
                group.title.clone(),
            )
        });
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .children(Some(plan.summary.trim()).filter(|summary| !summary.is_empty()).map(|summary| {
                div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(summary.to_string()))
            }))
            .children(self.plan_sketches(key, plan, None, page, look, cx))
            .children(self.plan_notes(pinned, page, look, cx))
            .child(labelled(div().id("review-plan-groups").flex().flex_col().gap(look.length(token::SPACE_4)).children(groups), Role::List, "Groups"))
            .into_any_element()
    }

    /// A question's options as rows to choose among, each with what choosing it means, and its number for the key.
    fn review_question(&self, item: &Item, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = item.key();
        let chosen = self.review().and_then(|review| review.choice.get(&key).copied());
        let rows: Vec<AnyElement> = self
            .question_options(item, cx)
            .into_iter()
            .enumerate()
            .map(|(at, option)| {
                let on = chosen == Some(at);
                let press = key.clone();
                let radio = div()
                    .flex_none()
                    .mt(look.length(token::SPACE_1) / 2.)
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::SPACE_4))
                    .rounded_full()
                    .border(look.length(token::BORDER_WIDTH) * 2.)
                    .border_color(look.color(if on { token::ACCENT } else { token::BORDER_STRONG }))
                    .when(on, |radio| radio.child(div().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::ACCENT))));
                let name = if on { format!("Chosen: {}", option.label) } else { option.label.clone() };
                labelled(
                    div()
                        .id(SharedString::from(format!("review-option:{key}:{at}")))
                        .flex()
                        .items_start()
                        .gap(look.length(token::SPACE_3))
                        .px(look.length(token::SPACE_4))
                        .py(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_LG))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(if on { token::ACCENT } else { token::BORDER_SUBTLE }))
                        .bg(look.color(if on { token::SURFACE_CARD } else { token::BG_RAISED }))
                        .when(on, |row| row.shadow(look.shadows(token::SHADOW_CARD)))
                        .cursor_pointer()
                        .hover(|style| style.border_color(look.color(if on { token::ACCENT } else { token::BORDER_STRONG })))
                        .child(radio)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(look.length(token::SPACE_1) / 2.)
                                .child(
                                    div()
                                        .text_size(look.length(token::TEXT_SM))
                                        .line_height(look.length(token::LEADING_SM))
                                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                                        .text_color(look.color(token::FG_DEFAULT))
                                        .child(SharedString::from(option.label.clone())),
                                )
                                .children(option.description.clone().filter(|said| !said.trim().is_empty()).map(|said| {
                                    div()
                                        .text_size(look.length(token::TEXT_XS))
                                        .line_height(look.length(token::LEADING_XS))
                                        .text_color(look.color(token::FG_MUTED))
                                        .child(SharedString::from(said))
                                })),
                        )
                        .children((at < 9).then(|| kbd(look, &(at + 1).to_string())))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.review_choose_option(&press, at, cx);
                            window.focus(&this.review_focus.list, cx);
                        })),
                    Role::RadioButton,
                    name,
                )
                .into_any_element()
            })
            .collect();
        labelled(div().id(SharedString::from(format!("review-options:{key}"))).w_full().flex().flex_col().gap(look.length(token::SPACE_2)).children(rows), Role::RadioGroup, item.name.clone()).into_any_element()
    }

    /// A build to try: where to look and the steps that try it, then what it holds.
    fn review_build(&self, card: &ipc::ThreadReadyToTry, look: &Look) -> AnyElement {
        let heading = |said: &'static str| {
            div()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_MUTED))
                .child(said)
        };
        let steps: Vec<AnyElement> = card
            .steps
            .iter()
            .enumerate()
            .map(|(at, step)| {
                div()
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_3))
                    .child(
                        div()
                            .flex_none()
                            .size(look.length(token::H_CONTROL_SM))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(look.color(token::BG_SUNKEN))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::BORDER_SUBTLE))
                            .text_size(look.length(token::TEXT_XS))
                            .text_color(look.color(token::FG_MUTED))
                            .child(SharedString::from((at + 1).to_string())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h(look.length(token::H_CONTROL_SM))
                            .flex()
                            .items_center()
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(step.say.clone())),
                    )
                    .into_any_element()
            })
            .collect();
        let look_at = card.look.clone().filter(|said| !said.trim().is_empty()).map(|said| {
            div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(said))
        });
        let tip_of: String = card.tip.chars().take(8).collect();
        let holds = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .text_color(look.color(token::FG_DEFAULT))
            .child(glyph(look, "git-branch", Size::Large, look.color(token::FG_MUTED)))
            .child(div().font_family(super::rows::mono(look)).child(SharedString::from(card.branch.clone())))
            .child(div().font_family(super::rows::mono(look)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(tip_of)));
        let block = |children: Vec<AnyElement>| {
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::SURFACE_CARD))
                .children(children)
        };
        let mut how: Vec<AnyElement> = vec![heading("How to try it").into_any_element()];
        how.extend(look_at.map(IntoElement::into_any_element));
        how.extend(steps);
        labelled(
            div()
                .id(SharedString::from(format!("review-build:{}", card.id)))
                .w_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_4))
                .child(block(how))
                .child(block(vec![heading("What it holds").into_any_element(), holds.into_any_element()])),
            Role::Group,
            "Build to try",
        )
        .into_any_element()
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
    fn an_answer_names_each_item_by_job_and_id_with_its_own_notes() {
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
    fn a_dismiss_names_each_item_by_job_and_id() {
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

    fn chosen_name(view: &Entity<JobView>, cx: &mut gpui::VisualTestContext) -> String {
        view.read_with(cx, |view, _| view.review_chosen().map(|item| item.name).expect("an ask is chosen"))
    }

    #[gpui::test]
    fn the_answer_bar_stands_at_the_foot_of_the_chosen_ask_and_a_dismissed_ask_leaves_the_list(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::{drawn, redraw};
        let (view, cx) = sheet_open(cx);
        let first = chosen_name(&view, cx);
        let list = drawn_at(cx, Role::List, "Asks");
        let detail = drawn_at(cx, Role::Region, &first);
        let bar = drawn_at(cx, Role::Toolbar, "Answer");
        let dismiss = drawn_at(cx, Role::Button, "Dismiss");
        assert!(detail.left() >= list.right() - px(1.), "the chosen ask stands beside the list: {detail:?} {list:?}");
        assert!(bar.left() >= detail.left() && bar.right() <= detail.right() + px(1.) && (bar.bottom() - detail.bottom()).abs() <= px(1.), "the bar is the detail's foot: {bar:?} {detail:?}");
        assert!(dismiss.left() >= bar.left() && dismiss.right() <= bar.right() && dismiss.top() >= bar.top() && dismiss.bottom() <= bar.bottom(), "Dismiss is in the bar: {dismiss:?} {bar:?}");
        for gone in ["Approve all", "Dismiss all", "More for every item"] {
            assert!(!drawn(cx, Role::Button, gone), "{gone} is not drawn");
        }
        assert!(drawn(cx, Role::ListItem, &first), "{first} is listed while it asks");

        cx.simulate_click(dismiss.center(), Modifiers::none());
        redraw(cx);
        assert!(!drawn(cx, Role::ListItem, &first), "the dismissed ask leaves the list");
        assert!(verdicts(&view, cx).contains(&Some(Verdict::Dismissed)), "and its row holds the verdict");
        if view.read_with(cx, |view, _| view.review_open()) {
            let next = chosen_name(&view, cx);
            assert_ne!(next, first, "the next ask is chosen");
            assert!(drawn(cx, Role::Region, &next), "and stands in the detail");
        }
    }

    /// The every-Job sheet opened on `mock-04`'s draft, with `mock-01` visited first so its thread and parts are held.
    fn every_job_sheet<'a>(cx: &'a mut gpui::TestAppContext) -> (Entity<JobView>, &'a mut gpui::VisualTestContext) {
        let (view, cx) = super::super::parts_fold::rooted_job(cx, "review-asks", "mock-01", gpui::size(px(1440.), px(900.)), |_| {});
        let item = ItemRef { session_id: SessionId::carried("mock-04".to_string()), item: "draft:dra-401".to_string() };
        view.update_in(cx, |view, window, cx| view.open_review_for(item, None, window, cx));
        super::super::parts_fold::redraw(cx);
        assert_eq!(view.read_with(cx, |view, _| view.shown.as_ref().map(|job| job.to_string())).as_deref(), Some("mock-04"));
        (view, cx)
    }

    fn choose_in(view: &Entity<JobView>, cx: &mut gpui::VisualTestContext, job: &str, kind: Kind) -> String {
        let key = view.read_with(cx, |view, _| view.review().and_then(|review| review.items().find(|item| item.job == job && item.kind == kind).map(Item::key)));
        let key = key.unwrap_or_else(|| panic!("the every-Job sheet lists {job}'s {kind:?}"));
        view.update_in(cx, |view, window, cx| view.review_choose(&key, window, cx));
        super::super::parts_fold::redraw(cx);
        key
    }

    #[gpui::test]
    fn another_jobs_build_is_answered_from_the_every_job_sheet_and_goes_to_that_jobs_session(cx: &mut gpui::TestAppContext) {
        let (view, cx) = every_job_sheet(cx);
        let key = choose_in(&view, cx, "mock-01", Kind::Build);
        view.update_in(cx, |view, window, cx| view.review_primary(window, cx));
        super::super::parts_fold::redraw(cx);
        let sent: Vec<String> = view.read_with(cx, |view, _| view.mock_ready.iter().map(|one| one.session_id.as_str().to_string()).collect());
        assert_eq!(sent, ["mock-01"], "the answer goes to the build's own Job");
        view.read_with(cx, |view, cx| {
            assert_eq!(view.shown.as_deref(), Some("mock-04"), "the Job behind the sheet stays");
            let review = view.review().expect("the sheet stays up while asks are left");
            let item = review.item(&key).expect("the build is still an item").clone();
            assert_eq!(view.item_row(&item, review, cx).verdict, Some(Verdict::Approved));
            assert_ne!(review.at.as_deref(), Some(key.as_str()), "the next ask comes up");
        });
    }

    /// `jobs-screens`' mock-01 with the review sheet open on its plan v2, which a judge has read. `tune` changes the record first.
    fn plan_sheet<'a>(cx: &'a mut gpui::TestAppContext, tune: impl FnOnce(&mut Vec<ipc::SessionRecord>)) -> (Entity<JobView>, &'a mut gpui::VisualTestContext) {
        let (view, cx) = super::super::parts_fold::rooted_job(cx, "jobs-screens", "mock-01", gpui::size(px(1440.), px(900.)), tune);
        let item = ItemRef { session_id: SessionId::carried("mock-01".to_string()), item: "plan:2".to_string() };
        view.update_in(cx, |view, window, cx| view.open_review_for(item, None, window, cx));
        super::super::parts_fold::redraw(cx);
        assert_eq!(view.read_with(cx, |view, _| view.review_chosen().map(|item| item.kind)), Some(Kind::Plan), "the sheet opens on the plan");
        (view, cx)
    }

    const UNANCHORED: &str = "Finding: Nothing says what the chip shows when the branch has no upstream";
    const ON_A_TASK: &str = "Finding: The tooltip names a figure no row carries yet";
    const ON_A_GROUP: &str = "Finding: behind_by counts commits on a branch with no merge base as zero";
    const PUT_AWAY: &str = "Finding dismissed: A row refreshed mid-press shows the old colour";

    #[gpui::test]
    fn a_waiting_plan_in_the_sheet_carries_the_judges_notes_and_their_keys(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::{drawn, redraw};
        let (_view, cx) = plan_sheet(cx, |_| {});
        for note in [UNANCHORED, ON_A_TASK, ON_A_GROUP, PUT_AWAY] {
            assert!(drawn(cx, Role::Group, note), "the revised plan in the sheet draws {note}");
        }
        let sheet = drawn_at(cx, Role::Dialog, "Review");
        let list = drawn_at(cx, Role::List, "Asks");
        let note = drawn_at(cx, Role::Group, UNANCHORED);
        assert!(note.left() >= list.right() && note.right() <= sheet.right() + px(1.), "the note stands in the sheet beside its list: {note:?} {sheet:?}");
        assert!(drawn(cx, Role::Button, "Judge plan"), "the Judge's press stands in the bar");

        cx.update(|_, cx| crate::keymap::install(&crate::keymap::Rebinding::default(), cx));
        cx.simulate_keystrokes("r");
        redraw(cx);
        assert!(drawn(cx, Role::Dialog, "Judge"), "R opens the Judge's box from the sheet");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(!drawn(cx, Role::Dialog, "Judge"), "Esc shuts the box");
        assert!(drawn(cx, Role::Dialog, "Review"), "and leaves the sheet up");

        let note = drawn_at(cx, Role::Group, UNANCHORED);
        cx.simulate_click(note.center(), Modifiers::none());
        redraw(cx);
        cx.simulate_keystrokes("enter");
        redraw(cx);
        assert!(drawn(cx, Role::Group, "Finding sent: Nothing says what the chip shows when the branch has no upstream"), "Enter sends the note in focus");
        assert!(drawn(cx, Role::Dialog, "Review"), "and the sheet stays on the plan");
    }

    #[gpui::test]
    fn a_plan_the_sheet_reads_whole_pins_each_note_where_the_card_did(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::drawn;
        let (_view, cx) = plan_sheet(cx, |jobs| {
            let job = jobs.iter_mut().find(|job| job.id.as_str() == "mock-01").expect("mock-01 is in the fixture");
            job.plan.as_mut().expect("mock-01 holds a plan").changes = None;
        });
        assert!(drawn(cx, Role::List, "Groups"), "an unrevised plan is the sheet's own reading");
        for note in [UNANCHORED, ON_A_TASK, ON_A_GROUP, PUT_AWAY] {
            assert!(drawn(cx, Role::Group, note), "the sheet's reading draws {note}");
        }
        let group = drawn_at(cx, Role::ListItem, "Fleet reports how far behind");
        let on_group = drawn_at(cx, Role::Group, ON_A_GROUP);
        assert!(on_group.top() >= group.top() && on_group.bottom() <= group.bottom() + px(1.), "a group's note stands in its group: {on_group:?} {group:?}");
        let groups = drawn_at(cx, Role::List, "Groups");
        assert!(drawn_at(cx, Role::Group, UNANCHORED).bottom() <= groups.top() + px(1.), "a note on the whole plan stands above the groups");
    }

    #[gpui::test]
    fn another_jobs_takes_is_picked_from_the_every_job_sheet_and_goes_to_that_jobs_session(cx: &mut gpui::TestAppContext) {
        let (view, cx) = every_job_sheet(cx);
        let key = choose_in(&view, cx, "mock-01", Kind::Artifact);
        let ready = |view: &Entity<JobView>, cx: &mut gpui::VisualTestContext| {
            view.read_with(cx, |view, cx| {
                let item = view.review_chosen().expect("chosen");
                view.review_primary_label(&item, view.ask_of(&item), cx).1
            })
        };
        assert!(ready(&view, cx), "the vote starts on the Job's pick, so its answer is one press");
        cx.simulate_keystrokes("1");
        super::super::parts_fold::redraw(cx);
        let first = view.read_with(cx, |view, _| {
            let item = view.review_chosen().expect("chosen");
            let vote = view.sheet_vote(&item).expect("the other Job's vote");
            let (takes, _, chosen) = view.takes_of_key_in("mock-01", &vote).expect("its takes");
            assert_eq!(chosen, Some(0), "a digit picks a take of the other Job's vote");
            takes[0].id.clone()
        });
        assert!(ready(&view, cx), "and the answer still sends");
        view.update_in(cx, |view, window, cx| view.review_primary(window, cx));
        super::super::parts_fold::redraw(cx);
        let sent: Vec<(String, Option<String>)> = view.read_with(cx, |view, _| view.mock_part_posts().iter().map(|one| (one.session_id.as_str().to_string(), one.take.clone())).collect());
        assert_eq!(sent.len(), 1, "one pick is sent: {sent:?}");
        assert_eq!(sent[0].0, "mock-01", "to the vote's own Job");
        assert_eq!(sent[0].1.as_deref(), Some(first.as_str()), "carrying the take he picked");
        view.read_with(cx, |view, _| {
            assert_eq!(view.shown.as_deref(), Some("mock-04"));
            assert_ne!(view.review().and_then(|review| review.at.clone()).as_deref(), Some(key.as_str()), "the next ask comes up");
        });
    }
}
