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
    div, prelude::*, App, Context, DismissEvent, Entity, FocusHandle, Focusable as _, FontWeight, KeyDownEvent, Pixels, Point,
    Role, SharedString, Subscription, Task, Window,
};
use gpui_component::input::{Input, InputEvent, InputState, TextareaState};
use ipc::PullReviewList;

use crate::dropdown::{Dropdown, Entry, Picked};
use crate::job_view::pane_tree::{self as tree, Arrangement, Dir, Shape, Size};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{card, sans, Btn, Weight};
use layout::{LayoutControl, Layouts};
use model::{Critique, DraftOf, Group, Inbox, Mine, Page, Verdict, Viewed};

/// Each tab a pull request's page can hold: its kind, as the pane tree keys it, and its name.
pub const KINDS: [(&str, &str); 6] = [
    ("pr-changed", "What changed"),
    ("pr-files", "Files"),
    ("pr-conversation", "Conversation"),
    ("pr-checks", "Checks"),
    ("pr-findings", "Findings"),
    ("pr-comments", "Comments"),
];

/// A tab kind's name, as its tab and VoiceOver say it.
pub fn said(kind: &str) -> &'static str {
    KINDS.iter().find(|(one, _)| *one == kind).map_or("Tab", |(_, name)| name)
}

/// The three columns he agreed: the inbox beside, the page's main tabs, and a side pane for what a review adds.
pub fn default_arrangement() -> Arrangement {
    tree::build(
        &Shape::split(Dir::Row, 0.64, Shape::pane(&["pr-changed", "pr-files", "pr-conversation", "pr-checks"]), Shape::pane(&["pr-findings", "pr-comments"])),
        None,
        &[],
    )
}

/// How long the mock forge takes to read one pull request's page.
const PAGE_READ: Duration = Duration::from_millis(1500);
/// The longest a refresh of the inbox takes before it ends on its own.
const REFRESH_MOST: Duration = Duration::from_secs(20);
/// How long the mock review run takes over a range.
const CRITIQUE_RUN: Duration = Duration::from_millis(3000);

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
    /// His last visit's pull request, which opens only once the refresh confirms it.
    restore: Option<u64>,
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
    /// Findings that went out as drafts with a review, by pull request and finding, so none goes twice.
    sent_findings: HashSet<(u64, String)>,
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
            restore: Some(mock::LAST_OPEN),
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

    /// Brought to the front: the keys go to the inbox, and the first visit reads the forge again.
    pub fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.rows, cx);
        if !self.refreshed && self.refresh.is_none() {
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
            cx.background_executor().timer(REFRESH_MOST).await;
            let _ = this.update_in(cx, |this, window, cx| this.end_refresh(window, cx));
        }));
        cx.notify();
    }

    /// The forge has answered: the inbox takes its fresh rows, and the pull request waiting on it opens.
    fn end_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.inbox.refreshing {
            self.inbox = mock::refreshed(std::mem::take(&mut self.inbox));
        }
        self.refreshed = true;
        self.refresh = None;
        self.reading = None;
        if let Some(number) = self.waiting.take().or_else(|| self.restore.take()) {
            self.show(number, window, cx);
        }
        cx.notify();
    }

    /// A row pressed. While the inbox refreshes, its page is read first and opens once read.
    fn press_row(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.focused = Some(number);
        window.focus(&self.rows, cx);
        if !self.inbox.refreshing {
            self.show(number, window, cx);
            return;
        }
        self.restore = None;
        self.waiting = Some(number);
        self.reading = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(PAGE_READ).await;
            let _ = this.update_in(cx, |this, window, cx| this.end_refresh(window, cx));
        }));
        cx.notify();
    }

    /// Open a pull request's page: on What changed where it changed since he looked, else on Files.
    fn show(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        if !self.pages.contains_key(&number) {
            let Some(page) = mock::page(number) else { return };
            self.pages.insert(number, page);
        }
        let changed = self.pages[&number].changed();
        let gone = self.pages[&number].seen.as_ref().is_some_and(|seen| seen.gone);
        self.open = Some(number);
        self.focused = Some(number);
        self.sheet = false;
        self.whole = !changed || gone;
        self.bring_kind(if changed { "pr-changed" } else { "pr-files" }, cx);
        self.summary = Some(self.boxed(&format!("reviews:summary:{number}"), None, 3, 8, window, cx));
        let mine: Vec<DraftOf> = self.drafts.iter().filter(|draft| draft.number == number).cloned().collect();
        for draft in mine {
            if !self.boxes.contains_key(&draft.id) {
                let boxed = self.boxed(&format!("reviews:draft:{}", draft.id), Some(&draft.body), 2, 6, window, cx);
                self.boxes.insert(draft.id.clone(), boxed);
            }
        }
        cx.notify();
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

    /// The tab of this kind in front, added to the first pane where no pane holds one.
    fn bring_kind(&mut self, kind: &str, cx: &mut Context<Self>) {
        let tab = tree::panes_of(&self.arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == kind).map(|tab| tab.id.clone());
        let next = match tab {
            Some(tab) => tree::rearrange(&self.arrangement, |was| tree::activate(was, &tab), None, true),
            None => {
                let first = tree::panes_of(&self.arrangement.root).first().map(|pane| pane.id.clone()).unwrap_or_default();
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
            let _ = this.update(cx, |this, cx| {
                if let Some(page) = this.pages.get_mut(&number) {
                    let findings = if number == 2198 { mock::retry_findings() } else { Vec::new() };
                    page.critique = Critique::Finished { range, findings };
                }
                this.critique = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Add as draft: the finding's comment becomes a draft on its line, to edit before Submit.
    fn add_draft(&mut self, finding: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        let Some(found) = self.page().and_then(|page| match &page.critique {
            Critique::Finished { findings, .. } => findings.iter().find(|one| one.id == finding).cloned(),
            _ => None,
        }) else {
            return;
        };
        if self.drafts.iter().any(|draft| draft.finding.as_deref() == Some(finding) && draft.number == number) {
            return;
        }
        let id = format!("d{number}-{}", self.next_draft);
        self.next_draft += 1;
        let draft = DraftOf { id: id.clone(), number, path: found.path, line: found.line, body: found.comment, finding: Some(found.id) };
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
        self.sent_findings.extend(sent.into_iter().map(|finding| (number, finding)));
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

    /// The page's first tab in front, as opening it puts it.
    fn reshow(&mut self, cx: &mut Context<Self>) {
        if let Some(changed) = self.page().map(Page::changed) {
            self.bring_kind(if changed { "pr-changed" } else { "pr-files" }, cx);
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
                said_line("reviews-dialog-said", format!("A review run reads {range} on {repository}. Its findings come to Findings."))
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
