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

mod calls;
mod composer;
mod head;
mod lines;
mod offer;
mod quick;
mod records;
mod rows;
mod runs;
mod talk;
mod tune;
mod wire;

use std::collections::{HashMap, HashSet};

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
        KeyBinding::new("pageup", PageUp, Some("thread")),
        KeyBinding::new("pagedown", PageDown, Some("thread")),
        KeyBinding::new("home", ToStart, Some("thread")),
        KeyBinding::new("end", ToEnd, Some("thread")),
    ]);
    head::bind(cx);
}

/// What a page draws after the thread's items: what stands over the whole thread, the person's
/// words not yet taken by Fleet, and the working line (A1.4).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Extra {
    Plan(PlanCardState),
    Sketch(String),
    Asked(String),
    Lapsed,
    Pending(usize),
    Working,
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
    count: usize,
    extras: Vec<Extra>,
    /// The thread version the list was last measured for.
    version: Option<u64>,
    /// The folds the person opened, by item id.
    opened: HashSet<String>,
    pending: Vec<Pending>,
    /// Fleet's reason for the last send it refused.
    refused: Option<SharedString>,
}

impl Page {
    fn new() -> Self {
        let list = ListState::new(0, ListAlignment::Top, gpui::px(600.));
        // The thread opens at its newest row and stays there until the person scrolls up.
        list.set_follow_mode(FollowMode::Tail);
        Page { list, items: 0, count: 0, extras: Vec::new(), version: None, opened: HashSet::new(), pending: Vec::new(), refused: None }
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
    if let Some(asked) = &head.asked {
        extras.push(Extra::Asked(asked.command.clone()));
    }
    if head.lapsed.is_some() {
        extras.push(Extra::Lapsed);
    }
    extras.extend((0..pending.len()).map(Extra::Pending));
    let running = items.iter().any(|item| matches!(item, ThreadItem::Subagent { subagent, .. } if subagent.state == ipc::ToolSubagentState::Running));
    let ends_in_calls = matches!(items.last(), Some(ThreadItem::Calls { .. }));
    if working && !running && (!ends_in_calls || head.asked.is_some()) {
        extras.push(Extra::Working);
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
            subscriptions.push(cx.observe(fleet, |this, _, cx| this.synced(cx)));
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
        let mock = mock
            .unwrap_or_default()
            .into_iter()
            .map(|(id, thread)| (id, HeldThread::from_read(thread, 1)))
            .collect();
        JobView {
            list,
            fleet,
            mock,
            shown: None,
            pages: HashMap::new(),
            head: head::Head::new(cx),
            composer,
            aged: false,
            visiting: None,
            switched: None,
            focus: cx.focus_handle(),
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
        }
    }

    /// Put another Job on screen: its place was kept, so this swaps what is drawn and the box's
    /// words, tells Fleet who is looking, and reads the thread if it is not held.
    fn switch(&mut self, to: Option<SharedString>, window: &mut Window, cx: &mut Context<Self>) {
        let started = std::time::Instant::now();
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
        let dirty = self.fleet.as_ref().and_then(|fleet| fleet.update(cx, |fleet, _| fleet.take_dirty(id)));
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
        let extras = extras_of(&held.head, &held.items, &page.pending, working);
        let page = self.pages.get_mut(id).expect("the page was just read");
        if page.version == Some(version) && page.extras == extras && dirty.is_none() {
            return;
        }
        let total = items + extras.len();
        let from = if page.version.is_none() { 0 } else { dirty.unwrap_or(page.items).min(page.items).min(items) };
        page.list.splice(from..page.count, total - from);
        page.items = items;
        page.count = total;
        page.extras = extras;
        page.version = Some(version);
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
        page.list.remeasure_items(at..at + 1);
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
        let look = cx.look();
        let Some(id) = self.shown.clone() else {
            return div().size_full().into_any_element();
        };
        let loaded = self.thread(&id, cx).is_some() && self.pages.get(&id).is_some_and(|page| page.version.is_some());
        let body = if loaded {
            let state = self.pages[&id].list.clone();
            labelled(
                div()
                    .id("thread")
                    .key_context("thread")
                    .track_focus(&self.focus)
                    .on_action(cx.listener(Self::on_page_up))
                    .on_action(cx.listener(Self::on_page_down))
                    .on_action(cx.listener(Self::on_to_start))
                    .on_action(cx.listener(Self::on_to_end))
                    .size_full()
                    .pt(look.length(token::SPACE_4))
                    .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                    .child(list(state, cx.processor(|this, ix: usize, window, cx| this.render_entry(ix, window, cx))).size_full()),
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
            .on_action(cx.listener(Self::on_act))
            .child(head)
            .children(origin)
            .child(div().relative().flex_1().min_h_0().child(body))
            .children(dock)
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
        let head = format!(r#"{{{TRAIL},"plan":{{"id":"plan:v2","version":2,"state":"waiting"}},"lapsed":{{"id":"l","questions":[]}}}}"#);
        let (head, items) = thread(&you("a", "hi"), &head);
        let pending = [Pending { key: 1, text: "x".into(), seen: 0 }];
        let extras = extras_of(&head, &items, &pending, true);
        assert_eq!(extras, [Extra::Plan(PlanCardState::Waiting), Extra::Lapsed, Extra::Pending(0), Extra::Working]);
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

    #[test]
    fn a_draft_is_set_cleared_and_only_a_change_asks_for_a_write() {
        let mut drafts = Drafts { by_job: HashMap::new(), remember: false };
        assert!(drafts.set("a", "words"));
        assert!(!drafts.set("a", "words"));
        assert_eq!(drafts.get("a"), "words");
        assert!(drafts.set("a", ""));
        assert!(!drafts.set("a", ""));
        assert_eq!(drafts.get("a"), "");
        assert!(drafts.encoded().is_none(), "mock data keeps nothing");
    }
}
