//! The Story tab of a Job: its asks as cards, open ones first, each a head line that unfolds to how it started, what he
//! was shown and decided, what was built and what was handed on, and under which feature it is filed.
//!
//! What is read is Fleet's own: `get_job_story`, off the main thread, drawn as bones until it answers. `story/model.rs`
//! works out every line. The tab is a view tab (`view_tabs.rs`) the Job's + offers, so no file of the Job view knows it.
//!
//! **A story is read again when Fleet says it moved** (`session.story`, as `StoryMoved`, for this Job), when the Job's record
//! changes and when the keys arrive. The last two are kept: they catch a story whose word was missed.
//!
//! **The overlay.** A Filed under row opens the feature's page over the tab in a [`FeatureOverlay`], which holds what was
//! opened from inside it on one stack and draws the top: one layer, one Esc back.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{prelude::*, App, Context, Entity, EventEmitter, FocusHandle, Focusable, ListAlignment, ListState, Subscription, Task, WeakEntity, Window};
use ipc::JobStory;

use super::JobView;
use crate::features::overlay::{FeatureOverlay, OverlayEvent, Source as OverlaySource};
use crate::features::page_model::Titles;
use crate::features::story_link;
use crate::fleet::{FleetConnection, StoryMoved};
use crate::keymap::Act;
use crate::settings::wire::{request, Ask, Refused};
use model::{Built, Features};

pub(crate) mod lead;
pub(crate) mod mock;
pub(crate) mod model;
mod view;

#[cfg(test)]
mod tests;

/// What the tab tells whoever holds it.
pub enum StoryEvent {
    /// Esc, with nothing in the tab left to put away: the keys go back to where the tab was opened from.
    Escape,
    /// A Filed under row, pressed. Only a tab inside an overlay says this; the tab in a pane opens its own overlay.
    Feature { slug: String, title: String },
}

enum Reads {
    Live(Entity<FleetConnection>),
    Mock,
}

/// A read that may not have answered yet.
pub(crate) enum Load<T> {
    Reading,
    Read(T),
    Failed(String),
}

/// The parts of the tab the keys are in: the ask heads, and the Filed under rows of the ask that is unfolded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Region {
    Asks,
    Links,
}

/// What of a Job's record puts its story in doubt: a Job that ended, merged or took a turn may have moved an ask.
pub(crate) type Stamp = (String, Option<String>, Option<String>, usize);

pub(crate) fn stamp_of(record: &ipc::SessionRecord) -> Stamp {
    (format!("{:?}", record.state), record.done_since.as_ref().map(|at| at.as_str().to_string()), record.last_turn_at.as_ref().map(|at| at.as_str().to_string()), record.attachments.len())
}

/// `GET /sessions/story?session_id=`. `crates/ipc/operations/get_job_story.toml`.
pub(crate) fn read_story(port: u16, job: &str, cx: &App) -> Task<Result<JobStory, Refused>> {
    request(port, Ask::Get(format!("/sessions/story?session_id={}", crate::link::percent_encoded(job))), "JobStory", cx)
}

pub struct StoryTab {
    job: String,
    source: Reads,
    /// A tab inside an overlay hands a Filed under press to it instead of opening one of its own.
    nested: bool,
    pub(crate) story: Load<JobStory>,
    titles: Titles,
    features: Features,
    pub(crate) built: Built,
    /// The ask whose rows show, by id. One at a time.
    pub(crate) unfolded: Option<String>,
    /// Whether an ask has been unfolded by default or by him, so a read again does not unfold one he folded.
    chosen: bool,
    /// The ask the keys are on, and the Filed under row they are on in the unfolded one.
    pub(crate) cursor: Option<usize>,
    pub(crate) region: Region,
    pub(crate) link_at: usize,
    pub(crate) focus: FocusHandle,
    pub(crate) list: ListState,
    /// The ask to put in front when the story is read: from a Story link, or asked for by whoever opened the tab.
    want: Option<String>,
    overlay: Option<Entity<FeatureOverlay>>,
    overlay_sub: Option<Subscription>,
    stamp: Option<Stamp>,
    read_port: Option<u16>,
    work: Option<Task<()>>,
    /// How many times the story was asked for, for a test of what makes it read again.
    #[cfg(test)]
    pub(crate) reads: usize,
    _subs: Vec<Subscription>,
}

impl EventEmitter<StoryEvent> for StoryTab {}

impl Focusable for StoryTab {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl StoryTab {
    /// On Fleet: the story of Job `job`.
    pub fn live(fleet: &Entity<FleetConnection>, job: String, nested: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Reads::Live(fleet.clone()), job, nested, window, cx);
        let watch = cx.observe(fleet, |this, _, cx| this.moved(cx));
        this._subs.push(watch);
        let told = cx.subscribe(fleet, |this, _, moved: &StoryMoved, cx| {
            if moved.0.iter().any(|one| one.session_id.as_str() == this.job) {
                this.refresh(cx);
            }
        });
        this._subs.push(told);
        this.titles = this.held_titles(cx);
        this.refresh(cx);
        this
    }

    /// On the mock story in `story/mock.rs`.
    pub fn mock(job: String, nested: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Reads::Mock, job.clone(), nested, window, cx);
        this.titles = mock::titles();
        this.features = mock::features();
        this.story = Load::Read(mock::story(&job));
        this.rebuild(cx);
        this
    }

    fn new(source: Reads, job: String, nested: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        // The keys arrive: the story is read again, in case a word that it moved was missed while the tab was away.
        let arrive = cx.on_focus_in(&focus, window, |this, _, cx| this.refresh(cx));
        let mut this = StoryTab {
            job,
            source,
            nested,
            story: Load::Reading,
            titles: Titles::new(),
            features: Features::new(),
            built: Built::default(),
            unfolded: None,
            chosen: false,
            cursor: None,
            region: Region::Asks,
            link_at: 0,
            focus,
            list: ListState::new(0, ListAlignment::Top, gpui::px(200.)),
            want: None,
            overlay: None,
            overlay_sub: None,
            stamp: None,
            read_port: None,
            work: None,
            #[cfg(test)]
            reads: 0,
            _subs: vec![arrive],
        };
        // A Story link left an ask for this Job: it is the one put in front.
        if !nested {
            if let Some(request) = story_link::take(cx) {
                if request.session_id.as_str() == this.job {
                    this.want = Some(request.ask_id);
                } else {
                    story_link::put(request, cx);
                }
            }
        }
        this
    }

    /// Put the ask `ask_id` in front, unfolded and with the keys on it.
    pub fn put_ask(&mut self, ask_id: String, cx: &mut Context<Self>) {
        self.want = Some(ask_id);
        self.settle(cx);
    }

    // ---- reading -----------------------------------------------------------------------------

    fn fleet(&self) -> Option<&Entity<FleetConnection>> {
        match &self.source {
            Reads::Live(fleet) => Some(fleet),
            Reads::Mock => None,
        }
    }

    fn port(&self, cx: &App) -> Option<u16> {
        self.fleet().and_then(|fleet| fleet.read(cx).held().port())
    }

    /// The titles of the Jobs the connection holds, by id.
    fn held_titles(&self, cx: &App) -> Titles {
        self.fleet().map(|fleet| fleet.read(cx).held().jobs.iter().filter_map(|job| job.title.clone().map(|title| (job.id.as_str().to_string(), title))).collect()).unwrap_or_default()
    }

    /// The project the Job was started for, by Fleet's id.
    pub fn project(&self, cx: &App) -> Option<String> {
        match &self.source {
            Reads::Live(fleet) => fleet.read(cx).held().jobs.iter().find(|one| one.id.as_str() == self.job)?.hosted.as_ref()?.project.clone(),
            Reads::Mock => Some(crate::features::mock::PROJECT.to_string()),
        }
    }

    /// Fleet's record of the Job moved, or the connection came back: read the story again where it may have moved.
    fn moved(&mut self, cx: &mut Context<Self>) {
        let titles = self.held_titles(cx);
        if titles != self.titles {
            self.titles = titles;
            self.rebuild(cx);
        }
        let stamp = self.fleet().and_then(|fleet| fleet.read(cx).held().jobs.iter().find(|one| one.id.as_str() == self.job).map(stamp_of));
        if stamp != self.stamp || self.port(cx) != self.read_port {
            self.refresh(cx);
        }
    }

    /// Read the story again, keeping what is drawn until the answer comes.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        #[cfg(test)]
        {
            self.reads += 1;
        }
        let Some(fleet) = self.fleet().cloned() else { return };
        self.stamp = fleet.read(cx).held().jobs.iter().find(|one| one.id.as_str() == self.job).map(stamp_of);
        let port = self.port(cx);
        self.read_port = port;
        let Some(port) = port else {
            if !matches!(self.story, Load::Read(_)) {
                self.story = Load::Failed("Fleet is not connected".into());
                cx.notify();
            }
            return;
        };
        let story = read_story(port, &self.job, cx);
        let library = self.project(cx).map(|project| crate::features::wire::library(port, &project, cx));
        self.work = Some(cx.spawn(async move |this, cx| {
            let story = story.await;
            let titles: Option<Features> = match library {
                Some(read) => read.await.ok().map(|library| library.areas.iter().flat_map(|area| &area.features).map(|row| (row.slug.clone(), row.title.clone())).collect()),
                None => None,
            };
            let _ = this.update(cx, |this, cx| this.took(story, titles, cx));
        }));
    }

    fn took(&mut self, story: Result<JobStory, Refused>, features: Option<Features>, cx: &mut Context<Self>) {
        if let Some(features) = features {
            self.features = features;
        }
        match story {
            Ok(story) => self.story = Load::Read(story),
            // A read that failed keeps what was read before, where there was one.
            Err(why) if !matches!(self.story, Load::Read(_)) => self.story = Load::Failed(why.said),
            Err(_) => {}
        }
        self.rebuild(cx);
    }

    /// The cards from what is held, and the list drawn again.
    pub(crate) fn rebuild(&mut self, cx: &mut Context<Self>) {
        self.built = match &self.story {
            Load::Read(story) => model::build(story, &self.titles, &self.features),
            _ => Built::default(),
        };
        let count = self.places();
        self.list.reset(count);
        self.settle(cx);
    }

    /// Whether the story has been read, as opposed to being read now or not read.
    #[cfg(test)]
    pub(crate) fn story_is_read(&self) -> bool {
        matches!(self.story, Load::Read(_))
    }

    /// The places of the list: each ask, and the group of what was shown before any.
    pub(crate) fn places(&self) -> usize {
        self.built.cards.len() + usize::from(!self.built.before.is_empty())
    }

    /// The ask asked for, or the one that was unfolded, or the first open one, is unfolded with the keys on it.
    fn settle(&mut self, cx: &mut Context<Self>) {
        let wanted = self.want.as_deref().and_then(|id| self.built.find(id));
        let kept = self.unfolded.as_deref().and_then(|id| self.built.find(id));
        // The first ask is unfolded once, when there are asks; an ask he folded stays folded as the story is read again.
        let first = if self.chosen { None } else { self.built.first_open() };
        let at = wanted.or(kept).or(first);
        self.chosen |= !self.built.cards.is_empty();
        if wanted.is_some() {
            self.want = None;
        }
        // An ask not found yet stays wanted until the story that holds it is read.
        self.unfolded = at.map(|at| self.built.cards[at].ask_id.clone());
        self.cursor = self.cursor.filter(|at| *at < self.built.cards.len()).or(at);
        if wanted.is_some() {
            self.cursor = at;
            self.region = Region::Asks;
        }
        self.link_at = self.link_at.min(self.feature_rows().len().saturating_sub(1));
        if let Some(at) = wanted {
            self.list.scroll_to_reveal_item(at);
        }
        cx.notify();
    }

    /// The places of the Filed under rows of the unfolded ask, in its rows.
    pub(crate) fn feature_rows(&self) -> Vec<String> {
        self.unfolded.as_deref().and_then(|id| self.built.find(id)).map(|at| self.built.cards[at].rows.iter().filter_map(|row| row.feature.clone()).collect()).unwrap_or_default()
    }

    // ---- what the keys are on ----------------------------------------------------------------

    /// Unfold the ask at `at`, which folds the one that was open; or fold it where it is the open one.
    pub(crate) fn toggle(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(card) = self.built.cards.get(at) else { return };
        let before = self.unfolded.as_deref().and_then(|id| self.built.find(id));
        self.unfolded = if self.unfolded.as_deref() == Some(card.ask_id.as_str()) { None } else { Some(card.ask_id.clone()) };
        self.cursor = Some(at);
        self.region = Region::Asks;
        self.link_at = 0;
        if let Some(before) = before {
            self.list.remeasure_items(before..before + 1);
        }
        self.list.remeasure_items(at..at + 1);
        self.list.scroll_to_reveal_item(at);
        cx.notify();
    }

    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        match self.region {
            Region::Asks => {
                let count = self.built.cards.len() as isize;
                if count == 0 {
                    return;
                }
                let at = self.cursor.map_or(if by > 0 { -1 } else { count }, |at| at as isize);
                let next = (at + by).clamp(0, count - 1) as usize;
                self.cursor = Some(next);
                self.list.scroll_to_reveal_item(next);
            }
            Region::Links => {
                let count = self.feature_rows().len() as isize;
                self.link_at = (self.link_at as isize + by).clamp(0, (count - 1).max(0)) as usize;
            }
        }
        cx.notify();
    }

    /// Tab: the keys go between the ask heads and the Filed under rows of the unfolded ask. A part with nothing in it is passed over.
    fn switch(&mut self, cx: &mut Context<Self>) {
        self.region = match self.region {
            Region::Asks if !self.feature_rows().is_empty() => Region::Links,
            _ => Region::Asks,
        };
        self.link_at = 0;
        cx.notify();
    }

    /// Enter: a head folds or unfolds, a Filed under row opens its feature.
    fn press(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.region {
            Region::Asks => {
                if let Some(at) = self.cursor {
                    self.toggle(at, cx);
                }
            }
            Region::Links => {
                if let Some(slug) = self.feature_rows().get(self.link_at).cloned() {
                    self.open_feature(slug, window, cx);
                }
            }
        }
    }

    /// The feature's page opens over the tab, in the overlay, or in the one this tab is inside.
    pub(crate) fn open_feature(&mut self, slug: String, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.features.get(&slug).cloned().unwrap_or_else(|| slug.clone());
        if self.nested {
            cx.emit(StoryEvent::Feature { slug, title });
            return;
        }
        let from = format!("Story of {}", self.name());
        let Some(project) = self.project(cx) else { return };
        match &self.overlay {
            Some(overlay) => overlay.update(cx, |overlay, cx| overlay.push_page(slug, title, window, cx)),
            None => {
                let source = match &self.source {
                    Reads::Live(fleet) => OverlaySource::Live(fleet.clone()),
                    Reads::Mock => OverlaySource::Mock,
                };
                let overlay = cx.new(|cx| FeatureOverlay::on_page(source, project, from, slug, title, window, cx));
                self.overlay_sub = Some(cx.subscribe_in(&overlay, window, |this, _, event: &OverlayEvent, window, cx| match event {
                    OverlayEvent::Closed => {
                        this.overlay = None;
                        this.overlay_sub = None;
                        window.focus(&this.focus, cx);
                        cx.notify();
                    }
                }));
                self.overlay = Some(overlay);
            }
        }
        cx.notify();
    }

    /// What the Job is called, or what stands in for its name.
    pub(crate) fn name(&self) -> String {
        self.titles.get(&self.job).cloned().unwrap_or_else(|| "this Job".into())
    }

    /// The overlay over the tab, where one is open.
    pub fn overlay(&self) -> Option<&Entity<FeatureOverlay>> {
        self.overlay.as_ref()
    }

    // ---- keys --------------------------------------------------------------------------------

    pub(crate) fn on_key(&mut self, event: &gpui::KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key != "escape" {
            return;
        }
        if self.region == Region::Links {
            self.region = Region::Asks;
            cx.notify();
            cx.stop_propagation();
        } else if self.nested {
            cx.emit(StoryEvent::Escape);
            cx.stop_propagation();
        }
    }

    /// The registry's `features` acts, pressed while the keys are on the tab: the same keys as the Features tab, which move,
    /// open and go between the parts.
    pub(crate) fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        match act.id {
            "features_move" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "features_open" => self.press(window, cx),
            "features_pane" => self.switch(cx),
            _ => cx.propagate(),
        }
    }
}

// ---- the Job's + and the tab in a pane ---------------------------------------------------------

/// The tab of each Job that has one, so a Story link puts its ask in front of the tab that is open.
#[derive(Default)]
struct Tabs(HashMap<String, WeakEntity<StoryTab>>);

impl gpui::Global for Tabs {}

fn tab_id(job: &str) -> String {
    format!("story:{job}")
}

/// Put Story in every Job's +, and hear which Job comes on screen: a Story link left an ask for it, and its tab opens at it.
pub fn init(cx: &mut App) {
    super::kind_chip::init(cx);
    super::thread_rail::init(cx);
    super::offer_tab(super::TabOffer { id: "story", label: "Story".into(), glyph: GLYPH, open: Rc::new(|job, pane, window, cx| open(&job, Some(pane), window, cx)) }, cx);
    super::on_job_shown(
        |job, window, cx| {
            let Some(job) = job else { return };
            lead::shown(&job, cx);
            if story_link::peek(cx).is_some_and(|request| request.session_id.as_str() == job) {
                open(&job, None, window, cx);
            }
        },
        cx,
    );
}

const GLYPH: &str = "book-open";

fn open(job: &str, pane: Option<super::ViewPane>, window: &mut Window, cx: &mut App) {
    let Some(view) = JobView::live(cx) else { return };
    let job = job.to_string();
    view.update(cx, |view, cx| view.open_story_tab(&job, pane, window, cx));
}

impl JobView {
    /// Story in front on Job `job`, in the pane asked for or the top one. A tab already open comes forward and takes the ask a
    /// Story link left.
    pub(super) fn open_story_tab(&mut self, job: &str, pane: Option<super::ViewPane>, window: &mut Window, cx: &mut Context<Self>) {
        let id = tab_id(job);
        let held = cx.default_global::<Tabs>().0.get(job).and_then(WeakEntity::upgrade);
        if let Some(tab) = held {
            if self.front_view_tab(job, &id, cx) {
                if let Some(request) = story_link::peek(cx).filter(|request| request.session_id.as_str() == job) {
                    story_link::take(cx);
                    tab.update(cx, |tab, cx| tab.put_ask(request.ask_id, cx));
                }
                let focus = tab.read(cx).focus.clone();
                window.focus(&focus, cx);
                return;
            }
        }
        let fleet = self.fleet.clone();
        let named = job.to_string();
        let tab = cx.new(|cx| match &fleet {
            Some(fleet) => StoryTab::live(fleet, named.clone(), false, window, cx),
            None => StoryTab::mock(named.clone(), false, window, cx),
        });
        cx.default_global::<Tabs>().0.insert(job.to_string(), tab.downgrade());
        let focus = tab.read(cx).focus.clone();
        let tab = super::ViewTab { id, title: "Story".into(), glyph: GLYPH, view: tab.into(), menu: Vec::new(), closing: None, closed: None, renamed: None };
        self.open_view_tab_in(job, pane.unwrap_or(super::ViewPane::Top), tab, true, cx);
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
    }
}
