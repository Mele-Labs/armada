//! The Story tab of a Job: what happened in it over time, oldest first, one row each for what he asked, what he was shown,
//! what was handed to another Job and what was merged. A row says when it was and how long its portion took, and opens into
//! the row itself as a card: the chat of it, the features an ask is filed under, a pull request's commits. A strip of
//! filters over the rows picks one kind; opening a row leaves it as it is.
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
//! **A pull request's commits land after the row opens**: the tab asks the Job view for them (`StoryEvent::ReadPull`), and a
//! press on a commit opens the Pull Requests tab at that commit's files (`StoryEvent::OpenPull`).
//!
//! **The overlay.** A Filed under row opens the feature's page over the tab in a [`FeatureOverlay`], which holds what was
//! opened from inside it on one stack and draws the top: one layer, one Esc back.

use std::collections::HashMap;

use gpui::{div, prelude::*, AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, ListAlignment, ListState, SharedString, Subscription, Task, Window};
use ipc::{JobStory, PullRequestCommit};

use super::JobView;
use crate::features::overlay::{FeatureOverlay, OverlayEvent, Source as OverlaySource};
use crate::features::page_model::Titles;
use crate::features::story_link;
use crate::fleet::{FleetConnection, LibrarianStepsMoved, StoryMoved};
use crate::keymap::Act;
use crate::settings::wire::{request, Ask, Refused};
use model::{Built, Entry, Features, Filter, Link};

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
    /// A pull request row opened: its commits are wanted.
    ReadPull { number: u64 },
    /// A press on a pull request, or on one of its commits: the Pull Requests tab opens on it.
    OpenPull { number: u64, commit: Option<(String, String)> },
    /// A press on the Job an ask was handed to.
    OpenJob(String),
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

/// The parts of the tab the keys are in: the filter tabs, the rows, and the presses inside the opened row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Region {
    Filters,
    Rows,
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
    /// The row that is opened, by key. One at a time.
    pub(crate) unfolded: Option<String>,
    /// Which rows show.
    pub(crate) filter: Filter,
    /// The row the keys are on, in the rows as drawn (`rows`), and the press they are on in the opened row.
    pub(crate) cursor: Option<usize>,
    pub(crate) region: Region,
    pub(crate) link_at: usize,
    /// The commits of the pull requests whose rows were opened, by number, as they were read.
    pub(crate) commits: HashMap<u64, Vec<PullRequestCommit>>,
    /// The tab has put the newest row in front once.
    placed: bool,
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
            filter: Filter::All,
            cursor: None,
            region: Region::Rows,
            link_at: 0,
            commits: HashMap::new(),
            placed: false,
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

    /// The rows from what is held, and the list drawn again.
    pub(crate) fn rebuild(&mut self, cx: &mut Context<Self>) {
        self.built = match &self.story {
            Load::Read(story) => model::build(story, &self.titles, &self.features, chrono::Utc::now()),
            _ => Built::default(),
        };
        if !self.built.filters().contains(&self.filter) {
            self.filter = Filter::All;
        }
        let count = self.places();
        self.list.reset(count);
        self.settle(cx);
    }

    /// Whether the story has been read, as opposed to being read now or not read.
    #[cfg(test)]
    pub(crate) fn story_is_read(&self) -> bool {
        matches!(self.story, Load::Read(_))
    }

    /// The rows as drawn under the filter: the timeline's, then what he was shown before any ask.
    pub(crate) fn rows(&self) -> Vec<&Entry> {
        let shown = self.built.visible(self.filter).into_iter().map(|at| &self.built.entries[at]);
        let before = if self.built.before_shown(self.filter) { self.built.before.iter().collect::<Vec<_>>() } else { Vec::new() };
        shown.chain(before).collect()
    }

    /// How many of `rows` are the timeline's, the rest standing under the heading of what he was shown before any ask.
    pub(crate) fn timeline_len(&self) -> usize {
        self.built.visible(self.filter).len()
    }

    /// The places of the list: each row, and the heading of the group of what he was shown before any ask.
    pub(crate) fn places(&self) -> usize {
        self.timeline_len() + if self.built.before_shown(self.filter) { 1 + self.built.before.len() } else { 0 }
    }

    /// The place in the list of row `at` of `rows`.
    pub(crate) fn place_of(&self, at: usize) -> usize {
        if at < self.timeline_len() { at } else { at + 1 }
    }

    /// The row asked for, or the one that was opened, is opened with the keys on it; the keys start on the newest row.
    fn settle(&mut self, cx: &mut Context<Self>) {
        if let Some(want) = self.want.as_deref().and_then(|id| self.built.find(id)).map(|at| self.built.entries[at].key.clone()) {
            // The row asked for is shown whatever the filter was.
            if !self.rows().iter().any(|entry| entry.key == want) {
                self.filter = Filter::All;
                let count = self.places();
                self.list.reset(count);
            }
            self.want = None;
            self.unfolded = Some(want.clone());
            self.cursor = self.rows().iter().position(|entry| entry.key == want);
            self.region = Region::Rows;
            self.link_at = 0;
            if let Some(at) = self.cursor {
                self.list.scroll_to_reveal_item(self.place_of(at));
            }
        }
        let held = self.unfolded.clone().filter(|key| self.rows().iter().any(|entry| &entry.key == key));
        self.unfolded = held;
        let count = self.rows().len();
        self.cursor = self.cursor.filter(|at| *at < count);
        if self.cursor.is_none() {
            let visible = self.built.visible(self.filter);
            self.cursor = self.built.newest(self.filter).and_then(|at| visible.iter().position(|one| *one == at)).or(if count == 0 { None } else { Some(count - 1) });
        }
        // The Job's present is at the foot: the list starts there, once.
        if !self.placed && count > 0 {
            self.placed = true;
            if let Some(at) = self.cursor {
                self.list.scroll_to_reveal_item(self.place_of(at));
            }
        }
        self.link_at = self.link_at.min(self.links().len().saturating_sub(1));
        if self.region == Region::Filters && self.built.filters().len() < 2 {
            self.region = Region::Rows;
        }
        cx.notify();
    }

    /// The presses of the opened row, in the order they are drawn.
    pub(crate) fn links(&self) -> Vec<Link> {
        let Some(key) = self.unfolded.as_deref() else { return Vec::new() };
        let Some(entry) = self.rows().into_iter().find(|entry| entry.key == key) else { return Vec::new() };
        let none = Vec::new();
        entry.links_with(entry.card.merge.as_ref().and_then(|merge| self.commits.get(&merge.number)).unwrap_or(&none))
    }

    /// The commits of pull request `number` were read.
    pub fn set_commits(&mut self, number: u64, commits: Vec<PullRequestCommit>, cx: &mut Context<Self>) {
        self.commits.insert(number, commits);
        let count = self.places();
        self.list.reset(count);
        cx.notify();
    }

    // ---- what the keys are on ----------------------------------------------------------------

    /// Put the filter on `filter`: the rows it lets through are drawn, and the opened row stays opened where it is still there.
    pub(crate) fn filter_by(&mut self, filter: Filter, cx: &mut Context<Self>) {
        if self.filter == filter {
            return;
        }
        self.filter = filter;
        self.cursor = None;
        let count = self.places();
        self.list.reset(count);
        self.settle(cx);
    }

    /// Open the row at `at` of `rows`, which closes the one that was open; or close it where it is the open one.
    pub(crate) fn toggle(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(key) = self.rows().get(at).map(|entry| entry.key.clone()) else { return };
        let before = self.unfolded.as_deref().and_then(|open| self.rows().iter().position(|entry| entry.key == open));
        self.unfolded = if self.unfolded.as_deref() == Some(key.as_str()) { None } else { Some(key) };
        self.cursor = Some(at);
        self.region = Region::Rows;
        self.link_at = 0;
        if let Some(before) = before {
            let place = self.place_of(before);
            self.list.remeasure_items(place..place + 1);
        }
        let place = self.place_of(at);
        self.list.remeasure_items(place..place + 1);
        self.list.scroll_to_reveal_item(place);
        let wanted = self.rows().get(at).filter(|_| self.unfolded.is_some()).and_then(|entry| entry.card.merge.as_ref().map(|merge| merge.number));
        if let Some(number) = wanted.filter(|number| !self.commits.contains_key(number)) {
            cx.emit(StoryEvent::ReadPull { number });
        }
        cx.notify();
    }

    /// Up and Down: along the rows, or along the presses of the opened row. Left and Right: along the filters.
    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let was = (self.cursor, self.link_at);
        match self.region {
            Region::Filters => {}
            Region::Rows => {
                let count = self.rows().len() as isize;
                if count > 0 {
                    let at = self.cursor.map_or(if by > 0 { -1 } else { count }, |at| at as isize);
                    let next = (at + by).clamp(0, count - 1) as usize;
                    self.cursor = Some(next);
                    self.list.scroll_to_reveal_item(self.place_of(next));
                }
            }
            Region::Links => {
                let count = self.links().len() as isize;
                self.link_at = (self.link_at as isize + by).clamp(0, (count - 1).max(0)) as usize;
            }
        }
        if was != (self.cursor, self.link_at) {
            crate::ui_sounds::moved();
        }
        cx.notify();
    }

    /// Left and Right on the filters: the next tab is the filter.
    fn step_filter(&mut self, by: isize, cx: &mut Context<Self>) {
        let tabs = self.built.filters();
        let at = tabs.iter().position(|one| *one == self.filter).unwrap_or(0) as isize;
        let next = (at + by).clamp(0, tabs.len() as isize - 1) as usize;
        if let Some(filter) = tabs.get(next).copied() {
            self.region = Region::Filters;
            self.filter_by(filter, cx);
            crate::ui_sounds::moved();
        }
    }

    /// Tab: the keys go from the filters to the rows to the presses of the opened row. A part with nothing in it is passed over.
    fn switch(&mut self, back: bool, cx: &mut Context<Self>) {
        let mut parts = Vec::new();
        if self.built.filters().len() > 1 {
            parts.push(Region::Filters);
        }
        parts.push(Region::Rows);
        if !self.links().is_empty() {
            parts.push(Region::Links);
        }
        let at = parts.iter().position(|one| *one == self.region).unwrap_or(0) as isize;
        self.region = parts[(at + if back { -1 } else { 1 }).rem_euclid(parts.len() as isize) as usize];
        self.link_at = 0;
        cx.notify();
    }

    /// Enter: a row opens or closes, a press in the opened row goes where it says.
    fn press(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.region {
            Region::Filters => {}
            Region::Rows => {
                if let Some(at) = self.cursor {
                    self.toggle(at, cx);
                }
            }
            Region::Links => {
                if let Some(link) = self.links().get(self.link_at).cloned() {
                    self.go(link, window, cx);
                }
            }
        }
    }

    /// A press in an opened row.
    pub(crate) fn go(&mut self, link: Link, window: &mut Window, cx: &mut Context<Self>) {
        match link {
            Link::Feature(slug) => self.open_feature(slug, window, cx),
            Link::Commit { number, sha, headline } => cx.emit(StoryEvent::OpenPull { number, commit: Some((sha, headline)) }),
            Link::Pull(number) => cx.emit(StoryEvent::OpenPull { number, commit: None }),
            Link::Job(id) => cx.emit(StoryEvent::OpenJob(id)),
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
        if self.region != Region::Rows {
            self.region = Region::Rows;
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
            "features_chip" => self.step_filter(if act.slot == 0 { -1 } else { 1 }, cx),
            "features_open" => self.press(window, cx),
            "features_pane" => self.switch(act.slot % 2 == 1, cx),
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
            match event {
                StoryEvent::Feature { slug, title } => this.open_feature_tab(&shown, slug.clone(), title.clone(), window, cx),
                StoryEvent::ReadPull { number } => this.read_story_pull(&shown, *number, window, cx),
                StoryEvent::OpenPull { number, commit } => this.open_story_pull(&shown, *number, commit.clone(), window, cx),
                StoryEvent::OpenJob(id) => this.open_job(id.clone(), cx),
                StoryEvent::Escape => {}
            }
        })
        .detach();
        // The strip over the column draws the tab's features, so it draws again when the tab does.
        cx.observe(&tab, |_, _, cx| cx.notify()).detach();
        tab
    }

    /// The commits of pull request `number`, which a Job's Story row opened: read from Fleet, and handed to the tab.
    fn read_story_pull(&mut self, job: &str, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(manifest) = self.pulls_of(job, cx).into_iter().find(|one| one.number == number).and_then(|one| one.manifest) else { return };
        let Some(tab) = cx.default_global::<Stories>().0.get(job).cloned() else { return };
        if self.fleet.is_none() {
            if let Some(page) = crate::mock::pull_page(&format!("{manifest}#{number}")) {
                tab.update(cx, |tab, cx| tab.set_commits(number, page.commits, cx));
            }
            return;
        }
        self.get(
            format!("/pull_request_details/{}/{number}", super::wire::encoded(&manifest)),
            window,
            cx,
            Box::new(move |_, answer, _, cx| {
                if let Ok(page) = answer.and_then(|bytes| ipc::decode::<ipc::PullRequestPage>("PullRequestPage", &bytes).map_err(|why| why.to_string())) {
                    tab.update(cx, |tab, cx| tab.set_commits(number, page.commits, cx));
                }
            }),
        );
    }

    /// A press on pull request `number` of a Job's Story, or on one of its commits: the pull request opens over the Job,
    /// at that commit's files where one was pressed, with Back to the Story over them.
    fn open_story_pull(&mut self, job: &str, number: u64, commit: Option<(String, String)>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pull) = self.pulls_of(job, cx).into_iter().find(|one| one.number == number) else { return };
        self.open_pull(pull, window, cx);
        if let Some((sha, headline)) = commit {
            self.show_commit(sha, headline, window, cx);
        }
        if let Some(panel) = self.pull_panel_mut() {
            panel.from_story = true;
        }
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
