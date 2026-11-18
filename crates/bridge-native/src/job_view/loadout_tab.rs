//! A Job's Loadout as a tab of its panes: what its agent knows and was told, what each turn used, a hosted Job's MCP
//! servers live with their acts, and Kit's acts on the skills it holds. Classic's `session-loadout.tsx` and
//! `LoadoutSheet`, in a tab rather than a sheet. `loadout_model.rs` says what each row is; `loadout_draw.rs` draws it.
//!
//! **Each Loadout tab has one entity**, made as the Job draws and dropped with its tab, as the Browser tab's page is.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, Context, Entity, EntityId, FocusHandle, Global, ScrollHandle, SharedString, Subscription, Task, WeakEntity, Window};
use ipc::{
    ActOnSessionMcp, KitInventory, KitOpenKind, KitPieceRemoved, KitPlace, LoadoutItem, LoadoutItemBody, LoadoutKind, LoadoutSummary, LoadoutTurn, McpServerAct, McpServerState,
    OpenKitPiece, SessionId, SessionLoadout, SessionMcp, SessionMcpActed, SessionMcpServer,
};

use self::loadout_model::{self as model, ItemRef, KitSkillMatch, McpGate, SkillAct};

// The Loadout's own files sit beside this one, as its children, so the Job view's module list names it once.
#[path = "loadout_draw.rs"]
mod loadout_draw;
#[path = "loadout_model.rs"]
mod loadout_model;
use super::pane_tree::{self as tree, Tab};
use super::JobView;
use crate::dropdown::Dropdown;
use crate::fleet::FleetConnection;
use crate::settings::wire::{body, request, Ask, Refused};

/// The tab's kind in a Job's panes.
const KIND: &str = "loadout";

/// Each Job view's Loadout tabs, by Job and then by tab id. Held app-wide rather than on the Job view, so the Job view
/// carries no field of the Loadout's.
#[derive(Default)]
struct Loadouts {
    by_view: HashMap<(EntityId, String), HashMap<String, Entity<LoadoutTab>>>,
}

impl Global for Loadouts {}

/// The Loadout as read: the whole of it, or the read refused. Until it lands the latest turn alone is drawn, from the record.
enum Read {
    Reading,
    Read(SessionLoadout),
    Failed,
}

/// The row a press opened, and its contents.
struct Opened {
    pub item: ItemRef,
    pub body: Body,
}

enum Body {
    Reading,
    Read(LoadoutItemBody),
    Failed,
    /// An MCP server: nothing is read, its tools are listed from the Loadout.
    Server,
}

/// A hosted Job's MCP servers as its process last said them, and the acts on them.
#[derive(Default)]
struct Mcp {
    pub servers: Option<Vec<SessionMcpServer>>,
    /// Fleet's words for a read it refused.
    pub said: Option<String>,
    pub acting: HashMap<String, McpServerAct>,
    /// Fleet's words for an act refused, and the server's state then: shown while the state holds.
    pub refused: HashMap<String, (String, McpServerState)>,
    /// Servers whose sign-in page was opened, whose face is Reconnect from then on.
    pub signed_in: HashSet<String>,
    pub gate: McpGate,
}

/// Where a request goes: Fleet's port, the mock data a walk runs on, or nowhere while Fleet is not connected.
enum Wire {
    Live(u16),
    Mock,
    Off,
}

struct LoadoutTab {
    pub job: SharedString,
    pub hosted: bool,
    fleet: Option<Entity<FleetConnection>>,
    view: WeakEntity<JobView>,
    pub summary: String,
    pub latest: Option<LoadoutTurn>,
    pub read: Read,
    reading: Option<Task<()>>,
    pub mcp: Mcp,
    mcp_reading: Option<Task<()>>,
    _poll: Option<Task<()>>,
    pub opened: Option<Opened>,
    opening: Option<Task<()>>,
    pub kit: Option<KitInventory>,
    pub matched: HashMap<String, KitSkillMatch>,
    kit_reading: Option<Task<()>>,
    /// Skills removed from this Mac, by row, with what the Trash holds of each.
    pub gone: HashMap<String, Option<String>>,
    pub skill_acting: HashMap<String, SkillAct>,
    pub skill_refused: HashMap<String, String>,
    /// The source the rows are narrowed to; None is All sources.
    pub source: Option<String>,
    /// Kinds whose whole list was asked for with Show more.
    pub whole: Vec<LoadoutKind>,
    /// The turn open, by its start; the latest is open until another is pressed.
    pub open_turn: Option<String>,
    pub scroll: ScrollHandle,
    reveal: bool,
    pub menu: Option<(Entity<Dropdown>, Vec<Subscription>)>,
    pub focus: FocusHandle,
}

impl LoadoutTab {
    pub fn new(job: SharedString, hosted: bool, fleet: Option<Entity<FleetConnection>>, view: WeakEntity<JobView>, summary: Option<&LoadoutSummary>, cx: &mut Context<Self>) -> Self {
        let latest = summary.and_then(|one| one.latest_turn.clone());
        let mut this = LoadoutTab {
            job,
            hosted,
            fleet,
            view,
            summary: model::summary_key(summary),
            open_turn: latest.as_ref().map(|turn| turn.started_at.as_str().to_string()),
            latest,
            read: Read::Reading,
            reading: None,
            mcp: Mcp::default(),
            mcp_reading: None,
            _poll: None,
            opened: None,
            opening: None,
            kit: None,
            matched: HashMap::new(),
            kit_reading: None,
            gone: HashMap::new(),
            skill_acting: HashMap::new(),
            skill_refused: HashMap::new(),
            source: None,
            whole: Vec::new(),
            scroll: ScrollHandle::new(),
            reveal: false,
            menu: None,
            focus: cx.focus_handle(),
        };
        this.read_loadout(cx);
        this.read_kit(cx);
        if hosted {
            this.read_mcp(cx);
            // Asked again every few seconds while the Job is on screen, as Classic's `MCP_EVERY_MS`.
            this._poll = Some(cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(model::MCP_EVERY).await;
                let Ok(()) = this.update(cx, |this, cx| {
                    if this.on_screen(cx) {
                        this.read_mcp(cx);
                    }
                }) else {
                    break;
                };
            }));
        }
        this
    }

    fn wire(&self, cx: &App) -> Wire {
        match &self.fleet {
            None => Wire::Mock,
            Some(fleet) => fleet.read(cx).held().port().map_or(Wire::Off, Wire::Live),
        }
    }

    fn on_screen(&self, cx: &App) -> bool {
        self.view.upgrade().is_some_and(|view| view.read(cx).shown.as_ref() == Some(&self.job))
    }

    // ---- the Loadout ------------------------------------------------------------------------

    fn read_loadout(&mut self, cx: &mut Context<Self>) {
        match self.wire(cx) {
            Wire::Mock => self.landed_loadout(Ok(mock::loadout()), cx),
            Wire::Off => self.landed_loadout(Err(Refused::said("Fleet is not connected")), cx),
            Wire::Live(port) => {
                let asked = request::<SessionLoadout>(port, Ask::Get(format!("/sessions/loadout?session_id={}", super::wire::encoded(&self.job))), "SessionLoadout", cx);
                self.reading = Some(cx.spawn(async move |this, cx| {
                    let got = asked.await;
                    let _ = this.update(cx, |this, cx| this.landed_loadout(got, cx));
                }));
            }
        }
    }

    fn landed_loadout(&mut self, got: Result<SessionLoadout, Refused>, cx: &mut Context<Self>) {
        match got {
            Ok(loadout) => {
                // A turn begun since the tab opened takes the open place only where the last latest still held it.
                let latest = loadout.turns.last().map(|turn| turn.started_at.as_str().to_string());
                let was = match &self.read {
                    Read::Read(old) => old.turns.last().map(|turn| turn.started_at.as_str().to_string()),
                    _ => self.latest.as_ref().map(|turn| turn.started_at.as_str().to_string()),
                };
                if self.open_turn == was {
                    self.open_turn = latest;
                }
                self.read = Read::Read(loadout);
                self.match_kit();
            }
            // A read that failed after one landed keeps the one drawn.
            Err(_) if matches!(self.read, Read::Read(_)) => {}
            Err(_) => self.read = Read::Failed,
        }
        self.reveal_if_asked();
        cx.notify();
    }

    /// The Job reported a new list or a new use: the open Loadout reads again, keeping what it draws until the answer lands.
    pub fn summary_moved(&mut self, summary: Option<&LoadoutSummary>, cx: &mut Context<Self>) {
        let key = model::summary_key(summary);
        if key == self.summary {
            return;
        }
        self.summary = key;
        self.latest = summary.and_then(|one| one.latest_turn.clone());
        self.read_loadout(cx);
    }

    pub fn items(&self) -> &[LoadoutItem] {
        match &self.read {
            Read::Read(loadout) => &loadout.items,
            _ => &[],
        }
    }

    pub fn turns(&self) -> Vec<&LoadoutTurn> {
        match &self.read {
            Read::Read(loadout) => loadout.turns.iter().rev().collect(),
            _ => self.latest.iter().collect(),
        }
    }

    // ---- reading one row ---------------------------------------------------------------------

    /// A press on a row opens it; the last press wins. An MCP server reads nothing and lists its tools.
    pub fn open_item(&mut self, item: ItemRef, cx: &mut Context<Self>) {
        if item.kind == LoadoutKind::McpServer {
            self.opening = None;
            self.opened = Some(Opened { item, body: Body::Server });
            cx.notify();
            return;
        }
        self.opened = Some(Opened { item: item.clone(), body: Body::Reading });
        match self.wire(cx) {
            Wire::Mock => self.landed_item(&item, Ok(mock::item(&item)), cx),
            Wire::Off => self.landed_item(&item, Err(Refused::said("Fleet is not connected")), cx),
            Wire::Live(port) => {
                let from = item.from.as_deref().map(|from| format!("&from={}", super::wire::encoded(from))).unwrap_or_default();
                let path = format!(
                    "/sessions/loadout/item?session_id={}&kind={}&name={}&source={}{from}",
                    super::wire::encoded(&self.job),
                    model::kind_wire(item.kind),
                    super::wire::encoded(&item.name),
                    model::source_wire(item.source),
                );
                let asked = request::<LoadoutItemBody>(port, Ask::Get(path), "LoadoutItemBody", cx);
                // A new press drops the read before it with its task.
                self.opening = Some(cx.spawn(async move |this, cx| {
                    let got = asked.await;
                    let _ = this.update(cx, |this, cx| this.landed_item(&item, got, cx));
                }));
            }
        }
        cx.notify();
    }

    fn landed_item(&mut self, item: &ItemRef, got: Result<LoadoutItemBody, Refused>, cx: &mut Context<Self>) {
        let Some(opened) = self.opened.as_mut().filter(|opened| opened.item == *item) else { return };
        opened.body = got.map_or(Body::Failed, Body::Read);
        cx.notify();
    }

    pub fn close_item(&mut self, cx: &mut Context<Self>) {
        self.opened = None;
        self.opening = None;
        cx.notify();
    }

    // ---- MCP servers ------------------------------------------------------------------------

    fn read_mcp(&mut self, cx: &mut Context<Self>) {
        if !self.hosted {
            return;
        }
        let seq = self.mcp.gate.ask();
        match self.wire(cx) {
            Wire::Mock => {
                let servers = self.mcp.servers.clone().unwrap_or_else(mock::servers);
                self.landed_mcp(seq, Ok(SessionMcp { servers }), cx);
            }
            Wire::Off => {}
            Wire::Live(port) => {
                let asked = request::<SessionMcp>(port, Ask::Get(format!("/sessions/mcp?session_id={}", super::wire::encoded(&self.job))), "SessionMcp", cx);
                self.mcp_reading = Some(cx.spawn(async move |this, cx| {
                    let got = asked.await;
                    let _ = this.update(cx, |this, cx| this.landed_mcp(seq, got, cx));
                }));
            }
        }
    }

    fn landed_mcp(&mut self, seq: u64, got: Result<SessionMcp, Refused>, cx: &mut Context<Self>) {
        if !self.mcp.gate.lands(seq) {
            return;
        }
        match got {
            Ok(read) => {
                self.mcp.servers = Some(read.servers);
                self.mcp.said = None;
            }
            Err(refused) => self.mcp.said = Some(refused.said),
        }
        self.reveal_if_asked();
        cx.notify();
    }

    /// An act on one server. One act at a time on a server; its answer lists every server again.
    pub fn mcp_act(&mut self, server: String, act: McpServerAct, cx: &mut Context<Self>) {
        if self.mcp.acting.contains_key(&server) {
            return;
        }
        self.mcp.refused.remove(&server);
        self.mcp.acting.insert(server.clone(), act);
        self.mcp.gate.act_sent();
        cx.notify();
        match self.wire(cx) {
            Wire::Mock => {
                let acted = mock::acted(self.mcp.servers.clone().unwrap_or_default(), &server, act);
                self.mcp_acted(server, act, Ok(acted), false, cx);
            }
            Wire::Off => self.mcp_acted(server, act, Err(Refused::said("Fleet is not connected")), false, cx),
            Wire::Live(port) => {
                let ask = ActOnSessionMcp { session_id: SessionId::carried(self.job.to_string()), server: server.clone(), act };
                let asked = request::<SessionMcpActed>(port, Ask::Post("/sessions/mcp/act".into(), body(&ask)), "SessionMcpActed", cx);
                cx.spawn(async move |this, cx| {
                    let got = asked.await;
                    let _ = this.update(cx, |this, cx| this.mcp_acted(server, act, got, true, cx));
                })
                .detach();
            }
        }
    }

    fn mcp_acted(&mut self, server: String, act: McpServerAct, got: Result<SessionMcpActed, Refused>, live: bool, cx: &mut Context<Self>) {
        self.mcp.acting.remove(&server);
        match got {
            Ok(acted) => {
                self.mcp.gate.act_answered(true);
                self.mcp.servers = Some(acted.servers);
                if let (McpServerAct::SignIn, Some(sign_in)) = (act, acted.sign_in) {
                    if live {
                        cx.open_url(&sign_in.url);
                    }
                    self.mcp.signed_in.insert(server);
                }
            }
            Err(refused) => {
                self.mcp.gate.act_answered(false);
                let state = self.server(&server).map_or(McpServerState::Connecting, |one| one.state);
                self.mcp.refused.insert(server, (refused.said, state));
            }
        }
        cx.notify();
    }

    pub fn server(&self, name: &str) -> Option<&SessionMcpServer> {
        self.mcp.servers.as_ref()?.iter().find(|one| one.name == name)
    }

    /// What a server's row offers: the act on its face, the rest behind its caret, and whether the face is an act out.
    pub fn server_acts(&self, server: &SessionMcpServer) -> (Option<McpServerAct>, Vec<McpServerAct>, bool) {
        let acts = model::mcp_acts_of(server.state, self.mcp.signed_in.contains(&server.name));
        let acting = self.mcp.acting.get(&server.name).copied();
        let face = acting.or_else(|| acts.first().copied());
        (face, acts.iter().copied().filter(|one| Some(*one) != face).collect(), acting.is_some())
    }

    /// Fleet's words for an act refused on a server, while the server is as it was then.
    pub fn server_refused(&self, server: &SessionMcpServer) -> Option<&str> {
        self.mcp.refused.get(&server.name).filter(|(_, state)| *state == server.state).map(|(said, _)| said.as_str())
    }

    // ---- skills Kit holds -------------------------------------------------------------------

    fn read_kit(&mut self, cx: &mut Context<Self>) {
        match self.wire(cx) {
            Wire::Mock => self.landed_kit(mock::inventory(), cx),
            Wire::Off => {}
            Wire::Live(port) => {
                let asked = crate::kit::wire::inventory(port, cx);
                self.kit_reading = Some(cx.spawn(async move |this, cx| {
                    if let Ok(setup) = asked.await {
                        let _ = this.update(cx, |this, cx| this.landed_kit(setup, cx));
                    }
                }));
            }
        }
    }

    fn landed_kit(&mut self, setup: KitInventory, cx: &mut Context<Self>) {
        self.kit = Some(setup);
        self.match_kit();
        cx.notify();
    }

    fn match_kit(&mut self) {
        let Some(setup) = &self.kit else { return };
        let matched = model::skill_acts_by_kit(self.items(), setup);
        self.matched = matched;
    }

    /// What a skill row offers on its face and behind its caret, and whether the face is an act out. None where Kit does not hold it.
    pub fn skill_acts(&self, key: &str) -> Option<(Option<SkillAct>, Vec<SkillAct>, bool)> {
        if let Some(act) = self.skill_acting.get(key) {
            return Some((Some(*act), Vec::new(), true));
        }
        if let Some(trashed) = self.gone.get(key) {
            let (face, menu) = model::skill_face(&Default::default(), Some(trashed.is_some()));
            return Some((face, menu, false));
        }
        let (face, menu) = model::skill_face(&self.matched.get(key)?.acts, None);
        Some((face, menu, false))
    }

    /// A skill row's act: Edit at once, Remove and Remove plugin once he confirms, Restore at once.
    pub fn skill_act(&mut self, key: String, name: String, act: SkillAct, window: &mut Window, cx: &mut Context<Self>) {
        let Some(found) = self.matched.get(&key).cloned() else {
            if act == SkillAct::Restore {
                self.restore(key, cx);
            }
            return;
        };
        self.skill_refused.remove(&key);
        match act {
            SkillAct::Edit => self.edit(key, found, cx),
            SkillAct::Restore => self.restore(key, cx),
            SkillAct::Remove => {
                let title = format!("Remove {name} from this Mac");
                let lines = ["Every Job on this Mac loses it from its next start.", "Its folder goes to the Trash. Restore on its row brings it back."];
                self.confirm(&title, &lines, "Remove", key, act, window, cx);
            }
            SkillAct::RemovePlugin => {
                let Some(plugin) = found.acts.plugin.clone() else { return };
                let title = format!("Remove plugin {} from this Mac", plugin.name);
                let takes = format!("Removes its skills: {}.", plugin.takes.join(", "));
                let mut lines: Vec<&str> = plugin.uninstall.as_deref().into_iter().collect();
                lines.extend([takes.as_str(), "Every Job on this Mac loses them from its next start.", "There is no Undo."]);
                self.confirm(&title, &lines, "Remove plugin", key, act, window, cx);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn confirm(&mut self, title: &str, lines: &[&str], yes: &str, key: String, act: SkillAct, window: &mut Window, cx: &mut Context<Self>) {
        let me = cx.entity().downgrade();
        let on_yes: Rc<dyn Fn(&mut Window, &mut App)> = Rc::new(move |_, cx| {
            let key = key.clone();
            let _ = me.update(cx, |this, cx| this.remove(key, act, cx));
        });
        let _ = self.view.update(cx, |view, cx| view.confirm(title, lines, yes, true, window, cx, on_yes));
    }

    fn edit(&mut self, key: String, found: KitSkillMatch, cx: &mut Context<Self>) {
        let Wire::Live(port) = self.wire(cx) else { return };
        let ask = OpenKitPiece { kind: KitOpenKind::Skills, name: found.kit_name, plugin: found.kit_plugin, place: KitPlace::Editor };
        let asked = crate::kit::wire::open(port, ask, cx);
        cx.spawn(async move |this, cx| {
            if let Err(refused) = asked.await {
                let _ = this.update(cx, |this, cx| {
                    this.skill_refused.insert(key, refused.said);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Remove or Remove plugin, confirmed.
    pub fn remove(&mut self, key: String, act: SkillAct, cx: &mut Context<Self>) {
        let Some(found) = self.matched.get(&key) else { return };
        let Some(piece) = (if act == SkillAct::RemovePlugin { found.remove_plugin.clone() } else { found.remove.clone() }) else { return };
        self.skill_acting.insert(key.clone(), act);
        cx.notify();
        match self.wire(cx) {
            Wire::Mock => {
                let trashed = (act == SkillAct::Remove).then(|| format!("/Users/a/.Trash/{}", piece.name));
                self.removed(key, piece, Ok(KitPieceRemoved { setup: mock::inventory(), trashed }), cx);
            }
            Wire::Off => self.removed(key, piece, Err(Refused::said("Fleet is not connected")), cx),
            Wire::Live(port) => {
                let asked = crate::kit::wire::remove(port, piece.clone(), cx);
                cx.spawn(async move |this, cx| {
                    let got = asked.await;
                    let _ = this.update(cx, |this, cx| this.removed(key, piece, got, cx));
                })
                .detach();
            }
        }
    }

    /// Kit's answer to a removal: a plugin's takes every row Kit matched to it.
    pub fn removed(&mut self, key: String, piece: ipc::RemoveKitPiece, got: Result<KitPieceRemoved, Refused>, cx: &mut Context<Self>) {
        self.skill_acting.remove(&key);
        match got {
            Ok(removed) => {
                let rows = if piece.kind == ipc::KitPieceKind::Plugin { model::rows_of_plugin(&self.matched, &piece) } else { vec![key] };
                for row in rows {
                    self.gone.insert(row, removed.trashed.clone());
                }
                self.kit = Some(removed.setup);
                self.match_kit();
            }
            Err(refused) => {
                self.skill_refused.insert(key, refused.said);
            }
        }
        cx.notify();
    }

    fn restore(&mut self, key: String, cx: &mut Context<Self>) {
        let Some(Some(trashed)) = self.gone.get(&key).cloned() else { return };
        self.skill_acting.insert(key.clone(), SkillAct::Restore);
        self.skill_refused.remove(&key);
        cx.notify();
        let held = trashed.clone();
        let done = move |this: &mut Self, got: Result<KitInventory, Refused>, cx: &mut Context<Self>| {
            this.skill_acting.remove(&key);
            match got {
                Ok(setup) => {
                    this.gone.retain(|_, one| one.as_deref() != Some(held.as_str()));
                    this.landed_kit(setup, cx);
                }
                Err(refused) => {
                    this.skill_refused.insert(key, refused.said);
                    cx.notify();
                }
            }
        };
        match self.wire(cx) {
            Wire::Mock => done(self, Ok(mock::inventory()), cx),
            Wire::Off => done(self, Err(Refused::said("Fleet is not connected")), cx),
            Wire::Live(port) => {
                let asked = crate::kit::wire::restore(port, trashed.clone(), cx);
                cx.spawn(async move |this, cx| {
                    let got = asked.await;
                    let _ = this.update(cx, |this, cx| done(this, got, cx));
                })
                .detach();
            }
        }
    }

    // ---- reaching the MCP servers -----------------------------------------------------------

    /// `/mcp` opened the tab: its MCP servers come to the top as soon as they are drawn.
    pub fn reveal_mcp(&mut self, cx: &mut Context<Self>) {
        self.reveal = true;
        self.reveal_if_asked();
        cx.notify();
    }

    fn reveal_if_asked(&mut self) {
        if !self.reveal {
            return;
        }
        if let Some(at) = self.mcp_at() {
            self.scroll.scroll_to_top_of_item(at);
            self.reveal = false;
        }
    }

    /// The kinds of what it knows that have rows under the filter, in the tab's order. While the Loadout is read, the live MCP servers alone.
    pub fn held_known(&self) -> Vec<LoadoutKind> {
        let live = self.live_servers().is_some_and(|servers| !servers.is_empty());
        match self.read {
            Read::Read(_) => model::KNOWN.into_iter().filter(|kind| (*kind == LoadoutKind::McpServer && self.mcp.servers.is_some() && live) || self.kept(*kind).next().is_some()).collect(),
            _ if live => vec![LoadoutKind::McpServer],
            _ => Vec::new(),
        }
    }

    /// The MCP group's place among the list's children: under the Knows heading, after the kinds before it.
    pub fn mcp_at(&self) -> Option<usize> {
        self.held_known().iter().position(|kind| *kind == LoadoutKind::McpServer).map(|at| at + 1)
    }

    /// Whether a row is under the source picked.
    pub fn shown(&self, said: &str) -> bool {
        self.source.as_deref().is_none_or(|source| source == said)
    }

    /// One kind's rows under the filter.
    pub fn kept(&self, kind: LoadoutKind) -> impl Iterator<Item = &LoadoutItem> {
        self.items().iter().filter(move |one| one.kind == kind && self.shown(&model::source_said(one.source, one.from.as_deref())))
    }

    /// The live servers under the filter: one the agent reported keeps that report's source, one it did not is under All sources only.
    pub fn live_servers(&self) -> Option<Vec<&SessionMcpServer>> {
        let servers = self.mcp.servers.as_ref()?;
        Some(
            servers
                .iter()
                .filter(|server| match self.items().iter().find(|one| one.kind == LoadoutKind::McpServer && one.name == server.name) {
                    Some(reported) => self.shown(&model::source_said(reported.source, reported.from.as_deref())),
                    None => self.source.is_none(),
                })
                .collect(),
        )
    }

    pub fn pick_source(&mut self, source: Option<String>, cx: &mut Context<Self>) {
        self.source = source;
        cx.notify();
    }
}

// ---- the Job view's hooks -----------------------------------------------------------------------

impl JobView {
    fn loadout_ids(&self, job: &str) -> Vec<String> {
        let Some(arrangement) = self.arrangement_of(job) else { return Vec::new() };
        tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).filter(|tab| tab.kind == KIND).map(|tab| tab.id.clone()).collect()
    }

    /// Each Loadout tab Job `job` has gets its entity, once; one whose tab closed or whose Job went goes with it. A
    /// Loadout open reads again when the Job's record says its list or its latest turn moved.
    fn sync_loadouts(&self, job: &SharedString, cx: &mut Context<Self>) {
        let me = cx.entity_id();
        let ids = self.loadout_ids(job);
        let gone: Vec<(EntityId, String)> =
            cx.try_global::<Loadouts>().map(|held| held.by_view.keys().filter(|(view, one)| *view == me && self.record(one, cx).is_none()).cloned().collect()).unwrap_or_default();
        let record = self.record(job, cx);
        let hosted = record.is_some_and(|one| one.hosted.is_some());
        let summary = record.and_then(|one| one.loadout_summary.clone());
        let (fleet, weak) = (self.fleet.clone(), cx.entity().downgrade());
        let all = cx.default_global::<Loadouts>();
        for key in gone {
            all.by_view.remove(&key);
        }
        let mut held = all.by_view.remove(&(me, job.to_string())).unwrap_or_default();
        held.retain(|id, _| ids.contains(id));
        for id in ids {
            match held.get(&id) {
                Some(tab) => {
                    if tab.read(cx).summary != model::summary_key(summary.as_ref()) {
                        tab.update(cx, |tab, cx| tab.summary_moved(summary.as_ref(), cx));
                    }
                }
                None => {
                    let tab = cx.new(|cx| LoadoutTab::new(job.clone(), hosted, fleet.clone(), weak.clone(), summary.as_ref(), cx));
                    held.insert(id, tab);
                }
            }
        }
        if !held.is_empty() {
            cx.default_global::<Loadouts>().by_view.insert((me, job.to_string()), held);
        }
    }

    fn held_loadout(&self, job: &str, tab: &str, cx: &Context<Self>) -> Option<Entity<LoadoutTab>> {
        cx.try_global::<Loadouts>()?.by_view.get(&(cx.entity_id(), job.to_string()))?.get(tab).cloned()
    }

    /// The body of a Loadout tab in front, its entity made as it first draws: a plain body, as the Browser tab's, since
    /// the tab scrolls its own list under its filter.
    pub(super) fn loadout_body(&self, job: &SharedString, tab: &Tab, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.sync_loadouts(job, cx);
        let held = self.held_loadout(job, &tab.id, cx)?;
        Some(div().id(SharedString::from(format!("pane-body:{}", tab.id))).flex_1().min_h_0().flex().flex_col().child(div().size_full().child(held)).into_any_element())
    }

    /// The Loadout tab in front where the Job keeps one.
    fn front_loadout(&self, job: &str, cx: &Context<Self>) -> Option<Entity<LoadoutTab>> {
        let arrangement = self.arrangement_of(job)?;
        let pane = tree::holder_of(arrangement, KIND)?;
        let id = pane.active.as_ref().filter(|id| pane.tabs.iter().any(|tab| tab.id == **id && tab.kind == KIND)).or_else(|| pane.tabs.iter().find(|tab| tab.kind == KIND).map(|tab| &tab.id))?;
        self.held_loadout(job, id, cx)
    }

    /// The Loadout act and `/mcp`: the Loadout tab in front, opened where none is, and the keys in it. `mcp` scrolls it to the MCP servers.
    pub(super) fn open_loadout(&mut self, mcp: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        self.add_tab(KIND, window, cx);
        self.sync_loadouts(&job, cx);
        let Some(tab) = self.front_loadout(&job, cx) else { return };
        if mcp {
            tab.update(cx, |tab, cx| tab.reveal_mcp(cx));
        }
        let keys = tab.read(cx).focus.clone();
        window.focus(&keys, cx);
    }

    /// `/mcp` sent alone: the box empties, and the Loadout opens at the MCP servers, with nothing sent to the agent
    /// (Classic's `opensMcp`).
    pub(super) fn mcp_typed(&mut self, job: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        self.set_box(&super::composer::Kept::default(), window, cx);
        self.keep_draft(job, super::composer::Kept::default(), cx);
        self.composer.offer = None;
        self.open_loadout(true, window, cx);
    }

    /// Whether the box's words are `/mcp` alone in a hosted Job, which opens the Loadout rather than going to the agent.
    pub(super) fn opens_mcp(&self, job: &SharedString, text: &str, cx: &App) -> bool {
        let hosted = self.record(job, cx).is_some_and(|one| one.hosted.is_some());
        model::opens_mcp(hosted, text, usize::from(self.composer.holds(job)))
    }

    /// The Loadout act under the message box, for any Job.
    pub(super) fn loadout_act(&self, job: &SharedString, cx: &Context<Self>) -> impl IntoElement {
        let look = crate::look::ActiveLook::look(&**cx);
        let open = self.arrangement_of(job).is_some_and(|arrangement| tree::has_kind(arrangement, KIND));
        crate::labels::labelled(
            super::head::act_button(look, "act-loadout", "layers", KIND, "Loadout".into(), open, false)
                .on_click(cx.listener(|this, _, window, cx| this.open_loadout(false, window, cx))),
            gpui::Role::Button,
            "Loadout",
        )
    }
}

/// The `/` commands a hosted Job offers, with `/mcp` where its process did not list it: it opens the Loadout (`hostedCommands`).
pub(super) fn with_mcp(commands: &[String]) -> Vec<String> {
    let mut all = commands.to_vec();
    if !all.iter().any(|one| one.trim_start_matches('/') == "mcp") {
        all.push("mcp".into());
    }
    all
}

/// What a walk on mock data reads: a Loadout of each kind, and a hosted Job's servers in each state.
mod mock {
    use ipc::{
        Instant, KitInventory, LoadoutItem, LoadoutItemBody, LoadoutItemFormat, LoadoutKind, LoadoutSource, LoadoutState, LoadoutTurn, LoadoutUse, LoadoutUseKind, McpServerAct,
        McpServerState, McpSignIn, SessionLoadout, SessionMcpActed, SessionMcpServer, SetupItem, SetupKindRow, WhatWasRead,
    };

    use super::ItemRef;

    fn row(kind: LoadoutKind, name: &str, source: LoadoutSource, from: Option<&str>, state: Option<LoadoutState>) -> LoadoutItem {
        LoadoutItem { kind, name: name.into(), source, from: from.map(str::to_string), state, tokens: None }
    }

    pub fn loadout() -> SessionLoadout {
        use LoadoutKind::*;
        use LoadoutSource::*;
        let items = vec![
            row(Skill, "tidy-imports", Kit, None, None),
            row(Skill, "docs:write-page", Plugin, Some("docs"), None),
            row(Skill, "docs:check-links", Plugin, Some("docs"), None),
            row(Skill, "release-notes", Repository, None, None),
            row(Subagent, "explorer", Armada, None, None),
            row(McpServer, "linear", Mcp, Some("linear"), Some(LoadoutState::Connected)),
            row(McpServer, "sentry", Mcp, Some("sentry"), Some(LoadoutState::Failed)),
            row(Tool, "Read", BuiltIn, None, None),
            row(Tool, "Edit", BuiltIn, None, None),
            row(Tool, "mcp__linear__list_issues", Mcp, Some("linear"), Some(LoadoutState::Loaded)),
            row(Tool, "mcp__linear__create_issue", Mcp, Some("linear"), Some(LoadoutState::Deferred)),
            row(MemoryFile, "CLAUDE.md", Repository, None, None),
            row(Rules, "Job rules", Armada, None, None),
            row(Injection, "Plan reminder", Armada, None, None),
        ];
        let at = |when: &str| Instant::carried(format!("2026-11-17T{when}:00Z"));
        let used = |turn: &str, kind, name: &str, source, from: Option<&str>, count| LoadoutUse {
            turn_started_at: at(turn),
            kind,
            name: name.into(),
            source,
            from: from.map(str::to_string),
            model: None,
            by: None,
            count,
        };
        let turns = vec![
            LoadoutTurn { started_at: at("09:00"), used: vec![used("09:00", LoadoutUseKind::Tool, "Read", BuiltIn, None, Some(6))] },
            LoadoutTurn {
                started_at: at("10:30"),
                used: vec![
                    used("10:30", LoadoutUseKind::Skill, "tidy-imports", Kit, None, None),
                    used("10:30", LoadoutUseKind::Tool, "mcp__linear__list_issues", Mcp, Some("linear"), Some(2)),
                    used("10:30", LoadoutUseKind::Subagent, "explorer", Armada, None, None),
                ],
            },
        ];
        SessionLoadout { items, turns }
    }

    pub fn servers() -> Vec<SessionMcpServer> {
        let server = |name: &str, state, error: Option<&str>, tools| SessionMcpServer { name: name.into(), state, error: error.map(str::to_string), scope: None, tools };
        vec![
            server("linear", McpServerState::Connected, None, 12),
            server("sentry", McpServerState::Failed, Some("Connection refused"), 0),
            server("figma", McpServerState::NeedsSignIn, None, 0),
            server("notion", McpServerState::Off, None, 0),
            server("github", McpServerState::Connecting, None, 0),
        ]
    }

    /// An act on mock data takes at once: the server goes where the act sends it, and Sign in hands back a page to open.
    pub fn acted(mut servers: Vec<SessionMcpServer>, name: &str, act: McpServerAct) -> SessionMcpActed {
        if let Some(server) = servers.iter_mut().find(|one| one.name == name) {
            server.state = match act {
                McpServerAct::Reconnect | McpServerAct::TurnOn => McpServerState::Connected,
                McpServerAct::TurnOff => McpServerState::Off,
                McpServerAct::SignIn => server.state,
            };
            server.error = None;
        }
        let sign_in = (act == McpServerAct::SignIn).then(|| McpSignIn { url: format!("https://example.com/{name}/sign-in"), callback_expected: true });
        SessionMcpActed { servers, sign_in }
    }

    pub fn item(item: &ItemRef) -> LoadoutItemBody {
        match item.kind {
            LoadoutKind::Skill => LoadoutItemBody::Read {
                path: Some(format!("/Users/a/.harness/skills/{}/SKILL.md", item.name)),
                format: LoadoutItemFormat::Markdown,
                body: format!("---\nname: {}\ndescription: What the skill does.\n---\n\n# {}\n\nThe steps it follows.", item.name, item.name),
                truncated: false,
            },
            LoadoutKind::Tool => LoadoutItemBody::NotRead { why: "A tool has no file of its own.".into() },
            _ => LoadoutItemBody::Read { path: None, format: LoadoutItemFormat::Text, body: "What the agent was given.".into(), truncated: false },
        }
    }

    pub fn inventory() -> KitInventory {
        let setup = |name: &str, source: &str, plugin: Option<&str>, uninstall: Option<&str>| SetupItem {
            name: name.into(),
            says: None,
            source: source.into(),
            style: None,
            plugin: plugin.map(str::to_string),
            uninstall: uninstall.map(str::to_string),
            server: None,
        };
        let kind = |kind: &str, items| SetupKindRow { kind: kind.into(), read: WhatWasRead::Read { items, unreadable: Vec::new() } };
        KitInventory {
            harness: "Harness".into(),
            home: "/Users/a/.harness".into(),
            present: true,
            kinds: vec![
                kind(
                    "skills",
                    vec![
                        setup("tidy-imports", "skills/tidy-imports", None, None),
                        setup("write-page", "plugins/docs/skills/write-page", Some("docs"), None),
                        setup("check-links", "plugins/docs/skills/check-links", Some("docs"), None),
                    ],
                ),
                kind("plugins", vec![setup("docs@market", "plugins/docs", None, Some("harness plugin uninstall docs@market"))]),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{div, Role, VisualTestContext};

    use super::*;
    use crate::labels::LabelBook;

    struct Rooted(Entity<LoadoutTab>);

    impl Render for Rooted {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            crate::labels::label_root(div().size_full().child(self.0.clone()))
        }
    }

    fn init(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
    }

    /// A hosted Job's Loadout tab on mock data, drawn.
    fn drawn(cx: &mut gpui::TestAppContext) -> (Entity<LoadoutTab>, &mut VisualTestContext) {
        init(cx);
        let (root, cx) = cx.add_window_view(|_, cx| Rooted(cx.new(|cx| LoadoutTab::new("mock-05".into(), true, None, WeakEntity::new_invalid(), None, cx))));
        let tab = root.read_with(cx, |root, _| root.0.clone());
        cx.run_until_parked();
        (tab, cx)
    }

    fn names(cx: &mut VisualTestContext, role: Role) -> Vec<String> {
        cx.update(|window, cx| {
            let drawn = cx.try_global::<LabelBook>().map(|book| book.drawn(window.window_handle().window_id()).to_vec()).unwrap_or_default();
            drawn.iter().filter(|one| one.role == role).map(|one| one.name.to_string()).collect()
        })
    }

    fn redraw(cx: &mut VisualTestContext) {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }

    #[gpui::test]
    fn each_server_offers_its_states_acts_face_first_named_for_the_server(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        let buttons = names(cx, Role::Button);
        for named in ["Turn off linear", "More for linear", "Reconnect sentry", "More for sentry", "Sign in figma", "More for figma", "Turn on notion"] {
            assert!(buttons.iter().any(|one| one == named), "{named} is drawn: {buttons:?}");
        }
        assert!(!buttons.iter().any(|one| one == "More for notion"), "Off takes Turn on alone, with no caret");
        assert!(!buttons.iter().any(|one| one.ends_with(" github")), "a server connecting offers nothing");
        // Once its sign-in page opened, a server that still needs sign-in offers Reconnect first.
        tab.update(cx, |tab, cx| tab.mcp_act("figma".into(), McpServerAct::SignIn, cx));
        redraw(cx);
        let buttons = names(cx, Role::Button);
        assert!(buttons.iter().any(|one| one == "Reconnect figma"), "{buttons:?}");
        assert!(!buttons.iter().any(|one| one == "Sign in figma"), "Sign in went behind the caret");
    }

    #[gpui::test]
    fn removing_a_plugin_marks_every_row_of_it_removed_and_a_removed_skill_offers_restore(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        let buttons = names(cx, Role::Button);
        assert!(buttons.iter().any(|one| one == "Edit tidy-imports"), "{buttons:?}");
        assert!(buttons.iter().any(|one| one == "More for docs:write-page"));
        let key = |name: &str| tab.read_with(cx, |tab, _| tab.items().iter().find(|one| one.name == name).map(|one| ItemRef::of(one).key())).expect("the row");
        let (write, links, tidy) = (key("docs:write-page"), key("docs:check-links"), key("tidy-imports"));
        tab.update(cx, |tab, cx| tab.remove(write, SkillAct::RemovePlugin, cx));
        redraw(cx);
        assert_eq!(names(cx, Role::Image).iter().filter(|one| *one == "Removed").count(), 2, "both of the plugin's skills read Removed");
        tab.read_with(cx, |tab, _| assert!(tab.gone.contains_key(&links)));
        let buttons = names(cx, Role::Button);
        for gone in ["Edit docs:check-links", "More for docs:check-links", "Restore docs:check-links"] {
            assert!(!buttons.iter().any(|one| one == gone), "a removed plugin's row offers nothing, so not {gone}: {buttons:?}");
        }
        tab.update(cx, |tab, cx| tab.remove(tidy, SkillAct::Remove, cx));
        redraw(cx);
        assert!(names(cx, Role::Button).iter().any(|one| one == "Restore tidy-imports"));
    }

    /// A live hosted Job on screen in the Job view, on mock data, drawn once.
    fn hosted_job(cx: &mut gpui::TestAppContext) -> (Entity<JobView>, SharedString, &mut VisualTestContext) {
        init(cx);
        let jobs = crate::mock::jobs("job-view").expect("the fixture decodes");
        let job: SharedString = jobs.iter().find(|one| one.hosted.is_some() && one.state == ipc::SessionState::Live).map(|one| one.id.as_str().to_string()).expect("a live hosted Job").into();
        let threads = crate::mock::threads("job-view").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        list.update(cx, |list, cx| list.open(job.clone(), cx));
        cx.run_until_parked();
        redraw(cx);
        (view, job, cx)
    }

    fn bounds_of(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
        // The last drawn, as the book keeps what earlier frames drew too.
        cx.update(|window, cx| cx.try_global::<LabelBook>()?.drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds))
    }

    fn has_loadout(view: &Entity<JobView>, cx: &mut VisualTestContext) -> bool {
        view.read_with(cx, |view, _| tree::has_kind(&view.arrangement(), KIND))
    }

    #[gpui::test]
    fn the_loadout_act_stands_in_the_job_acts_and_a_press_opens_the_tab(cx: &mut gpui::TestAppContext) {
        let (view, _, cx) = hosted_job(cx);
        let acts = bounds_of(cx, Role::Toolbar, "Job acts").expect("the Job's acts are drawn");
        let act = bounds_of(cx, Role::Button, "Loadout").expect("the Loadout act is drawn");
        assert!(acts.contains(&act.center()), "the Loadout act is among the Job's acts: {act:?} in {acts:?}");
        let seen = cx.update(|window, _| window.viewport_size());
        assert!(act.right() <= seen.width && act.bottom() <= seen.height, "the act is on screen: {act:?} in {seen:?}");
        assert!(!has_loadout(&view, cx), "no Loadout tab before the press");
        cx.simulate_click(act.center(), gpui::Modifiers::none());
        redraw(cx);
        assert!(has_loadout(&view, cx), "the press opened the Loadout tab");
        assert!(bounds_of(cx, Role::Group, "Loadout").is_some(), "the tab draws its Loadout");
        // A second press brings the same tab forward rather than opening another.
        let act = bounds_of(cx, Role::Button, "Loadout").expect("the Loadout act is drawn");
        cx.simulate_click(act.center(), gpui::Modifiers::none());
        redraw(cx);
        let tabs = view.read_with(cx, |view, _| tree::panes_of(&view.arrangement().root).iter().flat_map(|pane| pane.tabs.iter()).filter(|tab| tab.kind == KIND).count());
        assert_eq!(tabs, 1);
    }

    #[gpui::test]
    fn the_add_menu_offers_loadout(cx: &mut gpui::TestAppContext) {
        let (view, _, cx) = hosted_job(cx);
        let words = view.update_in(cx, |view, window, cx| {
            let pane = tree::panes_of(&view.arrangement().root).first().map(|pane| pane.id.clone()).expect("a pane");
            view.open_add_menu(&pane, gpui::point(gpui::px(0.), gpui::px(0.)), window, cx);
            let menu = view.tabs_menu.as_ref().expect("the menu is open").menu.clone();
            menu.read(cx)
                .entries
                .iter()
                .filter_map(|entry| match entry {
                    crate::dropdown::Entry::Item { id, label, .. } => Some((*id, label.to_string())),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });
        assert!(words.iter().any(|(id, word)| *id == KIND && word == "Loadout"), "{words:?}");
    }

    #[gpui::test]
    /// What reaches the agent is `opens_mcp`'s to say (its model test): here the box's `/mcp` opens the tab and empties the box.
    #[gpui::test]
    fn mcp_alone_in_the_box_opens_the_loadout_and_empties_the_box(cx: &mut gpui::TestAppContext) {
        let (view, _, cx) = hosted_job(cx);
        let send = |view: &Entity<JobView>, words: &str, cx: &mut VisualTestContext| {
            view.update_in(cx, |view, window, cx| {
                view.set_box(&super::super::composer::Kept { text: words.into(), tags: Vec::new() }, window, cx);
                view.send(window, cx);
            });
            redraw(cx);
        };
        send(&view, "/mcp list", cx);
        assert!(!has_loadout(&view, cx), "other words open nothing");
        send(&view, "/mcp", cx);
        assert!(has_loadout(&view, cx), "the Loadout tab opened");
        view.read_with(cx, |view, cx| assert_eq!(view.kept(cx).text, "", "the box is empty"));
    }

    #[gpui::test]
    fn a_read_answering_while_an_act_is_out_does_not_move_the_row(cx: &mut gpui::TestAppContext) {
        let (tab, cx) = drawn(cx);
        tab.update(cx, |tab, cx| {
            let seq = tab.mcp.gate.ask();
            tab.mcp.acting.insert("linear".into(), McpServerAct::Reconnect);
            tab.mcp.gate.act_sent();
            let mut stale = mock::servers();
            stale[0].state = McpServerState::Connecting;
            tab.landed_mcp(seq, Ok(SessionMcp { servers: stale }), cx);
            assert_eq!(tab.server("linear").map(|one| one.state), Some(McpServerState::Connected), "the read is dropped");
        });
    }
}
