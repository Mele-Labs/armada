//! Reviews: the pull requests waiting on him in an inbox, and the one he opens on free panes, with
//! what changed since he last looked. Drawn on mock data until Fleet's half lands; `model.rs` is the
//! view model Fleet's views fill then, and nothing here works one out.

mod bar;
mod inbox;
pub mod layout;
mod mock;
mod model;
mod panes;
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
/// How long the mock review run takes over a range.
const CRITIQUE_RUN: Duration = Duration::from_millis(3000);
/// How long the mock summary of the new commits takes to write.
const SUMMARY_WRITE: Duration = Duration::from_millis(2500);
/// How long a change set's mock How it works or How to test takes.
const EXPLAIN_RUN: Duration = Duration::from_millis(2000);
/// The change set tab's lines above its first hunk, so Show in change set can scroll to one.
pub(super) const HUNKS_AT: usize = 6;

/// A question the screen holds up before it acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// A critique starts a review run, which asks first as every run does.
    Critique,
    SaveAs,
    Rename,
}

/// A text box whose words are kept under a key until they are sent.
pub(crate) struct Boxed {
    pub state: Entity<TextareaState>,
    _changed: Subscription,
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
    layouts: Layouts,
    control: Entity<LayoutControl>,
    /// The panes' area as last drawn, so a drop that would squeeze a pane is refused.
    area: Rc<Cell<Option<Size>>>,
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
    name_box: Entity<InputState>,
    dialog: FocusHandle,
    pop: Option<Pop>,
    refresh: Option<Task<()>>,
    reading: Option<Task<()>>,
    critique: Option<Task<()>>,
    next_draft: u64,
    _subs: Vec<Subscription>,
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
            folded: HashSet::from([Group::Done]),
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
            control,
            area: Rc::default(),
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
            writing: HashMap::new(),
            explains: HashMap::new(),
            explain_runs: HashMap::new(),
            chosen: HashMap::new(),
            set_scroll: ScrollHandle::new(),
            asking: None,
            name_box,
            dialog: cx.focus_handle(),
            pop: None,
            refresh: None,
            reading: None,
            critique: None,
            next_draft: 1,
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
        if let Some(bar) = crate::titlebar::live(cx) {
            bar.update(cx, |bar, cx| bar.fill_job_slot(None, cx));
        }
    }

    fn start_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.inbox.refreshing = true;
        self.refresh = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(REFRESH_READ).await;
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
    }

    /// A row pressed. While the inbox refreshes, its page is read first and opens once read.
    fn press_row(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.focused = Some(number);
        window.focus(&self.rows, cx);
        if !self.inbox.refreshing || self.pages.get(&number).is_some_and(|page| page.kept) {
            self.waiting = None;
            self.show(number, window, cx);
            return;
        }
        self.waiting = Some(number);
        self.reading = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(PAGE_READ).await;
            let _ = this.update_in(cx, |this, window, cx| this.end_refresh(window, cx));
        }));
        cx.notify();
    }

    /// Open a pull request's page on its Overview.
    fn show(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        if !self.pages.contains_key(&number) {
            let Some(page) = mock::page(number) else { return };
            self.pages.insert(number, page);
        }
        let changed = self.pages[&number].changed() || (self.pages[&number].kept && self.pages[&number].seen.is_some());
        let gone = self.pages[&number].seen.as_ref().is_some_and(|seen| seen.gone);
        self.open = Some(number);
        self.focused = Some(number);
        self.sheet = false;
        self.whole = !changed || gone;
        self.bring_kind("pr-overview", cx);
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
        cx.notify();
    }

    /// A pull request that changed since he looked gets its summary written once for its head.
    fn write_summary(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.pages.get_mut(&number) else { return };
        if page.kept || !page.changed() || page.seen.as_ref().is_some_and(|seen| seen.gone) || page.summary != Summary::None {
            return;
        }
        page.summary = Summary::Writing;
        let task = cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(SUMMARY_WRITE).await;
            let _ = this.update(cx, |this, cx| {
                if let Some(page) = this.pages.get_mut(&number) {
                    page.summary = Summary::Written(mock::summary(number));
                }
                this.writing.remove(&number);
                cx.notify();
            });
        });
        self.writing.insert(number, task);
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
        let key = (number, set.clone(), explain);
        if self.explain_runs.contains_key(&key) {
            return;
        }
        self.explains.insert(key.clone(), None);
        let task = cx.spawn_in(window, {
            let key = key.clone();
            async move |this, cx| {
                cx.background_executor().timer(EXPLAIN_RUN).await;
                let _ = this.update(cx, |this, cx| {
                    this.explains.insert(key.clone(), Some(mock::explanation(number, &set, explain)));
                    this.explain_runs.remove(&key);
                    // The answer reads under the foot, so the set still in view scrolls down to it.
                    if this.open == Some(number) && this.rail_on().as_deref() == Some(set.as_str()) {
                        this.set_scroll.scroll_to_bottom();
                    }
                    cx.notify();
                });
            }
        });
        self.explain_runs.insert(key, task);
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
    fn toggle_viewed(&mut self, path: &str, cx: &mut Context<Self>) {
        if let Some(file) = self.page_mut().and_then(|page| page.files.iter_mut().find(|file| file.path == path)) {
            file.viewed = if file.viewed == Viewed::Yes { Viewed::No } else { Viewed::Yes };
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
        match ask {
            Ask::Critique => window.focus(&self.dialog, cx),
            Ask::SaveAs | Ask::Rename => {
                let first = if ask == Ask::Rename { self.layouts.name().to_string() } else { String::new() };
                self.name_box.update(cx, |field, cx| {
                    field.set_value(first, window, cx);
                    field.focus(window, cx);
                });
            }
        }
        cx.notify();
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = None;
        window.focus(&self.rows, cx);
        cx.notify();
    }

    /// Start critique: the review run reads the range and its findings come to Findings.
    fn start_critique(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.asking = None;
        let Some(number) = self.open else { return };
        let Some(page) = self.pages.get_mut(&number) else { return };
        let range = page.range();
        page.critique = Critique::Running { range: range.clone() };
        self.bring_kind("pr-findings", cx);
        window.focus(&self.rows, cx);
        self.critique = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(CRITIQUE_RUN).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if let Some(page) = this.pages.get_mut(&number) {
                    let findings = if number == 2198 { mock::retry_findings() } else { Vec::new() };
                    page.critique = Critique::Finished { range, findings };
                }
                this.suggestion_boxes(number, window, cx);
                this.critique = None;
                cx.notify();
            });
        }));
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
        let draft = DraftOf { id: id.clone(), number, path: found.path, line: found.line, body, finding: Some(found.id) };
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
        let gone: Vec<String> = self.drafts.iter().filter(|draft| draft.number == number).map(|draft| draft.id.clone()).collect();
        let sent: Vec<String> = self.drafts.iter().filter(|draft| draft.number == number).filter_map(|draft| draft.finding.clone()).collect();
        self.sent_findings.extend(sent.into_iter().map(|finding| ((number, finding), "today".to_string())));
        for id in gone {
            self.discard_draft(&id, cx);
        }
        crate::drafts::clear(&format!("reviews:summary:{number}"), cx);
        self.summary = Some(self.boxed(&format!("reviews:summary:{number}"), None, 3, 8, window, cx));
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

    fn send_notes(&mut self, cx: &mut Context<Self>) {
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
        match self.asking {
            Some(Ask::SaveAs) => self.layouts.save_as(&name, self.arrangement.clone()),
            Some(Ask::Rename) => self.layouts.rename(&name),
            _ => return,
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
        ];
        self.open_menu("Lists", entries, at, |this, id, _, cx| {
            this.lists = match id {
                "requested" => Some(PullReviewList::Requested),
                "team" => Some(PullReviewList::Team),
                "assigned" => Some(PullReviewList::Assigned),
                "mine" => Some(PullReviewList::Mine),
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
            Ask::Critique => ("Critique these changes", "Critique these changes?".into()),
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
            Ask::Critique => {
                let page = self.page();
                let range = page.map_or_else(|| "the whole pull request".to_string(), |page| page.range().said);
                let repository = page.map(|page| page.repository.clone()).unwrap_or_default();
                said_line("reviews-dialog-said", format!("A review run reads {range} on {repository}. Its suggestions come to Review comments."))
                    .into_any_element()
            }
            Ask::SaveAs | Ask::Rename => div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(said_line("reviews-dialog-said", "Every pull request opens on the picked layout.".into()))
                .child(labelled(div().id("layout-name").key_context(crate::keymap::TEXT_BOX).child(Input::new(&self.name_box)), Role::TextInput, "Layout name"))
                .into_any_element(),
        };
        let (go, icon) = match ask {
            Ask::Critique => ("Start critique", "bot"),
            Ask::SaveAs => ("Save layout", "check"),
            Ask::Rename => ("Rename layout", "pencil"),
        };
        let confirm = labelled(
            Btn::new("reviews-dialog-go", go, Weight::Primary).icon(icon).draw(look, cx).on_click(cx.listener(move |this, _, window, cx| match ask {
                Ask::Critique => this.start_critique(window, cx),
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
        let page = self.render_page(&look, cx);
        let dialog = self.asking.map(|ask| self.render_dialog(ask, &look, cx));
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

/// Where a menu opens under the labelled element of this role and name, or at `fallback`.
pub(crate) fn under_label(role: Role, name: &str, fallback: Point<Pixels>, window: &Window, cx: &App) -> Point<Pixels> {
    cx.try_global::<crate::labels::LabelBook>()
        .and_then(|book| book.find(window.window_handle().window_id(), role, name).map(|drawn| drawn.bounds))
        .map_or(fallback, |bounds| gpui::point(bounds.origin.x, bounds.origin.y + bounds.size.height))
}
