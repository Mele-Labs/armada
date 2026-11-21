//! Reviews: the pull requests waiting on him in an inbox, and the one he opens on free panes, with
//! what changed since he last looked. Drawn on mock data until Fleet's half lands; `model.rs` is the
//! view model Fleet's views fill then, and nothing here works one out.

mod bar;
mod inbox;
pub mod layout;
mod live;
mod markdown;
mod mock;
pub(crate) mod model;
mod panes;
mod rail_width;
mod tabs;
pub mod walk;

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    div, prelude::*, px, App, Context, DismissEvent, Entity, FocusHandle, Focusable as _, FontWeight, KeyDownEvent, Pixels, Point,
    Role, ScrollHandle, SharedString, Subscription, Task, Window,
};
use gpui_component::input::{Input, InputEvent, InputState, TextareaState};
use ipc::PullReviewList;

use crate::dropdown::{Dropdown, Entry, Picked};
use crate::job_view::pane_tree::{self as tree, Arrangement, Dir, Shape, Size};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{card, sans, Btn, Weight};
use layout::{LayoutControl, Layouts};
use live::Find;
pub(crate) use live::{post_run, read_runs, read_tools};
use model::{Critique, DraftOf, Explain, Group, Inbox, Mine, Page, Summary, Verdict, Viewed};

/// Each tab a pull request's page can hold: its kind, as the pane tree keys it, and its name.
pub const KINDS: [(&str, &str); 8] = [
    ("pr-overview", "Overview"),
    ("pr-change-set", "Change set"),
    ("pr-changed", "What changed"),
    ("pr-files", "Files"),
    ("pr-conversation", "Conversation"),
    ("pr-checks", "Checks"),
    ("pr-findings", "Review comments"),
    ("pr-comments", "Comments"),
];

/// A tab kind's name, as its tab and VoiceOver say it.
pub fn said(kind: &str) -> &'static str {
    KINDS.iter().find(|(one, _)| *one == kind).map_or("Tab", |(_, name)| name)
}

/// The three columns he agreed: the inbox beside, the page's main tabs, and a side pane for what a review adds.
pub fn default_arrangement() -> Arrangement {
    tree::build(
        &Shape::split(Dir::Row, 0.6, Shape::pane(&["pr-overview", "pr-changed", "pr-files", "pr-conversation", "pr-checks"]), Shape::pane(&["pr-findings", "pr-comments"])),
        None,
        &[],
    )
}

/// How long the mock forge takes to read one pull request's page.
const PAGE_READ: Duration = Duration::from_millis(1500);
/// How long the mock forge takes to read the inbox again.
const REFRESH_READ: Duration = Duration::from_millis(4000);
/// How long a rate-limited inbox waits before reading again.
const LIMIT_REREAD: Duration = Duration::from_secs(30);
/// How often the age of the lists in the header is said again.
const AGE_TICK: Duration = Duration::from_secs(30);
/// The menu on a practice he corrected.
const CHANGE_REASON: &str = "Change the reason";
const UNDO: &str = "Undo";
/// How long the mock review run takes over a range.
const CRITIQUE_RUN: Duration = Duration::from_millis(3000);
/// How long the mock summary of the new commits takes to write.
const SUMMARY_WRITE: Duration = Duration::from_millis(2500);
/// How long a change set's mock How it works or How to test takes.
const EXPLAIN_RUN: Duration = Duration::from_millis(2000);
/// How long the mock Find problems takes to write the change sets and their findings.
const FIND_RUN: Duration = Duration::from_millis(3000);
/// How long the mock Summarize, Explain or Simplify takes to write its answer.
const REWRITE_RUN: Duration = Duration::from_millis(2000);
/// How long the head says Link copied after Copy link.
const COPIED_FOR: Duration = Duration::from_millis(2000);
/// The change set tab's lines above its first hunk, so Show in change set can scroll to one.
pub(super) const HUNKS_AT: usize = 6;

/// A question the screen holds up before it acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    SaveAs,
    Rename,
}

/// A text box whose words are kept under a key until they are sent.
pub(crate) struct Boxed {
    pub state: Entity<TextareaState>,
    _changed: Subscription,
}

/// The one-line reason field under a practice he called wrong, open until Save or Cancel.
pub(crate) struct Correcting {
    pub at: model::PracticeAt,
    pub wrong: model::Wrong,
    pub field: Entity<InputState>,
    _enter: Subscription,
}

/// The sheet under Review these changes: what to look for, how hard and anything to focus on,
/// open until Start or Cancel.
pub(crate) struct AskingReview {
    /// The button it opened from, which names it.
    pub title: &'static str,
    pub at: Point<Pixels>,
    pub ask: model::ReviewAsk,
    pub focus: Entity<InputState>,
    _enter: Subscription,
}

/// A menu that is open, and what listens to it.
struct Pop {
    menu: Entity<Dropdown>,
    back: Option<FocusHandle>,
    _subs: [Subscription; 2],
}

pub struct ReviewsScreen {
    pub within: FocusHandle,
    /// The inbox's rows, which the arrows move through.
    pub rows: FocusHandle,
    /// The inbox's own scroll, kept as it is when fresh rows swap in.
    inbox_scroll: ScrollHandle,
    inbox: Inbox,
    folded: HashSet<Group>,
    lists: Option<PullReviewList>,
    search: Entity<InputState>,
    focused: Option<u64>,
    /// The pull request whose page is drawn.
    open: Option<u64>,
    /// Pages read so far, with what he did on them.
    pages: HashMap<u64, Page>,
    /// Pressed while a refresh runs: it opens once its fresh page is read.
    waiting: Option<u64>,
    /// His last visit's pull request as it was kept, which opens at once on his first visit.
    last_visit: Option<Page>,
    refreshed: bool,
    arrangement: Arrangement,
    /// The Files rail's width, as he last dragged it.
    rail_width: f32,
    layouts: Layouts,
    control: Entity<LayoutControl>,
    /// The panes' area as last drawn, so a drop that would squeeze a pane is refused.
    area: Rc<Cell<Option<Size>>>,
    /// Each pane's tab strip width as last drawn, so a crowded strip draws the tabs behind as glyphs.
    strips: Rc<std::cell::RefCell<HashMap<String, gpui::Pixels>>>,
    /// The pane a tab is dragged over, where on it, and whether it can land there.
    over: Option<(String, tree::Zone, bool)>,
    lifted: Option<String>,
    /// Files shows the whole pull request, not the change since he looked.
    whole: bool,
    /// Files he opened or shut by hand, by pull request and path; the rest are open until viewed.
    opened: HashMap<(u64, String), bool>,
    drafts: Vec<DraftOf>,
    boxes: HashMap<String, Boxed>,
    summary: Option<Boxed>,
    /// Review changes is open, in place of the panes, until Send or Cancel.
    sheet: bool,
    sent_notes: HashSet<u64>,
    /// Findings that went out as drafts with a review, by pull request and finding, and the day they
    /// went, so none goes twice.
    sent_findings: HashMap<(u64, String), String>,
    /// Sent lines he unfolded, by pull request and day.
    sent_open: HashSet<(u64, String)>,
    /// Each suggestion's words, which he can edit before it becomes a draft, by pull request and finding.
    suggestions: HashMap<(u64, String), Boxed>,
    dismissed: HashSet<(u64, String)>,
    /// His corrections of practices' results, until Fleet stores them.
    corrections: HashMap<model::PracticeAt, model::Correction>,
    /// The practice whose reason he is writing.
    correcting: Option<Correcting>,
    /// Change sets whose set-aside practices are unfolded, by pull request and set.
    aside_open: HashSet<(u64, String)>,
    /// Summaries being written, by pull request.
    writing: HashMap<u64, Task<()>>,
    /// Each change set's How it works and How to test: absent until run, None while it runs.
    explains: HashMap<(u64, String, Explain), Option<String>>,
    explain_runs: HashMap<(u64, String, Explain), Task<()>>,
    /// The change set the Change set tab shows, by pull request.
    chosen: HashMap<u64, String>,
    /// The Change set tab's own scroll, which Show in change set moves to a hunk.
    set_scroll: ScrollHandle,
    asking: Option<Ask>,
    /// The review sheet, while it is open.
    asking_review: Option<AskingReview>,
    /// The review tools a review can be asked to look for.
    review_tools: Vec<model::ReviewTool>,
    /// Opened from a ready review's toast or banner: once its page shows, Review comments comes to the front.
    findings_next: Option<u64>,
    name_box: Entity<InputState>,
    dialog: FocusHandle,
    pop: Option<Pop>,
    refresh: Option<Task<()>>,
    reading: Option<Task<()>>,
    critique: Option<Task<()>>,
    /// Each file's coloured lines, by path and then by line text, read off the main thread.
    hues: HashMap<String, crate::syntax::Read>,
    /// Pages whose diffs are being coloured.
    colouring: HashMap<u64, Task<()>>,
    /// The patches read for colour, by a hash of path and patch.
    coloured: HashSet<u64>,
    /// In a walk the mock's work waits for the walk's `Lands` step instead of its own clock, so no
    /// step races a person's Next.
    walked: bool,
    next_draft: u64,
    /// Fleet's half, on the live app; a walk plays the mock with none.
    live: Option<live::Live>,
    /// A walk's stand-in for Fleet's calls: the reviews asked of him, opened as he opens each, for the strip's figure.
    asked: Option<ipc::StandingCalls>,
    /// His login on the forge, so the head can say whether a review is asked of him.
    me: String,
    /// Copy link was pressed on this pull request: the head says so until the task clears it.
    copied: Option<(u64, Task<()>)>,
    /// The mock's Find problems runs, by pull request.
    finding: HashMap<u64, Task<()>>,
    /// The pull requests he started a review on in this run of the app: their rows keep the bot mark
    /// through a refresh that read the lists before Fleet recorded the run.
    ran_here: HashSet<u64>,
    /// The card above each page's description: which of Summarize, Explain or Simplify it shows.
    rewrites: HashMap<u64, model::Rewrite>,
    /// Each answer as it stands, by pull request and question.
    rewritten: HashMap<(u64, model::Rewrite), model::Answer>,
    rewrite_runs: HashMap<(u64, model::Rewrite), Task<()>>,
    /// Files' own scroll, which a file pressed in the rail moves to that file.
    files_scroll: ScrollHandle,
    _subs: Vec<Subscription>,
}

/// The reviews screen a walk is playing, so its `Lands` step reaches it.
struct Walked(gpui::WeakEntity<ReviewsScreen>);

impl gpui::Global for Walked {}

/// The reviews screen a walk is playing, if one is drawn.
pub fn walked(cx: &App) -> Option<Entity<ReviewsScreen>> {
    cx.try_global::<Walked>().and_then(|walked| walked.0.upgrade())
}

/// His last ask, by repository (`owner/name`), for this run of the app: the Reviews sheet and a review toast's
/// picker both open on it, and Start in either keeps it.
#[derive(Default)]
struct LastAsks(HashMap<String, model::ReviewAsk>);

impl gpui::Global for LastAsks {}

pub(crate) fn last_ask(repository: &str, cx: &App) -> Option<model::ReviewAsk> {
    cx.try_global::<LastAsks>().and_then(|kept| kept.0.get(repository).cloned())
}

pub(crate) fn keep_ask(repository: String, ask: model::ReviewAsk, cx: &mut App) {
    cx.default_global::<LastAsks>().0.insert(repository, ask);
}

impl ReviewsScreen {
    pub fn mock(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search or ask"));
        let name_box = cx.new(|cx| InputState::new(window, cx).placeholder("Layout name"));
        let me = cx.entity();
        let control = cx.new(|cx| LayoutControl::new(&me, cx));
        let subs = vec![
            cx.subscribe_in(&search, window, |_, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe_in(&name_box, window, |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.name_layout(window, cx);
                }
            }),
        ];
        let arrangement = default_arrangement();
        ReviewsScreen {
            within: cx.focus_handle(),
            rows: cx.focus_handle().tab_stop(true),
            inbox: mock::inbox(),
            folded: Group::ALL.into_iter().filter(|group| group.starts_folded()).collect(),
            lists: None,
            search,
            focused: None,
            open: None,
            pages: HashMap::new(),
            waiting: None,
            last_visit: mock::kept(mock::LAST_OPEN),
            refreshed: false,
            layouts: Layouts::new(arrangement.clone()),
            arrangement,
            rail_width: rail_width::load(),
            control,
            area: Rc::default(),
            strips: Rc::default(),
            over: None,
            lifted: None,
            whole: false,
            opened: HashMap::new(),
            drafts: mock::drafts(),
            boxes: HashMap::new(),
            summary: None,
            sheet: false,
            sent_notes: HashSet::new(),
            sent_findings: mock::sent_findings(),
            sent_open: HashSet::new(),
            suggestions: HashMap::new(),
            dismissed: HashSet::new(),
            corrections: HashMap::new(),
            correcting: None,
            aside_open: HashSet::new(),
            writing: HashMap::new(),
            explains: HashMap::new(),
            explain_runs: HashMap::new(),
            chosen: HashMap::new(),
            inbox_scroll: ScrollHandle::new(),
            set_scroll: ScrollHandle::new(),
            asking: None,
            asking_review: None,
            review_tools: mock::review_tools(),
            findings_next: None,
            name_box,
            dialog: cx.focus_handle(),
            pop: None,
            refresh: None,
            reading: None,
            critique: None,
            hues: HashMap::new(),
            colouring: HashMap::new(),
            coloured: HashSet::new(),
            walked: false,
            next_draft: 1,
            live: None,
            asked: None,
            me: mock::ME.into(),
            copied: None,
            finding: HashMap::new(),
            ran_here: HashSet::new(),
            rewrites: HashMap::new(),
            rewritten: HashMap::new(),
            rewrite_runs: HashMap::new(),
            files_scroll: ScrollHandle::new(),
            _subs: subs,
        }
    }

    /// Brought to the front: the keys go to the inbox, and the first visit opens the last pull request
    /// as it was kept while the forge is read again.
    pub fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.rows, cx);
        if !self.refreshed && self.refresh.is_none() {
            if let Some(page) = self.last_visit.take() {
                let number = page.number;
                self.pages.insert(number, page);
                self.show(number, window, cx);
            }
            self.start_refresh(window, cx);
        }
        cx.notify();
    }

    /// Another place in front: the title bar's slot is empty again.
    pub fn leave(&mut self, cx: &mut Context<Self>) {
        if let Some(number) = self.open.filter(|_| self.is_live()) {
            self.live_seen(number, cx);
        }
        if let Some(bar) = crate::titlebar::live(cx) {
            bar.update(cx, |bar, cx| bar.fill_job_slot(None, cx));
        }
    }

    fn start_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_live() {
            self.live_refresh(window, cx);
            return;
        }
        self.inbox.refreshing = true;
        let wait = self.wait(REFRESH_READ, cx);
        self.refresh = Some(cx.spawn_in(window, async move |this, cx| {
            wait.await;
            let _ = this.update_in(cx, |this, window, cx| this.end_refresh(window, cx));
        }));
        cx.notify();
    }

    /// The forge has answered: the inbox takes its fresh rows, a kept page takes its fresh parts, and
    /// the pull request waiting on it opens.
    fn end_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.inbox.refreshing {
            self.inbox = mock::refreshed(std::mem::take(&mut self.inbox));
        }
        self.refreshed = true;
        self.refresh = None;
        self.reading = None;
        let kept: Vec<u64> = self.pages.values().filter(|page| page.kept).map(|page| page.number).collect();
        for number in kept {
            self.land_fresh(number, window, cx);
        }
        if let Some(number) = self.waiting.take() {
            self.show(number, window, cx);
        }
        cx.notify();
    }

    /// A kept page's fresh read: its checks, commits since, replies and what came since swap in where
    /// they waited in grey, and what he did on the page stays. Closed or gone, the page says so.
    fn land_fresh(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(was) = self.pages.get_mut(&number) else { return };
        let Some(mut fresh) = mock::page(number) else {
            was.kept = false;
            was.ended = Some(model::Ended::Gone);
            return;
        };
        fresh.critique = std::mem::replace(&mut was.critique, Critique::None);
        *was = fresh;
        if self.open == Some(number) {
            let page = &self.pages[&number];
            self.whole = !page.changed() || page.seen.as_ref().is_some_and(|seen| seen.gone);
        }
        self.suggestion_boxes(number, window, cx);
        self.write_summary(number, window, cx);
        self.colour(number, cx);
    }

    /// Colours a page's diffs off the main thread, every file and change-set hunk at once, and redraws
    /// once when they are all read.
    fn colour(&mut self, number: u64, cx: &mut Context<Self>) {
        let Some(page) = self.pages.get(&number) else { return };
        let mut patches: Vec<(String, String)> = Vec::new();
        for file in &page.files {
            patches.push((file.path.clone(), file.patch.clone()));
            patches.extend(file.patch_since.clone().map(|patch| (file.path.clone(), patch)));
        }
        for set in &page.overview.sets {
            patches.extend(set.hunks.iter().map(|hunk| (hunk.path.clone(), hunk.patch.clone())));
        }
        // A patch read before is not read again.
        patches.retain(|one| {
            use std::hash::{BuildHasher, BuildHasherDefault};
            self.coloured.insert(BuildHasherDefault::<std::collections::hash_map::DefaultHasher>::default().hash_one(one))
        });
        if patches.is_empty() {
            return;
        }
        let reading = cx.background_executor().spawn(async move {
            let mut read: HashMap<String, crate::syntax::Read> = HashMap::new();
            for (path, patch) in patches {
                let drawn = crate::diff::file_diff_of(&path, Some(&patch), None);
                let lines = drawn.files.into_iter().next().map(|file| file.lines).unwrap_or_default();
                let found = crate::syntax::read(&path, &lines);
                read.entry(path).or_default().extend(found);
            }
            read
        });
        let task = cx.spawn(async move |this, cx| {
            let read = reading.await;
            let _ = this.update(cx, |this, cx| {
                for (path, found) in read {
                    this.hues.entry(path).or_default().extend(found);
                }
                this.colouring.remove(&number);
                cx.notify();
            });
        });
        self.colouring.insert(number, task);
    }

    /// Each line's coloured code, for the lines of one file's diff.
    pub(super) fn hues_of(&self, path: &str, lines: &[crate::diff::DiffLine]) -> Vec<Option<crate::syntax::Pieces>> {
        let Some(read) = self.hues.get(path) else { return Vec::new() };
        lines.iter().map(|line| read.get(line.text.as_str()).cloned()).collect()
    }

    /// A pull request asked of him, from its toast or its banner: opened as a press on its row opens it, though the
    /// inbox may not have listed it yet.
    pub fn open_asked(&mut self, asked: &ipc::ReviewAsked, window: &mut Window, cx: &mut Context<Self>) {
        self.live_hold_asked(asked);
        self.press_row(asked.number, window, cx);
    }

    /// A ready review, from its toast or its banner: the pull request opens as `open_asked` opens it, on Review comments.
    pub fn open_asked_findings(&mut self, asked: &ipc::ReviewAsked, window: &mut Window, cx: &mut Context<Self>) {
        self.findings_next = Some(asked.number);
        self.open_asked(asked, window, cx);
    }

    /// The pull request being opened or open, for a test.
    #[cfg(test)]
    pub fn showing(&self) -> Option<u64> {
        self.waiting.or(self.open)
    }

    /// A row pressed. While the inbox refreshes, its page is read first and opens once read.
    fn press_row(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.focused = Some(number);
        window.focus(&self.rows, cx);
        if self.is_live() {
            self.live_open(number, window, cx);
            return;
        }
        if !self.inbox.refreshing || self.pages.get(&number).is_some_and(|page| page.kept) {
            self.waiting = None;
            self.show(number, window, cx);
            return;
        }
        self.waiting = Some(number);
        let wait = self.wait(PAGE_READ, cx);
        self.reading = Some(cx.spawn_in(window, async move |this, cx| {
            wait.await;
            let _ = this.update_in(cx, |this, window, cx| this.end_refresh(window, cx));
        }));
        cx.notify();
    }

    /// Open a pull request's page on its Overview.
    fn show(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        if !self.pages.contains_key(&number) {
            if self.is_live() {
                return;
            }
            let Some(page) = mock::page(number) else { return };
            self.pages.insert(number, page);
        }
        // He left the one before: the head he leaves it at is what its next visit's change is since.
        if let Some(was) = self.open.filter(|was| *was != number && self.is_live()) {
            self.live_seen(was, cx);
        }
        // In a walk, opening a pull request asked of him opens its call, as Fleet does on the live app.
        if let Some(asked) = self.asked.as_mut() {
            for call in asked.calls.iter_mut().filter(|call| call.review.as_ref().is_some_and(|review| review.number == number)) {
                call.opened = true;
            }
            self.tell_asked(cx);
        }
        let changed = self.pages[&number].changed() || (self.pages[&number].kept && self.pages[&number].seen.is_some());
        let gone = self.pages[&number].seen.as_ref().is_some_and(|seen| seen.gone);
        self.open = Some(number);
        self.focused = Some(number);
        if self.is_live() {
            self.live_tools(number);
        }
        self.sheet = false;
        self.whole = !changed || gone;
        self.bring_kind("pr-overview", cx);
        if self.findings_next == Some(number) {
            self.findings_next = None;
            self.bring_kind("pr-findings", cx);
        }
        self.summary = Some(self.boxed(&format!("reviews:summary:{number}"), None, 3, 8, window, cx));
        let mine: Vec<DraftOf> = self.drafts.iter().filter(|draft| draft.number == number).cloned().collect();
        for draft in mine {
            if !self.boxes.contains_key(&draft.id) {
                let boxed = self.boxed(&format!("reviews:draft:{}", draft.id), Some(&draft.body), 2, 6, window, cx);
                self.boxes.insert(draft.id.clone(), boxed);
            }
        }
        self.suggestion_boxes(number, window, cx);
        self.write_summary(number, window, cx);
        self.colour(number, cx);
        cx.notify();
    }

    /// A pull request that changed since he looked gets its summary written once for its head.
    fn write_summary(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_live() {
            self.live_summary(number, window, cx);
            return;
        }
        let Some(page) = self.pages.get_mut(&number) else { return };
        if page.kept || !page.changed() || page.seen.as_ref().is_some_and(|seen| seen.gone) || page.summary != Summary::None {
            return;
        }
        page.summary = Summary::Writing;
        let wait = self.wait(SUMMARY_WRITE, cx);
        let task = cx.spawn_in(window, async move |this, cx| {
            wait.await;
            let _ = this.update(cx, |this, cx| this.end_summary(number, cx));
        });
        self.writing.insert(number, task);
    }

    fn end_summary(&mut self, number: u64, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(&number) {
            page.summary = Summary::Written(mock::summary(number));
        }
        self.writing.remove(&number);
        cx.notify();
    }

    /// A text box for each suggestion of the page's critique and each finding on its Overview, holding
    /// its words until he adds or dismisses it.
    fn suggestion_boxes(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.pages.get(&number) else { return };
        let mut said: Vec<(String, String)> = page.overview.flags.iter().map(|flag| (flag.id.clone(), flag.comment.clone())).collect();
        if let Critique::Finished { findings, .. } = &page.critique {
            said.extend(findings.iter().map(|finding| (finding.id.clone(), finding.comment.clone())));
        }
        for (id, comment) in said {
            let key = (number, id.clone());
            if !self.suggestions.contains_key(&key) {
                let boxed = self.boxed(&format!("reviews:suggestion:{number}:{id}"), Some(&comment), 2, 6, window, cx);
                self.suggestions.insert(key, boxed);
            }
        }
    }

    /// Dismiss: the suggestion leaves Review comments without becoming a draft.
    fn dismiss_suggestion(&mut self, finding: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        self.let_go(number, finding, window, cx);
        self.dismissed.insert((number, finding.to_string()));
        cx.notify();
    }

    /// A suggestion's box is about to go: the keys it held go back to the list, not to nothing.
    fn let_go(&self, number: u64, finding: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.suggestions.get(&(number, finding.to_string())).is_some_and(|boxed| boxed.state.focus_handle(cx).contains_focused(window, cx)) {
            window.focus(&self.rows, cx);
        }
    }

    /// Edit: the keys go to the draft's box.
    fn edit_draft(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(boxed) = self.boxes.get(id) {
            let keys = boxed.state.focus_handle(cx);
            window.focus(&keys, cx);
        }
    }

    fn toggle_sent(&mut self, day: &str, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        let key = (number, day.to_string());
        if !self.sent_open.remove(&key) {
            self.sent_open.insert(key);
        }
        cx.notify();
    }

    /// How it works or How to test on the change set in front: its answer reads under the set's foot.
    fn run_explain(&mut self, set: String, explain: Explain, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        if self.is_live() {
            return;
        }
        let key = (number, set.clone(), explain);
        if self.explain_runs.contains_key(&key) {
            return;
        }
        self.explains.insert(key.clone(), None);
        let wait = self.wait(EXPLAIN_RUN, cx);
        let task = cx.spawn_in(window, {
            let key = key.clone();
            async move |this, cx| {
                wait.await;
                let _ = this.update(cx, |this, cx| this.end_explain(key, cx));
            }
        });
        self.explain_runs.insert(key, task);
        cx.notify();
    }

    fn end_explain(&mut self, key: (u64, String, Explain), cx: &mut Context<Self>) {
        let (number, set, explain) = key.clone();
        self.explains.insert(key.clone(), Some(mock::explanation(number, &set, explain)));
        self.explain_runs.remove(&key);
        // The answer reads under the foot, so the set still in view scrolls down to it.
        if self.open == Some(number) && self.rail_on().as_deref() == Some(set.as_str()) {
            self.set_scroll.scroll_to_bottom();
        }
        cx.notify();
    }

    /// Find problems on the mock: the change sets and what was found in them land after a moment.
    pub(super) fn mock_find_problems(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        if self.finding.contains_key(&number) {
            return;
        }
        self.mark_reviewing(number);
        let wait = self.wait(FIND_RUN, cx);
        let task = cx.spawn_in(window, async move |this, cx| {
            wait.await;
            let _ = this.update_in(cx, |this, window, cx| this.end_find(number, window, cx));
        });
        self.finding.insert(number, task);
        cx.notify();
    }

    fn end_find(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(&number) {
            let why = std::mem::take(&mut page.overview.why);
            page.overview = model::Overview { why, ..mock::found(number) };
        }
        self.finding.remove(&number);
        self.suggestion_boxes(number, window, cx);
        self.colour(number, cx);
        cx.notify();
    }

    /// Summarize, Explain or Simplify: its card opens above the description, written once and kept.
    pub(super) fn ask_rewrite(&mut self, rewrite: model::Rewrite, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        self.rewrites.insert(number, rewrite);
        if self.is_live() {
            self.live_rewrite(number, rewrite, window, cx);
            cx.notify();
            return;
        }
        let key = (number, rewrite);
        if self.rewritten.contains_key(&key) {
            cx.notify();
            return;
        }
        self.rewritten.insert(key, model::Answer::Writing);
        let wait = self.wait(REWRITE_RUN, cx);
        let task = cx.spawn_in(window, async move |this, cx| {
            wait.await;
            let _ = this.update(cx, |this, cx| this.end_rewrite(key, cx));
        });
        self.rewrite_runs.insert(key, task);
        cx.notify();
    }

    fn end_rewrite(&mut self, key: (u64, model::Rewrite), cx: &mut Context<Self>) {
        self.rewritten.insert(key, model::Answer::Written(mock::rewrite(key.0, key.1)));
        self.rewrite_runs.remove(&key);
        cx.notify();
    }

    /// The card's x: the description stands alone again. The answer is kept for the next press.
    pub(super) fn close_rewrite(&mut self, cx: &mut Context<Self>) {
        if let Some(number) = self.open {
            self.rewrites.remove(&number);
        }
        cx.notify();
    }

    /// Copy link: the pull request's address on the clipboard, and the head says so for a moment.
    pub(super) fn copy_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.page().filter(|page| !page.address.is_empty()) else { return };
        let number = page.number;
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(page.address.clone()));
        let timer = cx.background_executor().timer(COPIED_FOR);
        let task = cx.spawn_in(window, async move |this, cx| {
            timer.await;
            let _ = this.update(cx, |this, cx| {
                this.copied = None;
                cx.notify();
            });
        });
        self.copied = Some((number, task));
        cx.notify();
    }

    pub(super) fn open_in_browser(&self, cx: &mut Context<Self>) {
        if let Some(page) = self.page().filter(|page| !page.address.is_empty()) {
            cx.open_url(&page.address);
        }
    }

    /// A file pressed in the rail: Files in front, on the whole pull request where the file is not in
    /// the change since he looked, the file open, and its card at the top once it is drawn.
    pub(super) fn show_file(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.page() else { return };
        let number = page.number;
        if !page.files.iter().any(|file| file.path == path && file.since) {
            self.whole = true;
        }
        self.opened.insert((number, path.to_string()), true);
        self.bring_kind("pr-files", cx);
        // Next-frame work runs before that frame draws, so the card is looked for one frame later.
        let name = format!("File: {path}");
        cx.on_next_frame(window, move |_, window, cx| cx.on_next_frame(window, move |this, window, cx| this.scroll_to_file(&name, window, cx)));
        cx.notify();
    }

    fn scroll_to_file(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(card) = cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), Role::Article, name).map(|drawn| drawn.bounds)) else { return };
        let body = self.files_scroll.bounds();
        let (offset, most) = (self.files_scroll.offset(), self.files_scroll.max_offset());
        let y = (offset.y - (card.origin.y - body.origin.y)).min(px(0.)).max(-most.y);
        self.files_scroll.set_offset(gpui::point(offset.x, y));
        cx.notify();
    }

    /// A change set pressed in the rail, or its Review: the Change set tab shows it, from its top.
    fn choose_set(&mut self, set: &str, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        self.chosen.insert(number, set.to_string());
        self.set_scroll.set_offset(gpui::point(px(0.), px(0.)));
        self.bring_kind("pr-change-set", cx);
    }

    /// Show in change set: the finding's set in front, scrolled to its hunk once the set is drawn.
    fn show_flag(&mut self, flag: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(found) = self.page().and_then(|page| page.flag(flag)).cloned() else { return };
        let at = self.page().and_then(|page| page.set(&found.set)).and_then(|set| set.hunks.iter().position(|hunk| hunk.flag.as_deref() == Some(flag)));
        self.choose_set(&found.set, cx);
        if let Some(at) = at {
            cx.on_next_frame(window, move |this, _, cx| {
                this.set_scroll.scroll_to_top_of_item(HUNKS_AT + at);
                cx.notify();
            });
        }
    }

    /// The Overview row in the rail: the Overview in front again.
    fn show_overview(&mut self, cx: &mut Context<Self>) {
        self.bring_kind("pr-overview", cx);
    }

    /// The rail's filled row: the chosen set while the Change set tab is in front, else the Overview.
    fn rail_on(&self) -> Option<String> {
        let number = self.open?;
        let front = tree::panes_of(&self.arrangement.root).into_iter().any(|pane| {
            let active = pane.active.clone().or_else(|| pane.tabs.first().map(|tab| tab.id.clone()));
            pane.tabs.iter().any(|tab| tab.kind == "pr-change-set" && Some(&tab.id) == active.as_ref())
        });
        if !front {
            return None;
        }
        self.chosen.get(&number).cloned().or_else(|| self.pages.get(&number).and_then(|page| page.overview.sets.first()).map(|set| set.id.clone()))
    }

    /// A text box that keeps its words under `keep`, starting from what was kept or else `first`.
    fn boxed(&mut self, keep: &str, first: Option<&str>, rows: usize, most: usize, window: &mut Window, cx: &mut Context<Self>) -> Boxed {
        let state = cx.new(|cx| TextareaState::new(window, cx).auto_grow(rows, most));
        if let Some(words) = crate::drafts::kept(keep, cx).or_else(|| first.map(str::to_string)) {
            state.update(cx, |state, cx| state.set_value(words, window, cx));
        }
        let keep = keep.to_string();
        let changed = cx.subscribe_in(&state, window, move |_, state, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                let text = state.read(cx).value().to_string();
                if text.is_empty() {
                    crate::drafts::clear(&keep, cx);
                } else {
                    crate::drafts::keep(&keep, &text, cx);
                }
            }
        });
        Boxed { state, _changed: changed }
    }

    /// The tab of this kind in front, added beside the Overview where no pane holds one, or else to the first pane.
    fn bring_kind(&mut self, kind: &str, cx: &mut Context<Self>) {
        let tab = tree::panes_of(&self.arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == kind).map(|tab| tab.id.clone());
        let next = match tab {
            Some(tab) => tree::rearrange(&self.arrangement, |was| tree::activate(was, &tab), None, true),
            None => {
                let panes = tree::panes_of(&self.arrangement.root);
                let beside = panes.iter().find(|pane| pane.tabs.iter().any(|tab| tab.kind == "pr-overview")).or(panes.first());
                let first = beside.map(|pane| pane.id.clone()).unwrap_or_default();
                tree::rearrange(&self.arrangement, |was| tree::add_tab(was, &first, kind, tree::Extras::default()), None, true)
            }
        };
        if let Some(next) = next {
            self.arrangement = next;
        }
        cx.notify();
    }

    fn page(&self) -> Option<&Page> {
        self.open.and_then(|number| self.pages.get(&number))
    }

    fn page_mut(&mut self) -> Option<&mut Page> {
        self.open.and_then(|number| self.pages.get_mut(&number))
    }

    /// Diff since you looked: Files in front, on the change since then.
    fn diff_since(&mut self, cx: &mut Context<Self>) {
        let gone = self.page().and_then(|page| page.seen.as_ref()).is_none_or(|seen| seen.gone);
        self.whole = gone;
        self.bring_kind("pr-files", cx);
    }

    /// A file's Viewed box. A file the author changed after he viewed it starts clear.
    fn toggle_viewed(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let mut now = None;
        if let Some(file) = self.page_mut().and_then(|page| page.files.iter_mut().find(|file| file.path == path)) {
            file.viewed = if file.viewed == Viewed::Yes { Viewed::No } else { Viewed::Yes };
            now = Some(file.viewed == Viewed::Yes);
        }
        if let (Some(viewed), Some(number)) = (now.filter(|_| self.is_live()), self.open) {
            self.live_viewed(number, path.to_string(), viewed, window, cx);
        }
        // Ticking it folds it and clearing it opens it, whatever his hand did before.
        if let Some(number) = self.open {
            self.opened.remove(&(number, path.to_string()));
        }
        cx.notify();
    }

    /// Whether a file's diff is drawn: as he left it, or open until he marks it viewed.
    fn file_open(&self, path: &str, viewed: bool) -> bool {
        self.open.and_then(|number| self.opened.get(&(number, path.to_string())).copied()).unwrap_or(!viewed)
    }

    /// A press on a file's row opens or shuts it.
    fn toggle_file(&mut self, path: &str, viewed: bool, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        let open = self.file_open(path, viewed);
        self.opened.insert((number, path.to_string()), !open);
        cx.notify();
    }

    fn ask(&mut self, ask: Ask, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = Some(ask);
        let first = if ask == Ask::Rename { self.layouts.name().to_string() } else { String::new() };
        self.name_box.update(cx, |field, cx| {
            field.set_value(first, window, cx);
            field.focus(window, cx);
        });
        cx.notify();
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = None;
        window.focus(&self.rows, cx);
        cx.notify();
    }

    /// Review these changes: the sheet opens under its button with his last ask on this repository.
    pub(super) fn open_review(&mut self, title: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.page() else { return };
        let ask = last_ask(&page.repository, cx).unwrap_or_else(|| model::ReviewAsk::first(&self.review_tools));
        let at = under_label(Role::Button, title, Point::default(), window, cx);
        let focus = cx.new(|cx| InputState::new(window, cx).placeholder("Optional"));
        let was = ask.focus.clone();
        focus.update(cx, |field, cx| field.set_value(was, window, cx));
        let enter = cx.subscribe_in(&focus, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.start_review(window, cx);
            }
        });
        self.asking_review = Some(AskingReview { title, at, ask, focus, _enter: enter });
        window.focus(&self.dialog, cx);
        cx.notify();
    }

    pub(super) fn toggle_review_tool(&mut self, tool: &str, cx: &mut Context<Self>) {
        if let Some(open) = self.asking_review.as_mut() {
            open.ask.toggle(tool, &self.review_tools);
            cx.notify();
        }
    }

    pub(super) fn pick_effort(&mut self, effort: model::Effort, cx: &mut Context<Self>) {
        if let Some(open) = self.asking_review.as_mut() {
            open.ask.effort = effort;
            cx.notify();
        }
    }

    pub(super) fn cancel_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.asking_review.take().is_some() {
            window.focus(&self.rows, cx);
            cx.notify();
        }
    }

    /// Start: the ask is remembered for the repository, the change sets and the overview run where
    /// they have not read this commit, and each ticked tool runs. Nothing ticked runs those two alone.
    pub(super) fn start_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mut open) = self.asking_review.take() else { return };
        open.ask.focus = open.focus.read(cx).value().trim().to_string();
        let Some(page) = self.page() else { return };
        let (number, repository) = (page.number, page.repository.clone());
        let found = page.kept
            || match self.live_finds(number) {
                Some(finds) => finds.flags == Find::Done || finds.sets == Find::Running || finds.flags == Find::Running,
                None => page.overview.read || self.finding.contains_key(&number),
            };
        keep_ask(repository, open.ask.clone(), cx);
        self.sheet = false;
        if !found {
            self.find_problems(window, cx);
        }
        if open.ask.tools.is_empty() {
            self.bring_kind("pr-overview", cx);
            window.focus(&self.rows, cx);
            cx.notify();
            return;
        }
        self.start_critique(open.ask, window, cx);
    }

    /// He started a review on it here: its row takes the bot mark at once, and keeps it.
    pub(super) fn mark_reviewing(&mut self, number: u64) {
        self.ran_here.insert(number);
        if let Some(row) = self.inbox.rows.iter_mut().find(|row| row.number == number) {
            row.reviewing = true;
        }
    }

    /// Whether he ran any review on it: a tool, the change sets or the overview, here or before.
    pub(super) fn ran_review(&self, page: &model::Page) -> bool {
        let row = self.inbox.rows.iter().any(|row| row.number == page.number && row.reviewing);
        let found = match self.live_finds(page.number) {
            Some(finds) => finds.sets != Find::NotRun || finds.flags != Find::NotRun,
            None => page.overview.read || self.finding.contains_key(&page.number),
        };
        row || found || !matches!(page.critique, Critique::None)
    }

    /// The review run reads the range for what he asked, and its findings come to Review comments.
    fn start_critique(&mut self, asked: model::ReviewAsk, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        let Some(page) = self.pages.get_mut(&number) else { return };
        let range = page.range();
        page.critique = Critique::Running { range: range.clone(), asked: asked.clone() };
        self.mark_reviewing(number);
        self.bring_kind("pr-findings", cx);
        window.focus(&self.rows, cx);
        if self.is_live() {
            self.live_critique(number, &asked, window, cx);
            cx.notify();
            return;
        }
        let wait = self.wait(CRITIQUE_RUN, cx);
        self.critique = Some(cx.spawn_in(window, async move |this, cx| {
            wait.await;
            let _ = this.update_in(cx, |this, window, cx| this.end_critique(number, window, cx));
        }));
        cx.notify();
    }

    fn end_critique(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(&number) {
            if let Critique::Running { range, asked } = &page.critique {
                let findings = if number == 2198 { mock::retry_findings() } else { Vec::new() };
                page.critique = Critique::Finished { range: range.clone(), findings, asked: asked.clone() };
            }
        }
        self.suggestion_boxes(number, window, cx);
        self.critique = None;
        cx.notify();
    }

    /// How long a piece of the mock's work takes: its own clock, or in a walk until the walk lands it.
    fn wait(&self, time: Duration, cx: &App) -> impl std::future::Future<Output = ()> + 'static {
        let timer = (!self.walked).then(|| cx.background_executor().timer(time));
        async move {
            match timer {
                Some(timer) => timer.await,
                None => std::future::pending().await,
            }
        }
    }

    /// Played by a walk: the mock's work waits for the walk's `Lands` step, not its clock.
    pub fn walk(&mut self, cx: &mut Context<Self>) {
        self.walked = true;
        cx.set_global(Walked(cx.entity().downgrade()));
        self.asked = Some(mock::asked());
        self.tell_asked(cx);
    }

    /// The strip's figure beside Reviews, from the walk's stand-in calls as the live app reads Fleet's.
    fn tell_asked(&self, cx: &mut App) {
        let Some(count) = self.asked.as_ref().map(ipc::StandingCalls::reviews_unopened) else { return };
        crate::titlebar::figure("reviews", count, cx);
    }

    /// The walk's `Lands`: every piece of the mock's work in flight ends now, as its clock would have
    /// ended it. The refresh first, since the page it lands starts a summary of its own.
    pub fn land(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.refresh.is_some() || self.reading.is_some() {
            self.end_refresh(window, cx);
        }
        let writing: Vec<u64> = self.writing.keys().copied().collect();
        for number in writing {
            self.end_summary(number, cx);
        }
        let running: Vec<u64> = self.pages.values().filter(|page| matches!(page.critique, Critique::Running { .. })).map(|page| page.number).collect();
        for number in running {
            self.end_critique(number, window, cx);
        }
        let explaining: Vec<(u64, String, Explain)> = self.explain_runs.keys().cloned().collect();
        for key in explaining {
            self.end_explain(key, cx);
        }
        let finding: Vec<u64> = self.finding.keys().copied().collect();
        for number in finding {
            self.end_find(number, window, cx);
        }
        let rewriting: Vec<(u64, model::Rewrite)> = self.rewrite_runs.keys().copied().collect();
        for key in rewriting {
            self.end_rewrite(key, cx);
        }
        cx.notify();
    }

    /// Add as draft: the finding's comment becomes a draft on its line, to edit before Submit. It is a
    /// critique's suggestion, or a finding on the Overview.
    fn add_draft(&mut self, finding: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        let Some(found) = self.page().and_then(|page| {
            let critique = match &page.critique {
                Critique::Finished { findings, .. } => findings.iter().find(|one| one.id == finding).cloned(),
                _ => None,
            };
            critique.or_else(|| {
                page.flag(finding).map(|flag| model::Finding { id: flag.id.clone(), title: flag.title.clone(), path: flag.path.clone(), line: flag.line, comment: flag.comment.clone() })
            })
        }) else {
            return;
        };
        if self.drafts.iter().any(|draft| draft.finding.as_deref() == Some(finding) && draft.number == number) {
            return;
        }
        let id = format!("d{number}-{}", self.next_draft);
        self.next_draft += 1;
        // His edit of the suggestion is what goes, not what the critique wrote.
        let body = self.suggestions.get(&(number, found.id.clone())).map(|boxed| boxed.state.read(cx).value().to_string()).filter(|said| !said.trim().is_empty()).unwrap_or(found.comment);
        self.let_go(number, &found.id, window, cx);
        let excerpt = self.page().and_then(|page| model::excerpt(&page.files, &found.path, found.line));
        let draft = DraftOf { id: id.clone(), number, path: found.path, line: found.line, body, finding: Some(found.id), excerpt };
        let boxed = self.boxed(&format!("reviews:draft:{id}"), Some(&draft.body), 2, 6, window, cx);
        self.boxes.insert(id, boxed);
        self.drafts.push(draft);
        cx.notify();
    }

    fn discard_draft(&mut self, id: &str, cx: &mut Context<Self>) {
        self.drafts.retain(|draft| draft.id != id);
        self.boxes.remove(id);
        crate::drafts::clear(&format!("reviews:draft:{id}"), cx);
        cx.notify();
    }

    /// Review changes, in place of the panes.
    fn open_sheet(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.sheet = open;
        if !open {
            window.focus(&self.rows, cx);
        }
        cx.notify();
    }

    /// Send: the drafts and the summary go as one review at the head, and the bar says so at once.
    fn submit(&mut self, verdict: Verdict, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        if self.is_live() {
            self.live_submit(verdict, false, window, cx);
            return;
        }
        self.sent_review(number, verdict, false, window, cx);
    }

    /// No problems found's Approve: an approving review at the head with no body and no comments, by
    /// the same send as Review changes. His drafts and summary stay for a later review.
    pub(super) fn approve_bare(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        if self.is_live() {
            self.live_submit(Verdict::Approved, true, window, cx);
            return;
        }
        self.sent_review(number, Verdict::Approved, true, window, cx);
    }

    /// No problems found's Approve with comments: Review changes opens with the summary in focus.
    pub(super) fn approve_with_comments(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_sheet(true, window, cx);
        if let Some(summary) = self.summary.as_ref() {
            summary.state.update(cx, |area, cx| area.focus(window, cx));
        }
    }

    /// The review went: his verdict stands on the page and row. A review that carried his drafts and
    /// summary clears them; a bare approval leaves them.
    fn sent_review(&mut self, number: u64, verdict: Verdict, bare: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !bare {
            self.clear_sent(number, window, cx);
        }
        if let Some(page) = self.pages.get_mut(&number) {
            page.mine = Mine { verdict, on: Some(page.head.clone()), day: Some("today".into()), requested: None, commits: 0, replies: 0 };
        }
        if let Some(row) = self.inbox.rows.iter_mut().find(|row| row.number == number) {
            row.mine = verdict;
            // A review at the head has seen every commit, so none is new to him any more.
            row.new_commits = None;
        }
        self.open_sheet(false, window, cx);
    }

    /// The drafts and summary that went with a review.
    fn clear_sent(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let gone: Vec<String> = self.drafts.iter().filter(|draft| draft.number == number).map(|draft| draft.id.clone()).collect();
        let sent: Vec<String> = self.drafts.iter().filter(|draft| draft.number == number).filter_map(|draft| draft.finding.clone()).collect();
        self.sent_findings.extend(sent.into_iter().map(|finding| ((number, finding), "today".to_string())));
        for id in gone {
            self.discard_draft(&id, cx);
        }
        crate::drafts::clear(&format!("reviews:summary:{number}"), cx);
        self.summary = Some(self.boxed(&format!("reviews:summary:{number}"), None, 3, 8, window, cx));
    }

    fn send_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_live() {
            self.live_send_notes(window, cx);
            return;
        }
        if let Some(number) = self.open {
            self.sent_notes.insert(number);
        }
        cx.notify();
    }

    // ---- Layouts ------------------------------------------------------------------------------------

    fn pick_layout(&mut self, at: usize, cx: &mut Context<Self>) {
        if let Some(arrangement) = self.layouts.pick(at) {
            self.arrangement = arrangement;
            self.reshow(cx);
            if self.is_live() {
                self.live_pick_layout(self.layouts.name().to_string(), cx);
            }
        }
        cx.notify();
    }

    /// The picked layout as it was saved, with the page's own tab in front again.
    fn reset_layout(&mut self, cx: &mut Context<Self>) {
        self.arrangement = self.layouts.arrangement();
        self.reshow(cx);
        cx.notify();
    }

    fn delete_layout(&mut self, cx: &mut Context<Self>) {
        if self.is_live() && !self.layouts.shipped() {
            self.live_delete_layout(self.layouts.name().to_string(), cx);
        }
        self.layouts.delete();
        self.reset_layout(cx);
    }

    /// The Overview in front, as opening the page puts it.
    fn reshow(&mut self, cx: &mut Context<Self>) {
        if self.page().is_some() {
            self.bring_kind("pr-overview", cx);
        }
    }

    /// Save layout or Rename layout, from the naming box.
    fn name_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.name_box.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        let was = self.layouts.name().to_string();
        match self.asking {
            Some(Ask::SaveAs) => self.layouts.save_as(&name, self.arrangement.clone()),
            Some(Ask::Rename) => self.layouts.rename(&name),
            _ => return,
        }
        // Fleet keeps what he named: saved under the new name, the old one deleted on a rename.
        if self.is_live() && self.layouts.name() == name {
            let renamed = (self.asking == Some(Ask::Rename)).then_some(was);
            self.live_save_layout(name, self.layouts.arrangement(), renamed, cx);
        }
        self.dismiss(window, cx);
    }

    // ---- Menus --------------------------------------------------------------------------------------

    /// Open a menu under `at`. `pick` runs with the entry's id, and the menu shuts.
    fn open_menu(
        &mut self,
        name: &str,
        entries: Vec<Entry>,
        at: Point<Pixels>,
        pick: impl Fn(&mut ReviewsScreen, &'static str, &mut Window, &mut Context<ReviewsScreen>) + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let menu = cx.new(|cx| Dropdown::new(name.to_string(), entries, at, gpui::Anchor::TopLeft, false, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, keep } = picked;
            if !keep {
                if let Some(back) = this.pop.take().and_then(|pop| pop.back) {
                    window.focus(&back, cx);
                }
            }
            pick(this, id, window, cx);
            cx.notify();
        });
        let dismissed = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            if let Some(back) = this.pop.take().and_then(|pop| pop.back) {
                window.focus(&back, cx);
            }
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.focus(&keys, cx);
        self.pop = Some(Pop { menu, back: Some(self.rows.clone()), _subs: [picked, dismissed] });
        cx.notify();
    }

    /// Wrong beside a practice: Doesn't apply here or Check is wrong, each then asking why.
    fn open_wrong(&mut self, at: model::PracticeAt, point: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        // A result he corrected offers its reason again and the way back.
        if let Some(fixed) = self.corrections.get(&at).cloned() {
            let entries = vec![
                Entry::Act { id: CHANGE_REASON, label: CHANGE_REASON.into(), icon: None, danger: false },
                Entry::Act { id: UNDO, label: UNDO.into(), icon: None, danger: false },
            ];
            self.open_menu("Wrong", entries, point, move |this, id, window, cx| {
                if id == UNDO {
                    this.undo_correction(&at, window, cx);
                } else {
                    this.start_correcting(at.clone(), fixed.wrong, Some(&fixed.reason), window, cx);
                }
            }, window, cx);
            return;
        }
        let entries = [model::Wrong::DoesNotApply, model::Wrong::CheckIsWrong]
            .into_iter()
            .map(|wrong| Entry::Act { id: wrong.said(), label: wrong.said().into(), icon: None, danger: false })
            .collect();
        self.open_menu("Wrong", entries, point, move |this, id, window, cx| {
            let wrong = if id == model::Wrong::DoesNotApply.said() { model::Wrong::DoesNotApply } else { model::Wrong::CheckIsWrong };
            this.start_correcting(at.clone(), wrong, None, window, cx);
        }, window, cx);
    }

    /// Undo: the result goes back to what the check found, and what he said goes with it.
    fn undo_correction(&mut self, at: &model::PracticeAt, window: &mut Window, cx: &mut Context<Self>) {
        self.corrections.remove(at);
        if self.correcting.as_ref().is_some_and(|open| &open.at == at) {
            self.correcting = None;
        }
        window.focus(&self.within, cx);
        cx.notify();
    }

    fn start_correcting(&mut self, at: model::PracticeAt, wrong: model::Wrong, was: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
        let field = cx.new(|cx| InputState::new(window, cx).placeholder(wrong.asks()));
        if let Some(was) = was {
            let was = was.to_string();
            field.update(cx, |field, cx| field.set_value(was, window, cx));
        }
        let enter = cx.subscribe_in(&field, window, |this, _, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.save_correction(window, cx);
            }
        });
        field.update(cx, |field, cx| field.focus(window, cx));
        self.correcting = Some(Correcting { at, wrong, field, _enter: enter });
        cx.notify();
    }

    /// Save keeps the correction once it has a reason; an empty one saves nothing.
    fn save_correction(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.correcting.as_ref() else { return };
        let reason = open.field.read(cx).value().trim().to_string();
        if reason.is_empty() {
            return;
        }
        if let Some(open) = self.correcting.take() {
            self.corrections.insert(open.at, model::Correction { wrong: open.wrong, reason });
        }
        window.focus(&self.within, cx);
        cx.notify();
    }

    fn cancel_correcting(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.correcting.take().is_some() {
            window.focus(&self.within, cx);
            cx.notify();
        }
    }

    fn fold_aside(&mut self, number: u64, set: String, cx: &mut Context<Self>) {
        let key = (number, set);
        if !self.aside_open.remove(&key) {
            self.aside_open.insert(key);
        }
        cx.notify();
    }

    /// Lists: the forge lists a pull request is on, as a filter on the inbox.
    fn open_lists(&mut self, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let on = self.lists;
        let entries = vec![
            Entry::Choice { id: "all", label: "All".into(), on: on.is_none() },
            Entry::Rule,
            Entry::Choice { id: "requested", label: "Requested".into(), on: on == Some(PullReviewList::Requested) },
            Entry::Choice { id: "team", label: "Team".into(), on: on == Some(PullReviewList::Team) },
            Entry::Choice { id: "assigned", label: "Assigned".into(), on: on == Some(PullReviewList::Assigned) },
            Entry::Choice { id: "mine", label: "Mine".into(), on: on == Some(PullReviewList::Mine) },
            Entry::Choice { id: "reviewing", label: model::Group::Reviewing.said().into(), on: on == Some(PullReviewList::Reviewing) },
        ];
        self.open_menu("Lists", entries, at, |this, id, _, cx| {
            this.lists = match id {
                "requested" => Some(PullReviewList::Requested),
                "team" => Some(PullReviewList::Team),
                "assigned" => Some(PullReviewList::Assigned),
                "mine" => Some(PullReviewList::Mine),
                "reviewing" => Some(PullReviewList::Reviewing),
                _ => None,
            };
            cx.notify();
        }, window, cx);
    }

    /// The + on a pane: each kind no pane holds yet.
    fn open_add(&mut self, pane: String, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let missing: Vec<Entry> = KINDS
            .iter()
            .filter(|(kind, _)| !tree::has_kind(&self.arrangement, kind))
            .map(|(kind, name)| Entry::Act { id: kind, label: (*name).into(), icon: Some(tabs::glyph_of(kind)), danger: false })
            .collect();
        let entries = if missing.is_empty() { vec![Entry::Off { label: "Every tab is open".into(), hint: "Drag one here from another pane" }] } else { missing };
        self.open_menu("Add tab", entries, at, move |this, kind, _, cx| {
            if let Some(next) = tree::rearrange(&this.arrangement, |was| tree::add_tab(was, &pane, kind, tree::Extras::default()), None, true) {
                this.arrangement = next;
            }
            cx.notify();
        }, window, cx);
    }

    // ---- Keys ---------------------------------------------------------------------------------------

    /// ⌥1 to ⌥4 bring the main pane's tabs forward, as a Job's panes do; Esc shuts a question.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keys = &event.keystroke;
        if keys.key == "escape" && self.asking.is_some() {
            self.dismiss(window, cx);
            cx.stop_propagation();
            return;
        }
        if keys.key == "escape" && self.asking_review.is_some() {
            self.cancel_review(window, cx);
            cx.stop_propagation();
            return;
        }
        if keys.key == "escape" && self.correcting.is_some() {
            self.cancel_correcting(window, cx);
            cx.stop_propagation();
            return;
        }
        if keys.key == "escape" && self.sheet {
            self.open_sheet(false, window, cx);
            cx.stop_propagation();
            return;
        }
        let digit = keys.key.parse::<usize>().ok().filter(|digit| (1..=4).contains(digit));
        if let Some(digit) = digit.filter(|_| keys.modifiers.alt && !keys.modifiers.shift && !keys.modifiers.platform && !keys.modifiers.control) {
            let tab = tree::panes_of(&self.arrangement.root).first().and_then(|pane| pane.tabs.get(digit - 1)).map(|tab| tab.id.clone());
            if let Some(next) = tab.and_then(|tab| tree::rearrange(&self.arrangement, |was| tree::activate(was, &tab), None, true)) {
                self.arrangement = next;
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }
        if crate::keymap::traverse(event, window, cx) {
            cx.stop_propagation();
        }
    }

    // ---- Drawing ------------------------------------------------------------------------------------

    fn render_dialog(&self, ask: Ask, look: &Look, cx: &mut Context<Self>) -> gpui::AnyElement {
        let (name, title): (&str, String) = match ask {
            Ask::SaveAs => ("Save this layout", "Save this layout".into()),
            Ask::Rename => ("Rename this layout", format!("Rename {}", self.layouts.name())),
        };
        let heading = labelled(
            div()
                .id("reviews-dialog-title")
                .text_size(look.length(token::TEXT_BASE))
                .line_height(look.length(token::LEADING_BASE))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .child(SharedString::from(title.clone())),
            Role::Heading,
            title,
        );
        let said_line = |id: &'static str, words: String| {
            labelled(
                div().id(id).text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(words.clone())),
                Role::Label,
                words,
            )
        };
        let body: gpui::AnyElement = match ask {
            Ask::SaveAs | Ask::Rename => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(said_line("reviews-dialog-said", "Every pull request opens on the picked layout.".into()))
                .child(labelled(div().id("layout-name").key_context(crate::keymap::TEXT_BOX).child(Input::new(&self.name_box)), Role::TextInput, "Layout name"))
                .into_any_element(),
        };
        let (go, icon) = match ask {
            Ask::SaveAs => ("Save layout", "check"),
            Ask::Rename => ("Rename layout", "pencil"),
        };
        let confirm = labelled(
            Btn::new("reviews-dialog-go", go, Weight::Primary).icon(icon).draw(look, cx).on_click(cx.listener(move |this, _, window, cx| match ask {
                Ask::SaveAs | Ask::Rename => this.name_layout(window, cx),
            })),
            Role::Button,
            go,
        );
        let cancel = labelled(
            Btn::new("reviews-dialog-cancel", "Cancel", Weight::Secondary).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.dismiss(window, cx))),
            Role::Button,
            "Cancel",
        );
        let dialog = labelled(
            card(look)
                .id("reviews-dialog")
                .track_focus(&self.dialog)
                .w(look.length(token::W_DIALOG))
                .p(look.length(token::SPACE_4))
                .gap(look.length(token::SPACE_3))
                .border_t(look.length(token::BORDER_WIDTH) * 2.)
                .border_color(look.color(token::ACCENT))
                .child(heading)
                .child(body)
                .child(div().flex().justify_end().gap(look.length(token::SPACE_2)).child(cancel).child(confirm)),
            Role::Dialog,
            name,
        );
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(look.color(token::SCRIM))
            .occlude()
            .child(dialog)
            .into_any_element()
    }
}

impl Render for ReviewsScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        // Each place draws its own title bar, made as it is first drawn: once it is, it takes the Layout control.
        if let Some(bar) = crate::titlebar::live(cx).filter(|bar| !bar.read(cx).job_slot_filled()) {
            let control = self.control.clone();
            window.defer(cx, move |_, cx| bar.update(cx, |bar, cx| bar.fill_job_slot(Some(control.into()), cx)));
        }
        let inbox = self.render_inbox(&look, window, cx);
        let page = self.render_page(&look, window, cx);
        let dialog = self.asking.map(|ask| self.render_dialog(ask, &look, cx));
        let review_ask = self.asking_review.as_ref().map(|open| self.render_review_ask(open, &look, cx));
        let menu = self.pop.as_ref().map(|pop| pop.menu.clone());
        div()
            .id("reviews")
            .track_focus(&self.within)
            .on_key_down(cx.listener(Self::on_key))
            .relative()
            .size_full()
            .flex()
            .gap(look.length(token::GAP_PANEL))
            .p(look.length(token::SPACE_3))
            .bg(look.color(token::BG_BASE))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(inbox)
            .child(page)
            .children(dialog)
            .children(review_ask)
            .children(menu)
    }
}

/// A raised panel, as a deck panel is: its own ground, a heavier top edge.
pub(crate) fn panel(look: &Look) -> gpui::Div {
    div()
        .bg(look.color(token::BG_RAISED))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_t(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::BORDER_DEFAULT))
        .shadow(look.shadows(token::SHADOW_CARD))
        .overflow_hidden()
}

/// The radius for a child that paints its own background flush against `panel`'s top edge. gpui clips children to the
/// rectangle, not the curve, so the child rounds its own top corners: the panel's radius less its doubled top border.
pub(crate) fn panel_inner_radius(look: &Look) -> Pixels {
    look.length(token::RADIUS_LG) - look.length(token::BORDER_WIDTH) * 2.
}

/// The same for a card with a border all round (a card on a panel): its radius less one border.
pub(crate) fn card_inner_radius(look: &Look) -> Pixels {
    look.length(token::RADIUS_LG) - look.length(token::BORDER_WIDTH)
}

/// Where a menu opens under the labelled element of this role and name, or at `fallback`.
pub(crate) fn under_label(role: Role, name: &str, fallback: Point<Pixels>, window: &Window, cx: &App) -> Point<Pixels> {
    cx.try_global::<crate::labels::LabelBook>()
        .and_then(|book| book.find(window.window_handle().window_id(), role, name).map(|drawn| drawn.bounds))
        .map_or(fallback, |bounds| gpui::point(bounds.origin.x, bounds.origin.y + bounds.size.height))
}
