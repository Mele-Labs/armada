//! One feature's page: its sections down the side, one open, with the filter chips over its asks. An entity of
//! its own, so the Features tab fills a slot with it and a later piece can open it in an overlay.
//!
//! What is read is Fleet's: `get_feature_page` and `list_feature_asks`, together, off the main thread, and the page
//! draws bones until they answer. `page_model.rs` works out every line it draws.
//!
//! **When he last looked at the feature is stamped when he leaves the page**, and never when he opens it, so the
//! chip Since my last look compares with the look before this one while he is on it.

use chrono::{DateTime, Utc};
use gpui::{App, Context, Entity, EventEmitter, FocusHandle, Focusable, KeyDownEvent, ListAlignment, ListState, Subscription, Task, Window};
use ipc::{FeatureAsks, FeaturePage};

use super::page_model::{self, Body, Chip, Entry, Section};
use super::{looked, mock, wire};
use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::settings::wire::Refused;

/// What the page tells whoever holds it.
pub enum PageEvent {
    /// Esc: the keys go back to where the page was opened from.
    Escape,
}

enum Source {
    Live(Entity<FleetConnection>),
    Mock,
}

/// A read that may not have answered yet.
pub(super) enum Load<T> {
    Reading,
    Read(T),
    Failed(String),
}

/// The three parts of a page the keys can be in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Region {
    Rail,
    Chips,
    Rows,
}

pub struct FeaturePageView {
    pub within: FocusHandle,
    pub(super) rail: FocusHandle,
    pub(super) chips: FocusHandle,
    pub(super) rows: FocusHandle,
    source: Source,
    project: String,
    slug: String,
    /// The area's title where whoever opened the page knows it.
    pub(super) area: Option<String>,
    pub(super) page: Load<FeaturePage>,
    pub(super) asks: Load<FeatureAsks>,
    pub(super) entries: Vec<Entry>,
    pub(super) body: Body,
    pub(super) section: Section,
    /// The chip applied, and the one the keys are on.
    pub(super) chip: Chip,
    pub(super) chip_at: Chip,
    /// The row the keys are on.
    pub(super) row_at: Option<usize>,
    pub(super) list: ListState,
    /// When he looked at the feature before this look, which the chip Since my last look compares with.
    pub(super) looked: Option<DateTime<Utc>>,
    /// The port the last read asked, so a connection that came back reads again.
    read_port: Option<u16>,
    work: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl EventEmitter<PageEvent> for FeaturePageView {}

impl Focusable for FeaturePageView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.rail.clone()
    }
}

impl FeaturePageView {
    /// On Fleet: the feature `slug` of `project`, by Fleet's id of the project.
    pub fn live(fleet: &Entity<FleetConnection>, project: String, slug: String, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        looked::persist(cx);
        let mut this = Self::new(Source::Live(fleet.clone()), project, slug, cx);
        let watch = cx.observe(fleet, |this, _, cx| {
            if this.port(cx) != this.read_port {
                this.read(cx);
            }
        });
        this._subs.push(watch);
        this.read(cx);
        this
    }

    /// On the mock page in `mock.rs`.
    pub fn mock(slug: String, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Source::Mock, mock::PROJECT.to_string(), slug.clone(), cx);
        this.page = Load::Read(mock::page(&slug));
        this.asks = Load::Read(mock::asks(&slug));
        this.rebuild(cx);
        this
    }

    fn new(source: Source, project: String, slug: String, cx: &mut Context<Self>) -> Self {
        let looked = looked::before(&project, &slug, cx);
        FeaturePageView {
            within: cx.focus_handle(),
            rail: cx.focus_handle(),
            chips: cx.focus_handle(),
            rows: cx.focus_handle(),
            source,
            project,
            slug,
            area: None,
            page: Load::Reading,
            asks: Load::Reading,
            entries: Vec::new(),
            body: Body::Rows(Vec::new()),
            section: Section::Who,
            chip: Chip::All,
            chip_at: Chip::All,
            row_at: None,
            list: ListState::new(0, ListAlignment::Top, gpui::px(200.)),
            looked,
            read_port: None,
            work: None,
            _subs: Vec::new(),
        }
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }

    pub fn project(&self) -> &str {
        &self.project
    }

    pub fn section(&self) -> Section {
        self.section
    }

    pub fn chip(&self) -> Chip {
        self.chip
    }

    /// The title of the area the feature is in, for the chip in the top strip.
    pub fn set_area(&mut self, title: Option<String>, cx: &mut Context<Self>) {
        if self.area != title {
            self.area = title;
            cx.notify();
        }
    }

    // ---- reading -----------------------------------------------------------------------------

    fn port(&self, cx: &App) -> Option<u16> {
        match &self.source {
            Source::Live(fleet) => fleet.read(cx).held().port(),
            Source::Mock => None,
        }
    }

    /// Read the page and its asks from Fleet again, keeping what is drawn until the answer comes.
    pub fn read(&mut self, cx: &mut Context<Self>) {
        let Source::Live(_) = &self.source else { return };
        let port = self.port(cx);
        self.read_port = port;
        let Some(port) = port else {
            if !matches!(self.page, Load::Read(_)) {
                self.page = Load::Failed("Fleet is not connected".into());
                cx.notify();
            }
            return;
        };
        let page = wire::page(port, &self.project, &self.slug, cx);
        let asks = wire::asks(port, &self.project, &self.slug, cx);
        self.work = Some(cx.spawn(async move |this, cx| {
            let page = page.await;
            let asks = asks.await;
            let _ = this.update(cx, |this, cx| this.took(page, asks, cx));
        }));
    }

    /// Both answers folded in, and the page drawn once.
    fn took(&mut self, page: Result<FeaturePage, Refused>, asks: Result<FeatureAsks, Refused>, cx: &mut Context<Self>) {
        match page {
            Ok(page) => self.page = Load::Read(page),
            // A read that failed keeps what was read before, where there was one.
            Err(why) if !matches!(self.page, Load::Read(_)) => self.page = Load::Failed(why.said),
            Err(_) => {}
        }
        match asks {
            Ok(asks) => self.asks = Load::Read(asks),
            Err(why) if !matches!(self.asks, Load::Read(_)) => self.asks = Load::Failed(why.said),
            Err(_) => {}
        }
        self.rebuild(cx);
    }

    fn asks_read(&self) -> Option<&[ipc::Ask]> {
        match &self.asks {
            Load::Read(read) => Some(&read.asks),
            _ => None,
        }
    }

    /// The rail's lines and the open section's body, from what is held now.
    pub(super) fn rebuild(&mut self, cx: &mut Context<Self>) {
        if let Load::Read(page) = &self.page {
            self.entries = page_model::entries(page, self.asks_read());
            self.body = page_model::body(self.section, page, self.asks_read(), self.chip, self.looked);
        } else {
            self.entries.clear();
            self.body = Body::Rows(Vec::new());
        }
        let count = self.row_count();
        self.list.reset(count);
        self.row_at = self.row_at.filter(|at| *at < count);
        cx.notify();
    }

    pub(super) fn row_count(&self) -> usize {
        match &self.body {
            Body::Rows(rows) => rows.len(),
            _ => 0,
        }
    }

    // ---- looking -----------------------------------------------------------------------------

    /// The page is in front again: what was last looked at is read again, and so is Fleet.
    pub fn enter(&mut self, cx: &mut Context<Self>) {
        self.looked = looked::before(&self.project, &self.slug, cx);
        self.rebuild(cx);
        self.read(cx);
    }

    /// He leaves the page, by closing it or opening another feature: this is when it counts as looked at.
    pub fn leave(&mut self, cx: &mut Context<Self>) {
        looked::stamp(&self.project, &self.slug, Utc::now(), cx);
    }

    // ---- what the keys are on ----------------------------------------------------------------

    pub(super) fn select(&mut self, section: Section, cx: &mut Context<Self>) {
        if self.section != section {
            self.section = section;
            self.row_at = None;
            self.rebuild(cx);
        }
    }

    fn step_section(&mut self, by: isize, cx: &mut Context<Self>) {
        let at = Section::ALL.iter().position(|one| *one == self.section).unwrap_or(0) as isize;
        let next = (at + by).clamp(0, Section::ALL.len() as isize - 1) as usize;
        self.select(Section::ALL[next], cx);
    }

    pub(super) fn apply(&mut self, chip: Chip, cx: &mut Context<Self>) {
        self.chip_at = chip;
        if self.chip != chip {
            self.chip = chip;
            self.row_at = None;
            self.rebuild(cx);
        } else {
            cx.notify();
        }
    }

    fn step_chip(&mut self, by: isize, cx: &mut Context<Self>) {
        let at = Chip::ALL.iter().position(|one| *one == self.chip_at).unwrap_or(0) as isize;
        self.chip_at = Chip::ALL[(at + by).clamp(0, Chip::ALL.len() as isize - 1) as usize];
        cx.notify();
    }

    pub(super) fn step_row(&mut self, by: isize, cx: &mut Context<Self>) {
        let count = self.row_count() as isize;
        if count == 0 {
            return;
        }
        let at = self.row_at.map_or(if by > 0 { -1 } else { count }, |at| at as isize);
        let next = (at + by).clamp(0, count - 1) as usize;
        self.row_at = Some(next);
        self.list.scroll_to_reveal_item(next);
        cx.notify();
    }

    pub(super) fn pick_row(&mut self, at: usize, cx: &mut Context<Self>) {
        self.row_at = Some(at);
        cx.notify();
    }

    /// The parts of the page that hold something, in the order Tab goes through them.
    pub(super) fn regions(&self) -> Vec<Region> {
        let mut regions = vec![Region::Rail];
        if self.section.is_asks() {
            regions.push(Region::Chips);
        }
        if self.row_count() > 0 {
            regions.push(Region::Rows);
        }
        regions
    }

    pub(super) fn region_in(&self, window: &Window) -> Region {
        if self.chips.is_focused(window) {
            Region::Chips
        } else if self.rows.is_focused(window) {
            Region::Rows
        } else {
            Region::Rail
        }
    }

    pub fn focus_rail(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_region(Region::Rail, window, cx);
    }

    pub(super) fn focus_region(&mut self, region: Region, window: &mut Window, cx: &mut Context<Self>) {
        if region == Region::Rows && self.row_at.is_none() && self.row_count() > 0 {
            self.row_at = Some(0);
        }
        let handle = match region {
            Region::Rail => &self.rail,
            Region::Chips => &self.chips,
            Region::Rows => &self.rows,
        };
        window.focus(handle, cx);
        cx.notify();
    }

    fn next_region(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let regions = self.regions();
        let at = regions.iter().position(|one| *one == self.region_in(window)).unwrap_or(0) as isize;
        let next = regions[(at + by).rem_euclid(regions.len() as isize) as usize];
        self.focus_region(next, window, cx);
    }

    // ---- keys --------------------------------------------------------------------------------

    pub(super) fn on_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" {
            cx.emit(PageEvent::Escape);
            cx.stop_propagation();
        }
    }

    /// The registry's `features` acts, pressed while the keys are on the page. All of them stop here except
    /// the filter, which is the list's: a page is never opened again by the list's own Enter.
    pub(super) fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if !act.id.starts_with("features_") || act.id == "features_filter" {
            cx.propagate();
            return;
        }
        let region = self.region_in(window);
        match act.id {
            "features_move" => {
                let by = if act.slot % 2 == 0 { 1 } else { -1 };
                match region {
                    Region::Rail => self.step_section(by, cx),
                    Region::Rows => self.step_row(by, cx),
                    Region::Chips => {}
                }
            }
            "features_chip" if region == Region::Chips => self.step_chip(if act.slot == 0 { -1 } else { 1 }, cx),
            "features_open" => match region {
                Region::Chips => self.apply(self.chip_at, cx),
                Region::Rail => self.next_region(1, window, cx),
                Region::Rows => {}
            },
            "features_pane" => self.next_region(if act.slot == 0 { 1 } else { -1 }, window, cx),
            _ => {}
        }
    }
}
