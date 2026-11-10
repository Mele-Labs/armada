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

mod acts;
mod calls;
mod cards;
mod deck;
mod dock;
mod keys;
mod lines;
mod menus;
mod perm;
mod plan;
mod pinned;
mod pop;
mod records;
mod rows;
mod runs;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    actions, canvas, div, list, prelude::*, svg, Animation, AnimationExt, AnyElement, App, Context, Entity,
    FocusHandle, Focusable, FollowMode, FontWeight, KeyBinding, ListAlignment, ListOffset, ListState, Role, SharedString,
    Subscription, Task, Window,
};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use ipc::{
    JobState, PlanCardState, SendSessionMessage, SendSessionMessageBody, SessionId, SessionRecord, SessionRow, SessionState,
    SessionVoice, ThreadHead, ThreadItem, ViewSession,
};

use crate::assets::icon_path;
use crate::fleet::{FleetConnection, HeldThread};
use crate::jobs::{glyph_of, hue_of, JobsList};
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
    /// The cards here that wait, and what the four thread keys do to each.
    keys: Rc<keys::Cards>,
}

impl Page {
    fn new() -> Self {
        let list = ListState::new(0, ListAlignment::Top, gpui::px(600.));
        // The thread opens at its newest row and stays there until the person scrolls up.
        list.set_follow_mode(FollowMode::Tail);
        Page { list, items: 0, lead: 0, count: 0, extras: Vec::new(), version: None, opened: HashSet::new(), pending: Vec::new(), refused: None, cards: acts::CardState::default(), dock: dock::DockState::default(), keys: Rc::default() }
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

/// The drafts the person left in Jobs' message boxes, by Job id. Kept between runs as Electron keeps
/// each box under `keep` key `session:<id>` (`keep.ts`), except on mock data.
struct Drafts {
    by_job: HashMap<String, String>,
    remember: bool,
}

fn drafts_file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("native-drafts"))
}

impl Drafts {
    fn read(remember: bool) -> Self {
        let by_job = remember
            .then(|| drafts_file().and_then(|path| std::fs::read(path).ok()))
            .flatten()
            .and_then(|bytes| ipc::decode::<HashMap<String, String>>("drafts", &bytes).ok())
            .unwrap_or_default();
        Drafts { by_job, remember }
    }

    fn set(&mut self, job: &str, text: &str) -> bool {
        if text.is_empty() {
            self.by_job.remove(job).is_some()
        } else if self.by_job.get(job).map(String::as_str) != Some(text) {
            self.by_job.insert(job.to_string(), text.to_string());
            true
        } else {
            false
        }
    }

    fn get(&self, job: &str) -> &str {
        self.by_job.get(job).map_or("", String::as_str)
    }

    /// The file's whole text, for a write off the main thread.
    fn encoded(&self) -> Option<(PathBuf, String)> {
        if !self.remember {
            return None;
        }
        Some((drafts_file()?, ipc::encode(&self.by_job).ok()?))
    }
}

pub struct JobView {
    list: Entity<JobsList>,
    fleet: Option<Entity<FleetConnection>>,
    /// The threads of a mock scenario, which never change but by the person's own words.
    mock: HashMap<String, HeldThread>,
    /// The Job on screen.
    shown: Option<SharedString>,
    pages: HashMap<SharedString, Page>,
    composer: Entity<TextareaState>,
    drafts: Drafts,
    /// The box is being set to a draft, which is not the person typing.
    setting: bool,
    keeping: Option<Task<()>>,
    /// Every stamp in the thread reads as an age, not a clock (a press flips them all).
    aged: bool,
    next_key: u64,
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
    confirm_focus: FocusHandle,
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
        let composer = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8).submit_on_enter(true));
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe_in(&composer, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::Change => this.typed(cx),
            InputEvent::PressEnter { .. } => this.send(window, cx),
            _ => {}
        }));
        subscriptions.push(cx.observe_in(&list, window, |this, _, window, cx| this.followed(window, cx)));
        if let Some(fleet) = &fleet {
            subscriptions.push(cx.observe(fleet, |this, _, cx| this.synced(cx)));
        }
        subscriptions.push(cx.observe_window_activation(window, |this, window, cx| {
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
            composer,
            drafts: Drafts::read(remember),
            setting: false,
            keeping: None,
            aged: false,
            next_key: 0,
            visiting: None,
            switched: None,
            focus: cx.focus_handle(),
            pop: None,
            dock_focus: cx.focus_handle(),
            confirm: None,
            confirm_focus: cx.focus_handle(),
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
        self.record(id, cx).is_some_and(|job| match &job.hosted {
            Some(hosted) => matches!(hosted.turn, ipc::SessionTurn::Working { .. }),
            None => job.mark.is_some_and(|mark| mark.state == JobState::Working),
        })
    }

    /// The Job's message box is not drawn once it has ended or gone quiet (`session.dead`).
    fn closed(&self, id: &str, cx: &App) -> bool {
        self.record(id, cx).is_none_or(|job| {
            job.state == SessionState::Ended || job.mark.is_some_and(|mark| matches!(mark.state, JobState::Ended | JobState::Quiet))
        })
    }

    /// ⌥M: the caret goes in the box, from anywhere in the Job's view (`message_job`).
    pub fn to_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.shown.as_ref().is_some_and(|id| !self.closed(id, cx)) {
            self.composer.update(cx, |composer, cx| composer.focus(window, cx));
        }
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
        self.leave_box(cx);
        self.shown = to.clone();
        self.switched = Some(started);
        if let Some(id) = &to {
            self.pages.entry(id.clone()).or_insert_with(Page::new);
            let text = self.drafts.get(id).to_string();
            self.setting = true;
            self.composer.update(cx, |composer, cx| composer.set_value(text, window, cx));
            self.setting = false;
            if let Some(fleet) = self.fleet.clone() {
                let id = id.clone();
                fleet.update(cx, |fleet, cx| fleet.read_thread(&id, true, cx));
            }
            self.sync(id, cx);
        } else {
            self.setting = true;
            self.composer.update(cx, |composer, cx| composer.set_value("", window, cx));
            self.setting = false;
        }
        let on = window.is_window_active().then(|| to.clone()).flatten();
        self.visit(on, cx);
        cx.notify();
    }

    /// The words in the box are the shown Job's draft; they are written down after a pause, not on every key.
    fn typed(&mut self, cx: &mut Context<Self>) {
        if self.setting {
            return;
        }
        let Some(id) = self.shown.clone() else { return };
        let text = self.composer.read(cx).value().to_string();
        if self.drafts.set(&id, &text) {
            self.keep_soon(cx);
        }
        cx.notify();
    }

    /// Fold what is in the box into the shown Job's draft before it leaves the screen.
    fn leave_box(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let text = self.composer.read(cx).value().to_string();
        if self.drafts.set(&id, &text) {
            self.keep_soon(cx);
        }
    }

    /// Write the drafts 400 ms after the last change, off the main thread (Electron saves a look
    /// that long after a change; `keep.ts` writes at once).
    fn keep_soon(&mut self, cx: &mut Context<Self>) {
        let Some((path, text)) = self.drafts.encoded() else { return };
        self.keeping = Some(cx.spawn(async move |_, cx| {
            cx.background_executor().timer(Duration::from_millis(400)).await;
            cx.background_executor()
                .spawn(async move {
                    let kept = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&path, text));
                    if let Err(why) = kept {
                        Note::warn("the drafts were not kept").field("why", why.to_string()).emit();
                    }
                })
                .await;
        }));
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
        self.arrive(id, cx);
    }

    /// Send what is in the box. The words are in the thread this frame; Fleet's row replaces them
    /// when it arrives, and a refusal puts them back with Fleet's reason.
    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        if self.closed(&id, cx) {
            return;
        }
        let text = self.composer.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.next_key += 1;
        let key = self.next_key;
        let seen = self.thread(&id, cx).map_or(0, |held| echoed(&held.items, &text));
        self.setting = true;
        self.composer.update(cx, |composer, cx| composer.set_value("", window, cx));
        self.setting = false;
        if self.drafts.set(&id, "") {
            self.keep_soon(cx);
        }
        let page = self.pages.entry(id.clone()).or_insert_with(Page::new);
        page.refused = None;
        match self.fleet.clone() {
            Some(fleet) => {
                page.pending.push(Pending { key, text: text.clone(), seen });
                let Some(port) = fleet.read(cx).held().port() else {
                    self.refused(&id, key, text, "Fleet is not connected".into(), window, cx);
                    return;
                };
                let body = SendSessionMessageBody {
                    message: SendSessionMessage { session_id: SessionId::carried(id.to_string()), text: text.clone(), attachments: Vec::new(), mentions: Vec::new() },
                    aim: None,
                };
                let sent = ipc::encode(&body).map_err(|why| format!("{why:?}"));
                let job = id.clone();
                cx.spawn_in(window, async move |this, cx| {
                    let answer = cx
                        .background_executor()
                        .spawn(async move {
                            let body = sent?;
                            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                            let (status, bytes) = runtime.block_on(crate::transport::post(port, "/sessions/message", body))?;
                            if (200..300).contains(&status) {
                                return Ok(());
                            }
                            Err(ipc::decode::<ipc::WireError>("WireError", &bytes)
                                .map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
                        })
                        .await;
                    if let Err(why) = answer {
                        let _ = this.update_in(cx, |this, window, cx| this.refused(&job, key, text, why.into(), window, cx));
                    }
                })
                .detach();
            }
            None => {
                // Mock data: Fleet's row for these words arrives at once.
                let at = ipc::Instant::carried(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
                if let Some(held) = self.mock.get_mut(id.as_str()) {
                    held.items.push(ThreadItem::Row {
                        row: SessionRow::Message {
                            id: format!("sent-{key}"),
                            at,
                            from: SessionVoice::You,
                            text,
                            files: Vec::new(),
                            tags: Vec::new(),
                            notice: None,
                            queued: false,
                        },
                    });
                    held.version += 1;
                    let at = held.items.len() - 1;
                    held.mark_dirty(at);
                }
                self.sync_mock(&id);
            }
        }
        self.pin(&id);
        self.sync(&id, cx);
        cx.notify();
    }

    fn sync_mock(&mut self, _id: &SharedString) {}

    /// A message from the person puts the thread at its end again (A1.5).
    fn pin(&mut self, id: &SharedString) {
        if let Some(page) = self.pages.get(id) {
            page.list.set_follow_mode(FollowMode::Tail);
        }
    }

    /// Fleet refused the words: they go back in the box unless something was typed since, and its
    /// reason stands over the box.
    fn refused(&mut self, id: &SharedString, key: u64, text: String, why: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(id) {
            page.pending.retain(|one| one.key != key);
            page.refused = Some(why);
        }
        if self.shown.as_ref() == Some(id) {
            if self.composer.read(cx).value().is_empty() {
                self.setting = true;
                self.composer.update(cx, |composer, cx| composer.set_value(text.clone(), window, cx));
                self.setting = false;
                self.drafts.set(id, &text);
                self.keep_soon(cx);
            }
        } else if self.drafts.get(id).is_empty() {
            self.drafts.set(id, &text);
            self.keep_soon(cx);
        }
        self.sync(id, cx);
        cx.notify();
    }

    fn on_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) {
        // ⌘Enter sends from the box too (`send_message`).
        let thread_key = [keys::Which::Yes, keys::Which::Change, keys::Which::SendBack, keys::Which::No].into_iter().find(|which| act.is(which.act()));
        if let Some(which) = thread_key {
            self.thread_key(which, window, cx);
        } else if act.is("send_message") && self.composer.read(cx).focus_handle(cx).contains_focused(window, cx) {
            self.send(window, cx);
        } else {
            cx.propagate();
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

    /// Words go in the box as a quote, and the caret with them (Reply, `quoteIntoDraft`).
    pub(crate) fn quote(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let quoted: String = text
            .trim()
            .lines()
            .map(|line| if line.is_empty() { ">".to_string() } else { format!("> {line}") })
            .collect::<Vec<_>>()
            .join("\n");
        let held = self.composer.read(cx).value().to_string();
        let joined = if held.trim().is_empty() { format!("{quoted}\n\n") } else { format!("{held}\n\n{quoted}\n\n") };
        self.composer.update(cx, |composer, cx| {
            composer.set_value(joined.clone(), window, cx);
            composer.focus(window, cx);
        });
        self.typed_now(cx);
    }

    fn typed_now(&mut self, cx: &mut Context<Self>) {
        self.setting = false;
        self.typed(cx);
    }

    /// The head: the Job's Thread tab, as the Job panes draw it (1.1). Its mark says the state, its
    /// title is the tab's label, and More opens the Job's acts.
    fn render_head(&self, id: &SharedString, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look();
        let job = self.record(id, cx);
        let paused = job.is_some_and(|job| job.hosted.as_ref().is_some_and(|hosted| hosted.paused.is_some()));
        let title: SharedString = job.and_then(|job| job.title.clone()).map_or_else(|| "Thread".into(), Into::into);
        let name: SharedString = job.and_then(|job| job.title.as_ref()).map_or_else(|| "Thread".into(), |title| format!("Thread: {title}").into());
        let mark = job.and_then(|job| job.mark).map(|mark| {
            if paused {
                (("circle-pause", SharedString::from("Paused")), token::FG_MUTED, false)
            } else {
                (glyph_of(mark.state, None), hue_of(mark.state), mark.state == JobState::Working)
            }
        });
        let mono = look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default();
        let glyph = look.length(token::SPACE_4);
        let tip = |said: SharedString| move |window: &mut Window, cx: &mut App| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx);
        let state = mark.map(|((icon, said), hue, working)| {
            let said_in: SharedString = format!("{said} \u{b7} {id}").into();
            let ink = look.color(hue);
            let drawn = if working {
                svg()
                    .path(icon_path("loader-circle"))
                    .size(glyph)
                    .text_color(ink)
                    .with_animation(
                        "head-working",
                        Animation::new(look.duration(token::DURATION_SPIN)).repeat(),
                        |mark, delta| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))),
                    )
                    .into_any_element()
            } else {
                svg().path(icon_path(icon)).size(glyph).text_color(ink).into_any_element()
            };
            labelled(div().id("head-state").flex_none().child(drawn).tooltip(tip(said_in)), Role::Image, said)
        });
        let more = labelled(
            div()
                .id("head-more")
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL))
                .rounded(look.length(token::RADIUS_LG))
                .cursor_pointer()
                .text_color(look.color(token::FG_MUTED))
                .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                .child(svg().path(icon_path("ellipsis")).size(glyph).text_color(look.color(token::FG_MUTED)))
                .tooltip(tip("More".into()))
                .on_click(cx.listener(|this, event: &gpui::ClickEvent, window, cx| {
                    // The list's own menu for this Job, which gives the keys back to the list when it shuts.
                    if let Some(id) = this.shown.clone() {
                        let at = event.position();
                        this.list.update(cx, |list, cx| list.open_menu(id, at, false, window, cx));
                    }
                })),
            Role::Button,
            "More",
        );
        let tab = labelled(
            div()
                .id("head-tab")
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .pl(look.length(token::SPACE_3))
                .pr(look.length(token::SPACE_1))
                .bg(look.color(token::BG_RAISED))
                .border_r(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_GLASS))
                .children(state)
                .child(
                    div()
                        .id("head-title")
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_family(mono)
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(title.to_uppercase())
                        .tooltip(tip(title.clone())),
                )
                .child(more),
            Role::Row,
            name,
        );
        div()
            .flex_none()
            .flex()
            .items_stretch()
            .h(look.length(token::H_CONTROL_LG))
            .bg(look.color(token::BG_SUNKEN))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_GLASS))
            .child(div().max_w(look.length(token::SPACE_12) * 8.).min_w_0().flex().h_full().child(tab))
            .into_any_element()
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

    fn render_dock(&self, id: &SharedString, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let look = cx.look();
        let refused = self.pages.get(id).and_then(|page| page.refused.clone());
        let empty = self.composer.read(cx).value().trim().is_empty();
        let focused = self.composer.read(cx).focus_handle(cx).contains_focused(window, cx);
        let send = labelled(
            div()
                .id("send")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL_LG))
                .px(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_LG))
                .bg(look.color(token::ACCENT))
                .text_color(look.color(token::TEXT_ON_ACCENT))
                .text_size(look.length(token::TEXT_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .when(empty, |button| button.opacity(0.4))
                .when(!empty, |button| {
                    button
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::ACCENT_HOVER)))
                        .on_click(cx.listener(|this, _, window, cx| this.send(window, cx)))
                })
                .child(svg().path(icon_path("send")).size(look.length(token::SPACE_3)).text_color(look.color(token::TEXT_ON_ACCENT)))
                .child("Send"),
            Role::Button,
            "Send",
        );
        let column = look.length(token::SPACE_12) * 20. + look.length(token::SPACE_6) + look.length(token::SPACE_2) + look.length(token::SPACE_4);
        div()
            .flex_none()
            .flex()
            .justify_center()
            .px(look.length(token::SPACE_4))
            .pb(look.length(token::SPACE_4))
            .child(
                div()
                    .w_full()
                    .max_w(column)
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .children(refused.map(|why| {
                        labelled(
                            div()
                                .id("refused")
                                .px(look.length(token::SPACE_3))
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_SM))
                                .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                                .child(why),
                            Role::Status,
                            "Refused",
                        )
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .rounded(look.length(token::RADIUS_LG))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(if focused { token::BORDER_DEFAULT } else { token::BORDER_SUBTLE }))
                            .bg(look.color(token::BG_SUNKEN))
                            .child(
                                labelled(
                                    div()
                                        .id("message")
                                        .key_context(crate::keymap::TEXT_BOX)
                                        .px(look.length(token::SPACE_4))
                                        .py(look.length(token::SPACE_3))
                                        .text_size(look.length(token::TEXT_BASE))
                                        .text_color(look.color(token::FG_DEFAULT))
                                        .child(Textarea::new(&self.composer).appearance(false).aria_label("Message")),
                                    Role::TextInput,
                                    "Message",
                                ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .gap(look.length(token::SPACE_2))
                                    .px(look.length(token::SPACE_2))
                                    .py(look.length(token::SPACE_2))
                                    .border_t(look.length(token::BORDER_WIDTH))
                                    .border_color(look.color(token::BORDER_SUBTLE))
                                    .child(send),
                            ),
                    ),
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
        if self.pages.get(&id).is_some_and(|page| page.keys.take_changed()) {
            // A card came or went: the cards draw once more, so the chip shows on the target.
            cx.notify();
        }
        let look = cx.look();
        let loaded = self.thread(&id, cx).is_some() && self.pages.get(&id).is_some_and(|page| page.version.is_some());
        let body = if loaded {
            let state = self.pages[&id].list.clone();
            labelled(
                div()
                    .id("thread")
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
                    .child(list(state, cx.processor(|this, ix: usize, window, cx| this.render_entry(ix, window, cx))).size_full()),
                Role::Region,
                "Thread",
            )
            .into_any_element()
        } else {
            self.render_skeleton(look)
        };
        let head = self.render_head(&id, cx);
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
            .child(head)
            .child(div().relative().flex_1().min_h_0().child(body).children(self.render_scrim(&id, cx)))
            .children(self.render_needs(&id, cx))
            .children(dock)
            .children(self.pop.as_ref().map(|pop| pop.menu.clone()))
            .children(self.render_confirm(cx))
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
