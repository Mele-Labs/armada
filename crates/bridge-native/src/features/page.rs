//! One feature's page: its sections down the side, one open, with the filter chips over its asks. An entity of
//! its own, so the Features tab fills a slot with it and a later piece can open it in an overlay.
//!
//! What is read is Fleet's: `get_feature_page` and `list_feature_asks`, together, off the main thread, and the page
//! draws bones until they answer. `page_model.rs` works out every line it draws.
//!
//! **When he last looked at the feature is stamped when he leaves the page**, and never when he opens it, so the
//! chip Since my last look compares with the look before this one while he is on it.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use gpui::{App, Context, Entity, EventEmitter, FocusHandle, Focusable, KeyDownEvent, ListAlignment, ListState, Pixels, Subscription, Task, Window};
use ipc::{FeatureAsks, FeaturePage, SessionId};

use super::page_model::{self, Body, Chip, Entry, Item, Section, Titles};
use super::{looked, mock, wire};
use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::look::Token;
use crate::settings::wire::Refused;

/// What the page tells whoever holds it.
pub enum FeaturePageEvent {
    /// Esc: the keys go back to where the page was opened from.
    Escape,
    /// The Story of an ask: the Job to open, and the ask to put in front on its Story tab.
    Story { session_id: SessionId, ask_id: String },
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

/// The parts of a page the keys can be in: the card they are on, the filter chips of the asks, the rows of the card, and the
/// button that starts a Job on the feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Region {
    Rail,
    Chips,
    Rows,
    /// The command wells of a written card, which Enter copies.
    Well,
    Start,
}

pub struct FeaturePageView {
    pub within: FocusHandle,
    pub(super) rail: FocusHandle,
    pub(super) chips: FocusHandle,
    pub(super) rows: FocusHandle,
    pub(super) well: FocusHandle,
    pub(super) start: FocusHandle,
    /// The command well the keys are on, among those of the open card.
    pub(super) well_at: usize,
    source: Source,
    project: String,
    slug: String,
    /// The area's title where whoever opened the page knows it.
    pub(super) area: Option<String>,
    pub(super) page: Load<FeaturePage>,
    pub(super) asks: Load<FeatureAsks>,
    pub(super) entries: Vec<Entry>,
    /// What every section holds under the chip applied, which is what the cards draw.
    pub(super) cards: HashMap<Section, Body>,
    /// Whether the feature has any ask at all, whatever the chip keeps.
    pub(super) has_asks: bool,
    /// What the open section holds.
    pub(super) body: Body,
    /// The card the keys are on, and the one whose rows and chips Tab reaches.
    pub(super) section: Section,
    /// The hue of the feature's area, where whoever opened the page knows it.
    pub(super) hue: Option<Token>,
    /// How much of the window's width is not the page's: the list beside it.
    pub(super) inset: Pixels,
    /// Whether the band is on the top edge of a rounded panel, and so rounds its top corners to follow it. Under a Back row it is not.
    pub(super) flush: bool,
    /// What the list holds, in its order: the rows, and the line that folds the older asks.
    pub(super) items: Vec<Item>,
    /// The ask sections whose older rows are open. Not saved.
    pub(super) unfolded: HashSet<Section>,
    /// The titles of the Jobs an ask names, by id, from the Jobs Fleet holds.
    pub(super) titles: Titles,
    /// The chip applied, and the one the keys are on.
    pub(super) chip: Chip,
    pub(super) chip_at: Chip,
    /// The row the keys are on.
    pub(super) row_at: Option<usize>,
    pub(super) list: ListState,
    /// When he looked at the feature before this look, which the chip Since my last look compares with.
    pub(super) looked: Option<DateTime<Utc>>,
    /// Whether he engaged with the page: Enter on it, a click in it, or Tab into its sections. A page drawn only because the
    /// selection passed over it stamps nothing when it is left.
    engaged: bool,
    /// The port the last read asked, so a connection that came back reads again.
    read_port: Option<u16>,
    work: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl EventEmitter<FeaturePageEvent> for FeaturePageView {}

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
            this.name_jobs(cx);
        });
        this._subs.push(watch);
        this.titles = this.held_titles(cx);
        this.read(cx);
        this
    }

    /// On the mock page in `mock.rs`.
    pub fn mock(slug: String, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Source::Mock, mock::PROJECT.to_string(), slug.clone(), cx);
        this.page = Load::Read(mock::page(&slug));
        this.asks = Load::Read(mock::asks(&slug));
        this.titles = mock::titles();
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
            well: cx.focus_handle(),
            start: cx.focus_handle(),
            well_at: 0,
            source,
            project,
            slug,
            engaged: false,
            area: None,
            page: Load::Reading,
            asks: Load::Reading,
            entries: Vec::new(),
            cards: HashMap::new(),
            has_asks: false,
            body: Body::Rows(Vec::new()),
            section: Section::Who,
            hue: None,
            inset: gpui::px(0.),
            flush: false,
            items: Vec::new(),
            unfolded: HashSet::new(),
            titles: Titles::new(),
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

    #[cfg(test)]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    #[cfg(test)]
    pub fn section(&self) -> Section {
        self.section
    }

    #[cfg(test)]
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

    /// The hue of the feature's area, for the band across the top.
    pub fn set_hue(&mut self, hue: Option<Token>, cx: &mut Context<Self>) {
        if self.hue != hue {
            self.hue = hue;
            cx.notify();
        }
    }

    /// Whether the page's band is on its panel's top edge, with nothing over it.
    pub fn set_flush(&mut self, flush: bool, cx: &mut Context<Self>) {
        if self.flush != flush {
            self.flush = flush;
            cx.notify();
        }
    }

    /// The width the list beside the page takes, so the page lays its cards out in what is left.
    pub fn set_inset(&mut self, inset: Pixels, cx: &mut Context<Self>) {
        if self.inset != inset {
            self.inset = inset;
            cx.notify();
        }
    }

    // ---- reading -----------------------------------------------------------------------------

    /// The titles of the Jobs the connection holds, by id.
    fn held_titles(&self, cx: &App) -> Titles {
        match &self.source {
            Source::Live(fleet) => fleet.read(cx).held().jobs.iter().filter_map(|job| job.title.clone().map(|title| (job.id.as_str().to_string(), title))).collect(),
            Source::Mock => Titles::new(),
        }
    }

    /// The Jobs Fleet holds changed: the rows that name one are drawn again where a title changed.
    fn name_jobs(&mut self, cx: &mut Context<Self>) {
        let titles = self.held_titles(cx);
        if titles != self.titles {
            self.titles = titles;
            self.rebuild(cx);
        }
    }

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
            let asks = match &self.asks {
                Load::Read(read) => Some(&read.asks[..]),
                _ => None,
            };
            self.cards = Section::ALL.into_iter().map(|section| (section, page_model::body(section, page, asks, self.chip, self.looked, &self.titles))).collect();
            self.has_asks = [Section::Bugs, Section::Changes, Section::Ideas].into_iter().any(|section| matches!(page_model::body(section, page, asks, Chip::All, None, &self.titles), Body::Rows(rows) if !rows.is_empty()));
            // A card left out for having nothing in it is not one the keys can be on.
            if !self.present(self.section) && !(self.section.is_asks() && self.has_asks) {
                if let Some(first) = Section::NAV.into_iter().find(|section| self.present(*section)) {
                    self.section = first;
                    self.row_at = None;
                }
            }
            self.body = self.cards.get(&self.section).cloned().unwrap_or(Body::Rows(Vec::new()));
        } else {
            self.entries.clear();
            self.cards.clear();
            self.has_asks = false;
            self.body = Body::Rows(Vec::new());
        }
        self.refold(cx);
    }

    /// The places of the list from the rows and the fold, and the list drawn again.
    fn refold(&mut self, cx: &mut Context<Self>) {
        self.items = match &self.body {
            Body::Rows(rows) => page_model::items(self.section, rows.len(), self.unfolded.contains(&self.section)),
            _ => Vec::new(),
        };
        let count = self.items.len();
        self.list.reset(count);
        self.row_at = self.row_at.filter(|at| *at < count);
        cx.notify();
    }

    /// Whether the section has something in its card. A card with nothing in it is still drawn, but the keys pass it over.
    pub(super) fn present(&self, section: Section) -> bool {
        match self.cards.get(&section) {
            Some(Body::Rows(rows)) => !rows.is_empty(),
            Some(Body::Code(code)) => !(code.paths.is_empty() && code.entry_points.is_empty() && code.files.is_empty()),
            Some(Body::Written(written)) => written.is_some(),
            None => false,
        }
    }

    /// The places the keys can be on: each row, and the fold line.
    pub(super) fn row_count(&self) -> usize {
        self.items.len()
    }

    /// Whether the open section's older asks are showing.
    pub fn is_unfolded(&self) -> bool {
        self.unfolded.contains(&self.section)
    }

    /// Open the older asks of the open section, or fold them again. The keys land on the fold line.
    pub(super) fn toggle_fold(&mut self, cx: &mut Context<Self>) {
        if !self.unfolded.remove(&self.section) {
            self.unfolded.insert(self.section);
        }
        self.refold(cx);
        if let Some(at) = self.items.iter().position(|item| *item == Item::Fold) {
            self.row_at = Some(at);
            self.list.scroll_to_reveal_item(at);
        }
    }

    // ---- looking -----------------------------------------------------------------------------

    /// The page is in front again: what was last looked at is read again, and so is Fleet.
    pub fn enter(&mut self, cx: &mut Context<Self>) {
        self.looked = looked::before(&self.project, &self.slug, cx);
        self.rebuild(cx);
        self.read(cx);
    }

    /// He leaves the page, by closing it or opening another feature: where he engaged with it, this is when it counts as looked at.
    pub fn leave(&mut self, cx: &mut Context<Self>) {
        if self.engaged {
            looked::stamp(&self.project, &self.slug, Utc::now(), cx);
            self.engaged = false;
        }
    }

    /// He is on this page, by Enter, a click or Tab: leaving it counts as looking at it.
    pub fn engage(&mut self) {
        self.engaged = true;
    }

    // ---- what the keys are on ----------------------------------------------------------------

    pub(super) fn select(&mut self, section: Section, cx: &mut Context<Self>) {
        if self.section != section {
            self.section = section;
            self.row_at = None;
            self.well_at = 0;
            self.rebuild(cx);
        }
    }

    fn step_section(&mut self, by: isize, cx: &mut Context<Self>) {
        let order: Vec<Section> = Section::NAV.into_iter().filter(|one| self.present(*one) || *one == self.section).collect();
        let at = order.iter().position(|one| *one == self.section).unwrap_or(0) as isize;
        let next = (at + by).clamp(0, order.len() as isize - 1) as usize;
        if order[next] != self.section {
            crate::ui_sounds::moved();
        }
        self.select(order[next], cx);
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
        let was = self.chip_at;
        self.chip_at = Chip::ALL[(at + by).clamp(0, Chip::ALL.len() as isize - 1) as usize];
        if self.chip_at != was {
            crate::ui_sounds::moved();
        }
        cx.notify();
    }

    pub(super) fn step_row(&mut self, by: isize, cx: &mut Context<Self>) {
        let count = self.row_count() as isize;
        if count == 0 {
            return;
        }
        let at = self.row_at.map_or(if by > 0 { -1 } else { count }, |at| at as isize);
        let next = (at + by).clamp(0, count - 1) as usize;
        if self.row_at != Some(next) {
            crate::ui_sounds::moved();
        }
        self.row_at = Some(next);
        self.list.scroll_to_reveal_item(next);
        cx.notify();
    }

    pub(super) fn pick_row(&mut self, at: usize, cx: &mut Context<Self>) {
        self.row_at = Some(at);
        cx.notify();
    }

    /// The ask on the place `at`, where it is one.
    fn ask_at(&self, at: usize) -> Option<&page_model::AskLine> {
        let Item::Row(row) = *self.items.get(at)? else { return None };
        match &self.body {
            Body::Rows(rows) => rows.get(row)?.ask.as_ref(),
            _ => None,
        }
    }

    /// Open the Job the ask on the place `at` came from, on its Story. The page says so and its holder opens it. An ask with
    /// no Job at all has no Story.
    pub(super) fn open_story(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(line) = self.ask_at(at) else { return };
        let Some(session_id) = line.story.clone() else { return };
        let ask_id = line.ask_id.clone();
        cx.emit(FeaturePageEvent::Story { session_id, ask_id });
    }

    /// Enter on the place the keys are on: a row opens its Story, and the fold line folds or opens.
    fn press_row(&mut self, cx: &mut Context<Self>) {
        match self.row_at.and_then(|at| self.items.get(at).map(|item| (at, *item))) {
            Some((_, Item::Fold)) => self.toggle_fold(cx),
            Some((at, Item::Row(_))) => self.open_story(at, cx),
            None => {}
        }
    }

    /// The commands of the open card, one for each well it draws.
    pub(super) fn wells(&self) -> Vec<String> {
        match self.cards.get(&self.section) {
            Some(Body::Written(Some(written))) => written.blocks.iter().filter(|block| block.command).map(|block| block.text.clone()).collect(),
            _ => Vec::new(),
        }
    }

    /// Put the command of the well the keys are on on the clipboard.
    pub(super) fn copy_well(&mut self, cx: &mut Context<Self>) {
        if let Some(command) = self.wells().get(self.well_at) {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(command.clone()));
        }
    }

    /// Start a Job on the feature: the dispatch panel opens as any screen opens it.
    pub(super) fn start_job(&mut self, cx: &mut Context<Self>) {
        self.engaged = true;
        if let Some(sink) = cx.try_global::<crate::titlebar::AskedSink>().cloned() {
            (sink.0)(&crate::titlebar::Asked::Dispatch, cx);
        }
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
        if !self.wells().is_empty() {
            regions.push(Region::Well);
        }
        if matches!(self.page, Load::Read(_)) {
            regions.push(Region::Start);
        }
        regions
    }

    pub(super) fn region_in(&self, window: &Window) -> Region {
        if self.chips.is_focused(window) {
            Region::Chips
        } else if self.rows.is_focused(window) {
            Region::Rows
        } else if self.well.is_focused(window) {
            Region::Well
        } else if self.start.is_focused(window) {
            Region::Start
        } else {
            Region::Rail
        }
    }

    pub fn focus_rail(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_region(Region::Rail, window, cx);
    }

    pub(super) fn focus_region(&mut self, region: Region, window: &mut Window, cx: &mut Context<Self>) {
        self.engaged = true;
        if region == Region::Rows && self.row_at.is_none() && self.row_count() > 0 {
            self.row_at = Some(0);
        }
        let handle = match region {
            Region::Rail => &self.rail,
            Region::Chips => &self.chips,
            Region::Rows => &self.rows,
            Region::Well => &self.well,
            Region::Start => &self.start,
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

    pub(super) fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" {
            // The older asks fold before the keys leave the page.
            if self.region_in(window) == Region::Rows && self.is_unfolded() {
                self.toggle_fold(cx);
            } else {
                cx.emit(FeaturePageEvent::Escape);
            }
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
                    Region::Well => {
                        let last = self.wells().len().saturating_sub(1);
                        let was = self.well_at;
                        self.well_at = (self.well_at as isize + by).clamp(0, last as isize) as usize;
                        if self.well_at != was {
                            crate::ui_sounds::moved();
                        }
                        cx.notify();
                    }
                    Region::Chips | Region::Start => {}
                }
            }
            "features_chip" if region == Region::Chips => self.step_chip(if act.slot == 0 { -1 } else { 1 }, cx),
            "features_open" => match region {
                Region::Chips => self.apply(self.chip_at, cx),
                Region::Rail => self.next_region(1, window, cx),
                Region::Rows => self.press_row(cx),
                Region::Well => self.copy_well(cx),
                Region::Start => self.start_job(cx),
            },
            "features_story" if region == Region::Rows => {
                if let Some(at) = self.row_at {
                    self.open_story(at, cx);
                }
            }
            "features_pane" => self.next_region(if act.slot == 0 { 1 } else { -1 }, window, cx),
            _ => {}
        }
    }
}
