//! Kit, as a place of its own: the owner's Claude Code setup, kind by kind. A rail of kinds, the pieces of
//! one kind grouped by where they come from, and the piece picked with what Armada can do to it.
//!
//! What is read is Fleet's own: `get_kit_inventory`, `get_kit_servers`, settings.json's skills-off map, and
//! `get_kit_link` for what crosses to his other Macs. `model.rs` and `sync.rs` work out what each pane draws from
//! them and nothing there touches the screen.

mod add;
mod file;
pub(crate) mod mock;
mod model;
mod sync;
mod sync_view;
mod view;
pub mod walk;
pub(crate) mod wire;

pub(crate) use model::source_path;
// The Account page's Kit row reads where Kit stands from here.
pub use sync::{sync_status, SyncStatus};

use std::collections::HashMap;

use gpui::{
    prelude::*, Focusable as _, App, Context, Entity, EventEmitter, FocusHandle, KeyDownEvent, ListAlignment, ListState, SharedString, Subscription, Task, Window,
};
use gpui_component::input::{InputEvent, InputState};
use ipc::{KitInventory, KitKeep, KitLink, KitPieceKind, KitServerRow, SettingsList};

use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::log::Note;
use crate::settings::wire::Refused;
use crate::titlebar::{self, Asked, AskedSink};
use add::Add;
use model::{Gone, Kind, Line, Listed, Read, Removal, Row, Scope, KINDS};
use sync::Cross;

/// The kinds as the palette lists them: the word that opens Kit on one, its label and its glyph.
pub fn kinds() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    KINDS.into_iter().map(|kind| (kind.id(), kind.label(), kind.glyph()))
}

/// The three panes the keys move between with Tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pane {
    Rail,
    List,
    Detail,
}

const PANES: [Pane; 3] = [Pane::Rail, Pane::List, Pane::Detail];

/// A control Tab stops on past the three panes: the bar's link to Settings, the picked row's own switch over the
/// link, and each side of a clash. Space or Enter presses it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stop {
    Account,
    CrossItem,
    Keep { mac: String, keep: KitKeep },
}

/// Where Tab can put the keys.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Place {
    Pane(Pane),
    Stop(Stop),
}

/// A clash's two sides, each read against the other, by Mac.
pub(super) struct Compared {
    pub mac: String,
    /// The other Mac's name, as its side names it.
    pub name: String,
    pub here_at: Option<ipc::Instant>,
    pub there_at: Option<ipc::Instant>,
    pub here: Vec<(sync::Tint, String)>,
    pub there: Vec<(sync::Tint, String)>,
}

/// Where Kit reads from.
enum Source {
    Live(Entity<FleetConnection>),
    Mock(Box<mock::MockKit>),
}

/// A read that may not have answered yet.
enum Load<T> {
    Reading,
    Read(T),
    Failed(String),
}

/// Something the screen holds a question over: the plugin about to be removed.
struct Confirm {
    row: Row,
    /// What the plugin brings, by name.
    brings: Vec<String>,
}

pub struct KitScreen {
    pub within: FocusHandle,
    /// The rail, the list and the detail: each takes the keys.
    pub panes: [FocusHandle; 3],
    pub pane: Pane,
    filter: Entity<InputState>,
    source: Source,
    inventory: Load<KitInventory>,
    servers: Vec<KitServerRow>,
    settings: Option<SettingsList>,
    pub kind: Kind,
    /// Kit over the Mac link: each kind's switch, and each row against the linked Macs.
    link: Load<KitLink>,
    /// The `kit.changed` count the link was last read at.
    link_read_at: u64,
    /// Whether Kit is in front, so a `kit.changed` reads the link again.
    shown: bool,
    /// How each listed row stands over the link, by its id.
    standings: HashMap<String, sync::Standing>,
    /// The picked row's clashes, read side against side when the link was read.
    compared: Vec<Compared>,
    /// The control past the panes that has the keys, where one does.
    pub stop: Option<Stop>,
    /// One handle for each stop, in [`KitScreen::stops`]' order, made when Tab first reaches it.
    stop_handles: Vec<FocusHandle>,
    selected: Option<String>,
    /// Pieces taken out, until he leaves Kit.
    removed: Vec<Gone>,
    listed: Listed,
    read: Read,
    list: ListState,
    scope: Scope,
    refused: Option<SharedString>,
    adding: Option<Add>,
    confirming: Option<Confirm>,
    files: HashMap<String, file::FileState>,
    editing: Option<file::Edit>,
    /// The read pane's scroll, which goes back to the top when another piece is picked.
    file_scroll: gpui::ScrollHandle,
    scrolled_for: Option<String>,
    /// A save was refused because the file moved: what is on disk now, where Fleet says.
    moved: Option<Option<String>>,
    work: HashMap<&'static str, Task<()>>,
    bar: Option<Subscription>,
    _subs: Vec<Subscription>,
}

impl EventEmitter<Asked> for KitScreen {}

/// What the filter keeps while it is typed in, under its own key so a draft survives a restart.
const KEEP_FILTER: &str = "kit:filter";

impl KitScreen {
    pub fn live(fleet: &Entity<FleetConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Source::Live(fleet.clone()), window, cx);
        // `kit.changed`, folded: the link is read again while Kit is in front.
        this._subs.push(cx.observe(fleet, |this, fleet, cx| {
            let moved = fleet.read(cx).held().kit_moved;
            if moved != this.link_read_at {
                this.link_read_at = moved;
                if this.shown {
                    this.read_link(cx);
                }
            }
        }));
        this
    }

    /// On the mock Fleet in `mock.rs`, with one piece already removed.
    pub fn mock(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut kit = mock::MockKit::new();
        let gone = kit.removed_first();
        let mut this = Self::new(Source::Mock(Box::new(kit)), window, cx);
        this.removed.push(gone);
        this.read_mock(cx);
        this
    }

    /// On the mock Fleet, with Kit over the Mac link standing as `linked` says.
    #[cfg(test)]
    fn mock_linked(linked: mock::Linked, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Source::Mock(Box::new(mock::MockKit::linked(linked))), window, cx);
        this.read_mock(cx);
        this
    }

    fn new(source: Source, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter = cx.new(|cx| {
            let state = InputState::new(window, cx).placeholder("Filter");
            state
        });
        if let Some(words) = crate::drafts::kept(KEEP_FILTER, cx) {
            filter.update(cx, |state, cx| state.set_value(words, window, cx));
        }
        let subs = vec![cx.subscribe_in(&filter, window, |this, state, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let text = state.read(cx).value().to_string();
                if text.is_empty() {
                    crate::drafts::clear(KEEP_FILTER, cx);
                } else {
                    crate::drafts::keep(KEEP_FILTER, &text, cx);
                }
                this.relist(cx);
            }
            InputEvent::PressEnter { .. } => this.focus(Pane::List, window, cx),
            _ => {}
        })];
        let mut this = KitScreen {
            within: cx.focus_handle(),
            panes: [cx.focus_handle(), cx.focus_handle(), cx.focus_handle()],
            pane: Pane::List,
            filter,
            inventory: Load::Reading,
            servers: Vec::new(),
            settings: None,
            kind: Kind::Skills,
            link: Load::Reading,
            link_read_at: 0,
            shown: false,
            standings: HashMap::new(),
            compared: Vec::new(),
            stop: None,
            stop_handles: Vec::new(),
            selected: None,
            removed: Vec::new(),
            listed: Listed::default(),
            read: Read::Rows(Vec::new()),
            list: ListState::new(0, ListAlignment::Top, gpui::px(200.)),
            scope: Scope::default(),
            refused: None,
            adding: None,
            confirming: None,
            files: HashMap::new(),
            editing: None,
            file_scroll: gpui::ScrollHandle::new(),
            scrolled_for: None,
            moved: None,
            work: HashMap::new(),
            bar: None,
            source,
            _subs: subs,
        };
        if matches!(this.source, Source::Mock(_)) {
            this.scope = mock::scope();
            this.read_mock(cx);
        }
        this
    }

    // ---- reading -----------------------------------------------------------------------------

    fn port(&self, cx: &App) -> Option<u16> {
        match &self.source {
            Source::Live(fleet) => fleet.read(cx).held().port(),
            Source::Mock(_) => None,
        }
    }

    fn is_mock(&self) -> bool {
        matches!(self.source, Source::Mock(_))
    }

    /// The mock's pieces, as if Fleet had just answered.
    fn read_mock(&mut self, cx: &mut Context<Self>) {
        if let Source::Mock(kit) = &self.source {
            self.inventory = Load::Read(kit.inventory.clone());
            self.servers = kit.servers.clone();
            self.link = Load::Read(kit.link.clone());
            self.settings = None;
        }
        self.relist(cx);
    }

    /// The project Kit is scoped to now, as the title bar says it: its name and its repositories' Manifest ids.
    fn scope_now(&self, cx: &App) -> Scope {
        let Source::Live(fleet) = &self.source else { return mock::scope() };
        let (Some(bar), Some(served)) = (titlebar::live(cx), fleet.read(cx).held().repositories.clone()) else { return Scope::default() };
        let face = &bar.read(cx).face;
        let id_of = |at: usize| served.get(at).and_then(|one| one.manifest.as_ref().map(|manifest| manifest.id.as_str().to_string()).or_else(|| one.implicit_id.as_ref().map(|id| id.as_str().to_string())));
        if let Some(project) = face.scoped_to() {
            return Scope { name: project.name.to_string(), repositories: project.members.iter().filter_map(|member| id_of(member.repository)).collect() };
        }
        if let Some(at) = face.chosen {
            return Scope { name: face.repositories.get(at).map(|one| one.name.to_string()).unwrap_or_default(), repositories: id_of(at).into_iter().collect() };
        }
        Scope { name: "all repositories".into(), repositories: (0..served.len()).filter_map(id_of).collect() }
    }

    /// Read what Kit shows from Fleet: the inventory, the skills each repository's Jobs start without, and the servers.
    fn read_all(&mut self, cx: &mut Context<Self>) {
        let Some(port) = self.port(cx) else {
            if !self.is_mock() {
                self.inventory = Load::Failed("Fleet is not connected".into());
                self.relist(cx);
            }
            return;
        };
        self.scope = self.scope_now(cx);
        if self.editing.is_none() {
            self.files.clear();
        }
        self.read_link(cx);
        let inventory = wire::inventory(port, cx);
        let settings = wire::settings(port, cx);
        let manifest = self.scope.repositories.first().cloned();
        let servers = manifest.map(|manifest| wire::servers(port, &manifest, cx));
        self.work.insert(
            "read",
            cx.spawn(async move |this, cx| {
                let inventory = inventory.await;
                let settings = settings.await;
                let servers = match servers {
                    Some(servers) => servers.await.ok(),
                    None => None,
                };
                let _ = this.update(cx, |this, cx| {
                    match inventory {
                        Ok(read) => this.inventory = Load::Read(read),
                        // A read that failed keeps what was read before, where there was one.
                        Err(why) if !matches!(this.inventory, Load::Read(_)) => this.inventory = Load::Failed(why.said),
                        Err(_) => {}
                    }
                    if let Ok(list) = settings {
                        this.settings = Some(list);
                    }
                    if let Some(read) = servers {
                        this.servers = read.servers;
                    }
                    this.relist(cx);
                });
            }),
        );
        cx.notify();
    }

    /// Read `get_kit_link`. A read that failed keeps what was read before, where there was one.
    fn read_link(&mut self, cx: &mut Context<Self>) {
        let Some(port) = self.port(cx) else { return };
        let ask = wire::link(port, cx);
        self.work.insert(
            "link",
            cx.spawn(async move |this, cx| {
                let answer = ask.await;
                let _ = this.update(cx, |this, cx| {
                    match answer {
                        Ok(link) => this.link = Load::Read(link),
                        Err(why) if !matches!(this.link, Load::Read(_)) => this.link = Load::Failed(why.said),
                        Err(_) => {}
                    }
                    this.relist(cx);
                });
            }),
        );
    }

    /// The title bar moved to another project or repository: the servers are read again, in its Manifest.
    fn scope_moved(&mut self, cx: &mut Context<Self>) {
        if self.is_mock() {
            return;
        }
        let now = self.scope_now(cx);
        if now != self.scope && !now.repositories.is_empty() {
            self.read_all(cx);
        }
    }

    /// Everything the panes draw from what is held: the current kind's pieces, grouped and filtered, and the one picked.
    fn relist(&mut self, cx: &mut Context<Self>) {
        self.read = match &self.inventory {
            Load::Read(inventory) => model::read_kind(self.kind, inventory, &self.servers),
            _ => Read::Rows(Vec::new()),
        };
        // What is on a linked Mac and not here joins his own group.
        if let (Load::Read(link), Read::Rows(rows)) = (&self.link, &mut self.read) {
            let there = sync::elsewhere(link, self.kind, rows);
            rows.extend(there);
        }
        let query = self.filter.read(cx).value().to_string();
        let gone: Vec<Gone> = self.removed.iter().filter(|one| one.row.kind == self.kind).cloned().collect();
        self.listed = match &self.read {
            Read::Rows(rows) => model::listed(rows, &gone, &query),
            Read::NotRead(_) => Listed::default(),
        };
        self.list.reset(self.listed.lines.len());
        if !self.listed.shown.iter().any(|one| Some(&one.row.id) == self.selected.as_ref()) {
            self.selected = self.listed.shown.first().map(|one| one.row.id.clone());
        }
        self.standings = match (&self.link, &self.read) {
            (Load::Read(link), Read::Rows(rows)) => rows
                .iter()
                .filter_map(|row| {
                    let standing = match &row.elsewhere {
                        Some(macs) => Some(sync::Standing::OnlyThere(macs.clone())),
                        None => sync::link_row(link, row).and_then(|one| sync::standing(link, one)),
                    };
                    standing.map(|standing| (row.id.clone(), standing))
                })
                .collect(),
            _ => HashMap::new(),
        };
        self.compare_picked();
        self.leave_edit_of_another();
        self.want_file(cx);
        cx.notify();
    }

    /// The picked row's clashes, each side read against the other.
    fn compare_picked(&mut self) {
        self.compared = match (&self.link, self.picked()) {
            (Load::Read(link), Some(shown)) if shown.gone.is_none() => sync::link_row(link, &shown.row)
                .map(|one| {
                    sync::clashes(one)
                        .into_iter()
                        .filter_map(|against| {
                            let (here, there) = (against.here.as_ref()?, against.there.as_ref()?);
                            Some(Compared {
                                mac: against.mac.clone(),
                                name: there.mac_name.clone(),
                                here_at: here.changed_at.clone(),
                                there_at: there.changed_at.clone(),
                                here: sync::compared(&here.text, &there.text),
                                there: sync::compared(&there.text, &here.text),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
    }

    // ---- what is picked ----------------------------------------------------------------------

    fn picked(&self) -> Option<&model::Shown> {
        self.listed.shown.iter().find(|one| Some(&one.row.id) == self.selected.as_ref())
    }

    fn picked_at(&self) -> Option<usize> {
        self.listed.shown.iter().position(|one| Some(&one.row.id) == self.selected.as_ref())
    }

    pub fn select_kind(&mut self, kind: Kind, cx: &mut Context<Self>) {
        if self.kind != kind {
            self.kind = kind;
            self.selected = None;
            self.refused = None;
            self.relist(cx);
        }
    }

    /// A kind named by its word, as the palette and a link say it.
    pub fn show_kind(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(kind) = Kind::from_id(id) {
            self.select_kind(kind, cx);
            self.focus(Pane::List, window, cx);
        }
    }

    fn pick(&mut self, id: String, cx: &mut Context<Self>) {
        self.selected = Some(id);
        self.refused = None;
        self.compare_picked();
        self.leave_edit_of_another();
        self.want_file(cx);
        if let Some(at) = self.picked_at() {
            // The list's lines hold a group's chip before its first piece.
            if let Some(line) = self.listed.lines.iter().position(|line| *line == Line::Row(at)) {
                self.list.scroll_to_reveal_item(line);
            }
        }
        cx.notify();
    }

    /// J and K: the next or the previous piece in the list, or the next or previous kind in the rail.
    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        match self.pane {
            Pane::Rail => {
                let at = KINDS.iter().position(|kind| *kind == self.kind).unwrap_or(0) as isize;
                let to = KINDS[(at + by).clamp(0, KINDS.len() as isize - 1) as usize];
                if to != self.kind {
                    crate::ui_sounds::moved();
                }
                self.select_kind(to, cx);
            }
            Pane::List => {
                let count = self.listed.shown.len() as isize;
                if count == 0 {
                    return;
                }
                let at = self.picked_at().map_or(if by > 0 { -1 } else { count }, |at| at as isize);
                let next = (at + by).clamp(0, count - 1) as usize;
                let id = self.listed.shown[next].row.id.clone();
                if self.picked_at() != Some(next) {
                    crate::ui_sounds::moved();
                }
                self.pick(id, cx);
            }
            Pane::Detail => self.scroll_file(file::Scroll::Lines(by), cx),
        }
    }

    fn focus(&mut self, pane: Pane, window: &mut Window, cx: &mut Context<Self>) {
        self.pane = pane;
        self.stop = None;
        let handle = &self.panes[PANES.iter().position(|one| *one == pane).unwrap_or(1)];
        window.focus(handle, cx);
        cx.notify();
    }

    /// The stops drawn now, in Tab's order: the bar's link first, then the picked row's switch, which comes after the
    /// list, and its clash's buttons, which come after the read pane.
    pub(super) fn stops(&self) -> Vec<Stop> {
        let mut stops = Vec::new();
        if let Load::Read(link) = &self.link {
            if link.macs.is_empty() {
                stops.push(Stop::Account);
            }
            if self.picked().is_some_and(|one| one.gone.is_none() && sync::row_switchable(&one.row) && sync::refusal(link, &one.row).is_none()) {
                stops.push(Stop::CrossItem);
            }
        }
        for one in &self.compared {
            stops.push(Stop::Keep { mac: one.mac.clone(), keep: KitKeep::Here });
            stops.push(Stop::Keep { mac: one.mac.clone(), keep: KitKeep::There });
        }
        stops
    }

    /// The handle a stop's control tracks, where Tab has reached it.
    pub(super) fn stop_handle(&self, stop: &Stop) -> Option<&FocusHandle> {
        self.stops().iter().position(|one| one == stop).and_then(|at| self.stop_handles.get(at))
    }

    fn places(&self) -> Vec<Place> {
        let stops = self.stops();
        let row = stops.contains(&Stop::CrossItem).then_some(Place::Stop(Stop::CrossItem));
        let (before, after): (Vec<Stop>, Vec<Stop>) = stops.into_iter().filter(|stop| *stop != Stop::CrossItem).partition(|stop| *stop == Stop::Account);
        let panes = [Place::Pane(Pane::Rail), Place::Pane(Pane::List)].into_iter().chain(row).chain([Place::Pane(Pane::Detail)]);
        before.into_iter().map(Place::Stop).chain(panes).chain(after.into_iter().map(Place::Stop)).collect()
    }

    fn next_pane(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let places = self.places();
        let now = match &self.stop {
            Some(stop) => Place::Stop(stop.clone()),
            None => Place::Pane(self.pane),
        };
        let at = places.iter().position(|one| *one == now).unwrap_or(2) as isize;
        match places[(at + by).rem_euclid(places.len() as isize) as usize].clone() {
            Place::Pane(pane) => self.focus(pane, window, cx),
            Place::Stop(stop) => self.focus_stop(stop, window, cx),
        }
    }

    fn focus_stop(&mut self, stop: Stop, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = self.stops().iter().position(|one| *one == stop) else { return };
        while self.stop_handles.len() <= at {
            self.stop_handles.push(cx.focus_handle());
        }
        match stop {
            Stop::Account => {}
            Stop::CrossItem => self.pane = Pane::List,
            Stop::Keep { .. } => self.pane = Pane::Detail,
        }
        window.focus(&self.stop_handles[at], cx);
        self.stop = Some(stop);
        cx.notify();
    }

    /// Space or Enter on the stop that has the keys.
    fn press_stop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.stop.clone() {
            Some(Stop::Account) => self.settings_page("fleet:account", cx),
            Some(Stop::CrossItem) => {
                if let Some(id) = self.selected.clone() {
                    self.cross_row(&id, cx);
                }
            }
            Some(Stop::Keep { mac, keep }) => self.settle(mac, keep, window, cx),
            None => {}
        }
    }

    // ---- entering and leaving ----------------------------------------------------------------

    /// Brought to the front: the keys go to the list, and Fleet is read again.
    pub fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shown = true;
        self.focus(Pane::List, window, cx);
        if self.bar.is_none() {
            if let Some(bar) = titlebar::live(cx) {
                self.bar = Some(cx.observe(&bar, |this, _, cx| this.scope_moved(cx)));
            }
        }
        self.read_all(cx);
    }

    /// Another place in front: what he removed stops being offered back.
    pub fn leave(&mut self, cx: &mut Context<Self>) {
        self.shown = false;
        self.stop = None;
        self.removed.clear();
        self.editing = None;
        self.moved = None;
        self.refused = None;
        self.adding = None;
        self.confirming = None;
        self.relist(cx);
    }

    // ---- acts on the piece picked ------------------------------------------------------------

    fn refuse(&mut self, why: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.refused = Some(why.into());
        cx.notify();
    }

    fn off_here(&self) -> std::collections::BTreeMap<String, Vec<String>> {
        match &self.source {
            Source::Mock(kit) => kit.skills_off.clone(),
            Source::Live(_) => model::skills_off(self.settings.as_ref()),
        }
    }

    /// The project's switch on the piece picked: skills are switched off or on for each repository's Jobs, a
    /// server's reach is the Manifest's word.
    fn toggle(&mut self, cx: &mut Context<Self>) {
        let Some(row) = self.picked().filter(|one| one.gone.is_none()).map(|one| one.row.clone()).filter(Row::switchable) else { return };
        let on = !row.lamp(&self.off_here(), &self.scope).unwrap_or(true);
        self.refused = None;
        if let Source::Mock(kit) = &mut self.source {
            match (&row.jobs, &row.server) {
                (Some(name), _) => kit.skills_off = model::parse_off(model::switched(&kit.skills_off, &self.scope.repositories, name, on).get(model::SKILLS_OFF).and_then(Option::as_ref)),
                (None, Some(_)) => kit.reach(&row.name, on),
                _ => {}
            }
            self.read_mock(cx);
            return;
        }
        let Some(port) = self.port(cx) else { return self.refuse("Fleet is not connected, so nothing can be changed.", cx) };
        if let Some(name) = &row.jobs {
            let changes = model::switched(&model::skills_off(self.settings.as_ref()), &self.scope.repositories, name, on);
            let save = wire::save(port, changes, cx);
            self.work.insert(
                "switch",
                cx.spawn(async move |this, cx| {
                    let answer = save.await;
                    let _ = this.update(cx, |this, cx| match answer {
                        Ok(list) => {
                            this.settings = Some(list);
                            cx.notify();
                        }
                        Err(why) => this.refuse(why.said, cx),
                    });
                }),
            );
        } else if let (Some(_), Some(manifest)) = (&row.server, self.scope.repositories.first().cloned()) {
            let save = wire::reach(port, &manifest, &row.name, on, cx);
            let task = self.hold_servers(save, cx);
            self.work.insert("switch", task);
        }
        cx.notify();
    }

    /// A task that takes Fleet's answer about the servers.
    fn hold_servers(&mut self, answer: Task<Result<ipc::KitServers, Refused>>, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            let answer = answer.await;
            let _ = this.update(cx, |this, cx| match answer {
                Ok(read) => {
                    this.servers = read.servers;
                    this.relist(cx);
                }
                Err(why) => this.refuse(why.said, cx),
            });
        })
    }

    /// Remove: a skill goes to the Trash and stays in the list, amber, with Restore; a plugin asks first; an allowed
    /// command and a server Kit holds are taken out.
    fn remove(&mut self, cx: &mut Context<Self>) {
        let Some(shown) = self.picked().filter(|one| one.gone.is_none()).cloned() else { return };
        let row = shown.row;
        self.refused = None;
        match row.removal() {
            Removal::Piece(piece) if piece.kind == KitPieceKind::Plugin => {
                let brings = self.brings_of(&row);
                self.confirming = Some(Confirm { row, brings });
                cx.notify();
            }
            Removal::Piece(piece) => self.remove_piece(row, piece, cx),
            Removal::Command(run) => self.remove_command(&run, cx),
            Removal::Server(name) => self.forget_server(&name, cx),
            Removal::Locked(_) | Removal::None => {}
        }
    }

    /// The pieces of other kinds a plugin brings, by name.
    fn brings_of(&self, plugin: &Row) -> Vec<String> {
        self.brings_pieces(plugin).into_iter().map(|(_, _, name)| name).collect()
    }

    /// The same, with each piece's kind and id, so a link can pick it.
    fn brings_pieces(&self, plugin: &Row) -> Vec<(Kind, String, String)> {
        let Load::Read(inventory) = &self.inventory else { return Vec::new() };
        let provider = plugin.name.clone();
        [Kind::Skills, Kind::SubAgents, Kind::Commands, Kind::OutputStyles, Kind::Servers]
            .into_iter()
            .filter_map(|kind| match model::read_kind(kind, inventory, &[]) {
                Read::Rows(rows) => Some(rows),
                Read::NotRead(_) => None,
            })
            .flatten()
            .filter(|row| row.comes_with.as_deref() == Some(provider.as_str()))
            .map(|row| (row.kind, row.id, row.name))
            .collect()
    }

    /// A link in a plugin's facts: the kind it is in, and the piece picked there.
    pub(super) fn go_to(&mut self, kind: Kind, id: String, cx: &mut Context<Self>) {
        self.select_kind(kind, cx);
        self.pick(id, cx);
    }

    /// The press on a plugin's Remove, confirmed.
    fn confirm_remove(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.confirming.take() else { return };
        if let Removal::Piece(piece) = confirm.row.removal() {
            self.remove_piece(confirm.row, piece, cx);
        }
        cx.notify();
    }

    fn remove_piece(&mut self, row: Row, piece: model::Piece, cx: &mut Context<Self>) {
        if let Source::Mock(kit) = &mut self.source {
            if piece.kind == KitPieceKind::Plugin {
                kit.remove_plugin(&piece.name);
            } else if let Some(trashed) = kit.remove_skill(&piece.name) {
                self.removed.push(Gone { row, trashed: Some(trashed) });
            }
            self.read_mock(cx);
            return;
        }
        let Some(port) = self.port(cx) else { return self.refuse("Fleet is not connected, so nothing can be changed.", cx) };
        let ask = wire::remove(port, ipc::RemoveKitPiece { kind: piece.kind, name: piece.name.clone() }, cx);
        self.work.insert(
            "remove",
            cx.spawn(async move |this, cx| {
                let answer = ask.await;
                let _ = this.update(cx, |this, cx| match answer {
                    Ok(done) => {
                        // A skill stays where it was, amber, until he leaves; a plugin has no way back.
                        if let Some(trashed) = done.trashed {
                            this.removed.push(Gone { row, trashed: Some(trashed) });
                        }
                        this.inventory = Load::Read(done.setup);
                        this.relist(cx);
                    }
                    Err(why) => this.refuse(why.said, cx),
                });
            }),
        );
    }

    fn remove_command(&mut self, run: &str, cx: &mut Context<Self>) {
        let Some(port) = self.port(cx) else { return self.refuse("Fleet is not connected, so nothing can be changed.", cx) };
        let ask = wire::remove_allowed(port, run, cx);
        let again = wire::inventory(port, cx);
        self.work.insert(
            "remove",
            cx.spawn(async move |this, cx| {
                let answer = ask.await;
                let read = if answer.is_ok() { again.await.ok() } else { None };
                let _ = this.update(cx, |this, cx| match answer {
                    Ok(_) => {
                        if let Some(read) = read {
                            this.inventory = Load::Read(read);
                        }
                        this.relist(cx);
                    }
                    Err(why) => this.refuse(why.said, cx),
                });
            }),
        );
    }

    fn forget_server(&mut self, name: &str, cx: &mut Context<Self>) {
        let (Some(port), Some(manifest)) = (self.port(cx), self.scope.repositories.first().cloned()) else {
            return self.refuse("Fleet is not connected, so nothing can be changed.", cx);
        };
        let ask = wire::forget(port, &manifest, name, cx);
        let task = self.hold_servers(ask, cx);
        self.work.insert("remove", task);
    }

    /// Restore (⌘Z): the skill removed last comes back from the Trash.
    fn restore(&mut self, cx: &mut Context<Self>) {
        // The piece picked if it is one of the removed, else the one removed last.
        let picked = self.picked().filter(|one| one.gone.as_ref().is_some_and(Option::is_some)).map(|one| one.row.id.clone());
        let at = match picked {
            Some(id) => self.removed.iter().position(|one| one.row.id == id),
            None => self.removed.iter().rposition(|one| one.trashed.is_some() && one.row.kind == self.kind),
        };
        let Some(at) = at else { return };
        let Some(trashed) = self.removed[at].trashed.clone() else { return };
        self.refused = None;
        if let Source::Mock(kit) = &mut self.source {
            if kit.restore(&trashed) {
                self.removed.remove(at);
            }
            self.read_mock(cx);
            return;
        }
        let Some(port) = self.port(cx) else { return self.refuse("Fleet is not connected, so nothing can be changed.", cx) };
        let ask = wire::restore(port, trashed, cx);
        self.work.insert(
            "restore",
            cx.spawn(async move |this, cx| {
                let answer = ask.await;
                let _ = this.update(cx, |this, cx| match answer {
                    Ok(setup) => {
                        this.removed.remove(at.min(this.removed.len().saturating_sub(1)));
                        this.inventory = Load::Read(setup);
                        this.relist(cx);
                    }
                    Err(why) => this.refuse(why.said, cx),
                });
            }),
        );
    }

    // ---- over the Mac link --------------------------------------------------------------------

    /// A task that takes Fleet's answer about the link.
    fn hold_link(&mut self, key: &'static str, answer: Task<Result<KitLink, Refused>>, cx: &mut Context<Self>) {
        self.work.insert(
            key,
            cx.spawn(async move |this, cx| {
                let answer = answer.await;
                let _ = this.update(cx, |this, cx| match answer {
                    Ok(link) => {
                        this.link = Load::Read(link);
                        this.relist(cx);
                    }
                    Err(why) => this.refuse(why.said, cx),
                });
            }),
        );
    }

    /// The rail's switch over the link: from some or off, the kind on; from on, everything of the kind off.
    pub(super) fn cross_kind(&mut self, kind: Kind, cx: &mut Context<Self>) {
        let Load::Read(link) = &self.link else { return };
        let sent = sync::rail_press(link, kind);
        self.send_crosses(sent, cx);
    }

    /// A row's own switch over the link, `id` naming the row in the kind shown.
    pub(super) fn cross_row(&mut self, id: &str, cx: &mut Context<Self>) {
        let (Load::Read(link), Read::Rows(rows)) = (&self.link, &self.read) else { return };
        let Some(row) = rows.iter().find(|row| row.id == id) else { return };
        let sent = sync::row_press(link, row, rows);
        self.send_crosses(sent, cx);
    }

    /// The calls a switch sends, one after another, each once the one before was taken. A refusal stops there, says
    /// Fleet's words, and reads the link again for where it stands.
    fn send_crosses(&mut self, sent: Vec<Cross>, cx: &mut Context<Self>) {
        if sent.is_empty() {
            return;
        }
        self.refused = None;
        if let Source::Mock(mock) = &mut self.source {
            let refused = sent.iter().find_map(|one| mock.send(one).err());
            self.read_mock(cx);
            if let Some(why) = refused {
                self.refuse(why, cx);
            }
            return;
        }
        let Some(port) = self.port(cx) else { return self.refuse("Fleet is not connected, so nothing can be changed.", cx) };
        self.work.insert(
            "cross",
            cx.spawn(async move |this, cx| {
                let mut last = None;
                for one in sent {
                    let ask = cx.update(|cx| match one {
                        Cross::Kind(kind, on) => wire::cross_kind(port, ipc::SetKitLinkKind { kind, on }, cx),
                        Cross::Item(item, on) => wire::cross_item(port, ipc::SetKitLinkItem { item, on }, cx),
                    });
                    match ask.await {
                        Ok(link) => last = Some(link),
                        Err(why) => {
                            let _ = this.update(cx, |this, cx| {
                                this.refuse(why.said, cx);
                                this.read_link(cx);
                            });
                            return;
                        }
                    }
                }
                if let Some(link) = last {
                    let _ = this.update(cx, |this, cx| {
                        this.link = Load::Read(link);
                        this.relist(cx);
                    });
                }
            }),
        );
    }

    /// Keep one side of the picked row's clash with `mac`, on both Macs. The file is read again, since keeping the
    /// other Mac's writes it here.
    pub(super) fn settle(&mut self, mac: String, keep: KitKeep, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.picked().filter(|one| one.gone.is_none()).map(|one| one.row.clone()) else { return };
        let Some(item) = sync::item_of(&row) else { return };
        self.refused = None;
        self.files.remove(&row.id);
        self.focus(Pane::Detail, window, cx);
        if let Source::Mock(mock) = &mut self.source {
            mock.settle(&mac, &item, keep, &row.id);
            return self.read_mock(cx);
        }
        let Some(port) = self.port(cx) else { return self.refuse("Fleet is not connected, so nothing can be changed.", cx) };
        let ask = wire::settle(port, ipc::SettleKitClash { mac, item, keep }, cx);
        self.hold_link("settle", ask, cx);
    }

    /// Settings opens on the page for Models, Prompts or Workflows.
    fn settings_page(&mut self, page: &'static str, cx: &mut Context<Self>) {
        match cx.try_global::<AskedSink>().cloned() {
            Some(sink) => (sink.0)(&Asked::SettingsPage(page), cx),
            None => Note::info("a Kit link to Settings was left to nobody").field("page", page).emit(),
        }
    }

    // ---- keys --------------------------------------------------------------------------------

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keys = &event.keystroke;
        if self.confirming.is_some() {
            match keys.key.as_str() {
                "escape" => self.confirming = None,
                "enter" => self.confirm_remove(cx),
                _ => return,
            }
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if self.adding.is_some() {
            if keys.key == "escape" {
                self.adding = None;
                self.focus(Pane::List, window, cx);
                cx.stop_propagation();
            } else if keys.key == "enter" {
                self.add_enter(cx);
                cx.stop_propagation();
            }
            return;
        }
        if self.editing.is_some() {
            if keys.key == "escape" {
                self.cancel_edit(window, cx);
                cx.stop_propagation();
            } else if keys.modifiers.platform && keys.key == "s" {
                self.save_edit(window, cx);
                cx.stop_propagation();
            }
            return;
        }
        if keys.key == "escape" && self.pane == Pane::Detail && view::narrow(window) {
            self.focus(Pane::List, window, cx);
            cx.stop_propagation();
            return;
        }
        let typing = self.filter.read(cx).focus_handle(cx).is_focused(window);
        if typing {
            if keys.key == "escape" {
                self.focus(Pane::List, window, cx);
                cx.stop_propagation();
            }
            return;
        }
        if keys.modifiers.platform && !keys.modifiers.shift && keys.key == "z" {
            self.restore(cx);
            cx.stop_propagation();
        }
    }

    /// The registry's `kit` acts, pressed: the keys the person has bound, which `keymap` binds under the `kit` context.
    /// Not answered while a modal state or a text box has the keys, where `on_key` reads Esc, Enter and ⌘S as before.
    fn on_kit_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let typing = self.filter.read(cx).focus_handle(cx).is_focused(window);
        if self.confirming.is_some() || self.adding.is_some() || self.editing.is_some() || typing {
            return false;
        }
        let down = act.slot % 2 == 0;
        if self.stop.is_some() && matches!(act.id, "kit_toggle" | "kit_open") {
            self.press_stop(window, cx);
            return true;
        }
        match act.id {
            "kit_move" => self.step(if down { 1 } else { -1 }, cx),
            "kit_pane" => self.next_pane(if act.slot == 0 { 1 } else { -1 }, window, cx),
            // In the rail, Space is the kind's switch over the link; elsewhere the picked piece's in the project. A
            // row's own switch over the link is pressed from its Tab stop.
            "kit_toggle" if self.pane == Pane::Rail => self.cross_kind(self.kind, cx),
            "kit_toggle" => self.toggle(cx),
            "kit_remove" => self.remove(cx),
            "kit_edit" => self.start_edit(window, cx),
            "kit_open" => self.open_piece(ipc::KitPlace::Editor, cx),
            "kit_filter" => {
                let filter = self.filter.read(cx).focus_handle(cx);
                window.focus(&filter, cx);
            }
            "kit_add" => self.start_add(window, cx),
            // The read pane's own: page, page, top, bottom (slots 0 to 5), and only while it has the keys.
            "kit_scroll" if self.pane == Pane::Detail => self.scroll_file(
                match act.slot {
                    0 | 2 => file::Scroll::Pages(1),
                    1 | 3 => file::Scroll::Pages(-1),
                    4 => file::Scroll::Top,
                    _ => file::Scroll::Bottom,
                },
                cx,
            ),
            _ => return false,
        }
        true
    }

    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if act.is("pane_undo") && self.editing.is_none() && self.removed.iter().any(|one| one.trashed.is_some()) {
            self.restore(cx);
        } else if !act.id.starts_with("kit_") || !self.on_kit_act(act, window, cx) {
            cx.propagate();
        }
    }

    #[cfg(test)]
    fn names(&self) -> Vec<String> {
        self.listed.shown.iter().map(|one| one.row.name.clone()).collect()
    }
}

#[cfg(test)]
mod tests;
