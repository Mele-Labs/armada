//! The Vitals page: this Mac's load, memory, Checks, compiler, Drones and disk over the last day, each tile with its graph against
//! the settings that limit it, and those settings in the tile's foot. A setting changed there moves its dashed line at once, in the
//! accent while it is only typed, and is saved as Settings › Limits saves it: the foot's controls are that page's own, drawn by a
//! Settings overlay this page holds as its store. Every change lands in the Limit changes tile with an Undo.
//!
//! Fleet has no route for the readings yet. `wire.rs` is the one seam: the live page reads the fixture through it and keeps its
//! settings in memory, as the mock does. Like Usage it is a place of the Operator's screen (`operator/screen.rs`), reached by name
//! from the palette or by `open_vitals`.

mod mock;
pub mod model;
mod view;
pub mod walk;
mod wire;

use std::collections::BTreeMap;

use gpui::{prelude::*, App, Context, Entity, FocusHandle, Global, KeyDownEvent, ScrollHandle, Subscription, Task, Window};
use gpui_component::input::InputEvent;
use ipc::{SettingKind, SettingValue, SettingsList};

use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::settings::mock::MockFleet;
use crate::settings::overlay::{typed_value, SettingsOverlay, Typed};
use crate::settings::wire::Wire;
use model::{Change, Control, Face, Limit, Limits, Reading, Short, Span, Tile, WINDOWS, YOU};

/// Set once a Vitals page exists in the app, so the palette offers the row that brings it up.
pub struct Mounted;

impl Global for Mounted {}

/// Where the reading comes from.
enum Source {
    Live(Entity<FleetConnection>),
    /// The mock scenario the reading is made from.
    Mock(&'static str),
}

pub struct VitalsScreen {
    pub within: FocusHandle,
    /// A stop a tile, in `Tile::ALL`'s order: the arrows move between them and Tab goes on into the tile's settings.
    tiles: Vec<FocusHandle>,
    /// The Undo of each row of the change log, by the change's id.
    undo_stops: BTreeMap<u64, FocusHandle>,
    source: Source,
    span: Span,
    reading: Option<Reading>,
    /// The change log, newest first: the reading's, then what was changed here.
    changes: Vec<Change>,
    faces: Vec<Face>,
    short: Option<Short>,
    /// The settings and their controls: Settings › Limits' own, on an in-memory store until the wire lands.
    limits: Entity<SettingsOverlay>,
    /// What each setting held when last looked at, so a change adds its row to the log.
    held: BTreeMap<String, SettingValue>,
    /// The settings an Undo is putting back, and the row it takes out once it has.
    undoing: BTreeMap<String, u64>,
    next_id: u64,
    /// The tile the keys were last on.
    at: usize,
    scroll: ScrollHandle,
    entered: bool,
    work: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl VitalsScreen {
    /// On Fleet: the reading goes through `wire.rs`, which reads the fixture for now.
    pub fn live(fleet: &Entity<FleetConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let list = wire::settings(fleet, cx);
        Self::new(Source::Live(fleet.clone()), list, window, cx)
    }

    /// On the mock reading of the scenario last named, tonight's stall unless it names another.
    pub fn mock(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let scenario = if crate::mock::scenario_is(mock::LOADING) { mock::LOADING } else { mock::STALL };
        let mut this = Self::new(Source::Mock(scenario), mock::settings(), window, cx);
        this.read(cx);
        this
    }

    fn new(source: Source, list: SettingsList, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.set_global(Mounted);
        let mut store = MockFleet::new();
        store.list = list.clone();
        let limits = cx.new(|cx| {
            let mut limits = SettingsOverlay::new(Wire::Mock(Box::new(store)), window, cx);
            limits.set_list(list, window, cx);
            limits.live = true;
            limits
        });
        let mut subs = vec![cx.observe(&limits, |this, _, cx| this.limits_moved(cx))];
        // Enter in a typed figure saves it, as Save beside it does.
        let lines: Vec<(String, Entity<gpui_component::input::InputState>)> = limits.read(cx).typed.iter().filter_map(|(key, typed)| if let Typed::Line(state) = typed { Some((key.clone(), state.clone())) } else { None }).collect();
        for (key, state) in lines {
            subs.push(cx.subscribe_in(&state, window, move |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.limits.update(cx, |limits, cx| limits.save_typed(&key, window, cx));
                }
            }));
        }
        let mut this = VitalsScreen {
            within: cx.focus_handle(),
            tiles: Tile::ALL.iter().map(|_| cx.focus_handle().tab_stop(true)).collect(),
            undo_stops: BTreeMap::new(),
            source,
            span: Span::Hour,
            reading: None,
            changes: Vec::new(),
            faces: Vec::new(),
            short: None,
            limits,
            held: BTreeMap::new(),
            undoing: BTreeMap::new(),
            next_id: 100,
            at: 0,
            scroll: ScrollHandle::new(),
            entered: false,
            work: None,
            _subs: subs,
        };
        this.held = this.values(cx);
        this
    }

    // ---- reading -----------------------------------------------------------------------------

    fn read(&mut self, cx: &mut Context<Self>) {
        match &self.source {
            Source::Mock(mock::LOADING) => {}
            Source::Mock(_) => self.arrived(mock::reading(), cx),
            Source::Live(fleet) => {
                let ask = wire::reading(fleet, cx);
                self.work = Some(cx.spawn(async move |this, cx| {
                    if let Ok(reading) = ask.await {
                        let _ = this.update(cx, |this, cx| this.arrived(reading, cx));
                    }
                }));
            }
        }
    }

    fn arrived(&mut self, reading: Reading, cx: &mut Context<Self>) {
        // The log keeps what was changed here over a read again.
        let here: Vec<Change> = self.changes.iter().filter(|change| change.id >= 100).cloned().collect();
        self.changes = here.into_iter().chain(reading.changes.iter().cloned()).collect();
        self.reading = Some(reading);
        self.settle(cx);
    }

    /// What each setting holds now, saved or shipped.
    fn values(&self, cx: &App) -> BTreeMap<String, SettingValue> {
        self.limits.read(cx).list.as_ref().map(|list| list.settings.iter().map(|one| (one.key.clone(), crate::settings::limits::current(one).clone())).collect()).unwrap_or_default()
    }

    /// The limits as the page stands on them: a figure typed and not saved counts, marked as such.
    fn limits_now(&self, cx: &App) -> Limits {
        let store = self.limits.read(cx);
        let Some(list) = store.list.as_ref() else { return Limits::new() };
        list.settings
            .iter()
            .filter_map(|one| {
                let held = match crate::settings::limits::current(one) {
                    SettingValue::Integer(n) => *n as f64,
                    _ => return None,
                };
                let typed = store.dirty(&one.key, cx).then(|| store.typed.get(&one.key).map(|typed| typed.text(cx))).flatten().and_then(|text| typed_value(&one.kind, &text));
                Some((one.key.clone(), match typed {
                    Some(SettingValue::Integer(n)) => Limit { at: n as f64, unsaved: true },
                    _ => Limit { at: held, unsaved: false },
                }))
            })
            .collect()
    }

    /// The store moved: what was typed, or a save answered. A value that changed adds its row to the log, but where an Undo put it back.
    fn limits_moved(&mut self, cx: &mut Context<Self>) {
        let now = self.values(cx);
        for (key, value) in &now {
            let Some(was) = self.held.get(key) else { continue };
            if was == value {
                continue;
            }
            if let Some(id) = self.undoing.remove(key) {
                self.changes.retain(|change| change.id != id);
                continue;
            }
            self.changes.insert(0, Change { id: self.next_id, who: YOU.into(), key: key.clone(), at: chrono::Utc::now().timestamp_millis(), from: was.clone(), to: value.clone() });
            self.next_id += 1;
        }
        self.held = now;
        self.settle(cx);
    }

    fn settle(&mut self, cx: &mut Context<Self>) {
        let limits = self.limits_now(cx);
        if let Some(reading) = &self.reading {
            self.faces = model::faces(reading, self.span, &limits);
            self.short = model::short(reading, &limits);
        }
        for change in &self.changes {
            if !self.undo_stops.contains_key(&change.id) {
                self.undo_stops.insert(change.id, cx.focus_handle().tab_stop(true));
            }
        }
        let ids: Vec<u64> = self.changes.iter().map(|change| change.id).collect();
        self.undo_stops.retain(|id, _| ids.contains(id));
        cx.notify();
    }

    // ---- what the keys do ------------------------------------------------------------------

    /// The tile the keys are in, or on.
    fn tile_now(&self, window: &Window, cx: &App) -> usize {
        self.tiles.iter().position(|tile| tile.contains_focused(window, cx)).unwrap_or(self.at)
    }

    fn focus_tile(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tile) = self.tiles.get(at) else { return };
        if at != self.at {
            crate::ui_sounds::moved();
        }
        self.at = at;
        window.focus(tile, cx);
        self.scroll.scroll_to_item(at / 2);
        cx.notify();
    }

    /// The setting the keys are on: one of the tile's controls, or the tile itself, which stands for its first setting.
    fn key_now(&self, window: &Window, cx: &App) -> Option<String> {
        let tile = Tile::ALL[self.tile_now(window, cx)];
        let store = self.limits.read(cx);
        let on = tile.keys().iter().find(|key| {
            let stops = store.stops.get(**key);
            let typed = store.typed.get(**key).is_some_and(|typed| typed.focus(cx).is_focused(window));
            typed || stops.is_some_and(|stops| [&stops.control, &stops.more, &stops.save, &stops.reset].iter().any(|stop| stop.is_focused(window)))
        });
        let resting = self.tiles.iter().any(|stop| stop.is_focused(window));
        on.or_else(|| resting.then(|| tile.keys().first()).flatten()).map(|key| key.to_string())
    }

    /// Whether the keys are on the window switch of the load tile, where the side arrows move the pick.
    fn on_windows(&self, window: &Window, cx: &App) -> bool {
        self.limits.read(cx).stops.get("limits.loadWindow").is_some_and(|stops| stops.control.is_focused(window))
    }

    /// Save a setting, as its own control would.
    pub(crate) fn save(&mut self, key: &str, value: Option<SettingValue>, window: &mut Window, cx: &mut Context<Self>) {
        let key = key.to_string();
        self.limits.update(cx, |limits, cx| limits.save(key, value, window, cx));
    }

    /// + and −: the setting a step up or down, kept inside its range, and saved. A window moves to the next or the previous.
    fn step(&mut self, by: i64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.key_now(window, cx) else { return };
        let Some(one) = self.limits.read(cx).setting(&key).cloned() else { return };
        let SettingValue::Integer(now) = *crate::settings::limits::current(&one) else { return };
        let to = match (model::control_of(&key), &one.kind) {
            (Control::Windows, _) => {
                let at = WINDOWS.iter().position(|one| *one == now).unwrap_or(0) as i64;
                WINDOWS[(at + by).clamp(0, WINDOWS.len() as i64 - 1) as usize]
            }
            (_, SettingKind::Integer { min, max, .. }) => crate::settings::limits::stepped(now, by, *min, *max),
            (_, SettingKind::Seconds { min, max }) => crate::settings::limits::stepped(now, by * if *max > 600 { 60 } else { 5 }, *min, *max),
            _ => return,
        };
        if to != now {
            self.save(&key, Some(SettingValue::Integer(to)), window, cx);
        }
    }

    /// r: the setting back to what ships.
    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.key_now(window, cx) else { return };
        if self.limits.read(cx).setting(&key).is_some_and(|one| one.saved.is_some()) {
            self.save(&key, None, window, cx);
        }
    }

    /// u, or Undo on a row: the limit goes back to what it was before that change, and the row leaves the log once it has.
    pub(crate) fn undo(&mut self, id: Option<u64>, window: &mut Window, cx: &mut Context<Self>) {
        let id = id.or_else(|| self.undo_stops.iter().find(|(_, stop)| stop.is_focused(window)).map(|(id, _)| *id)).or_else(|| {
            let on_log = self.tiles.get(Tile::ALL.iter().position(|tile| *tile == Tile::Changes).unwrap_or(0)).is_some_and(|stop| stop.is_focused(window));
            on_log.then(|| self.changes.first().map(|change| change.id)).flatten()
        });
        let Some(change) = id.and_then(|id| self.changes.iter().find(|change| change.id == id)).cloned() else { return };
        let Some(one) = self.limits.read(cx).setting(&change.key).cloned() else { return };
        if *crate::settings::limits::current(&one) == change.from {
            self.changes.retain(|row| row.id != change.id);
            self.settle(cx);
            return;
        }
        self.undoing.insert(change.key.clone(), change.id);
        let value = (change.from != one.default).then(|| change.from.clone());
        self.save(&change.key, value, window, cx);
    }

    /// A span picked: the graphs draw it.
    pub(crate) fn pick(&mut self, span: Span, cx: &mut Context<Self>) {
        if span != self.span {
            self.span = span;
            self.settle(cx);
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if crate::keymap::traverse(event, window, cx) {
            cx.stop_propagation();
            return;
        }
        // Esc in a tile's settings brings the keys back to the tile.
        if event.keystroke.key == "escape" {
            let at = self.tile_now(window, cx);
            if !self.tiles[at].is_focused(window) && self.tiles[at].contains_focused(window, cx) {
                self.focus_tile(at, window, cx);
                cx.stop_propagation();
            }
        }
    }

    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        match act.id {
            "vitals_move" => {
                let key = ["left", "right", "up", "down"][usize::from(act.slot.min(3))];
                if matches!(key, "left" | "right") && self.on_windows(window, cx) {
                    self.step(if key == "left" { -1 } else { 1 }, window, cx);
                    return;
                }
                let at = self.tile_now(window, cx);
                if let Some(to) = Tile::moved(at, key) {
                    self.focus_tile(to, window, cx);
                }
            }
            "vitals_step" => self.step(if act.slot < 2 { 1 } else { -1 }, window, cx),
            "vitals_reset" => self.reset(window, cx),
            "vitals_undo" => self.undo(None, window, cx),
            "vitals_span" => match Span::at(act.slot) {
                Some(span) => self.pick(span, cx),
                None => cx.propagate(),
            },
            _ => cx.propagate(),
        }
    }

    // ---- entering ----------------------------------------------------------------------------

    /// Brought to the front: the keys go to the tile they were last on, and the reading is taken where none is held.
    pub fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.entered = true;
        if self.reading.is_none() {
            self.read(cx);
        }
        let at = self.at;
        self.focus_tile(at, window, cx);
    }

    /// Another place in front.
    pub fn leave(&mut self, _cx: &mut Context<Self>) {
        self.entered = false;
        self.work = None;
    }
}

/// Whether the app holds a Vitals page, so the palette can offer it.
pub fn offered(cx: &App) -> bool {
    cx.has_global::<Mounted>()
}

/// Whether a mock scenario of this name opens on the Vitals page.
pub fn is_scenario(name: &str) -> bool {
    mock::is_vitals(name)
}
