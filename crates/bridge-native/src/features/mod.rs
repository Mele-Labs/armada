//! The Features tab: a project's feature library as areas and the features in each, with who is on each now
//! and a stale mark, in a list on the left and a slot for the feature's page on the right.
//!
//! What is read is Fleet's own: `list_features`. `model.rs` works out the lines the list draws from it and
//! nothing there touches the screen. The screen is an entity with a `within` handle, a place of its own in the
//! title bar's strip (`operator/screen.rs`).

pub(crate) mod asks;
mod curate;
mod curate_view;
pub(crate) mod dossier;
mod looked;
pub(crate) mod mock;
mod model;
pub mod of_path;
pub mod overlay;
pub mod page;
pub(crate) mod page_model;
mod page_view;
// Pictures for a Job to look at, taken from a temporary hook in `main()`, which is not kept.
#[allow(dead_code)]
pub mod shots;
pub mod story_link;
mod view;
pub(crate) mod wire;

use gpui::{prelude::*, AnyWindowHandle, App, Context, Entity, FocusHandle, Focusable as _, KeyDownEvent, ListAlignment, ListState, Subscription, Task, Window};
use gpui_component::input::{InputEvent, InputState};
use ipc::FeatureLibrary;

use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::settings::wire::Refused;
use crate::titlebar;
use model::{Line, Listed};
use page::{FeaturePageEvent, FeaturePageView};

/// Where the list reads from.
enum Source {
    Live(Entity<FleetConnection>),
    Mock,
}

/// A read that may not have answered yet.
enum Load {
    Reading,
    Read(FeatureLibrary),
    Failed(String),
}

/// What a read was made for: the port, the project asked for (none is All) and the projects Fleet lists.
type Key = (u16, Option<String>, Vec<String>);

pub struct FeaturesScreen {
    pub within: FocusHandle,
    /// The list takes the keys.
    list_focus: FocusHandle,
    filter: Entity<InputState>,
    source: Source,
    /// The project given by `set_project`, which the title bar's pick does not move.
    explicit: Option<String>,
    library: Load,
    listed: Listed,
    list: ListState,
    /// The feature the keys are on, by slug.
    cursor: Option<String>,
    /// The window the screen is in, so the page of the row the keys are on can be drawn when the library arrives.
    window: AnyWindowHandle,
    /// The feature whose page is open beside the list, by slug.
    opened: Option<String>,
    /// That page, and what it says to this screen.
    page: Option<Entity<FeaturePageView>>,
    /// The window is narrower than `model::STACKED_BELOW`: the list and the page take turns at the full width.
    stacked: bool,
    /// Stacked: the page is the one drawn, full width, and the list is under it. Never set at a width where they sit side by side.
    showing: bool,
    page_sub: Option<Subscription>,
    /// The project the library on screen belongs to.
    library_project: Option<String>,
    read_for: Option<Key>,
    entered: bool,
    work: Option<Task<()>>,
    /// The buttons, editor, picker and sheet that curate the library from the list.
    curate: curate::Curate,
    bar: Option<Subscription>,
    _subs: Vec<Subscription>,
}

/// What the filter keeps while it is typed in, under its own key so a draft survives a restart.
const KEEP_FILTER: &str = "features:filter";

impl FeaturesScreen {
    /// On Fleet. `project` is Fleet's id of the project to show; none follows the project the title bar is
    /// scoped to, and where none is picked, the first project that holds features.
    pub fn live(fleet: &Entity<FleetConnection>, project: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Source::Live(fleet.clone()), window, cx);
        this.explicit = project;
        let watch = cx.observe(fleet, |this, _, cx| this.moved(cx));
        this._subs.push(watch);
        this
    }

    /// On the mock library in `mock.rs`.
    pub fn mock(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Source::Mock, window, cx);
        this.library = Load::Read(mock::library());
        this.relist(cx);
        this
    }

    fn new(source: Source, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Find a feature"));
        if let Some(words) = crate::drafts::kept(KEEP_FILTER, cx) {
            filter.update(cx, |state, cx| state.set_value(words, window, cx));
        }
        let mut subs = vec![cx.subscribe_in(&filter, window, |this, state, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let text = state.read(cx).value().to_string();
                if text.is_empty() {
                    crate::drafts::clear(KEEP_FILTER, cx);
                } else {
                    crate::drafts::keep(KEEP_FILTER, &text, cx);
                }
                this.relist(cx);
            }
            InputEvent::PressEnter { .. } => this.focus_list(window, cx),
            _ => {}
        })];
        subs.push(cx.observe_window_bounds(window, |this, window, cx| this.resized(window, cx)));
        FeaturesScreen {
            within: cx.focus_handle(),
            list_focus: cx.focus_handle(),
            filter,
            source,
            explicit: None,
            library: Load::Reading,
            listed: Listed::default(),
            list: ListState::new(0, ListAlignment::Top, gpui::px(200.)),
            cursor: None,
            window: window.window_handle(),
            opened: None,
            page: None,
            stacked: window.viewport_size().width < gpui::px(model::STACKED_BELOW),
            showing: false,
            page_sub: None,
            library_project: None,
            read_for: None,
            entered: false,
            work: None,
            curate: curate::Curate::default(),
            bar: None,
            _subs: subs,
        }
    }

    // ---- reading -----------------------------------------------------------------------------

    /// The project asked for: the one given, else the one the title bar is scoped to. None is All.
    fn wanted(&self, cx: &App) -> Option<String> {
        self.explicit.clone().or_else(|| titlebar::live(cx).and_then(|bar| bar.read(cx).face.scoped_project().map(str::to_string)))
    }

    /// What a read would be made for now, or none where Fleet cannot be asked.
    fn key(&self, cx: &App) -> Option<Key> {
        let Source::Live(fleet) = &self.source else { return None };
        let held = fleet.read(cx).held();
        let projects = held.projects.as_ref().map(|read| read.projects.iter().map(|one| one.id.clone()).collect()).unwrap_or_default();
        Some((held.port()?, self.wanted(cx), projects))
    }

    /// The project is given another, or the connection came back: read again if the screen is in front.
    fn moved(&mut self, cx: &mut Context<Self>) {
        if self.entered && self.key(cx) != self.read_for {
            self.read(cx);
        }
    }

    /// Read the library from Fleet again, keeping what is drawn until the answer comes. A project that is
    /// not the one on screen draws the skeleton.
    // The door for the palette and a project page; nothing opens it yet.
    #[allow(dead_code)]
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.read(cx);
    }

    /// Show another project, or follow the title bar again with none.
    // The door for a project page; the title bar's pick is all that moves the project today.
    #[allow(dead_code)]
    pub fn set_project(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        if self.explicit != project {
            self.explicit = project;
            if self.entered {
                self.read(cx);
            }
        }
    }

    fn read(&mut self, cx: &mut Context<Self>) {
        let Source::Live(_) = &self.source else { return };
        let Some(key) = self.key(cx) else {
            if !matches!(self.library, Load::Read(_)) {
                self.library = Load::Failed("Fleet is not connected".into());
                self.relist(cx);
            }
            self.read_for = None;
            return;
        };
        let (port, wanted, listed) = key.clone();
        if self.read_for.as_ref().is_some_and(|before| before.1 != wanted) {
            self.library = Load::Reading;
            self.relist(cx);
        }
        self.read_for = Some(key);
        // A named project is the one asked; with none picked, each project in Fleet's order is asked and the
        // first that holds features is the one shown.
        let asked: Vec<String> = wanted.map_or(listed, |one| vec![one]);
        let reads: Vec<_> = asked.iter().map(|project| wire::library(port, project, cx)).collect();
        self.work = Some(cx.spawn(async move |this, cx| {
            let mut answer: Option<Result<FeatureLibrary, Refused>> = None;
            for read in reads {
                match (read.await, &answer) {
                    (Ok(library), _) if model::holds_features(&library) => {
                        answer = Some(Ok(library));
                        break;
                    }
                    (Ok(library), None | Some(Err(_))) => answer = Some(Ok(library)),
                    (Err(why), None) => answer = Some(Err(why)),
                    _ => {}
                }
            }
            let answer = answer.unwrap_or_else(|| Ok(FeatureLibrary { project: String::new(), areas: Vec::new(), librarian: Default::default() }));
            let _ = this.update(cx, |this, cx| this.took(answer, cx));
        }));
        cx.notify();
    }

    fn took(&mut self, answer: Result<FeatureLibrary, Refused>, cx: &mut Context<Self>) {
        match answer {
            Ok(library) => {
                if self.library_project.as_deref() != Some(library.project.as_str()) {
                    self.cursor = None;
                    self.close_page(cx);
                }
                self.library_project = Some(library.project.clone());
                self.library = Load::Read(library);
            }
            // A read that failed keeps what was read before, where there was one.
            Err(why) if !matches!(self.library, Load::Read(_)) => self.library = Load::Failed(why.said),
            Err(_) => {}
        }
        self.relist(cx);
    }

    /// The lines the list draws from what is held and what the filter says, and the row the keys are on.
    fn relist(&mut self, cx: &mut Context<Self>) {
        let query = self.filter.read(cx).value().to_string();
        self.keep_stops(cx);
        self.listed = match &self.library {
            Load::Read(library) => model::listed(library, &query),
            _ => Listed::default(),
        };
        self.list.reset(self.listed.lines.len());
        if !self.cursor.as_deref().is_some_and(|slug| self.listed.find(slug).is_some()) {
            self.cursor = self.listed.shown.first().map(|one| one.row.slug.clone());
        }
        self.preview(cx);
        self.tune_page(cx);
        cx.notify();
    }

    /// The page of the feature the keys are on draws beside the list as the row moves, with the keys staying on the list.
    fn preview(&mut self, cx: &mut Context<Self>) {
        // Stacked, the list is alone: a page is opened by Enter or a press, never by the row moving.
        if self.stacked {
            return;
        }
        let Some(slug) = self.cursor.clone().filter(|slug| self.opened.as_deref() != Some(slug.as_str())) else { return };
        let (me, handle) = (cx.entity(), self.window);
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                me.update(cx, |this, cx| {
                    if !this.stacked && this.cursor.as_deref() == Some(slug.as_str()) && this.opened.as_deref() != Some(slug.as_str()) && this.listed.find(&slug).is_some() {
                        this.open_at(slug, window, cx);
                    }
                })
            });
        });
    }

    // ---- what the keys are on ----------------------------------------------------------------

    fn cursor_at(&self) -> Option<usize> {
        self.cursor.as_deref().and_then(|slug| self.listed.find(slug))
    }

    /// The feature whose page is open beside the list.
    #[cfg(test)]
    pub fn opened(&self) -> Option<&ipc::FeatureRow> {
        let slug = self.opened.as_deref()?;
        self.listed.find(slug).map(|at| &self.listed.shown[at].row).or_else(|| match &self.library {
            Load::Read(library) => library.areas.iter().flat_map(|area| &area.features).find(|row| row.slug == slug),
            _ => None,
        })
    }

    /// Whether the screen is in front, following Fleet.
    #[cfg(test)]
    pub fn entered(&self) -> bool {
        self.entered
    }

    /// Why the library was not read, where it was not.
    #[cfg(test)]
    pub fn not_read(&self) -> Option<&str> {
        match &self.library {
            Load::Failed(why) => Some(why),
            _ => None,
        }
    }

    /// J and K, Down and Up: the next or the previous feature, past the area heads.
    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let count = self.listed.shown.len() as isize;
        if count == 0 {
            return;
        }
        let at = self.cursor_at().map_or(if by > 0 { -1 } else { count }, |at| at as isize);
        let next = (at + by).clamp(0, count - 1) as usize;
        self.cursor = Some(self.listed.shown[next].row.slug.clone());
        if let Some(line) = self.listed.line_of(next) {
            // Going up to the first feature of an area shows its head too.
            let head = line.checked_sub(1).filter(|before| matches!(self.listed.lines[*before], Line::Head { .. }));
            self.list.scroll_to_reveal_item(if by < 0 { head.unwrap_or(line) } else { line });
        }
        self.preview(cx);
        cx.notify();
    }

    /// Enter, or a press on a row: the feature's page opens beside the list, in place of the one that was open.
    fn open_at(&mut self, slug: String, window: &mut Window, cx: &mut Context<Self>) {
        self.cursor = Some(slug.clone());
        if self.opened.as_deref() != Some(slug.as_str()) {
            self.close_page(cx);
            let project = self.library_project.clone();
            let area = self.listed.find(&slug).and_then(|at| self.area_title(&self.listed.shown[at].row.area));
            let page = match (&self.source, project) {
                (Source::Live(fleet), Some(project)) => Some(cx.new(|cx| FeaturePageView::live(fleet, project, slug.clone(), window, cx))),
                (Source::Mock, _) => Some(cx.new(|cx| FeaturePageView::mock(slug.clone(), window, cx))),
                _ => None,
            };
            if let Some(page) = page {
                page.update(cx, |page, cx| page.set_area(area, cx));
                self.page_sub = Some(cx.subscribe_in(&page, window, |this, _, event: &FeaturePageEvent, window, cx| match event {
                    FeaturePageEvent::Escape if this.stacked => this.back(window, cx),
                    FeaturePageEvent::Escape => this.focus_list(window, cx),
                    FeaturePageEvent::Story { session_id, ask_id } => this.open_story(session_id.clone(), ask_id.clone(), cx),
                }));
                self.page = Some(page);
                self.opened = Some(slug);
                self.tune_page(cx);
            }
        }
        cx.notify();
    }

    /// What the open page takes from the list beside it: its area's hue, and the width the list leaves it.
    fn tune_page(&mut self, cx: &mut Context<Self>) {
        let Some(page) = self.page.clone() else { return };
        let hue = self.opened.as_deref().and_then(|slug| self.listed.find(slug)).map(|at| self.hue_of(&self.listed.shown[at].row.area));
        let inset = self.inset(cx);
        let flush = !self.stacked;
        page.update(cx, |page, cx| {
            page.set_flush(flush, cx);
            if let Some(hue) = hue {
                page.set_hue(Some(hue), cx);
            }
            page.set_inset(inset, cx);
        });
    }

    /// The Story of an ask: the request is left for the Story tab to read, and the Job opens as any screen opens one.
    fn open_story(&mut self, session_id: ipc::SessionId, ask_id: String, cx: &mut Context<Self>) {
        let job = session_id.as_str().to_string();
        story_link::put(story_link::StoryRequest { session_id, ask_id }, cx);
        if let Some(sink) = cx.try_global::<titlebar::AskedSink>().cloned() {
            (sink.0)(&titlebar::Asked::OpenJob(job), cx);
        }
    }

    fn open_cursor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(slug) = self.cursor.clone() {
            self.open_at(slug, window, cx);
            self.engage_page(cx);
            self.show_page(window, cx);
        }
    }

    /// Stacked, the page just opened is drawn full width and has the keys, as the list is not drawn to hold them.
    fn show_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.stacked {
            return;
        }
        if let Some(page) = self.page.clone() {
            self.showing = true;
            page.update(cx, |page, cx| page.focus_rail(window, cx));
            cx.notify();
        }
    }

    /// Stacked, Back or Esc on the page: the list is drawn again with the same row selected, and the page is put away,
    /// which stamps it looked at where he engaged with it.
    fn back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_page(cx);
        self.focus_list(window, cx);
    }

    /// The window grew past or shrank under the width the list and the page share. Narrowing puts the open page away so
    /// the list is alone; widening draws the page of the row the keys are on beside the list again.
    fn resized(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let stacked = window.viewport_size().width < gpui::px(model::STACKED_BELOW);
        if stacked == self.stacked {
            return;
        }
        self.stacked = stacked;
        if stacked {
            if self.page.is_some() {
                self.back(window, cx);
            }
        } else {
            self.showing = false;
            self.preview(cx);
        }
        self.tune_page(cx);
        cx.notify();
    }

    /// Enter or a press on the feature: he is on its page, so leaving it counts as looked at.
    pub(super) fn engage_page(&mut self, cx: &mut Context<Self>) {
        if let Some(page) = &self.page {
            page.update(cx, |page, _| page.engage());
        }
    }

    /// The open page is put away, which is when it counts as looked at.
    fn close_page(&mut self, cx: &mut Context<Self>) {
        if let Some(page) = self.page.take() {
            page.update(cx, |page, cx| page.leave(cx));
        }
        self.page_sub = None;
        self.opened = None;
        self.showing = false;
    }

    /// The title of an area of the library on screen, by its slug.
    fn area_title(&self, slug: &str) -> Option<String> {
        match &self.library {
            Load::Read(library) => library.areas.iter().find(|area| area.slug == slug).map(|area| area.title.clone()),
            _ => None,
        }
    }

    fn focus_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.list_focus, cx);
        cx.notify();
    }

    fn typing(&self, window: &Window, cx: &App) -> bool {
        self.filter.read(cx).focus_handle(cx).is_focused(window)
    }

    // ---- entering ----------------------------------------------------------------------------

    /// Brought to the front: the keys go to the list, and Fleet is read again.
    pub fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.entered = true;
        match self.page.clone().filter(|_| self.showing) {
            Some(page) => page.update(cx, |page, cx| page.focus_rail(window, cx)),
            None => self.focus_list(window, cx),
        }
        if let Some(page) = &self.page {
            page.update(cx, |page, cx| page.enter(cx));
        }
        if self.bar.is_none() {
            if let Some(bar) = titlebar::live(cx) {
                self.bar = Some(cx.observe(&bar, |this, _, cx| this.moved(cx)));
            }
        }
        self.read(cx);
    }

    /// Another place in front: the screen stops following Fleet until it is entered again.
    pub fn leave(&mut self, cx: &mut Context<Self>) {
        self.entered = false;
        if let Some(page) = &self.page {
            page.update(cx, |page, cx| page.leave(cx));
        }
    }

    // ---- keys --------------------------------------------------------------------------------

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" && self.escape_curate(window, cx) {
            cx.stop_propagation();
            return;
        }
        if event.keystroke.key == "escape" && self.typing(window, cx) {
            self.focus_list(window, cx);
            cx.stop_propagation();
        }
    }

    /// The registry's `features` acts, pressed. Not answered while the filter has the keys, which take them as typing.
    fn on_features_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // Tab goes through a row's own buttons, an editor or the sheet before it goes on to the page.
        let tab_here = act.id == "features_pane" && (!self.typing(window, cx) || self.typing_in_curate(window, cx));
        if tab_here && self.tab_curate(act.slot % 2 == 1, window, cx) {
            return true;
        }
        if self.typing(window, cx) || self.curating(window, cx) {
            // Up and Down move along the sheet's checklist; Enter on a button of a row presses it.
            return match act.id {
                "features_move" => self.curate.sheet.is_some() && self.step_split(if act.slot % 2 == 0 { 1 } else { -1 }, window, cx),
                "features_open" if !self.typing(window, cx) && !self.typing_in_curate(window, cx) && self.curate.menu.is_none() => self.press_stop(window, cx),
                _ => false,
            };
        }
        match act.id {
            "features_move" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "features_open" => self.open_cursor(window, cx),
            // Stacked, the list has no page beside it: Tab opens the row's page, as Enter does.
            "features_pane" if self.stacked && !self.showing => self.open_cursor(window, cx),
            "features_pane" => match self.page.clone() {
                Some(page) => page.update(cx, |page, cx| page.focus_rail(window, cx)),
                None => return false,
            },
            "features_filter" if self.showing => return false,
            "features_filter" => {
                let filter = self.filter.read(cx).focus_handle(cx);
                window.focus(&filter, cx);
            }
            _ => return false,
        }
        true
    }

    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if !act.id.starts_with("features_") || !self.on_features_act(act, window, cx) {
            cx.propagate();
        }
    }
}

#[cfg(test)]
mod ask_tests;
#[cfg(test)]
mod curate_tests;
#[cfg(test)]
mod of_path_tests;
#[cfg(test)]
mod page_tests;
#[cfg(test)]
mod tests;
