//! Kit, as a place of its own: the owner's Claude Code setup, kind by kind. A rail of kinds, the pieces of
//! one kind grouped by where they come from, and the piece picked with what Armada can do to it.
//!
//! What is read is Fleet's own: `get_kit_inventory`, `get_kit_servers` and settings.json's skills-off map.
//! `model.rs` works out what each pane draws from them and nothing there touches the screen.

mod add;
mod file;
mod mock;
mod model;
mod view;
pub mod walk;
pub(crate) mod wire;

pub(crate) use model::source_path;

use std::collections::HashMap;

use gpui::{
    prelude::*, Focusable as _, App, Context, Entity, EventEmitter, FocusHandle, KeyDownEvent, ListAlignment, ListState, SharedString, Subscription, Task, Window,
};
use gpui_component::input::{InputEvent, InputState};
use ipc::{KitInventory, KitPieceKind, KitServerRow, SettingsList};

use crate::fleet::FleetConnection;
use crate::keymap::Act;
use crate::log::Note;
use crate::settings::wire::Refused;
use crate::titlebar::{self, Asked, AskedSink};
use add::Add;
use model::{Gone, Kind, Line, Listed, Read, Removal, Row, Scope, KINDS};

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
        Self::new(Source::Live(fleet.clone()), window, cx)
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
        if let Some(project) = face.scoped.and_then(|at| face.projects.get(at)) {
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
        self.leave_edit_of_another();
        self.want_file(cx);
        cx.notify();
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
                self.select_kind(KINDS[(at + by).clamp(0, KINDS.len() as isize - 1) as usize], cx);
            }
            Pane::List => {
                let count = self.listed.shown.len() as isize;
                if count == 0 {
                    return;
                }
                let at = self.picked_at().map_or(if by > 0 { -1 } else { count }, |at| at as isize);
                let next = (at + by).clamp(0, count - 1) as usize;
                let id = self.listed.shown[next].row.id.clone();
                self.pick(id, cx);
            }
            Pane::Detail => self.scroll_file(file::Scroll::Lines(by), cx),
        }
    }

    fn focus(&mut self, pane: Pane, window: &mut Window, cx: &mut Context<Self>) {
        self.pane = pane;
        let handle = &self.panes[PANES.iter().position(|one| *one == pane).unwrap_or(1)];
        window.focus(handle, cx);
        cx.notify();
    }

    fn next_pane(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let at = PANES.iter().position(|one| *one == self.pane).unwrap_or(1) as isize;
        let next = (at + by).rem_euclid(PANES.len() as isize) as usize;
        self.focus(PANES[next], window, cx);
    }

    // ---- entering and leaving ----------------------------------------------------------------

    /// Brought to the front: the keys go to the list, and Fleet is read again.
    pub fn enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        match act.id {
            "kit_move" => self.step(if down { 1 } else { -1 }, cx),
            "kit_pane" => self.next_pane(if act.slot == 0 { 1 } else { -1 }, window, cx),
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
