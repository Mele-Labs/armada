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
mod dock;
mod fork;
mod head;
mod judge;
mod keys;
mod lines;
mod menus;
mod offer;
mod perm;
mod pinned;
mod plan;
mod plan_edit;
mod plan_marks;
mod pop;
mod quick;
mod teach;
mod records;
mod review;
mod rows;
mod runs;
mod talk;
mod tune;
mod wire;
mod workflow_card;

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
}

/// One Job's place: kept while it is not on screen.
struct Page {
    list: ListState,
    /// How many of the list's elements are the thread's items; the rest are [`Extra`]s.
    items: usize,
    /// Whether the list opens with the Show earlier row (1) or not (0): older items are served.
    lead: usize,
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
}

impl Page {
    fn new() -> Self {
        let list = ListState::new(0, ListAlignment::Top, gpui::px(600.));
        // The thread opens at its newest row and stays there until the person scrolls up.
        list.set_follow_mode(FollowMode::Tail);
        Page { list, items: 0, lead: 0, count: 0, extras: Vec::new(), version: None, opened: HashSet::new(), pending: Vec::new(), refused: None, cards: acts::CardState::default(), dock: dock::DockState::default(), bands: bands::BandState::default(), keys: Rc::default(), arrived: 0..0, review: review::ReviewState::default(), plan_edit: plan_edit::EditState::default() }
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
fn extras_of(head: &ThreadHead, items: &[ThreadItem], pending: &[Pending], working: bool) -> Vec<Extra> {
    let mut extras = Vec::new();
    if let Some(plan) = &head.plan {
        if plan.state != PlanCardState::Approved {
            extras.push(Extra::Plan(plan.state));
        }
    }
    if let Some(sketch) = &head.sketch {
        extras.push(Extra::Sketch(sketch.title.clone()));
    }
    extras.extend((0..pending.len()).map(Extra::Pending));
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
    /// The workflow changes of a mock scenario, by id, as Fleet's `workflow_changes` holds them live.
    mock_changes: std::collections::BTreeMap<i64, ipc::WorkflowChange>,
    /// The Job on screen.
    shown: Option<SharedString>,
    pages: HashMap<SharedString, Page>,
    head: head::Head,
    composer: composer::Composer,
    /// Every stamp in the thread reads as an age, not a clock (a press flips them all).
    aged: bool,
    /// The Job fleet was last told is on screen.
    visiting: Option<SharedString>,
    switched: Option<std::time::Instant>,
    focus: FocusHandle,
    /// The menu a thread element opened, if one is open.
    pop: Option<pop::Pop>,
    /// The Needs you panel, so its keys work once focus is in it.
    dock_focus: FocusHandle,
    /// The dialog a thread key opens to ask which card it goes to.
    confirm: Option<keys::Confirm>,
    /// The bar under agent messages: its responses, what was just copied, and the editor of a response.
    bar: bar::Bar,
    /// Words selected in the thread and the bar over them.
    fork: fork::Fork,
    /// The Judge's box on a plan, while it is open.
    judge: Option<judge::Open>,
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
    _subscriptions: Vec<Subscription>,
}

impl JobView {
    pub fn new(
        list: Entity<JobsList>,
        mock: Option<HashMap<String, ipc::JobThread>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let fleet = list.read(cx).fleet();
        let remember = list.read(cx).remembers();
        let (mut composer, mut subscriptions) = composer::build(remember, window, cx);
        if let Some(catalogue) = crate::mock::catalogue() {
            composer.catalog.models = Some(catalogue.models);
            composer.catalog.models_asked = true;
            composer.catalog.mock_files = catalogue.files;
            composer.catalog.issues.insert(catalogue.issues.manifest_id.as_str().to_string(), catalogue.issues.issues);
            composer.quick = quick::Quick::mock(catalogue.look, catalogue.quick);
            composer.catalog.scripts = catalogue.talk;
        }
        subscriptions.push(cx.observe_in(&list, window, |this, _, window, cx| this.followed(window, cx)));
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
        subscriptions.push(cx.on_release(|this, cx| {
            if let Some(job) = this.visiting.take() {
                let port = this.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port());
                tell_fleet(port, &job, false, cx);
            }
        }));
        let mock_changes = match &mock {
            Some(_) => crate::mock::workflow_changes().unwrap_or_else(|why| {
                crate::log::Note::warn("the mock's workflow changes did not decode").field("why", why).emit();
                Default::default()
            }),
            None => Default::default(),
        };
        let mock = mock
            .unwrap_or_default()
            .into_iter()
            .map(|(id, thread)| (id, HeldThread::from_read(thread, 1)))
            .collect();
        JobView {
            list,
            fleet,
            mock,
            mock_changes,
            shown: None,
            pages: HashMap::new(),
            head: head::Head::new(cx),
            composer,
            aged: false,
            visiting: None,
            switched: None,
            focus: cx.focus_handle(),
            pop: None,
            dock_focus: cx.focus_handle(),
            confirm: None,
            bar: bar::Bar::new(),
            fork: fork::Fork::default(),
            judge: None,
            teach: None,
            teach_said: None,
            teach_clock: 0,
            confirm_focus: cx.focus_handle(),
            confirm_stops: [cx.focus_handle(), cx.focus_handle()],
            review_focus: review::Focuses::new(cx),
            review_rows: Rc::new(RefCell::new(HashMap::new())),
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
        let opened = self.list.read(cx).opened_id();
        if opened != self.shown {
            self.switch(opened, window, cx);
        } else {
            self.synced(cx);
            self.review_sync(window, cx);
        }
    }

    /// Put another Job on screen: its place was kept, so this swaps what is drawn and the box's
    /// words, tells Fleet who is looking, and reads the thread if it is not held.
    fn switch(&mut self, to: Option<SharedString>, window: &mut Window, cx: &mut Context<Self>) {
        let started = std::time::Instant::now();
        // The sheet is the Job's: it shuts when another Job comes on screen.
        self.close_review(window, cx);
        self.composer_leave(cx);
        self.shown = to.clone();
        self.switched = Some(started);
        if let Some(id) = &to {
            self.pages.entry(id.clone()).or_insert_with(Page::new);
            if let Some(fleet) = self.fleet.clone() {
                let id = id.clone();
                fleet.update(cx, |fleet, cx| fleet.read_thread(&id, true, cx));
            }
            self.sync(id, cx);
        }
        self.composer_enter(to.as_ref(), window, cx);
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
        self.read_workflow_changes(cx);
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
        let dirty = self.fleet.as_ref().and_then(|fleet| fleet.update(cx, |fleet, _| fleet.take_dirty(id)));
        let added = self.fleet.as_ref().map_or(0, |fleet| fleet.update(cx, |fleet, _| fleet.take_prepended(id)));
        let working = self.working(id, cx);
        let settled: Vec<u64> = match (self.thread(id, cx), self.pages.get(id)) {
            (Some(held), Some(page)) => {
                page.pending.iter().filter(|one| echoed(&held.items, &one.text) > one.seen).map(|one| one.key).collect()
            }
            _ => Vec::new(),
        };
        if let Some(page) = self.pages.get_mut(id) {
            page.pending.retain(|one| !settled.contains(&one.key));
        }
        let Some(page) = self.pages.get(id) else { return };
        let Some(held) = self.thread(id, cx) else { return };
        let (version, items) = (held.version, held.items.len());
        let lead = usize::from(held.older.is_some());
        let extras = extras_of(&held.head, &held.items, &page.pending, working);
        let page = self.pages.get_mut(id).expect("the page was just read");
        if page.version == Some(version) && page.extras == extras && dirty.is_none() && lead == page.lead {
            return;
        }
        let total = lead + items + extras.len();
        let from = if page.version.is_none() { 0 } else { dirty.unwrap_or(page.items).min(page.items).min(items) };
        let from_list = if lead != page.lead || from == 0 { 0 } else { lead + from };
        // Items put in front leave the person where they were reading.
        let top = (added > 0).then(|| page.list.logical_scroll_top());
        page.list.splice(from_list..page.count, total - from_list);
        if let Some(top) = top {
            page.list.scroll_to(ListOffset { item_ix: (top.item_ix + added + lead).saturating_sub(page.lead), offset_in_item: top.offset_in_item });
        }
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

    fn on_page_up(&mut self, _: &PageUp, _: &mut Window, cx: &mut Context<Self>) {
        self.scroll_by(-0.9, cx);
    }

    fn on_page_down(&mut self, _: &PageDown, _: &mut Window, cx: &mut Context<Self>) {
        self.scroll_by(0.9, cx);
    }

    /// Read the thread's older items when the top comes near, and when the row is pressed.
    pub(crate) fn want_earlier(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else { return };
        if self.thread(id, cx).is_some_and(|held| held.older.is_some()) {
            fleet.update(cx, |fleet, cx| fleet.read_older(id, cx));
        }
    }

    fn on_to_start(&mut self, _: &ToStart, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get(id)) {
            page.list.scroll_to(ListOffset { item_ix: 0, offset_in_item: gpui::px(0.) });
        }
        cx.notify();
    }

    fn on_to_end(&mut self, _: &ToEnd, _: &mut Window, cx: &mut Context<Self>) {
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
        page.list.remeasure_items(at + page.lead..at + page.lead + 1);
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
                .px(look.length(token::SPACE_6) + look.length(token::SPACE_2))
                .pt(look.length(token::SPACE_4))
                .child(div().w_full().max_w(look.length(token::SPACE_12) * 20.).child(body)),
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
        if self.pages.get(&id).is_some_and(|page| page.keys.begin_frame(window)) {
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
        let look = cx.look();
        // Aa moved how the thread reads (or Fleet's look arrived, or refused a change): measure every list again.
        let drawn = self.composer.quick.thread();
        if self.composer.thread_drawn != Some(drawn) {
            self.composer.thread_drawn = Some(drawn);
            self.pages.values().for_each(|page| page.list.remeasure());
        }
        let loaded = self.thread(&id, cx).is_some() && self.pages.get(&id).is_some_and(|page| page.version.is_some());
        let body = if loaded {
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
        let head = self.render_head(&id, cx);
        let origin = self.render_origin(&id, cx);
        let dock = (!self.closed(&id, cx)).then(|| self.render_dock(&id, window, cx));
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
            .child(div().relative().flex_1().min_h_0().child(body).children(self.render_scrim(&id, cx)).children(self.render_foot(&id, cx)))
            .children(self.render_bands(&id, cx))
            .children(dock)
            .children(self.pop.as_ref().map(|pop| pop.menu.clone()))
            .children(self.render_confirm(window, cx))
            .children(self.render_editor(window, cx))
            .children(self.render_fork_bar(cx))
            .children(self.render_teach(cx))
            .children(self.render_review(&id, window, cx))
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
        let pending = [Pending { key: 1, text: "x".into(), seen: 0 }];
        let extras = extras_of(&head, &items, &pending, true);
        assert_eq!(extras, [Extra::Plan(PlanCardState::Waiting), Extra::Pending(0), Extra::Working]);
    }

    #[test]
    fn an_approved_plan_is_an_item_and_not_an_extra() {
        let head = format!(r#"{{{TRAIL},"plan":{{"id":"plan:v1","version":1,"state":"approved"}}}}"#);
        let (head, items) = thread("", &head);
        assert!(extras_of(&head, &items, &[], false).is_empty());
    }

    #[test]
    fn a_working_thread_that_ends_in_calls_draws_its_live_list_and_no_working_line() {
        let calls = r#"{"kind":"calls","id":"t1","names":["Bash"],"rows":[{"kind":"tool","id":"t1","at":"2026-11-09T10:00:00Z","text":"Bash ls"}]}"#;
        let (head, items) = thread(calls, &format!("{{{TRAIL}}}"));
        assert!(extras_of(&head, &items, &[], true).is_empty());
        let (head, items) = thread(&you("a", "hi"), &format!("{{{TRAIL}}}"));
        assert_eq!(extras_of(&head, &items, &[], true), [Extra::Working]);
    }

    #[test]
    fn the_persons_words_are_taken_once_fleets_row_for_them_is_there() {
        let (_, items) = thread(&format!("{},{}", you("a", "go on"), you("b", "other")), &format!("{{{TRAIL}}}"));
        assert_eq!(echoed(&items, "go on"), 1);
        assert_eq!(echoed(&items, "nope"), 0);
    }
}

/// The thread's age of a time, which the Operator's page flips to as the thread does.
pub(crate) use rows::age;
