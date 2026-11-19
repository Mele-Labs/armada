//! The Usage page: where the burn was wasted, kind by kind, largest first. A kind opens a panel under the list with its
//! fix and the Jobs it hit.
//!
//! It leads with waste, over the window the switch names, read from Fleet's `GET /usage/waste` (`wire.rs`): on open, when the
//! window changes, and every minute while the page is shown. `model.rs` works out what the page draws from the reading and
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

use gpui::{prelude::*, App, Context, Entity, FocusHandle, Global, KeyDownEvent, ListAlignment, ListState, Subscription, Task, Window};

use crate::fleet::FleetConnection;
use crate::keymap::Act;
use model::{Board, Held, JobLine, Selection, Span};

/// How often a page that is shown reads Fleet again.
const EVERY: Duration = Duration::from_secs(60);

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
    selection: Selection,
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
    /// The read in flight: a new read drops it.
    work: Option<Task<()>>,
    /// The minute clock, alive while the page is in front.
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
        let scenario = [mock::LOADING, mock::EMPTY, mock::FAILED].into_iter().find(|name| crate::mock::scenario_is(name)).unwrap_or(mock::WEEK);
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
            selection: Selection::default(),
            titles: HashMap::new(),
            roots: Vec::new(),
            lines: Vec::new(),
            jobs: ListState::new(0, ListAlignment::Top, gpui::px(200.)),
            entered: false,
            read_port: None,
            work: None,
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
                    mock::EMPTY => self.held.arrived(span, Ok(mock::empty(span))),
                    mock::FAILED => self.held.arrived(span, Err(mock::REFUSED.into())),
                    _ => self.held.arrived(span, Ok(mock::waste(span))),
                }
                self.settle(cx);
            }
            Source::Live(fleet) => {
                let fleet = fleet.clone();
                self.take_roots(cx);
                let Some(port) = fleet.read(cx).held().port() else {
                    self.read_port = None;
                    self.work = None;
                    self.held.arrived(span, Err("Fleet is not connected".into()));
                    self.settle(cx);
                    return;
                };
                self.read_port = Some(port);
                let ask = wire::waste(port, span, cx);
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
        let kinds: Vec<String> = self.board().map(|board| board.kinds().into_iter().map(str::to_string).collect()).unwrap_or_default();
        let kinds: Vec<&str> = kinds.iter().map(String::as_str).collect();
        self.selection.keep(&kinds);
        self.lines = match (self.board(), self.selection.opened.as_deref()) {
            (Some(board), Some(kind)) => board.lines_of(kind, &self.titles, &self.roots),
            _ => Vec::new(),
        };
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

    /// J and K, Down and Up.
    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let kinds = self.kinds_now();
        let kinds: Vec<&str> = kinds.iter().map(String::as_str).collect();
        self.selection.step(&kinds, by);
        cx.notify();
    }

    /// Enter, Space or a press on a row: its panel opens, or shuts where it is open.
    fn open(&mut self, kind: Option<String>, cx: &mut Context<Self>) {
        let Some(kind) = kind.or_else(|| self.selection.cursor.clone()) else { return };
        self.selection.toggle(&kind);
        self.settle(cx);
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

    fn on_act(&mut self, act: &Act, _window: &mut Window, cx: &mut Context<Self>) {
        match act.id {
            "usage_move" => self.step(if act.slot % 2 == 0 { 1 } else { -1 }, cx),
            "usage_open" => self.open(None, cx),
            "usage_window" => match Span::at(act.slot) {
                Some(span) => self.pick(span, cx),
                None => cx.propagate(),
            },
            _ => cx.propagate(),
        }
    }

    // ---- entering ----------------------------------------------------------------------------

    /// Brought to the front: the keys go to the list, Fleet is read, and again every minute while the page is shown.
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
