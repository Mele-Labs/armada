//! The open Job, in the middle of the Jobs screen: its head, its thread and its message box
//! (`docs/spikes/native-jobs-inventory/job-view.md`, sections 0 to 4).
//!
//! **A screen draws a Fleet view.** The thread is `get_job_thread` folded by Fleet and kept
//! current by `job_thread.tail` (`crate::fleet::HeldThread`); this file draws it and works nothing
//! out. What Fleet does not carry yet, the words of a run of calls, is `calls.rs`, ported from
//! Electron's `calls-line.ts`.
//!
//! Every Job keeps its place: its list, its open folds and its draft stay in a [`Page`] while the
//! person is in another Job, so a switch is a swap of what is drawn and nothing is read or measured
//! again. A page is made on the first visit.

mod aa;
mod acts;
mod bands;
mod bar;
mod calls;
mod callchip;
mod cards;
mod composer;
mod deck;
mod draft_doc;
mod dock;
mod done;
mod now;
mod open;
mod deferred;
mod watching;
mod edit;
mod fork;
mod format;
mod head;
mod judge;
mod keys;
mod lines;
mod menus;
mod offer;
mod perm;
mod pictures;
mod pinned;
mod plan;
mod plan_edit;
mod plan_marks;
mod pop;
mod quick;
mod teach;
mod records;
mod refs;
mod review;
mod row_tabs;
mod rows;
mod judge_tab;
mod lanes;
mod lanes_copy;
mod ledger;
mod mods_made;
mod mod_panel;
mod report;
mod sketch_pad;
mod sketch_read;
mod subagent_read;
mod zoom;
mod panes;
mod pulls;
mod issues;
mod system;
mod bays;
mod pr_files;
mod pr_notes;
mod pr_panel;
mod report_brief;
mod side;
mod spine;
mod workflow;
mod tabs;
pub use side::JobSide;
pub use mods_made::OpenMod;
pub use mod_panel::{ModPanel, ModPanelAsked, OpenJobFrom};
pub use teach::notice_name;
mod runs;
mod talk;
mod tool_icons;
mod tune;
mod wire;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::{
    actions, canvas, div, list, prelude::*, Animation, AnimationExt, AnyElement, App, Context, Entity, FocusHandle, FollowMode,
    KeyBinding, ListAlignment, ListOffset, ListState, Role, SharedString, Subscription, Window,
};
use ipc::{JobState, PlanCardState, SessionId, SessionRecord, SessionRow, SessionState, SessionVoice, ThreadHead, ThreadItem, ViewSession};

use crate::fleet::{FleetConnection, HeldThread};
use crate::jobs::JobsList;
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, ActiveLook, Look};

actions!(job_view, [PageUp, PageDown, ToStart, ToEnd]);

/// The thread's own keys, which the registry does not carry: Electron lets the browser scroll a
/// focused scroller on these (`SessionThread.tsx`, A1.5).
pub fn bind(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("pageup", PageUp, Some("thread_scroll")),
        KeyBinding::new("pagedown", PageDown, Some("thread_scroll")),
        KeyBinding::new("home", ToStart, Some("thread_scroll")),
        KeyBinding::new("end", ToEnd, Some("thread_scroll")),
    ]);
    review::bind(cx);
    head::bind(cx);
    lanes_copy::bind(cx);
}

/// What a page draws after the thread's items: what stands over the whole thread, the person's
/// words not yet taken by Fleet, and the working line (A1.4).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Extra {
    Plan(PlanCardState),
    Sketch(String),
    Pending(usize),
    Working,
    /// The card of the subagents running now.
    Subagents,
    /// Where a moved Job went, at the foot of its thread (`movedRowsOf`'s tail).
    MovedTo,
    /// The thread held is being read again (`transcript: "refreshing"`).
    Refreshing,
}

/// The person's words, drawn in the thread the moment they are sent and until Fleet's own row for
/// them arrives (`outgoing.ts`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Pending {
    key: u64,
    text: String,
    /// How many of the person's messages in the thread already read this, so Fleet's row for
    /// these words is the one that makes it more.
    seen: usize,
    /// What it tags, drawn in its words as Fleet's row will draw them.
    tags: Vec<ipc::SessionTag>,
    /// The body posted, so Retry sends it again as it first went. `None` until its files are read.
    body: Option<String>,
    /// Fleet did not take it: it stays in the thread marked Not sent, with a Retry.
    failed: bool,
}

/// Whether Fleet's row `row` reads `words` now, so the edit drawn over it can go.
fn edit_read(items: &[ThreadItem], row: &str, words: &str) -> bool {
    items.iter().any(|item| matches!(item, ThreadItem::Row { row: SessionRow::Message { id, text, .. } } if id == row && text == words))
}

/// One Job's place: kept while it is not on screen.
struct Page {
    list: ListState,
    /// How many of the list's elements are the thread's items; the rest are [`Extra`]s.
    items: usize,
    /// Whether the list opens with the Show earlier row (1) or not (0): older items are served.
    lead: usize,
    /// The elements the Job this one took the place of stands for above this Job's own items: its
    /// items and the plain rule under them. Drawn once this Job's own older items are all read.
    before: usize,
    /// That earlier Job's thread version the list was last measured for.
    earlier_version: Option<u64>,
    count: usize,
    extras: Vec<Extra>,
    /// The thread version the list was last measured for.
    version: Option<u64>,
    /// The folds the person opened, by item id.
    opened: HashSet<String>,
    pending: Vec<Pending>,
    /// Fleet's reason for the last send it refused.
    refused: Option<SharedString>,
    /// What the cards in this Job's thread remember.
    cards: acts::CardState,
    /// What the Needs you panel remembers of this Job.
    dock: dock::DockState,
    /// What the tools, ending and waiting bands remember of this Job.
    bands: bands::BandState,
    /// The cards here that wait, and what the four thread keys do to each.
    keys: Rc<keys::Cards>,
    /// Items that came in after the first draw, at most four at once: they fade and slide in (A1.8).
    arrived: std::ops::Range<usize>,
    /// The review sheet, while it is open on this Job.
    review: review::ReviewState,
    /// His marks on the plan, not sent yet, and the field open on it.
    plan_edit: plan_edit::EditState,
    side: side::SideState,
    /// The whole Job's Judge pane, while it is open.
    judge: Option<judge_tab::JudgePane>,
    /// His bubble open for new words.
    editing: Option<edit::Editing>,
    /// New words saved on his messages, by row id, drawn until Fleet's row reads them.
    edits: HashMap<String, String>,
    /// The path of the page a draft became, gone to once its card is in the thread.
    made: Option<String>,
    /// The pages his own press made, drawn whole when their card comes in, as his own words are.
    made_pages: HashSet<String>,
}

impl Page {
    fn new() -> Self {
        let list = ListState::new(0, ListAlignment::Top, gpui::px(600.));
        // The thread opens at its newest row and stays there until the person scrolls up.
        list.set_follow_mode(FollowMode::Tail);
        Page { list, items: 0, lead: 0, before: 0, earlier_version: None, count: 0, extras: Vec::new(), version: None, opened: HashSet::new(), pending: Vec::new(), refused: None, cards: acts::CardState::default(), dock: dock::DockState::default(), bands: bands::BandState::default(), keys: Rc::default(), arrived: 0..0, review: review::ReviewState::default(), plan_edit: plan_edit::EditState::default(), side: side::SideState::default(), judge: None, editing: None, edits: HashMap::new(), made: None, made_pages: HashSet::new() }
    }

    /// The list element of this Job's item `at`. An item of the earlier Job above is drawn with an
    /// `at` that wraps below zero, so it comes back to its own element here.
    fn entry(&self, at: usize) -> usize {
        at.wrapping_add(self.lead + self.before)
    }

    /// Measure item `at`'s element again. An item drawn outside the list, as the fork's fold
    /// draws its origin's, has no element, and nothing is measured.
    fn remeasure(&self, at: usize) {
        let at = self.entry(at);
        if at < self.count {
            self.list.remeasure_items(at..at + 1);
        }
    }
}

/// The thread's head and foot fade into the panel: 32px at the top, 16px at the bottom (A1.3). The words
/// are not masked as Electron does; a gradient of the panel's own colour is laid over them, which reads
/// the same on a flat panel.
fn fade(look: &Look, top: bool) -> gpui::Div {
    let panel = look.color(token::BG_RAISED);
    let clear = panel.opacity(0.);
    let edge = if top { gpui::linear_color_stop(panel, 0.) } else { gpui::linear_color_stop(clear, 0.) };
    let far = if top { gpui::linear_color_stop(clear, 1.) } else { gpui::linear_color_stop(panel, 1.) };
    let band = div().absolute().left_0().right_0().bg(gpui::linear_gradient(180., edge, far));
    if top {
        band.top_0().h(look.length(token::SPACE_8))
    } else {
        band.bottom_0().h(look.length(token::SPACE_4))
    }
}

/// What the thread draws after its items, from what Fleet says stands over it (A1.4 steps 7 and 8).
fn extras_of(head: &ThreadHead, items: &[ThreadItem], pending: &[Pending], working: bool, moved_to: bool, refreshing: bool) -> Vec<Extra> {
    let mut extras = Vec::new();
    if moved_to {
        extras.push(Extra::MovedTo);
    }
    if let Some(plan) = &head.plan {
        if plan.state != PlanCardState::Approved {
            extras.push(Extra::Plan(plan.state));
        }
    }
    if let Some(sketch) = &head.sketch {
        extras.push(Extra::Sketch(sketch.title.clone()));
    }
    extras.extend((0..pending.len()).map(Extra::Pending));
    if refreshing {
        extras.push(Extra::Refreshing);
    }
    let running = items.iter().any(|item| matches!(item, ThreadItem::Subagent { subagent, .. } if subagent.state == ipc::ToolSubagentState::Running));
    let ends_in_calls = matches!(items.last(), Some(ThreadItem::Calls { .. }));
    if working && !running && (!ends_in_calls || head.asked.is_some()) {
        extras.push(Extra::Working);
    }
    if running {
        extras.push(Extra::Subagents);
    }
    extras
}

/// How many of the person's own messages read exactly `text`.
fn echoed(items: &[ThreadItem], text: &str) -> usize {
    items
        .iter()
        .filter(|item| matches!(item, ThreadItem::Row { row: SessionRow::Message { from: SessionVoice::You, text: said, .. } } if said.trim() == text))
        .count()
}

/// Tell Fleet a Job's view is on screen or off it (`view_session`).
fn tell_fleet(port: Option<u16>, job: &str, on: bool, cx: &mut App) {
    Note::info("a Job's view was told to Fleet").field("job", job).field("on", on).emit();
    let body = ipc::encode(&ViewSession { session_id: SessionId::carried(job), on }).map_err(|why| why.why);
    crate::jobs::send(port, "/sessions/view", body, cx);
}

pub struct JobView {
    list: Entity<JobsList>,
    fleet: Option<Entity<FleetConnection>>,
    /// The threads of a mock scenario, which never change but by the person's own words.
    mock: HashMap<String, HeldThread>,
    /// What the mock's Needs you says for each Job, as Fleet's view would: grouped from the record.
    mock_needs: HashMap<String, ipc::NeedsYou>,
    /// How many more sends the mock refuses, so a walk can show a message Fleet did not take.
    mock_refusals: usize,
    /// The Job on screen.
    shown: Option<SharedString>,
    /// What Fleet refused a terminal Job's Compact for, by Job, until Compact is pressed again.
    compact_refused: HashMap<SharedString, SharedString>,
    pages: HashMap<SharedString, Page>,
    head: head::Head,
    composer: composer::Composer,
    /// Every stamp in the thread reads as an age, not a clock (a press flips them all).
    aged: bool,
    /// How Done lists what was made, for the run.
    ledger_view: done::LedgerView,
    /// Mod switches he pressed that Fleet's list has not caught up with: was, then pressed.
    mod_switch: HashMap<String, (bool, bool)>,
    /// The Job fleet was last told is on screen.
    visiting: Option<SharedString>,
    switched: Option<std::time::Instant>,
    /// How wide the thread was last drawn: the words' column grows with it, as Electron's does with its pane.
    pane: Option<gpui::Pixels>,
    /// The list's pick count last acted on: a new pick puts the keys in the box (his word, 10 Nov).
    picked: u64,
    /// The right column's top pane: Open, Done or Deferred.
    zone: side::Zone,
    /// The tabs closed on each Job, and + 's menu while it is open.
    tabs_closed: tabs::Closed,
    /// The pane zoomed over the Job, and the panes closed on each Job.
    zoom: zoom::Zoom,
    tabs_menu: Option<tabs::AddMenu>,
    /// The tabs beside each Job's thread: a draft, a document or a walk.
    row_tabs: HashMap<SharedString, row_tabs::RowTabs>,
    /// The body of the tab beside the thread in front, which takes the keys when it opens.
    row_focus: FocusHandle,
    /// The thread's strip, which scrolls sideways once its tabs are wider than the pane.
    row_strip: gpui::ScrollHandle,
    focus: FocusHandle,
    /// The menu a thread element opened, if one is open.
    pop: Option<pop::Pop>,
    /// Hide Open group pressed here, with Fleet's setting then: it stands until Fleet's says otherwise.
    open_group_set: Option<(bool, bool)>,
    /// The Needs you panel, so its keys work once focus is in it.
    dock_focus: FocusHandle,
    /// A thread scroll key ran in the frame now drawing: a row it carries out of the list keeps the keys nearby.
    scroll_keys: bool,
    /// The code Fleet refused the last answered act with (`WireError.code`), read by the `done` that answers it.
    refused_code: Option<String>,
    /// The dialog a thread key opens to ask which card it goes to.
    confirm: Option<keys::Confirm>,
    /// The bar under agent messages: its responses, what was just copied, and the editor of a response.
    bar: bar::Bar,
    /// Words selected in the thread and the bar over them.
    fork: fork::Fork,
    /// The Judge's box on a plan, while it is open.
    judge: Option<judge::Open>,
    /// The Report dialog, while it is open over the Job.
    reporting: Option<report::Report>,
    /// The Draw sketch sheet is up, over the Job's pad.
    sketch_open: bool,
    pad: Option<sketch_pad::Pad>,
    /// The Workflow tab's canvas: where each Job's lanes are looked at, and whether the keys are on it.
    lanes: lanes::Lanes,
    /// The Teach panel, while it is open, Fleet's refusal of its last call, and the count that tells its answers apart.
    teach: Option<teach::Open>,
    teach_said: Option<SharedString>,
    teach_clock: u64,
    confirm_focus: FocusHandle,
    /// The confirm dialog's two buttons, which Tab goes round with its body.
    confirm_stops: [FocusHandle; 2],
    /// The review sheet's focus stops, and where its rows were drawn.
    review_focus: review::Focuses,
    review_rows: review::Rows,
    /// The pictures his messages carried, and the one open over the window.
    pictures: pictures::Pictures,
    /// The sketch open over the window, read on its own.
    sketch_reading: Option<sketch_read::Reading>,
    /// A subagent's thread open on its sheet, and the last threads read, newest last.
    subagent_reading: Option<subagent_read::Reading>,
    subagents_read: Vec<(String, subagent_read::SubagentRead)>,
    /// The reads kept going for subagents open as tabs beside the thread, by `job/subagent`.
    subagent_tab_polls: HashMap<String, gpui::Task<()>>,
    /// The Plan, Changes, Files and Checks bodies' state, and the panel one of their rows opened.
    panes: panes::Panes,
    /// The Spine's folds, scroll and keys.
    spine: spine::Spine,
    /// The Jobs a walk reads again for a moment when they come on screen, standing in for Fleet's read.
    mock_reading: HashSet<String>,
    _subscriptions: Vec<Subscription>,
}

/// How far cards, code and his bubbles reach past the words either side, so their own words line up with the prose
/// (`--spacious-bleed`).
pub(super) fn bleed(look: &Look) -> gpui::Pixels {
    look.length(token::SPACE_4) + look.length(token::SPACE_1)
}

impl JobView {
    /// The words' column, as Electron's `--spacious-column`: 70% of the pane, from 680 to 960 wide.
    pub(super) fn column(&self, look: &Look) -> gpui::Pixels {
        let (least, most) = (look.length(token::SPACE_12) * 14. + look.length(token::SPACE_2), look.length(token::SPACE_12) * 20.);
        self.pane.map_or(most, |pane| gpui::px((f32::from(pane) * 0.7).clamp(f32::from(least), f32::from(most))))
    }

    pub fn new(
        list: Entity<JobsList>,
        mock: Option<HashMap<String, ipc::JobThread>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let fleet = list.read(cx).fleet();
        if let Some(threads) = mock.as_ref().filter(|_| fleet.is_none()) {
            runs::start_mock_clock(threads);
        }
        let remember = list.read(cx).remembers();
        let (mut composer, mut subscriptions) = composer::build(remember, window, cx);
        if let Some(models) = crate::mock::models() {
            composer.catalog.models = Some(models);
            composer.catalog.models_asked = true;
        }
        if let Some(catalogue) = crate::mock::catalogue() {
            composer.catalog.mock_files = catalogue.files;
            composer.catalog.issues.insert(catalogue.issues.manifest_id.as_str().to_string(), catalogue.issues.issues);
            composer.quick = quick::Quick::mock(catalogue.look, catalogue.quick);
            composer.catalog.scripts = catalogue.talk;
        }
        subscriptions.push(cx.observe_in(&list, window, |this, _, window, cx| this.followed(window, cx)));
        let lanes = lanes::Lanes::new(cx);
        subscriptions.push(cx.on_focus(&lanes.frame, window, |this, _, cx| {
            this.lanes.keyed.set(true);
            cx.notify();
        }));
        subscriptions.push(cx.on_blur(&lanes.frame, window, |this, _, cx| {
            this.lanes.keyed.set(false);
            cx.notify();
        }));
        if let Some(fleet) = &fleet {
            subscriptions.push(cx.observe_in(fleet, window, |this, _, window, cx| {
                this.synced(cx);
                this.review_sync(window, cx);
            }));
        }
        subscriptions.push(cx.observe_window_activation(window, |this, window, cx| {
            // Back from Settings, the card that sent him there goes.
            if window.is_window_active() {
                this.talk_put_away(cx);
            }
            let on = window.is_window_active().then(|| this.shown.clone()).flatten();
            this.visit(on, cx);
        }));
        // A press that takes away the thing the keys were on leaves them on nothing drawn, where no
        // key reaches anything: they go to the message box, where a zone did not take them first. A row a
        // scroll key carried out of the list hands them to what holds it, as Classic's focused row keeps them.
        subscriptions.push(cx.on_focus_lost(window, |this, window, cx| {
            if std::mem::take(&mut this.scroll_keys) {
                if let Some(near) = window.focus_lost_restore_target(cx) {
                    window.focus(&near, cx);
                    return;
                }
            }
            let keys = gpui::Focusable::focus_handle(this.composer.input.read(cx), cx);
            if this.composer.frame.contains(&keys, window) {
                window.focus(&keys, cx);
            }
        }));
        subscriptions.push(cx.on_release(|this, cx| {
            if let Some(job) = this.visiting.take() {
                let port = this.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port());
                tell_fleet(port, &job, false, cx);
            }
        }));
        Self::offer_open_mod(cx);
        let mock = mock
            .unwrap_or_default()
            .into_iter()
            .map(|(id, thread)| (id, HeldThread::from_read(thread, 1)))
            .collect();
        JobView {
            list,
            fleet,
            mock,
            mock_needs: HashMap::new(),
            mock_refusals: crate::mock::refusals(),
            shown: None,
            compact_refused: HashMap::new(),
            pages: HashMap::new(),
            head: head::Head::new(cx),
            composer,
            aged: false,
            ledger_view: if remember { done::read_ledger_view() } else { done::LedgerView::default() },
            mod_switch: HashMap::new(),
            visiting: None,
            switched: None,
            pane: None,
            picked: 0,
            zone: side::Zone::default(),
            tabs_closed: tabs::Closed::default(),
            zoom: zoom::Zoom::default(),
            tabs_menu: None,
            row_tabs: HashMap::new(),
            row_focus: cx.focus_handle(),
            row_strip: gpui::ScrollHandle::new(),
            focus: cx.focus_handle(),
            pop: None,
            open_group_set: None,
            dock_focus: cx.focus_handle(),
            scroll_keys: false,
            refused_code: None,
            confirm: None,
            bar: bar::Bar::new(),
            fork: fork::Fork::default(),
            judge: None,
            reporting: None,
            sketch_open: false,
            pad: None,
            lanes,
            teach: None,
            teach_said: None,
            teach_clock: 0,
            confirm_focus: cx.focus_handle(),
            confirm_stops: [cx.focus_handle(), cx.focus_handle()],
            review_focus: review::Focuses::new(cx),
            review_rows: Rc::new(RefCell::new(HashMap::new())),
            pictures: pictures::Pictures::default(),
            sketch_reading: None,
            subagent_reading: None,
            subagents_read: Vec::new(),
            subagent_tab_polls: HashMap::new(),
            panes: panes::Panes::default(),
            spine: spine::Spine::new(remember, cx),
            mock_reading: HashSet::new(),
            _subscriptions: subscriptions,
        }
    }

    /// The thread Fleet (or the mock scenario) holds for a Job.
    fn thread<'a>(&'a self, id: &str, cx: &'a App) -> Option<&'a HeldThread> {
        match &self.fleet {
            Some(fleet) => fleet.read(cx).held().threads.get(id),
            None => self.mock.get(id),
        }
    }

    fn record<'a>(&self, id: &str, cx: &'a App) -> Option<&'a SessionRecord> {
        self.list.read(cx).record(id)
    }

    /// The Job this one took the place of when it was handed on: its turns stand above this one's,
    /// over a plain rule, as Electron's `replaces` draws them.
    /// A Job moved here carries the one it was moved from the same way, under its Moved from row and
    /// with no rule, as Electron's `carried` origin rows.
    fn earlier_of(&self, id: &str, cx: &App) -> Option<String> {
        let record = self.record(id, cx)?;
        record.attachments.iter().find(|one| one.kind == "replaces").map(|one| one.target.clone()).or_else(|| self.moved(id, "forked_from", cx).map(|(from, _)| from))
    }

    /// The Job a move took this one to (`forked_to`) or brought it from (`forked_from`), and the
    /// repository it names there, as `movedTo` and `movedFrom` read the ledger's fork pair.
    fn moved(&self, id: &str, kind: &str, cx: &App) -> Option<(String, String)> {
        let fork = self.record(id, cx)?.attachments.iter().find(|one| one.kind == kind && one.detail.contains_key("moved"))?;
        Some((fork.target.clone(), fork.detail.get("moved")?.clone()))
    }

    /// A repository by the word the picker names it by, or its id where none is served (`manifestLabel`).
    fn repository_word(&self, manifest: &str, cx: &App) -> String {
        let served = match &self.fleet {
            Some(fleet) => fleet.read(cx).held().repositories.clone().unwrap_or_default(),
            None => crate::mock::repositories(),
        };
        served
            .iter()
            .find(|one| one.manifest.as_ref().map(|held| held.id.as_str()).or(one.implicit_id.as_ref().map(|id| id.as_str())) == Some(manifest))
            .map_or_else(
                || manifest.to_string(),
                |one| one.manifest.as_ref().map_or_else(|| one.root.rsplit('/').find(|part| !part.is_empty()).unwrap_or(&one.root).to_string(), |held| held.repository.clone()),
            )
    }

    /// Whether the thread held is being read again now.
    fn refreshing(&self, id: &str, cx: &App) -> bool {
        self.mock_reading.contains(id) || self.fleet.as_ref().is_some_and(|fleet| fleet.read(cx).reading(id))
    }

    /// A walk's stand-in for Fleet's read: the Job reads again for a moment, then holds what it held.
    fn read_slowly(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        const READ_FOR: std::time::Duration = std::time::Duration::from_millis(1000);
        self.mock_reading.insert(id.to_string());
        let id = id.clone();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(READ_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.mock_reading.remove(id.as_ref());
                this.sync(&id, cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// Whether the agent's turn is running, as the record says (`session.turn.state`).
    fn working(&self, id: &str, cx: &App) -> bool {
        !self.head.stopped(id)
            && self.record(id, cx).is_some_and(|job| match &job.hosted {
            Some(hosted) => matches!(hosted.turn, ipc::SessionTurn::Working { .. }),
            None => job.mark.is_some_and(|mark| mark.state == JobState::Working),
        })
    }

    /// The Job's message box is not drawn once it has ended or gone quiet (`session.dead`).
    fn closed(&self, id: &str, cx: &App) -> bool {
        self.head.ended(id)
            || self.record(id, cx).is_none_or(|job| {
            job.state == SessionState::Ended || job.mark.is_some_and(|mark| matches!(mark.state, JobState::Ended | JobState::Quiet))
        })
    }

    /// The list opened another Job, or the same Job's record moved.
    fn followed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (opened, picks) = {
            let list = self.list.read(cx);
            (list.opened_id(), list.picks())
        };
        if opened != self.shown {
            self.switch(opened, window, cx);
        } else {
            self.synced(cx);
            self.review_sync(window, cx);
        }
        if picks != self.picked {
            self.picked = picks;
            self.to_composer(window, cx);
        }
    }

    /// Put another Job on screen: its place was kept, so this swaps what is drawn and the box's
    /// words, tells Fleet who is looking, and reads the thread if it is not held.
    fn switch(&mut self, to: Option<SharedString>, window: &mut Window, cx: &mut Context<Self>) {
        let started = std::time::Instant::now();
        // The sheet is the Job's: it shuts when another Job comes on screen.
        self.close_review(window, cx);
        self.composer_leave(cx);
        // A zoom is where he was looking on that Job: another Job on screen ends it.
        self.end_zoom();
        self.shown = to.clone();
        self.switched = Some(started);
        // Each Job has its own tabs, so the top strip starts at its head with the tab in front in view.
        self.panes.top_tabs.set_offset(gpui::point(gpui::px(0.), gpui::px(0.)));
        if let Some(at) = self.zone_shown().and_then(|zone| side::Zone::ALL.into_iter().filter(|one| !self.tab_closed(one.id())).position(|one| one == zone)) {
            self.panes.top_tabs.scroll_to_item(at);
        }
        if let Some(id) = &to {
            self.pages.entry(id.clone()).or_insert_with(Page::new);
            if let Some(fleet) = self.fleet.clone() {
                let id = id.clone();
                fleet.update(cx, |fleet, cx| fleet.read_thread(&id, true, cx));
            } else if let Some(why) = crate::mock::thread_refused(id) {
                cx.defer(move |cx| crate::toasts::tell(why, cx));
            } else if crate::mock::reads_slowly() {
                self.read_slowly(id, cx);
            }
            self.sync(id, cx);
        }
        self.composer_enter(to.as_ref(), window, cx);
        self.want_pane_reads(window, cx);
        let on = window.is_window_active().then(|| to.clone()).flatten();
        self.visit(on, cx);
        cx.notify();
    }

    /// Tell Fleet which Job is on screen: off the one it was told of, on the one now (`view_session`,
    /// as `useViewing` does, once on and once off so Fleet's count of places stays even).
    fn visit(&mut self, to: Option<SharedString>, cx: &mut Context<Self>) {
        if self.visiting == to {
            return;
        }
        let port = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port());
        for (job, on) in [(self.visiting.take(), false), (to.clone(), true)] {
            if let Some(job) = job {
                tell_fleet(port, &job, on, cx);
            }
        }
        self.visiting = to;
    }

    /// Fleet or the list moved: draw the pages again where they changed.
    fn synced(&mut self, cx: &mut Context<Self>) {
        let ids: Vec<SharedString> = self.pages.keys().cloned().collect();
        for id in ids {
            self.sync(&id, cx);
        }
        self.want_look(cx);
        if let Some(id) = self.shown.clone() {
            self.judge_read(&id, cx);
        }
        self.quick_follow(cx);
        // A Fleet that was not there when the Job opened is asked again once it is.
        if let (Some(id), Some(fleet)) = (self.shown.clone(), self.fleet.clone()) {
            if self.thread(&id, cx).is_none() {
                fleet.update(cx, |fleet, cx| fleet.read_thread(&id, false, cx));
            }
        }
        cx.notify();
    }

    /// Bring a page's list in step with the thread held: measure again from the first item that
    /// changed and from no earlier, so a tail does not cost a thread's worth of layout.
    fn sync(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        self.want_needs(id, cx);
        self.want_watches(id, cx);
        let dirty = self.fleet.as_ref().and_then(|fleet| fleet.update(cx, |fleet, _| fleet.take_dirty(id)));
        let added = self.fleet.as_ref().map_or(0, |fleet| fleet.update(cx, |fleet, _| fleet.take_prepended(id)));
        let working = self.working(id, cx);
        let settled: Vec<u64> = match (self.thread(id, cx), self.pages.get(id)) {
            (Some(held), Some(page)) => {
                page.pending.iter().filter(|one| echoed(&held.items, &one.text) > one.seen).map(|one| one.key).collect()
            }
            _ => Vec::new(),
        };
        let read: Vec<String> = match (self.thread(id, cx), self.pages.get(id)) {
            (Some(held), Some(page)) => page.edits.iter().filter(|(row, words)| edit_read(&held.items, row, words)).map(|(row, _)| row.clone()).collect(),
            _ => Vec::new(),
        };
        if let Some(page) = self.pages.get_mut(id) {
            page.pending.retain(|one| !settled.contains(&one.key));
            page.edits.retain(|row, _| !read.contains(row));
        }
        // Once this Job's own older items are all read, the Job it took the place of stands above them.
        let own_older = self.thread(id, cx).is_some_and(|held| held.older.is_some());
        let earlier = if own_older { None } else { self.earlier_of(id, cx) };
        let earlier_read = earlier.as_deref().and_then(|earlier| self.thread(earlier, cx)).map(|held| (held.version, held.items.len(), held.older.is_some()));
        if let (Some(earlier), None, Some(fleet)) = (&earlier, earlier_read, self.fleet.clone()) {
            let earlier = earlier.clone();
            fleet.update(cx, |fleet, cx| fleet.read_thread(&earlier, false, cx));
        }
        let added_earlier = match (&earlier, &self.fleet) {
            (Some(earlier), Some(fleet)) => fleet.update(cx, |fleet, _| fleet.take_prepended(earlier)),
            _ => 0,
        };
        let Some(page) = self.pages.get(id) else { return };
        let Some(held) = self.thread(id, cx) else { return };
        let (version, items) = (held.version, held.items.len());
        let lead = usize::from(own_older || earlier_read.is_some_and(|(_, _, older)| older)) + usize::from(self.moved(id, "forked_from", cx).is_some());
        // A Job handed on stands over a rule; one moved here runs straight into this one's.
        let rule = usize::from(self.record(id, cx).is_some_and(|job| job.attachments.iter().any(|one| one.kind == "replaces")));
        let before = earlier_read.map_or(0, |(_, items, _)| items + rule);
        let earlier_version = earlier_read.map(|(version, ..)| version);
        let extras = extras_of(&held.head, &held.items, &page.pending, working, self.moved(id, "forked_to", cx).is_some(), self.refreshing(id, cx));
        let page = self.pages.get_mut(id).expect("the page was just read");
        let above = (lead, before, earlier_version);
        let above_was = (page.lead, page.before, page.earlier_version);
        if page.version == Some(version) && page.extras == extras && dirty.is_none() && above == above_was {
            return;
        }
        let total = lead + before + items + extras.len();
        let from = if page.version.is_none() { 0 } else { dirty.unwrap_or(page.items).min(page.items).min(items) };
        let from_list = if above != above_was || from == 0 { 0 } else { lead + before + from };
        // Items put in front, of this Job or the one above it, leave the person where they were reading.
        let shift = (lead + before + added) as isize - (page.lead + page.before) as isize;
        let top = (page.version.is_some() && (added > 0 || added_earlier > 0 || before != page.before)).then(|| page.list.logical_scroll_top());
        page.list.splice(from_list..page.count, total - from_list);
        if let Some(top) = top {
            page.list.scroll_to(ListOffset { item_ix: (top.item_ix as isize + shift).max(0) as usize, offset_in_item: top.offset_in_item });
        }
        page.before = before;
        page.earlier_version = earlier_version;
        let first = page.version.is_none();
        let grown = items.saturating_sub(page.items);
        let arrived = (!first && added == 0 && (1..=4).contains(&grown)).then(|| page.items..items);
        page.items = items;
        page.lead = lead;
        page.count = total;
        page.extras = extras;
        page.version = Some(version);
        if first {
            // Within a screen of the top, the older items are read: the row says so when it draws.
            let (weak, job) = (cx.weak_entity(), id.clone());
            page.list.set_scroll_handler(move |event, _, cx| {
                if event.visible_range.start <= 1 {
                    weak.update(cx, |this, cx| this.want_earlier(&job, cx)).ok();
                }
            });
        }
        if let Some(arrived) = arrived {
            if let Some(page) = self.pages.get_mut(id) {
                page.arrived = arrived;
            }
            // The animation is over after its own time; the rows are plain again.
            let (job, over) = (id.clone(), cx.look().duration(token::DURATION_ENTER));
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(over).await;
                this.update(cx, |this, _| {
                    if let Some(page) = this.pages.get_mut(&job) {
                        page.arrived = 0..0;
                    }
                })
                .ok();
            })
            .detach();
        }
        self.arrive(id, cx);
        self.open_made(id, cx);
    }

    /// A message from the person puts the thread at its end again (A1.5).
    pub(super) fn pin(&mut self, id: &SharedString) {
        if let Some(page) = self.pages.get(id) {
            page.list.set_follow_mode(FollowMode::Tail);
        }
    }

    fn scroll_by(&mut self, pages: f32, cx: &mut Context<Self>) {
        let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get(id)) else { return };
        let height = page.list.viewport_bounds().size.height;
        page.list.scroll_by(height * pages);
        cx.notify();
    }

    /// Marks the next frame drawn, the one where a row this key moves out of the list stops being drawn.
    fn scrolled_by_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.scroll_keys = true;
        let view = cx.entity().downgrade();
        window.on_next_frame(move |window, _| {
            window.on_next_frame(move |_, cx| {
                let _ = view.update(cx, |this, _| this.scroll_keys = false);
            })
        });
    }

    fn on_page_up(&mut self, _: &PageUp, window: &mut Window, cx: &mut Context<Self>) {
        self.scrolled_by_key(window, cx);
        self.scroll_by(-0.9, cx);
    }

    fn on_page_down(&mut self, _: &PageDown, window: &mut Window, cx: &mut Context<Self>) {
        self.scrolled_by_key(window, cx);
        self.scroll_by(0.9, cx);
    }

    /// Read the thread's older items when the top comes near, and when the row is pressed.
    pub(crate) fn want_earlier(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else { return };
        if self.thread(id, cx).is_some_and(|held| held.older.is_some()) {
            fleet.update(cx, |fleet, cx| fleet.read_older(id, cx));
        } else if let Some(earlier) = self.earlier_of(id, cx) {
            fleet.update(cx, |fleet, cx| fleet.read_older(&earlier, cx));
        }
    }

    /// The Job whose older items the top row reads: this one's, else the one it took the place of.
    pub(crate) fn older_of(&self, id: &str, cx: &App) -> String {
        match self.thread(id, cx).is_some_and(|held| held.older.is_some()) {
            true => id.to_string(),
            false => self.earlier_of(id, cx).unwrap_or_else(|| id.to_string()),
        }
    }

    fn on_to_start(&mut self, _: &ToStart, window: &mut Window, cx: &mut Context<Self>) {
        self.scrolled_by_key(window, cx);
        if let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get(id)) {
            page.list.scroll_to(ListOffset { item_ix: 0, offset_in_item: gpui::px(0.) });
        }
        cx.notify();
    }

    fn on_to_end(&mut self, _: &ToEnd, window: &mut Window, cx: &mut Context<Self>) {
        self.scrolled_by_key(window, cx);
        if let Some(id) = self.shown.clone() {
            self.pin(&id);
        }
        cx.notify();
    }

    /// A fold in the thread opened or shut: that element measures again.
    pub(crate) fn toggle(&mut self, at: usize, key: String, cx: &mut Context<Self>) {
        let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get_mut(id)) else { return };
        if !page.opened.remove(&key) {
            page.opened.insert(key);
        }
        page.remeasure(at);
        cx.notify();
    }

    pub(crate) fn flip_stamps(&mut self, cx: &mut Context<Self>) {
        self.aged = !self.aged;
        cx.notify();
    }

    /// Another Job from a row of this one: the list opens it.
    pub(crate) fn open_job(&mut self, id: String, cx: &mut Context<Self>) {
        self.list.update(cx, |list, cx| list.open(id.into(), cx));
    }

    /// The shapes a thread draws while Fleet has not answered (A1.7): the person's turn, then the agent's.
    fn render_skeleton(&self, look: &Look) -> AnyElement {
        let bar = |width: f32| {
            div()
                .h(look.length(token::SPACE_2))
                .w(gpui::relative(width))
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(token::FG_DEFAULT).opacity(0.07))
        };
        let bars = |widths: &[f32]| {
            div().flex().flex_col().gap(look.length(token::SPACE_2)).w_full().children(widths.iter().map(|width| bar(*width)))
        };
        let turn = |you: &[f32], agent: &[f32]| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .child(
                    div()
                        .p(look.length(token::SPACE_3))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .rounded(look.length(token::RADIUS_LG))
                        .bg(look.color(token::BG_RAISED))
                        .child(bars(you)),
                )
                .child(div().flex().flex_col().gap(look.length(token::SPACE_3)).child(bar(0.15)).child(bars(agent)))
        };
        let body = div()
            .size_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_6))
            .child(turn(&[0.7, 0.45], &[0.92, 0.8, 0.86, 0.4]))
            .child(turn(&[0.55], &[0.88, 0.64]))
            .with_animation(
                "skeleton",
                Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                |shapes, at| shapes.opacity(at),
            );
        labelled(
            div()
                .id("thread-loading")
                .size_full()
                .flex()
                .justify_center()
                .px(look.length(token::SPACE_4) + bleed(look))
                .pt(look.length(token::SPACE_4))
                .child(div().w_full().max_w(self.column(look)).child(body)),
            Role::Status,
            "Loading",
        )
        .into_any_element()
    }

}

impl Render for JobView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(id) = self.shown.clone() else {
            return div().size_full().into_any_element();
        };
        let picture_lost = self.picture_lost(window);
        if self.pages.get(&id).is_some_and(|page| page.keys.begin_frame(window)) || picture_lost {
            // The focused row scrolled out of the list: focus goes to the thread, so keys still reach it.
            window.focus(&self.focus, cx);
        }
        // Focus in a waiting card makes it the target; the Needs you panel keeps the one it answered last; focus
        // anywhere else gives the target back to the newest card.
        let dock_holds = self.dock_focus.contains_focused(window, cx);
        if let Some(page) = self.pages.get_mut(&id) {
            let held = page.keys.focused_card(window, cx);
            if held.is_some() && page.cards.target != held {
                page.cards.target = held;
            } else if held.is_none() && !dock_holds && page.cards.target.is_some() {
                page.cards.target = None;
            }
        }
        if self.pages.get(&id).is_some_and(|page| page.keys.take_changed()) {
            // A card came or went: the cards draw once more, so the chip shows on the target.
            cx.notify();
        }
        self.tend_kept(window, cx);
        self.sync_walk_tabs(&id, cx);
        let look = cx.look();
        // Aa moved how the thread reads (or Fleet's look arrived, or refused a change): measure every list again.
        let drawn = self.composer.quick.thread();
        if self.composer.thread_drawn != Some(drawn) {
            self.composer.thread_drawn = Some(drawn);
            self.pages.values().for_each(|page| page.list.remeasure());
        }
        // The column follows the width the thread was last drawn at; a change draws once more at the new one.
        let pane = head::drawn(window, cx, Role::Region, "Thread").map(|bounds| bounds.size.width);
        if pane.is_some() && pane != self.pane {
            self.pane = pane;
            self.pages.values().for_each(|page| page.list.remeasure());
            window.request_animation_frame();
        }
        let loaded = self.thread(&id, cx).is_some() && self.pages.get(&id).is_some_and(|page| page.version.is_some());
        // A refused read was said in a toast: the shapes stop, and the thread draws once a read is answered.
        let refused = !loaded
            && match &self.fleet {
                Some(fleet) => fleet.read(cx).thread_refused(&id),
                None => crate::mock::thread_refused(&id).is_some(),
            };
        let body = if refused {
            div().size_full().into_any_element()
        } else if loaded {
            let state = self.pages[&id].list.clone();
            labelled(
                self.fork_hooks(div().id("thread"), cx)
                    .key_context("thread_scroll")
                    .track_focus(&self.focus)
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| {
                        if !this.focus.contains_focused(window, cx) {
                            window.focus(&this.focus, cx);
                        }
                    }))
                    .on_action(cx.listener(Self::on_page_up))
                    .on_action(cx.listener(Self::on_page_down))
                    .on_action(cx.listener(Self::on_to_start))
                    .on_action(cx.listener(Self::on_to_end))
                    .size_full()
                    .pt(look.length(token::SPACE_4))
                    .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                    .child(list(state, cx.processor(|this, ix: usize, window, cx| this.render_entry(ix, window, cx))).size_full())
                    .child(fade(&look, true))
                    .child(fade(&look, false)),
                Role::Region,
                "Thread",
            )
            .into_any_element()
        } else {
            self.render_skeleton(look)
        };
        // A closed Thread tab leaves its pane empty under the strip, as Electron's does; a tab beside
        // it in front draws its own body there.
        let front = self.row_front(&id).cloned();
        let open = !self.tab_closed("thread") && front.is_none();
        let body = match &front {
            Some(tab) => self.row_tab_body(&id, tab, look, window, cx),
            None if open => body,
            None => div().into_any_element(),
        };
        let head = self.render_head(&id, cx);
        let origin = if open { self.render_origin(&id, window, cx) } else { None };
        let dock = (open && !self.closed(&id, cx)).then(|| self.render_dock(&id, window, cx));
        // The time from the press that opened the Job to this frame's paint, once per switch.
        let timing = self.switched.take();
        div()
            .size_full()
            .flex()
            .flex_col()
            .relative()
            .key_context("thread")
            .on_action(cx.listener(Self::on_act))
            .on_key_down(cx.listener(Self::on_job_escape))
            // Selected words need the window's selection layer; the Job's view is where words are selected.
            .child(gpui_base::TextSelectionLayer)
            .child(head)
            .children(origin)
            .child(div().relative().flex_1().min_h_0().child(body).children(self.render_scrim(&id, cx).filter(|_| open)).children(self.render_foot(&id, cx).filter(|_| open)))
            .children(self.render_bands(&id, cx).filter(|_| open))
            .children(dock)
            .children(self.tabs_menu.as_ref().map(|open| open.menu.clone()))
            .children(self.pop.as_ref().map(|pop| pop.menu.clone()))
            .children(self.render_confirm(window, cx))
            .children(self.render_editor(window, cx))
            .children(self.render_teach(cx))
            .children(self.render_review(&id, window, cx))
            .children(self.render_report(window, cx))
            .children(self.render_sketch(window, cx))
            .children(self.render_picture(window, cx))
            .children(self.render_sketch_reading(window, cx))
            .children(self.render_subagent_reading(window, cx))
            .children(self.render_pane_panel(window, cx))
            .child(canvas(
                |_, _, _| (),
                move |_, _, _, _| {
                    if let Some(started) = timing {
                        Note::info("a Job's view was drawn after a switch").field("micros", started.elapsed().as_micros() as i64).emit();
                    }
                },
            ))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thread(wire_items: &str, head: &str) -> (ThreadHead, Vec<ThreadItem>) {
        let wire = format!(r#"{{"session_id":"j","head":{head},"items":[{wire_items}]}}"#);
        let read = ipc::decode::<ipc::JobThread>("thread", wire.as_bytes()).expect("a thread");
        (read.head, read.items)
    }

    const TRAIL: &str = r#""trail":{"started":"2026-11-09T09:00:00Z"}"#;

    fn you(id: &str, text: &str) -> String {
        format!(r#"{{"kind":"row","row":{{"kind":"message","id":"{id}","at":"2026-11-09T10:00:00Z","from":{{"kind":"you"}},"text":"{text}"}}}}"#)
    }

    #[test]
    fn what_stands_over_the_thread_is_drawn_after_its_items_in_electrons_order() {
        let head = format!(r#"{{{TRAIL},"plan":{{"id":"plan:v2","version":2,"state":"waiting"}}}}"#);
        let (head, items) = thread(&you("a", "hi"), &head);
        let pending = [Pending { key: 1, text: "x".into(), seen: 0, tags: Vec::new(), body: None, failed: false }];
        let extras = extras_of(&head, &items, &pending, true, false, false);
        assert_eq!(extras, [Extra::Plan(PlanCardState::Waiting), Extra::Pending(0), Extra::Working]);
    }

    #[test]
    fn an_approved_plan_is_an_item_and_not_an_extra() {
        let head = format!(r#"{{{TRAIL},"plan":{{"id":"plan:v1","version":1,"state":"approved"}}}}"#);
        let (head, items) = thread("", &head);
        assert!(extras_of(&head, &items, &[], false, false, false).is_empty());
    }

    #[test]
    fn a_working_thread_that_ends_in_calls_draws_its_live_list_and_no_working_line() {
        let calls = r#"{"kind":"calls","id":"t1","names":["Bash"],"rows":[{"kind":"tool","id":"t1","at":"2026-11-09T10:00:00Z","text":"Bash ls"}]}"#;
        let (head, items) = thread(calls, &format!("{{{TRAIL}}}"));
        assert!(extras_of(&head, &items, &[], true, false, false).is_empty());
        let (head, items) = thread(&you("a", "hi"), &format!("{{{TRAIL}}}"));
        assert_eq!(extras_of(&head, &items, &[], true, false, false), [Extra::Working]);
    }

    #[test]
    fn the_persons_words_are_taken_once_fleets_row_for_them_is_there() {
        let (_, items) = thread(&format!("{},{}", you("a", "go on"), you("b", "other")), &format!("{{{TRAIL}}}"));
        assert_eq!(echoed(&items, "go on"), 1);
        assert_eq!(echoed(&items, "nope"), 0);
    }

    #[test]
    fn saved_words_stand_over_a_message_until_fleets_row_reads_them() {
        let (_, items) = thread(&format!("{},{}", you("a", "first try"), you("b", "other")), &format!("{{{TRAIL}}}"));
        assert!(!edit_read(&items, "a", "second try"), "Fleet's row still reads the old words");
        assert!(edit_read(&items, "a", "first try"));
        assert!(!edit_read(&items, "c", "first try"), "a row not in the thread reads nothing");
    }
}

/// The thread's age of a time, which the Operator's page flips to as the thread does.
pub(crate) use rows::age;
