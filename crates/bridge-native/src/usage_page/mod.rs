//! The Usage page: how much of the burn was waste a person can fix, the settings that change it, what each change did, and where
//! the waste was, kind by kind, largest first. A kind opens a panel under the list with its fix and the Jobs it hit.
//!
//! It reads the window the switch names from Fleet's `GET /usage/waste` and `GET /usage/adjustments` (`wire.rs`): on open, when the
//! window changes, after a change is saved, and every five minutes while the page is shown, which is as long as Fleet holds either.
//! A setting is changed on the page by the Settings overlay's own save. `model.rs` works out what the page draws from the reading and
//! nothing there touches the screen; the mock scenarios (`mock.rs`) stand in for Fleet in walks. Like Kit and Features it
//! is a place of the Operator's screen (`operator/screen.rs`), reached by name from the palette: Classic has no tab for it.

mod mock;
pub mod model;
#[cfg(test)]
mod tests;
mod view;
pub mod walk;
mod wire;

use std::collections::HashMap;
use std::time::Duration;

use gpui::{prelude::*, App, Context, DismissEvent, Entity, FocusHandle, Focusable, Global, KeyDownEvent, ListAlignment, ListState, Role, ScrollHandle, Subscription, Task, Window};
use ipc::SettingValue;

use crate::dropdown::{Dropdown, Entry, Picked};
use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::settings::overlay::AT;
use crate::settings::view::bounds_of;
use model::{Board, Changing, Held, JobLine, Selection, Span};

/// How often a page that is shown reads Fleet again: Fleet holds each reading for this long.
const EVERY: Duration = Duration::from_secs(5 * 60);

/// Set once a Usage page exists in the app, so the palette offers the row that brings it up and no other build draws one that does nothing.
pub struct Mounted;

impl Global for Mounted {}

/// Where the reading comes from.
enum Source {
    Live(Entity<FleetConnection>),
    /// The mock scenario the reading is made from.
    Mock(&'static str),
}

impl Source {
    fn fleet(&self) -> Option<Entity<FleetConnection>> {
        match self {
            Source::Live(fleet) => Some(fleet.clone()),
            Source::Mock(_) => None,
        }
    }
}

pub struct UsageScreen {
    pub within: FocusHandle,
    /// The list takes the keys.
    keys: FocusHandle,
    source: Source,
    held: Held,
    /// The settings that change the burn, what each change did, and what the person has done to a row.
    changing: Changing,
    selection: Selection,
    /// The page's scroll: its panels are the items, so the row the keys are on can be brought into view.
    scroll: ScrollHandle,
    /// The Jobs' titles: the page's own, and those the Jobs list holds.
    titles: HashMap<String, String>,
    /// The repositories' roots, which a path in the Cause column is drawn from.
    roots: Vec<String>,
    /// The table of the open kind's Jobs, and the list that draws it.
    lines: Vec<JobLine>,
    jobs: ListState,
    /// The page is in front, so it follows Fleet.
    entered: bool,
    /// The port the last read was made on, so a Fleet that came back is read again.
    read_port: Option<u16>,
    /// The reads in flight: a new read drops them.
    work: Option<Task<()>>,
    adjusting: Option<Task<()>>,
    /// The changes on their way to Fleet, a task a setting.
    saving: HashMap<String, Task<()>>,
    /// The list of values a setting's dropdown opened.
    popup: Option<(Entity<Dropdown>, Vec<Subscription>)>,
    /// What a mock Fleet has been told to change on this page.
    mock_saved: Vec<(String, SettingValue)>,
    /// The five minute clock, alive while the page is in front.
    clock: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl UsageScreen {
    /// On Fleet.
    pub fn live(fleet: &Entity<FleetConnection>, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Source::Live(fleet.clone()), cx);
        let watch = cx.observe(fleet, |this, _, cx| this.moved(cx));
        this._subs.push(watch);
        this
    }

    /// On the mock reading of the scenario last named, the week's unless it names another.
    pub fn mock(cx: &mut Context<Self>) -> Self {
        let scenario = [mock::LOADING, mock::EMPTY, mock::FAILED, mock::APPLY_FAILED].into_iter().find(|name| crate::mock::scenario_is(name)).unwrap_or(mock::WEEK);
        let mut this = Self::new(Source::Mock(scenario), cx);
        this.titles = mock::titles();
        this.roots = mock::roots();
        this.read(cx);
        this
    }

    fn new(source: Source, cx: &mut Context<Self>) -> Self {
        cx.set_global(Mounted);
        UsageScreen {
            within: cx.focus_handle(),
            keys: cx.focus_handle(),
            source,
            held: Held::new(Span::Week),
            changing: Changing::default(),
            selection: Selection::default(),
            scroll: ScrollHandle::new(),
            titles: HashMap::new(),
            roots: Vec::new(),
            lines: Vec::new(),
            jobs: ListState::new(0, ListAlignment::Top, gpui::px(200.)),
            entered: false,
            read_port: None,
            work: None,
            adjusting: None,
            saving: HashMap::new(),
            popup: None,
            mock_saved: Vec::new(),
            clock: None,
            _subs: Vec::new(),
        }
    }

    // ---- reading -----------------------------------------------------------------------------

    /// The connection came back on another port: read again if the page is in front.
    fn moved(&mut self, cx: &mut Context<Self>) {
        let Some(fleet) = self.source.fleet() else { return };
        if self.take_roots(cx) {
            self.settle(cx);
        }
        if self.entered && fleet.read(cx).held().port() != self.read_port {
            self.read(cx);
        }
    }

    /// The roots of the repositories Fleet serves. True where they changed.
    fn take_roots(&mut self, cx: &App) -> bool {
        let Some(fleet) = self.source.fleet() else { return false };
        let roots: Vec<String> = fleet.read(cx).held().repositories.as_deref().unwrap_or_default().iter().map(|repository| repository.root.clone()).collect();
        let changed = roots != self.roots;
        self.roots = roots;
        changed
    }

    /// Ask for the span now held. What is drawn stays until the answer comes.
    fn read(&mut self, cx: &mut Context<Self>) {
        let span = self.held.span();
        match &self.source {
            Source::Mock(scenario) => {
                match *scenario {
                    mock::LOADING => {}
                    mock::EMPTY => {
                        self.held.arrived(span, Ok(mock::empty(span)));
                        self.changing.arrived(span, span, Ok(mock::no_adjustments()));
                    }
                    mock::FAILED => self.held.arrived(span, Err(mock::REFUSED.into())),
                    _ => {
                        self.held.arrived(span, Ok(mock::waste(span)));
                        self.changing.arrived(span, span, Ok(mock::adjustments(&self.mock_saved)));
                    }
                }
                self.settle(cx);
            }
            Source::Live(fleet) => {
                let fleet = fleet.clone();
                self.take_roots(cx);
                let Some(port) = fleet.read(cx).held().port() else {
                    self.read_port = None;
                    self.work = None;
                    self.adjusting = None;
                    self.held.arrived(span, Err("Fleet is not connected".into()));
                    self.settle(cx);
                    return;
                };
                if self.read_port.is_some_and(|before| before != port) {
                    self.changing.restarted();
                }
                self.read_port = Some(port);
                let ask = wire::waste(port, span, cx);
                let adjust = wire::adjustments(port, span, cx);
                self.adjusting = Some(cx.spawn(async move |this, cx| {
                    let answer = adjust.await.map_err(|why| why.said);
                    let _ = this.update(cx, |this, cx| {
                        this.changing.arrived(span, this.held.span(), answer);
                        this.settle(cx);
                    });
                }));
                self.work = Some(cx.spawn(async move |this, cx| {
                    let answer = ask.await.map_err(|why| why.said);
                    let _ = this.update(cx, |this, cx| {
                        this.held.arrived(span, answer);
                        this.settle(cx);
                    });
                }));
                cx.notify();
            }
        }
    }

    /// What is held changed: the selection holds only kinds that are there, and the table is made again.
    fn settle(&mut self, cx: &mut Context<Self>) {
        let kinds = self.kinds_now();
        let kinds: Vec<&str> = kinds.iter().map(String::as_str).collect();
        // The kind kept open is the one the panel is drawn for, so its lines come first and the keys' stops after them.
        let opened = self.selection.opened.clone().filter(|opened| kinds.contains(&opened.as_str()));
        self.lines = match (self.board(), opened.as_deref()) {
            (Some(board), Some(kind)) => board.lines_of(kind, &self.titles, &self.roots),
            _ => Vec::new(),
        };
        let stops = self.stops_now();
        let stops: Vec<&str> = stops.iter().map(String::as_str).collect();
        self.selection.keep(&stops, &kinds);
        self.jobs.reset(self.lines.len());
        cx.notify();
    }

    fn board(&self) -> Option<&Board> {
        self.held.board()
    }

    /// The titles of the Jobs the app holds, kept beside the page's own.
    pub fn set_titles(&mut self, titles: &HashMap<String, String>, cx: &mut Context<Self>) {
        if !titles.iter().any(|(id, title)| self.titles.get(id) != Some(title)) {
            return;
        }
        self.titles.extend(titles.iter().map(|(id, title)| (id.clone(), title.clone())));
        self.settle(cx);
    }

    // ---- what the keys do ------------------------------------------------------------------

    fn kinds_now(&self) -> Vec<String> {
        self.board().map(|board| board.kinds().into_iter().map(str::to_string).collect()).unwrap_or_default()
    }

    /// The rows the keys move between, in the order they are drawn: the settings, the kinds, then the Jobs of the kind that is open.
    /// Nothing is drawn to move between until a reading is.
    fn stops_now(&self) -> Vec<String> {
        if self.board().is_none() {
            return Vec::new();
        }
        self.changing.keys().into_iter().map(str::to_string).chain(self.kinds_now()).chain(self.lines.iter().map(|line| model::job_key(&line.id))).collect()
    }

    fn is_setting(&self, row: &str) -> bool {
        self.board().is_some() && self.changing.keys().contains(&row)
    }

    /// J and K, Down and Up.
    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let stops = self.stops_now();
        let stops: Vec<&str> = stops.iter().map(String::as_str).collect();
        self.selection.step(&stops, by);
        self.reveal();
        cx.notify();
    }

    /// Scroll the panel the keys are in into view. The panels are the scroll's items: settings, changes where there are any, then the kinds.
    fn reveal(&self) {
        let Some(cursor) = self.selection.cursor.as_deref() else { return };
        if let Some(ix) = self.lines.iter().position(|line| model::job_key(&line.id) == cursor) {
            self.scroll.scroll_to_bottom();
            self.jobs.scroll_to_reveal_item(ix);
            return;
        }
        let has_settings = !self.changing.keys().is_empty();
        let has_changes = !self.changing.changes(None).is_empty();
        let ix = if self.is_setting(cursor) { 0 } else { usize::from(has_settings) + usize::from(has_changes) };
        self.scroll.scroll_to_item(ix);
    }

    /// Enter, Space or a press on a row: a kind's panel opens, or shuts where it is open; a setting's list of values opens.
    fn open(&mut self, row: Option<String>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = row.or_else(|| self.selection.cursor.clone()) else { return };
        // A Job's row opens that Job's Tokens tab.
        if let Some(job) = model::job_of_key(&row) {
            self.selection.cursor = Some(row.clone());
            crate::job_tokens::open(job, cx);
            cx.notify();
            return;
        }
        if self.is_setting(&row) {
            self.selection.cursor = Some(row.clone());
            self.open_menu(row, from_keys, window, cx);
            return;
        }
        self.selection.toggle(&row);
        self.settle(cx);
        if self.selection.opened.is_some() {
            self.scroll.scroll_to_bottom();
        }
    }

    /// A setting's values in a list under its dropdown, as the Settings overlay opens a choice. Picking one moves the dropdown to it
    /// and nothing is sent until Apply.
    fn open_menu(&mut self, key: String, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.changing.rows(self.board());
        let Some(row) = rows.iter().find(|row| row.key == key) else { return };
        let name = view::dropdown_name(&row.label);
        let entries = self.changing.menu(&key).into_iter().enumerate().take(AT.len()).map(|(at, (label, on))| Entry::Choice { id: AT[at], label: label.into(), on }).collect();
        let at = bounds_of(window, cx, Role::ComboBox, &name).map(|bounds| bounds.bottom_left()).unwrap_or_default();
        let menu = cx.new(|cx| Dropdown::new(name, entries, at, gpui::Anchor::TopLeft, from_keys, cx));
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, _, cx| {
            let Picked::Entry { id, .. } = picked;
            if let Some(at) = AT.iter().position(|one| one == id) {
                this.changing.choose(&key, at);
                cx.notify();
            }
        });
        let dismissed = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            this.popup = None;
            window.focus(&this.keys, cx);
            cx.notify();
        });
        let keys = menu.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.popup = Some((menu, vec![picked, dismissed]));
        cx.notify();
    }

    /// A press on a setting's row, off its controls: the keys go there.
    fn rest_on(&mut self, key: String, cx: &mut Context<Self>) {
        self.selection.cursor = Some(key);
        cx.notify();
    }

    /// `a` or a press on Apply: the value the dropdown stands on goes to Fleet, and the row shows it pending in this frame.
    fn apply(&mut self, key: Option<String>, cx: &mut Context<Self>) {
        let Some(key) = key.or_else(|| self.selection.cursor.clone()).filter(|key| self.is_setting(key)) else { return };
        let Some(value) = self.changing.apply(&key) else { return };
        self.selection.cursor = Some(key.clone());
        cx.notify();
        match &self.source {
            Source::Mock(scenario) => {
                let answer = if *scenario == mock::APPLY_FAILED { Err(mock::REFUSED_SAVE.to_string()) } else { Ok(key == mock::QUIET) };
                let task = cx.spawn({
                    let key = key.clone();
                    async move |this, cx| {
                        let _ = this.update(cx, |this, cx| this.applied(&key, value, answer, cx));
                    }
                });
                self.saving.insert(key, task);
            }
            Source::Live(fleet) => {
                let fleet = fleet.clone();
                if fleet.read(cx).held().port().is_none() {
                    self.changing.refused(&key, "Fleet is not connected".into());
                    cx.notify();
                    return;
                }
                let ask = wire::save(&fleet, &key, value.clone(), cx);
                let task = cx.spawn({
                    let key = key.clone();
                    async move |this, cx| {
                        let answer = ask.await.map(|list| list.settings.iter().any(|one| one.key == key && one.pending_restart)).map_err(|why| why.said);
                        let _ = this.update(cx, |this, cx| this.applied(&key, value, answer, cx));
                    }
                });
                self.saving.insert(key, task);
            }
        }
    }

    /// Fleet's answer to a change. Taken: the page reads the adjustments and the waste again. Refused: the old value is back and the row says why.
    fn applied(&mut self, key: &str, value: SettingValue, answer: Result<bool, String>, cx: &mut Context<Self>) {
        match answer {
            Ok(restart) => {
                self.changing.saved(key, restart);
                if matches!(self.source, Source::Mock(_)) {
                    self.mock_saved.push((key.to_string(), value));
                }
                self.read(cx);
            }
            Err(why) => {
                self.changing.refused(key, why);
                cx.notify();
            }
        }
    }

    /// Esc: the open panel shuts. False where none was open, so the key goes on.
    fn close(&mut self, cx: &mut Context<Self>) -> bool {
        let closed = self.selection.close();
        if closed {
            self.settle(cx);
        }
        closed
    }

    /// A window picked: the switch moves now, the rows stay until Fleet answers, and the panel is kept where its kind is still there.
    /// A press on the window already shown, after a failed read, asks again.
    fn pick(&mut self, span: Span, cx: &mut Context<Self>) {
        let moved = self.held.pick(span);
        if moved || self.held.failed() {
            cx.notify();
            self.read(cx);
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" && self.close(cx) {
            cx.stop_propagation();
        }
    }

    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        match act.id {
            "usage_move" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "usage_open" => self.open(None, true, window, cx),
            "usage_apply" => self.apply(None, cx),
            "usage_window" => match Span::at(act.slot) {
                Some(span) => self.pick(span, cx),
                None => cx.propagate(),
            },
            _ => cx.propagate(),
        }
    }

    // ---- entering ----------------------------------------------------------------------------

    /// Brought to the front: the keys go to the list, Fleet is read, and again every five minutes while the page is shown.
    pub fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.keys, cx);
        self.entered = true;
        if matches!(self.source, Source::Live(_)) {
            self.clock = Some(cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(EVERY).await;
                if this.update(cx, |this, cx| this.read(cx)).is_err() {
                    return;
                }
            }));
            self.read(cx);
        }
        cx.notify();
    }

    /// Another place in front: the page stops reading until it is entered again.
    pub fn leave(&mut self, _cx: &mut Context<Self>) {
        self.entered = false;
        self.clock = None;
        self.work = None;
        self.adjusting = None;
        self.popup = None;
    }
}

/// Whether the app holds a Usage page, so the palette can offer it.
pub fn offered(cx: &App) -> bool {
    cx.has_global::<Mounted>()
}

/// Whether a mock scenario of this name opens on the Usage page.
pub fn is_scenario(name: &str) -> bool {
    mock::is_usage(name)
}
