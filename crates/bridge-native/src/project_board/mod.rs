//! The Project place: a header over five tabs. Dashboard is the board of blocks (`board.rs`, `view.rs`); Features, Since you looked
//! and Who's on what are the Features screen drawn in a mode of its own, so the three stand on the one library; Settings is the
//! sheet that edits a project's members and values, drawn as a page.
//!
//! The header is the same on every tab: the project chips, the box that asks the librarian, Edit, Members, Settings and Start.
//! Members and Settings open the Settings tab.

mod board;
mod layout;
mod mock;
mod model;
mod presets;
#[cfg(test)]
mod tests;
mod view;
mod wire;

use gpui::{deferred, div, prelude::*, AnyElement, AnyWindowHandle, Context, Entity, FocusHandle, FontWeight, KeyDownEvent, Role, SharedString, Subscription, Toggled, Window};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::features::{chosen, FeaturesScreen, Section};
use crate::fleet::FleetConnection;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::new_project::{Edited, Left, NewProjectSheet};
use crate::reviews::ReviewsScreen;
use crate::settings::view::{bone, bounds_of, ring_colour};
use crate::titlebar::parts::{card, glyph, sans, tip, Btn, Weight};
use crate::titlebar;
use board::{Board, BoardEvent, Checkout};
use layout::BlockKind;

/// Asks the librarian box offers when it takes the keys, so a person sees what it could be asked.
const EXAMPLE_ASKS: [&str; 5] = ["Show errors as a graph", "Add a Docs block", "File this link under a feature", "What runs this project?", "Hide Checks on this board"];

/// Where the box that asks the librarian keeps what was typed.
const KEEP_ASK: &str = "project.ask";

/// The five tabs, in the order they are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Dashboard,
    Features,
    Since,
    Who,
    Settings,
}

impl Tab {
    const ALL: [Tab; 5] = [Tab::Dashboard, Tab::Features, Tab::Since, Tab::Who, Tab::Settings];

    fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Features => "Features",
            Tab::Since => "Since you looked",
            Tab::Who => "Who's on what",
            Tab::Settings => "Settings",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Tab::Dashboard => "dashboard",
            Tab::Features => "features",
            Tab::Since => "since",
            Tab::Who => "who",
            Tab::Settings => "settings",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Tab::Dashboard => "layout-dashboard",
            Tab::Features => "library",
            Tab::Since => "eye",
            Tab::Who => "users",
            Tab::Settings => "sliders-horizontal",
        }
    }

    fn section(self) -> Option<Section> {
        match self {
            Tab::Dashboard | Tab::Settings => None,
            Tab::Features => Some(Section::Features),
            Tab::Since => Some(Section::Since),
            Tab::Who => Some(Section::Who),
        }
    }
}

/// The menu beside Start: the checkouts a run can start on, and the row the keys are on.
struct StartMenu {
    choices: Vec<Checkout>,
    at: usize,
    focus: FocusHandle,
}

/// The menu beside Add block: the kinds that are not on the board, and the row the keys are on.
struct AddMenu {
    choices: Vec<BlockKind>,
    at: usize,
    focus: FocusHandle,
}

/// The list of example asks under the librarian box: open while the box has the keys, and the row the arrows are on.
#[derive(Default)]
struct Asks {
    open: bool,
    at: Option<usize>,
}

pub struct ProjectScreen {
    board: Entity<Board>,
    features: Entity<FeaturesScreen>,
    /// The sheet that edits a project, drawn as the Settings tab's page.
    settings: Entity<NewProjectSheet>,
    tab: Tab,
    within: FocusHandle,
    ask: Entity<InputState>,
    asks: Asks,
    menu: Option<StartMenu>,
    adding: Option<AddMenu>,
    window: AnyWindowHandle,
    _subs: Vec<Subscription>,
}

impl ProjectScreen {
    /// On Fleet.
    pub fn live(fleet: &Entity<FleetConnection>, reviews: Option<Entity<ReviewsScreen>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let board = cx.new(|cx| Board::live(fleet, reviews, window, cx));
        let features = cx.new(|cx| FeaturesScreen::live(fleet, None, window, cx));
        let settings = cx.new(|cx| NewProjectSheet::new(crate::new_project::wire::Wire::Live(fleet.clone()), window, cx));
        let mut this = Self::new(board, features, settings, window, cx);
        // The project the page edits is read from what Fleet holds, which may arrive after the tab is opened.
        let watch = cx.observe_in(fleet, window, |this, _, window, cx| this.keep_settings(window, cx));
        this._subs.push(watch);
        this
    }

    /// On the mock fixtures.
    pub fn mock(reviews: Option<Entity<ReviewsScreen>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let board = cx.new(|cx| Board::mock(reviews, window, cx));
        let features = cx.new(|cx| FeaturesScreen::mock(window, cx));
        let mut wire = crate::new_project::wire::Mock::new();
        wire.projects.projects.extend(mock::projects());
        let settings = cx.new(|cx| NewProjectSheet::new(crate::new_project::wire::Wire::Mock(Box::new(wire)), window, cx));
        Self::new(board, features, settings, window, cx)
    }

    /// As [`Self::mock`], with the feature library the Cockpit's floor draws given.
    pub fn mock_holding(reviews: Option<Entity<ReviewsScreen>>, library: ipc::FeatureLibrary, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let board = cx.new(|cx| Board::mock(reviews, window, cx));
        let features = cx.new(|cx| FeaturesScreen::mock_holding(library, window, cx));
        let mut wire = crate::new_project::wire::Mock::new();
        wire.projects.projects.extend(mock::projects());
        let settings = cx.new(|cx| NewProjectSheet::new(crate::new_project::wire::Wire::Mock(Box::new(wire)), window, cx));
        Self::new(board, features, settings, window, cx)
    }

    /// The feature library's screen under the Features tab, which the Cockpit's floor plan reads as well.
    pub fn library_screen(&self) -> Entity<FeaturesScreen> {
        self.features.clone()
    }

    fn new(board: Entity<Board>, features: Entity<FeaturesScreen>, settings: Entity<NewProjectSheet>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let ask = cx.new(|cx| InputState::new(window, cx).placeholder("Ask the librarian"));
        if let Some(words) = crate::drafts::kept(KEEP_ASK, cx) {
            ask.update(cx, |state, cx| state.set_value(words, window, cx));
        }
        let mut subs = vec![cx.subscribe_in(&ask, window, |this, state, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let text = state.read(cx).value().to_string();
                if text.is_empty() {
                    crate::drafts::clear(KEEP_ASK, cx);
                } else {
                    crate::drafts::keep(KEEP_ASK, &text, cx);
                }
            }
            // The examples are offered while the box has the keys, and Enter takes the one the arrows are on.
            InputEvent::Focus => {
                this.asks = Asks { open: true, at: None };
                cx.notify();
            }
            InputEvent::Blur => {
                this.asks = Asks::default();
                cx.notify();
            }
            InputEvent::PressEnter { .. } => {
                if let Some(at) = this.asks.at.filter(|_| this.asks.open) {
                    this.use_ask(at, window, cx);
                }
            }
        })];
        // A save through the page tells the title bar, as the sheet over the window does; leaving it shows the project again.
        subs.push(cx.subscribe(&settings, |_, _, edited: &Edited, cx| {
            if let Some(bar) = titlebar::live(cx) {
                bar.update(cx, |bar, cx| bar.set_projects(&edited.served, &edited.projects, cx));
            }
        }));
        subs.push(cx.subscribe_in(&settings, window, |this, _, _: &Left, window, cx| this.keep_settings(window, cx)));
        subs.push(cx.observe(&settings, |_, _, cx| cx.notify()));
        subs.push(cx.subscribe_in(&board, window, |this, _, event: &BoardEvent, window, cx| match event {
            BoardEvent::Feature(slug) => {
                this.show(Tab::Features, window, cx);
                this.features.update(cx, |features, cx| features.open_feature(slug.clone(), window, cx));
            }
        }));
        // The header and the tabs draw from what the board and the library hold.
        subs.push(cx.observe(&board, |_, _, cx| cx.notify()));
        let this = ProjectScreen { board, features, settings, tab: Tab::Dashboard, within: cx.focus_handle(), ask, asks: Asks::default(), menu: None, adding: None, window: window.window_handle(), _subs: subs };
        this.features.update(cx, |features, cx| features.set_section(Section::Features, window, cx));
        this
    }

    // ---- entering ------------------------------------------------------------------------------

    /// Brought to the front: the tab on screen follows Fleet, and the keys go to it.
    pub fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync_project(cx);
        self.board.update(cx, |board, cx| board.enter(cx));
        // The library is read for the Features tabs from the first, so a feature the board is asked to open is there to open.
        self.features.update(cx, |features, cx| features.enter(window, cx));
        if self.tab == Tab::Dashboard {
            window.focus(&self.board.read(cx).root.clone(), cx);
        }
        self.keep_settings(window, cx);
    }

    pub fn leave(&mut self, cx: &mut Context<Self>) {
        self.menu = None;
        self.adding = None;
        self.board.update(cx, |board, _| board.leave());
        self.features.update(cx, |features, cx| features.leave(cx));
    }

    /// Whether the Features screen under the tabs is in front, following Fleet.
    #[cfg(test)]
    pub fn entered(&self, cx: &gpui::App) -> bool {
        self.features.read(cx).entered()
    }

    /// Why the library was not read, where it was not.
    #[cfg(test)]
    pub fn not_read(&self, cx: &gpui::App) -> Option<String> {
        self.features.read(cx).not_read().map(str::to_string)
    }

    // ---- which project and which tab -------------------------------------------------------------

    /// Both halves show the project the board is on.
    fn sync_project(&mut self, cx: &mut Context<Self>) {
        if let Some(project) = self.board.read(cx).project(cx) {
            self.features.update(cx, |features, cx| {
                if features.project_shown(cx).as_deref() != Some(project.id.as_str()) {
                    features.show_project(project.id.clone(), cx);
                }
            });
        }
    }

    fn choose(&mut self, project: String, window: &mut Window, cx: &mut Context<Self>) {
        chosen::set(&project, cx);
        self.board.update(cx, |board, cx| board.set_project(project.clone(), cx));
        self.features.update(cx, |features, cx| features.show_project(project, cx));
        self.keep_settings(window, cx);
        cx.notify();
    }

    fn show(&mut self, tab: Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.menu = None;
        self.adding = None;
        if self.tab == tab {
            return;
        }
        self.tab = tab;
        match tab {
            Tab::Settings => self.keep_settings(window, cx),
            Tab::Dashboard => window.focus(&self.board.read(cx).root.clone(), cx),
            _ => {}
        }
        if let Some(section) = tab.section() {
            self.features.update(cx, |features, cx| {
                features.set_section(section, window, cx);
                features.enter(window, cx);
            });
        }
        cx.notify();
    }

    /// The header's Members and Settings: the Settings tab opens, and Members has the keys in the list of members.
    fn open_settings(&mut self, members: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.show(Tab::Settings, window, cx);
        if let Some(list) = members.then(|| self.settings.read(cx).members_handle()).flatten() {
            window.focus(&list, cx);
        }
    }

    /// While the Settings tab is in front the sheet is open on the project on screen, read from what Fleet holds: it is opened
    /// again when the project changes, or when the project was not yet held, and left alone while it is being edited.
    fn keep_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab != Tab::Settings {
            return;
        }
        let Some(project) = self.board.read(cx).project(cx) else { return };
        if self.settings.read(cx).inline_on() != Some(project.id.as_str()) {
            self.settings.update(cx, |sheet, cx| sheet.show_inline(&project.id, window, cx));
        }
    }

    // ---- the librarian box's examples ----------------------------------------------------------

    /// A press or Enter on an example: it is the box's words, and the box keeps the keys.
    fn use_ask(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = EXAMPLE_ASKS.get(at) else { return };
        self.asks = Asks::default();
        self.ask.update(cx, |state, cx| state.set_value(*text, window, cx));
        crate::drafts::keep(KEEP_ASK, text, cx);
        cx.notify();
    }

    /// ↓ opens the examples where they are shut, and ↑ ↓ move along them; Esc puts them away with the keys still in the box.
    fn on_ask_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let last = EXAMPLE_ASKS.len() - 1;
        match (event.keystroke.key.as_str(), self.asks.open) {
            ("down", false) => self.asks = Asks { open: true, at: Some(0) },
            ("down", true) => self.asks.at = Some(self.asks.at.map_or(0, |at| (at + 1).min(last))),
            ("up", true) => self.asks.at = self.asks.at.and_then(|at| at.checked_sub(1)),
            ("escape", true) => self.asks = Asks::default(),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    // ---- the Start menu ------------------------------------------------------------------------

    fn open_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.board.read(cx).project(cx) else { return };
        let choices = self.board.read(cx).checkouts(&project, cx);
        let now = self.board.read(cx).checkout.clone();
        let at = choices.iter().position(|one| *one == now).unwrap_or(0);
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.menu = Some(StartMenu { choices, at, focus });
        cx.notify();
    }

    fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.take().is_some() {
            if self.tab == Tab::Dashboard {
                window.focus(&self.board.read(cx).root.clone(), cx);
            }
            cx.notify();
        }
    }

    fn pick_checkout(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(choice) = self.menu.as_ref().and_then(|menu| menu.choices.get(at).cloned()) else { return };
        self.close_menu(window, cx);
        self.board.update(cx, |board, cx| board.use_checkout(choice, cx));
    }

    // ---- the Add block menu --------------------------------------------------------------------

    fn open_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.board.read(cx).project(cx) else { return };
        let choices = self.board.update(cx, |board, _| board.layout(&project.id).missing());
        if choices.is_empty() {
            return;
        }
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        self.adding = Some(AddMenu { choices, at: 0, focus });
        cx.notify();
    }

    fn close_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.adding.take().is_some() {
            if self.tab == Tab::Dashboard {
                window.focus(&self.board.read(cx).root.clone(), cx);
            }
            cx.notify();
        }
    }

    /// The block goes on the board in the first free cells, and the keys are on the board.
    fn pick_block(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(kind) = self.adding.as_ref().and_then(|menu| menu.choices.get(at).copied()) else { return };
        self.close_add(window, cx);
        self.board.update(cx, |board, cx| board.add_block(kind, cx));
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = self.adding.as_mut() {
            match event.keystroke.key.as_str() {
                "escape" => self.close_add(window, cx),
                "down" => menu.at = (menu.at + 1).min(menu.choices.len().saturating_sub(1)),
                "up" => menu.at = menu.at.saturating_sub(1),
                "enter" => {
                    let at = menu.at;
                    self.pick_block(at, window, cx);
                }
                _ => return,
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if let Some(menu) = self.menu.as_mut() {
            match event.keystroke.key.as_str() {
                "escape" => self.close_menu(window, cx),
                "down" => menu.at = (menu.at + 1).min(menu.choices.len().saturating_sub(1)),
                "up" => menu.at = menu.at.saturating_sub(1),
                "enter" => {
                    let at = menu.at;
                    self.pick_checkout(at, window, cx);
                }
                _ => return,
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if crate::keymap::traverse(event, window, cx) {
            cx.stop_propagation();
        }
    }

    // ---- drawing -------------------------------------------------------------------------------

    fn chips(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let chips = self.board.read(cx).chips(cx);
        let shown = self.board.read(cx).project(cx).map(|project| project.id);
        let mut row = div().id("project-chips").flex().flex_wrap().items_center().gap(look.length(token::SPACE_2));
        for (id, name) in chips {
            let on = shown.as_deref() == Some(id.as_str());
            let press = id.clone();
            let pill = div()
                .id(SharedString::from(format!("project-chip-{id}")))
                .flex_none()
                .flex()
                .items_center()
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::H_CONTROL_SM) / 2.)
                .border(look.length(token::FOCUS_RING_WIDTH))
                .border_color(if on { look.color(token::ACCENT) } else { gpui::transparent_black() })
                .bg(if on { look.color(token::ACCENT_MUTED) } else { look.color(token::BG_HOVER) })
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(if on { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) })
                .whitespace_nowrap()
                .cursor_pointer()
                .focusable()
                .tab_stop(true)
                .when(!on, |pill| pill.hover(|style| style.bg(look.color(token::FG_DEFAULT).opacity(0.14))))
                .aria_toggled(if on { Toggled::True } else { Toggled::False })
                .on_click(cx.listener(move |this, _, window, cx| this.choose(press.clone(), window, cx)))
                .child(SharedString::from(name.clone()));
            row = row.child(labelled(pill, Role::Button, name));
        }
        row.into_any_element()
    }

    /// The box that asks the librarian. It is drawn and keeps what is typed; nothing sends it yet, and its tooltip says so. While
    /// it has the keys the examples hang under it.
    fn ask_box(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let rows = EXAMPLE_ASKS.iter().enumerate().map(|(at, text)| {
            let on = self.asks.at == Some(at);
            labelled(
                div()
                    .id(SharedString::from(format!("project-ask-example-{at}")))
                    .flex()
                    .items_center()
                    .h(look.length(token::H_ROW_ACTION))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .when(on, |row| row.bg(look.color(token::BG_HOVER)))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focus_visible(move |style| style.border_color(ring))
                    // A press keeps the keys in the box, which a press on anything else would take.
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
                        cx.stop_propagation();
                        window.prevent_default();
                    })
                    .on_click(cx.listener(move |this, _, window, cx| this.use_ask(at, window, cx)))
                    .child(div().min_w_0().truncate().child(*text)),
                Role::MenuItem,
                *text,
            )
        });
        let examples = self.asks.open.then(|| {
            deferred(
                labelled(
                    card(look).id("project-ask-examples").absolute().top_full().mt(look.length(token::SPACE_1)).left_0().w_full().p(look.length(token::SPACE_1)).children(rows),
                    Role::Menu,
                    "Example asks",
                ),
            )
            .with_priority(2)
        });
        div()
            .relative()
            .flex_none()
            .child(labelled(
                div()
                    .id("project-ask")
                    .w(look.length(token::SPACE_12) * 4.0)
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_RAISED))
                    .tooltip(tip("Asking the librarian is not connected yet"))
                    .capture_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| this.on_ask_key(event, cx)))
                    .child(glyph("sparkles", look.length(token::SPACE_4), look.color(token::ASK_FEATURES)))
                    .child(div().flex_1().min_w_0().key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_XS)).child(Input::new(&self.ask).appearance(false))),
                Role::TextInput,
                "Ask the librarian",
            ))
            .children(examples)
            .into_any_element()
    }

    fn header(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let editing = self.board.read(cx).edit;
        let on_board = self.tab == Tab::Dashboard;
        let started = self.board.read(cx).starting();
        let free = self.board.update(cx, |board, cx| board.project(cx).map_or(0, |project| board.layout(&project.id).missing().len()));
        let button = |id: &'static str, text: &'static str, mark: &'static str, weight: Weight, said: &'static str| {
            Btn::new(id, text, weight).icon(mark).draw(look, cx).tooltip(tip(said))
        };
        let edit = labelled(
            button("project-edit", if editing { "Done" } else { "Edit" }, if editing { "check" } else { "pencil" }, Weight::Secondary, if editing { "Stop moving and resizing blocks" } else { "Move and resize the blocks of the board" })
                .on_click(cx.listener(|this, _, _, cx| this.board.update(cx, |board, cx| board.toggle_edit(cx)))),
            Role::Button,
            if editing { "Done" } else { "Edit" },
        );
        let add = labelled(
            Btn::new("project-add-block", "Add block", Weight::Secondary)
                .icon("plus")
                .disabled(free == 0)
                .draw(look, cx)
                .tooltip(tip("Put a block that is not on the board back on it"))
                .when(free > 0, |button| button.on_click(cx.listener(|this, _, window, cx| this.open_add(window, cx)))),
            Role::Button,
            "Add block",
        );
        let members = labelled(
            button("project-members", "Members", "users", Weight::Secondary, "Change which repositories belong to the project").on_click(cx.listener(|this, _, window, cx| this.open_settings(true, window, cx))),
            Role::Button,
            "Members",
        );
        let settings = labelled(
            button("project-settings", "Settings", "sliders-horizontal", Weight::Secondary, "Open the settings of the project").on_click(cx.listener(|this, _, window, cx| this.open_settings(false, window, cx))),
            Role::Button,
            "Settings",
        );
        let start = labelled(
            Btn::new("project-start", "Start", Weight::Primary)
                .icon("play")
                .pending(started)
                .draw(look, cx)
                .tooltip(tip("Start the run on the checkout beside it"))
                .when(!started, |button| button.on_click(cx.listener(|this, _, _, cx| this.board.update(cx, |board, cx| board.start(cx))))),
            Role::Button,
            "Start",
        );
        let checkout = self.board.read(cx).checkout.clone();
        let named = match &checkout {
            Checkout::Main => "Main".to_string(),
            Checkout::Job { label, .. } => label.clone(),
        };
        let menu_button = labelled(
            Btn::new("project-checkout", named.clone(), Weight::Secondary)
                .icon("git-branch")
                .draw(look, cx)
                .tooltip(tip("Choose the checkout Start runs on"))
                .on_click(cx.listener(|this, _, window, cx| this.open_menu(window, cx))),
            Role::Button,
            format!("Checkout, {named}"),
        );
        div()
            .id("project-header")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .pt(look.length(token::SPACE_3))
            .child(div().flex_1().min_w_0().child(self.chips(look, cx)))
            .child(self.ask_box(look, cx))
            .when(on_board && editing, |row| row.child(add))
            .when(on_board, |row| row.child(edit))
            .child(members)
            .child(settings)
            .child(menu_button)
            .child(start)
            .into_any_element()
    }

    /// The tabs: each a filled pill when it is the one on screen, with no line under it.
    fn tabs(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let ring = ring_colour(look);
        let mut row = div().id("project-tabs").flex_none().flex().items_center().gap(look.length(token::SPACE_1)).px(look.length(token::SPACE_3)).pt(look.length(token::SPACE_3));
        for tab in Tab::ALL {
            let on = tab == self.tab;
            let pill = div()
                .id(SharedString::from(format!("project-tab-{}", tab.slug())))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .text_size(look.length(token::TEXT_SM))
                .whitespace_nowrap()
                .when(on, |pill| pill.bg(look.color(token::BG_RAISED)).shadow(look.shadows(token::SHADOW_CARD)).text_color(look.color(token::FG_DEFAULT)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                .when(!on, |pill| pill.text_color(look.color(token::FG_MUTED)).cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))))
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .aria_toggled(if on { Toggled::True } else { Toggled::False })
                .on_click(cx.listener(move |this, _, window, cx| this.show(tab, window, cx)))
                .child(glyph(tab.glyph(), look.length(token::SPACE_4), if on { look.color(token::ACCENT) } else { look.color(token::FG_SUBTLE) }))
                .child(tab.title());
            row = row.child(labelled(pill, Role::Tab, tab.title()));
        }
        labelled(row, Role::TabList, "Project").into_any_element()
    }

    /// The Start menu, hung under the header's right edge.
    fn menu(&self, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.menu.as_ref()?;
        let ring = ring_colour(look);
        let rows = menu.choices.iter().enumerate().map(|(at, choice)| {
            let on = at == menu.at;
            let said = match choice {
                Checkout::Main => "Main".to_string(),
                Checkout::Job { label, .. } => label.clone(),
            };
            labelled(
                div()
                    .id(SharedString::from(format!("project-checkout-{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_ROW_ACTION))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .when(on, |row| row.bg(look.color(token::BG_HOVER)))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focus_visible(move |style| style.border_color(ring))
                    .on_click(cx.listener(move |this, _, window, cx| this.pick_checkout(at, window, cx)))
                    .child(glyph(if matches!(choice, Checkout::Main) { "house" } else { "git-branch" }, look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                    .child(div().min_w_0().truncate().child(SharedString::from(said.clone()))),
                Role::MenuItem,
                said,
            )
        });
        Some(
            labelled(
                card(look)
                    .id("project-checkout-menu")
                    .track_focus(&menu.focus)
                    .absolute()
                    .top(look.length(token::SPACE_3) + look.length(token::H_CONTROL) + look.length(token::SPACE_1))
                    .right(look.length(token::SPACE_3))
                    .w(look.length(token::SPACE_12) * 4.0)
                    .p(look.length(token::SPACE_1))
                    .children(rows),
                Role::Menu,
                "Checkouts",
            )
            .into_any_element(),
        )
    }
}

impl ProjectScreen {
    /// The menu of kinds not on the board, hung under the Add block button.
    fn add_menu(&self, look: &Look, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.adding.as_ref()?;
        let ring = ring_colour(look);
        let from = bounds_of(window, cx, Role::Button, "Add block").map(|bounds| bounds.bottom_left()).unwrap_or_default();
        let rows = menu.choices.iter().enumerate().map(|(at, kind)| {
            let on = at == menu.at;
            labelled(
                div()
                    .id(SharedString::from(format!("project-add-{}", kind.slug())))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_ROW_ACTION))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .when(on, |row| row.bg(look.color(token::BG_HOVER)))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .focus_visible(move |style| style.border_color(ring))
                    .on_click(cx.listener(move |this, _, window, cx| this.pick_block(at, window, cx)))
                    .child(glyph(view::mark_of(*kind), look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                    .child(div().min_w_0().truncate().child(kind.title())),
                Role::MenuItem,
                kind.title(),
            )
        });
        Some(
            deferred(
                gpui::anchored().position(from).anchor(gpui::Anchor::TopLeft).child(
                    labelled(
                        card(look).id("project-add-menu").track_focus(&menu.focus).mt(look.length(token::SPACE_1)).w(look.length(token::SPACE_12) * 4.0).p(look.length(token::SPACE_1)).children(rows),
                        Role::Menu,
                        "Blocks to add",
                    ),
                ),
            )
            .with_priority(3)
            .into_any_element(),
        )
    }

    /// The Settings tab: the board's options over the sheet that edits the project, and bones while Fleet has not said which.
    fn settings_page(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let project = self.board.read(cx).project(cx);
        let shown = project.as_ref().is_some_and(|project| self.settings.read(cx).inline_on() == Some(project.id.as_str()));
        let reset = labelled(
            Btn::new("project-reset-board", "Reset board", Weight::Secondary)
                .icon("rotate-ccw")
                .draw(look, cx)
                .tooltip(tip("Put the blocks back where the preset for this project has them"))
                .on_click(cx.listener(|this, _, _, cx| this.board.update(cx, |board, cx| board.reset_layout(cx)))),
            Role::Button,
            "Reset board",
        );
        let options = div()
            .id("project-board-options")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(glyph("layout-dashboard", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
            .child(div().flex_1().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Board"))
            .child(reset);
        let page: AnyElement = if shown {
            self.settings.clone().into_any_element()
        } else {
            let bones = (0..4).map(|at| bone(look, look.length(token::SPACE_12) * (5.0 - at as f32), look.length(token::SPACE_3)));
            labelled(div().id("project-settings-loading").flex().flex_col().gap(look.length(token::SPACE_2)).children(bones), Role::Image, "Loading").into_any_element()
        };
        div()
            .id("project-settings-page")
            .size_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .pb(look.length(token::SPACE_3))
            .child(options)
            .child(div().flex_1().min_h_0().child(page))
            .into_any_element()
    }
}

impl Render for ProjectScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let body: AnyElement = match self.tab {
            Tab::Dashboard => self.board.clone().into_any_element(),
            Tab::Settings => self.settings_page(&look, cx),
            _ => self.features.clone().into_any_element(),
        };
        let _ = self.window;
        div()
            .id("project")
            .track_focus(&self.within)
            .on_key_down(cx.listener(Self::on_key))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(self.header(&look, cx))
            .child(self.tabs(&look, cx))
            .child(div().flex_1().min_h_0().pt(look.length(token::SPACE_3)).child(body))
            .children(self.menu(&look, cx))
            .children(self.add_menu(&look, window, cx))
    }
}
