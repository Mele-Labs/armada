//! The Story tab of a Job: its asks as cards grouped under the feature they touch, open ones first in a group, each a head
//! line of his title and what came of it that unfolds to how it started, what he
//! was shown and decided, what was built and what was handed on, and under which feature it is filed.
//!
//! What is read is Fleet's own: `get_job_story`, off the main thread, drawn as bones until it answers. `story/model.rs`
//! works out every line. The tab is a fixed kind of a Job's panes (`Zone::Story`), drawn from `place_draw.rs` by [`JobView::pane_story`].
//!
//! **A story is read again when Fleet says it moved** (`session.story`, as `StoryMoved`, for this Job), when the Job's record
//! changes and when the keys arrive. The last two are kept: they catch a story whose word was missed.
//!
//! **The library's names land after the story**: the titles of the features its asks are filed under and the order of the
//! areas come from `list_features`, read once for every tab of the project (`story/library.rs`), so a row reads by its slug
//! until they land.
//!
//! **The features the Job is on** head the tab, above the asks: Up from the first ask reaches them, and a row opens its
//! feature as a Filed under row does.
//!
//! **The overlay.** A Filed under row opens the feature's page over the tab in a [`FeatureOverlay`], which holds what was
//! opened from inside it on one stack and draws the top: one layer, one Esc back.

use std::collections::HashMap;

use gpui::{div, prelude::*, AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, ListAlignment, ListState, SharedString, Subscription, Task, Window};
use ipc::JobStory;

use super::JobView;
use crate::features::overlay::{FeatureOverlay, OverlayEvent, Source as OverlaySource};
use crate::features::page_model::Titles;
use crate::features::story_link;
use crate::fleet::{FleetConnection, LibrarianStepsMoved, StoryMoved};
use crate::keymap::Act;
use crate::settings::wire::{request, Ask, Refused};
use model::{Built, Features};

pub(crate) mod lead;
pub(crate) mod library;
pub(crate) mod mock;
pub(crate) mod model;
#[allow(dead_code)]
pub(crate) mod shots;
mod view;

#[cfg(test)]
mod tests;

/// What the tab tells whoever holds it.
pub enum StoryEvent {
    /// Esc, with nothing in the tab left to put away: the keys go back to where the tab was opened from.
    Escape,
    /// A feature opened from a row of the tab. A tab inside an overlay says this, and so does one a Job view holds in its panes, which opens the page as a tab.
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

/// The parts of the tab the keys are in: the features the Job is on, the ask heads, and the Filed under rows of the ask
/// that is unfolded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Region {
    Features,
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
    /// The slugs of the library's areas in its order, which give a feature's hue as the Features list draws it.
    pub(crate) areas: Vec<String>,
    pub(crate) built: Built,
    /// The read of the library's names the tab holds, as [`library::names`] counts them.
    names_at: u64,
    /// The ask whose rows show, by id. One at a time.
    pub(crate) unfolded: Option<String>,
    /// The ask the keys are on, and the Filed under row they are on in the unfolded one.
    pub(crate) cursor: Option<usize>,
    pub(crate) region: Region,
    pub(crate) link_at: usize,
    /// The row of the features the Job is on that the keys are on.
    pub(crate) feature_at: usize,
    pub(crate) focus: FocusHandle,
    pub(crate) list: ListState,
    /// The ask to put in front when the story is read: from a Story link, or asked for by whoever opened the tab.
    want: Option<String>,
    overlay: Option<Entity<FeatureOverlay>>,
    /// Set by the Job view that holds the tab in its panes: a feature opens as a tab beside the thread, so the tab says so up
    /// instead of drawing an overlay over itself.
    routed: bool,
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
        // A step of the project's librarian did its work: its names may have moved, and the tabs of the project take them when they land.
        let librarian = cx.subscribe(fleet, |this, _, told: &LibrarianStepsMoved, cx| {
            let (Some(project), Some(port)) = (this.project(cx), this.port(cx)) else { return };
            if told.0.iter().any(|moved| moved.project == project && moved.state == ipc::LibrarianStepState::Done) {
                library::librarian_moved(&project, port, cx);
            }
        });
        this._subs.push(librarian);
        let names = cx.observe_global::<library::Libraries>(|this, cx| this.names_landed(cx));
        this._subs.push(names);
        this.titles = this.held_titles(cx);
        this.refresh(cx);
        this
    }

    /// On the mock story in `story/mock.rs`.
    pub fn mock(job: String, nested: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Reads::Mock, job.clone(), nested, window, cx);
        this.titles = mock::titles();
        this.features = mock::features();
        this.areas = mock::areas();
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
            areas: Vec::new(),
            built: Built::default(),
            names_at: 0,
            unfolded: None,
            cursor: None,
            region: Region::Asks,
            link_at: 0,
            feature_at: 0,
            focus,
            list: ListState::new(0, ListAlignment::Top, gpui::px(200.)),
            want: None,
            overlay: None,
            routed: false,
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
        // The story draws as soon as it is read; the library's names land on it after, read once for every tab of the project.
        self.want_names(port, cx);
        let story = read_story(port, &self.job, cx);
        self.work = Some(cx.spawn(async move |this, cx| {
            let story = story.await;
            let _ = this.update(cx, |this, cx| this.took(story, cx));
        }));
    }

    fn took(&mut self, story: Result<JobStory, Refused>, cx: &mut Context<Self>) {
        match story {
            Ok(story) => self.story = Load::Read(story),
            // A read that failed keeps what was read before, where there was one.
            Err(why) if !matches!(self.story, Load::Read(_)) => self.story = Load::Failed(why.said),
            Err(_) => {}
        }
        if let Some(port) = self.read_port {
            self.want_names(port, cx);
        }
        self.take_names(cx);
        self.rebuild(cx);
    }

    /// The library's names are asked for where they are due, for the features the story read names.
    fn want_names(&self, port: u16, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx) else { return };
        let named: Vec<String> = match &self.story {
            Load::Read(story) => story.asks.iter().flat_map(|one| one.ask.features.iter().cloned()).collect(),
            _ => Vec::new(),
        };
        library::want(&project, port, named.iter().map(String::as_str), cx);
    }

    /// The library's names, where a read of them landed that the tab does not hold yet. Whether they were taken.
    fn take_names(&mut self, cx: &App) -> bool {
        let Some(project) = self.project(cx) else { return false };
        let Some((landed, names)) = library::names(&project, cx).filter(|(landed, _)| *landed != self.names_at) else { return false };
        self.names_at = landed;
        self.features = names.features.clone();
        self.areas = names.areas.clone();
        true
    }

    /// A read of the library's names landed: the tab draws them where they are new to it.
    fn names_landed(&mut self, cx: &mut Context<Self>) {
        if self.take_names(cx) {
            self.rebuild(cx);
        }
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

    /// The places of the list: the features the Job is on, each ask, and the group of what was shown before any.
    pub(crate) fn places(&self) -> usize {
        self.head() + self.built.cards.len() + usize::from(!self.built.before.is_empty())
    }

    /// The places before the first ask: one for the features the Job is on, where it is on any.
    pub(crate) fn head(&self) -> usize {
        usize::from(!self.built.features.is_empty())
    }

    /// The ask asked for, or the one that was unfolded, or the first open one, is unfolded with the keys on it.
    fn settle(&mut self, cx: &mut Context<Self>) {
        let wanted = self.want.as_deref().and_then(|id| self.built.find(id));
        let kept = self.unfolded.as_deref().and_then(|id| self.built.find(id));
        // Every ask is one outcome line until he presses it; the keys start on the first open one.
        let at = wanted.or(kept);
        if wanted.is_some() {
            self.want = None;
        }
        // An ask not found yet stays wanted until the story that holds it is read.
        self.unfolded = at.map(|at| self.built.cards[at].ask_id.clone());
        self.cursor = self.cursor.filter(|at| *at < self.built.cards.len()).or(at).or(self.built.first_open());
        if wanted.is_some() {
            self.cursor = at;
            self.region = Region::Asks;
        }
        self.link_at = self.link_at.min(self.feature_rows().len().saturating_sub(1));
        self.feature_at = self.feature_at.min(self.built.features.len().saturating_sub(1));
        if self.region == Region::Features && self.built.features.is_empty() {
            self.region = Region::Asks;
        }
        if let Some(at) = wanted {
            self.list.scroll_to_reveal_item(self.head() + at);
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
        let head = self.head();
        if let Some(before) = before {
            self.list.remeasure_items(head + before..head + before + 1);
        }
        self.list.remeasure_items(head + at..head + at + 1);
        self.list.scroll_to_reveal_item(head + at);
        cx.notify();
    }

    /// Up and Down: the features the Job is on come before the first ask, so the keys go from one part to the other.
    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let was = (self.region, self.feature_at, self.cursor, self.link_at);
        match self.region {
            Region::Features => {
                let next = self.feature_at as isize + by;
                if next >= self.built.features.len() as isize && !self.built.cards.is_empty() {
                    self.region = Region::Asks;
                    self.cursor = Some(0);
                    self.list.scroll_to_reveal_item(self.head());
                } else {
                    self.feature_at = next.clamp(0, self.built.features.len() as isize - 1) as usize;
                    self.list.scroll_to_reveal_item(0);
                }
            }
            Region::Asks => {
                let count = self.built.cards.len() as isize;
                let at = self.cursor.map_or(if by > 0 { -1 } else { count }, |at| at as isize);
                if at + by < 0 && !self.built.features.is_empty() {
                    self.region = Region::Features;
                    self.feature_at = self.built.features.len() - 1;
                    self.list.scroll_to_reveal_item(0);
                } else if count > 0 {
                    let next = (at + by).clamp(0, count - 1) as usize;
                    self.cursor = Some(next);
                    self.list.scroll_to_reveal_item(self.head() + next);
                }
            }
            Region::Links => {
                let count = self.feature_rows().len() as isize;
                self.link_at = (self.link_at as isize + by).clamp(0, (count - 1).max(0)) as usize;
            }
        }
        if was != (self.region, self.feature_at, self.cursor, self.link_at) {
            crate::ui_sounds::moved();
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

    /// Enter: a feature the Job is on opens, a head folds or unfolds, a Filed under row opens its feature.
    fn press(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.region {
            Region::Features => {
                if let Some(line) = self.built.features.get(self.feature_at) {
                    self.open_feature(line.slug.clone(), window, cx);
                }
            }
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
        let title = self.feature_title(&slug);
        if self.nested || self.routed {
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

    /// What feature `slug` is called here, or its slug where nothing names it.
    pub(crate) fn feature_title(&self, slug: &str) -> String {
        let listed = self.built.features.iter().find(|one| one.slug == slug).map(|one| one.title.clone());
        self.features.get(slug).cloned().or(listed).unwrap_or_else(|| slug.to_string())
    }

    /// Features opened from this tab go up as [`StoryEvent::Feature`] to whoever holds it in a pane, not into an overlay.
    pub(crate) fn route_features(&mut self) {
        self.routed = true;
    }

    /// Where a feature's page reads from, the project it is in, and the place it was opened from.
    pub(crate) fn feature_source(&self, cx: &App) -> Option<(OverlaySource, String, String)> {
        let source = match &self.source {
            Reads::Live(fleet) => OverlaySource::Live(fleet.clone()),
            Reads::Mock => OverlaySource::Mock,
        };
        Some((source, self.project(cx)?, format!("Story of {}", self.name())))
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

// ---- the tab in a pane ------------------------------------------------------------------------

/// The shown Job's tab, so a Story link puts its ask in front of the tab that is open. Only that Job's is held: the tab
/// reads again on every word Fleet sends, which no Job off screen needs.
#[derive(Default)]
struct Stories(HashMap<String, Entity<StoryTab>>);

impl gpui::Global for Stories {}

/// Hear which Job comes on screen: the tabs of the others go, and a Story link left an ask for it, whose tab comes forward.
pub fn init(cx: &mut App) {
    super::kind_chip::init(cx);
    super::thread_rail::init(cx);
    super::on_job_shown(
        |job, window, cx| {
            cx.default_global::<Stories>().0.retain(|held, _| job.as_deref() == Some(held.as_str()));
            let Some(job) = job else { return };
            lead::shown(&job, cx);
            if let Some(view) = JobView::live(cx) {
                // The strip over the column names the Job's features, so the story is read as the Job comes on screen.
                view.update(cx, |view, cx| {
                    view.story_of(&job, window, cx);
                    view.open_for_link(&job, cx);
                });
            }
        },
        cx,
    );
}

impl JobView {
    /// A Story link left an ask for Job `job`, which is on screen with its panes read: its Story tab comes to the front,
    /// where it stands or where a new one opens. The ask is put in front when the tab draws.
    pub(super) fn open_for_link(&mut self, job: &str, cx: &mut Context<Self>) {
        let asked = story_link::peek(cx).is_some_and(|request| request.session_id.as_str() == job);
        if asked && self.shown.as_ref().is_some_and(|shown| shown.as_ref() == job) && self.panes_answered(job) {
            self.open_kind("story", cx);
        }
    }

    /// The body of a Story tab: the Job's tab, made the first time it draws (after this draw, so none is made while drawing).
    /// A Story link for the Job puts its ask in front of a tab that stands, and the keys go to the tab.
    pub(super) fn pane_story(&mut self, job: &SharedString, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let Some(tab) = cx.default_global::<Stories>().0.get(job.as_ref()).cloned() else {
            let job = job.to_string();
            window.defer(cx, move |window, cx| {
                if let Some(view) = JobView::live(cx) {
                    view.update(cx, |view, cx| {
                        // A new tab takes the Story link's request as it is made, so the keys go to it too.
                        let asked = story_link::peek(cx).is_some_and(|request| request.session_id.as_str() == job);
                        view.story_of(&job, window, cx);
                        if asked {
                            view.focus_story(window, cx);
                        }
                        cx.notify();
                    });
                }
            });
            return None;
        };
        if let Some(request) = story_link::peek(cx).filter(|request| request.session_id.as_str() == job.as_ref()) {
            story_link::take(cx);
            tab.update(cx, |tab, cx| tab.put_ask(request.ask_id, cx));
            self.focus_story(window, cx);
        }
        Some(div().size_full().child(tab).into_any_element())
    }

    /// The keys go to the shown Job's Story tab, which is made if it is not yet.
    pub(super) fn focus_story(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let focus = self.story_of(&job, window, cx).read(cx).focus.clone();
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
    }

    /// Job `job`'s Story tab, made where there is none.
    pub(super) fn story_of(&mut self, job: &str, window: &mut Window, cx: &mut Context<Self>) -> Entity<StoryTab> {
        if let Some(tab) = cx.default_global::<Stories>().0.get(job) {
            return tab.clone();
        }
        let (fleet, named) = (self.fleet.clone(), job.to_string());
        let tab = cx.new(|cx| match &fleet {
            Some(fleet) => StoryTab::live(fleet, named.clone(), false, window, cx),
            None => StoryTab::mock(named.clone(), false, window, cx),
        });
        cx.default_global::<Stories>().0.insert(job.to_string(), tab.clone());
        tab.update(cx, |tab, _| tab.route_features());
        let shown = job.to_string();
        cx.subscribe_in(&tab, window, move |this, _, event: &StoryEvent, window, cx| {
            if let StoryEvent::Feature { slug, title } = event {
                this.open_feature_tab(&shown, slug.clone(), title.clone(), window, cx);
            }
        })
        .detach();
        // The strip over the column draws the tab's features, so it draws again when the tab does.
        cx.observe(&tab, |_, _, cx| cx.notify()).detach();
        tab
    }

    /// The features Job `job` is on as its Story tab holds them, most touched first, each with the hue the Features list gives
    /// its area. None are held until the tab has read the story.
    pub(super) fn story_features(&self, job: &str, cx: &App) -> Vec<super::column_strip::StripFeature> {
        let Some(tab) = cx.try_global::<Stories>().and_then(|held| held.0.get(job)) else { return Vec::new() };
        let tab = tab.read(cx);
        tab.built.features.iter().map(|line| super::column_strip::StripFeature { slug: line.slug.clone(), title: line.title.clone(), hue: crate::features::hue_among(tab.areas.iter().map(String::as_str), &line.area) }).collect()
    }

    /// A feature chip of the strip pressed: the feature's page opens as a tab beside the thread, as a row of the Story tab's does.
    pub(super) fn open_story_feature(&mut self, slug: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let title = self.story_of(&job, window, cx).read(cx).feature_title(&slug);
        self.open_feature_tab(&job, slug, title, window, cx);
    }
}
