//! The System block: a project drawn as a graph that zooms through three levels, with a panel of what Fleet knows of the part
//! that is selected. Level one is the project's people, its boundary with the parts inside it, and what it reaches outside;
//! level two is the inside of one part; level three is one feature's path through that part.
//!
//! Fleet finds the parts and answers each of the nine sections (`docs/practices/protocol/projects.md`, *The system map*), so this
//! draws what it is given. The block owns its own reads, one level at a time, and tells the board only what the board acts on.
//!
//! Its keys are the block's own while it has them, as a menu's are: the arrows move between parts, Return looks inside, Esc steps
//! out. Nothing here is a binding of the registry, so a rebinding does not move them.

mod arrange;
mod graph;
pub(super) mod mock;
mod panel;
pub mod shots;
mod trail;
mod tree;
mod wire;

#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{canvas, div, prelude::*, AnyElement, App, Bounds, Context, Entity, EventEmitter, FocusHandle, Pixels, Subscription, Task, Window};
use gpui_component::input::{InputEvent, TextareaState};
use ipc::{AdditionAnswer, AnswerSystemAddition, SystemMap, SystemPartAnswers};

use super::layout::View;
use crate::fleet::FleetConnection;
use crate::look::{token, ActiveLook as _, Look};
use arrange::{Arranged, Dir, Hover, Rect};
use trail::{Leave, Level, Trail};
use tree::Listing;

pub use arrange::Rect as PoseRect;

/// The block is full width on every board, and this tall.
pub const BOARD_COLUMNS: u8 = super::layout::COLUMNS;
pub const BOARD_ROWS: u8 = 4;

/// Where each draft the block keeps is kept.
const KEEP_FEATURE: &str = "system.feature";
const KEEP_REWORK: &str = "system.rework";

/// What a read of Fleet has come to.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Read<T> {
    Reading,
    Ready(T),
    Failed(String),
}

impl<T> Read<T> {
    pub(super) fn ready(&self) -> Option<&T> {
        match self {
            Read::Ready(one) => Some(one),
            _ => None,
        }
    }
}

/// What the block tells the board it sits on.
pub enum SystemEvent {
    /// A feature was pressed: the Features tab opens it.
    Feature(String),
    /// A Job was pressed, or one was started: it opens.
    Job(String),
    /// Fleet kept the Job a draft started on the Operator's desk, which the board's toast tells of.
    Held { id: String, words: String },
}

impl EventEmitter<SystemEvent> for SystemBlock {}

pub enum Source {
    Live(Entity<FleetConnection>),
    Mock,
}

/// A level of the map, as Fleet is asked for it: the part zoomed into and the feature whose path is drawn.
pub(super) type Asked = (Option<String>, Option<String>);

/// A box he types a sentence into, over the panel.
pub(super) enum Compose {
    /// A new feature, started as a Job where the panel is looking.
    Feature { input: Entity<TextareaState>, _typed: Subscription },
    /// A note for the librarian about a part it added.
    Rework { addition: String, input: Entity<TextareaState>, _typed: Subscription },
}

/// The block growing to fill the board, or going back to its place.
#[derive(Clone, Copy)]
struct Motion {
    growing: bool,
    gen: u64,
}

/// Where the block stands on the board this frame, for the board to place its frame by.
pub struct Overlay {
    pub from: Rect,
    pub to: Rect,
    pub gen: u64,
    pub moving: bool,
}

/// What the arrangement was made for, so it is made again only when one of them changes.
#[derive(Clone, PartialEq)]
struct LaidFor {
    asked: Asked,
    size: (i32, i32),
    gen: u64,
}

pub struct SystemBlock {
    source: Source,
    project: Option<String>,
    name: String,
    manifests: Vec<String>,
    port: Option<u16>,
    pub(super) view: View,
    pub(super) trail: Trail,
    grown: bool,
    motion: Option<Motion>,
    /// Counts the levels and the growings, so an animation that has played does not play again.
    gen: u64,
    pub(super) maps: HashMap<Asked, Read<SystemMap>>,
    pub(super) answers: HashMap<String, Read<SystemPartAnswers>>,
    pub(super) listings: HashMap<String, Listing>,
    /// The folders he opened or shut by hand, by the part whose tree they are in.
    pub(super) toggled: HashMap<String, HashMap<String, bool>>,
    pub(super) selected: Option<String>,
    pub(super) hover: Hover,
    pub(super) compose: Option<Compose>,
    pub(super) asking_librarian: bool,
    pub(super) starting: bool,
    pub(super) focus: FocusHandle,
    /// Where the graph's room was drawn this frame, so the arrangement fits it.
    stage: Rc<Cell<Option<Bounds<Pixels>>>>,
    laid: Option<(LaidFor, Arranged)>,
    grid_seen: Rc<Cell<Option<Bounds<Pixels>>>>,
    root_seen: Rc<Cell<Option<Bounds<Pixels>>>>,
    tasks: HashMap<&'static str, Task<()>>,
    /// What a mock block was asked to do, for a test to read.
    pub(super) opened: Vec<String>,
    pub(super) started: Vec<String>,
    pub(super) answered: Vec<AnswerSystemAddition>,
}

impl SystemBlock {
    pub fn live(fleet: &Entity<FleetConnection>, cx: &mut Context<Self>) -> Self {
        Self::new(Source::Live(fleet.clone()), cx)
    }

    pub fn mock(cx: &mut Context<Self>) -> Self {
        Self::new(Source::Mock, cx)
    }

    fn new(source: Source, cx: &mut Context<Self>) -> Self {
        SystemBlock {
            source,
            project: None,
            name: String::new(),
            manifests: Vec::new(),
            port: None,
            view: View::Graph,
            trail: Trail::default(),
            grown: false,
            motion: None,
            gen: 0,
            maps: HashMap::new(),
            answers: HashMap::new(),
            listings: HashMap::new(),
            toggled: HashMap::new(),
            selected: None,
            hover: Hover::None,
            compose: None,
            asking_librarian: false,
            starting: false,
            focus: cx.focus_handle().tab_stop(true),
            stage: Rc::default(),
            laid: None,
            grid_seen: Rc::default(),
            root_seen: Rc::default(),
            tasks: HashMap::new(),
            opened: Vec::new(),
            started: Vec::new(),
            answered: Vec::new(),
        }
    }

    // ---- which project -------------------------------------------------------------------------

    fn port_now(&self, cx: &App) -> Option<u16> {
        match &self.source {
            Source::Live(fleet) => fleet.read(cx).held().port(),
            Source::Mock => None,
        }
    }

    /// The board is on `project`: the block reads its map, from level one, and forgets what it held of another.
    pub fn follow(&mut self, project: &str, name: &str, manifests: Vec<String>, cx: &mut Context<Self>) {
        let port = self.port_now(cx);
        self.name = name.to_string();
        self.manifests = manifests;
        if self.project.as_deref() == Some(project) && self.port == port {
            return;
        }
        let changed = self.project.as_deref() != Some(project);
        self.project = Some(project.to_string());
        self.port = port;
        if changed {
            self.trail = Trail::default();
            self.grown = false;
            self.motion = None;
            self.selected = None;
            self.hover = Hover::None;
            self.compose = None;
            self.listings.clear();
            self.toggled.clear();
            self.answers.clear();
        }
        self.maps.clear();
        self.laid = None;
        self.ask_level(cx);
        cx.notify();
    }

    // ---- reading -------------------------------------------------------------------------------

    fn asked(&self) -> Asked {
        (self.trail.part().map(str::to_string), self.trail.feature().map(str::to_string))
    }

    /// The level on screen, where Fleet has said.
    pub(super) fn level(&self) -> Option<&SystemMap> {
        self.maps.get(&self.asked()).and_then(Read::ready)
    }

    pub(super) fn level_read(&self) -> Option<&Read<SystemMap>> {
        self.maps.get(&self.asked())
    }

    /// Level one, which the names of the parts a deeper level is in are read from.
    pub(super) fn top(&self) -> Option<&SystemMap> {
        self.maps.get(&(None, None)).and_then(Read::ready)
    }

    fn ask_level(&mut self, cx: &mut Context<Self>) {
        let asked = self.asked();
        self.ask(asked, cx);
        if let Some(part) = self.panel_part() {
            self.ask_answers(&part, cx);
        }
    }

    fn ask(&mut self, asked: Asked, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else { return };
        if matches!(self.maps.get(&asked), Some(Read::Ready(_) | Read::Reading)) {
            return;
        }
        match &self.source {
            Source::Mock => {
                self.maps.insert(asked.clone(), Read::Ready(mock::map(&project, asked.0.as_deref(), asked.1.as_deref())));
            }
            Source::Live(_) => {
                let Some(port) = self.port else { return };
                self.maps.insert(asked.clone(), Read::Reading);
                let task = wire::system(port, &project, asked.0.as_deref(), asked.1.as_deref(), cx);
                self.tasks.insert(
                    "level",
                    cx.spawn(async move |this, cx| {
                        let answer = task.await;
                        let _ = this.update(cx, |this, cx| {
                            if this.project.as_deref() == Some(project.as_str()) {
                                this.maps.insert(asked, match answer {
                                    Ok(map) => Read::Ready(map),
                                    Err(why) => Read::Failed(why.said),
                                });
                                cx.notify();
                            }
                        });
                    }),
                );
            }
        }
    }

    fn ask_answers(&mut self, part: &str, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else { return };
        if matches!(self.answers.get(part), Some(Read::Ready(_) | Read::Reading)) {
            return;
        }
        match &self.source {
            Source::Mock => {
                self.answers.insert(part.to_string(), Read::Ready(mock::answers(&project, part)));
            }
            Source::Live(_) => {
                let Some(port) = self.port else { return };
                self.answers.insert(part.to_string(), Read::Reading);
                let task = wire::system_part(port, &project, part, cx);
                let asked = part.to_string();
                self.tasks.insert(
                    "answers",
                    cx.spawn(async move |this, cx| {
                        let answer = task.await;
                        let _ = this.update(cx, |this, cx| {
                            if this.project.as_deref() == Some(project.as_str()) {
                                this.answers.insert(asked, match answer {
                                    Ok(answers) => Read::Ready(answers),
                                    Err(why) => Read::Failed(why.said),
                                });
                                cx.notify();
                            }
                        });
                    }),
                );
            }
        }
    }

    /// The part the panel is about: the one selected, else the one the block is zoomed into.
    pub(super) fn panel_part(&self) -> Option<String> {
        self.selected.clone().or_else(|| self.trail.part().map(str::to_string))
    }

    // ---- looking at a part ---------------------------------------------------------------------

    pub(super) fn select(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        if self.selected == id {
            return;
        }
        self.selected = id;
        self.compose = None;
        if let Some(part) = self.panel_part() {
            self.ask_answers(&part, cx);
        }
        cx.notify();
    }

    /// An arrow key: the selection goes to the nearest part that way, and starts at the top left one where none is selected.
    fn move_selection(&mut self, dir: Dir, cx: &mut Context<Self>) {
        let Some((_, arranged)) = self.laid.as_ref() else { return };
        let at = self.selected.as_deref().and_then(|id| arranged.index_of(id));
        let to = match at {
            Some(at) => arrange::toward(&arranged.nodes, at, dir),
            None => arrange::first(&arranged.nodes),
        };
        if let Some(id) = to.map(|to| arranged.nodes[to].id.clone()) {
            self.select(Some(id), cx);
        }
    }

    pub(super) fn set_hover(&mut self, hover: Hover, cx: &mut Context<Self>) {
        if self.hover != hover {
            self.hover = hover;
            cx.notify();
        }
    }

    // ---- zooming -------------------------------------------------------------------------------

    fn level_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.gen += 1;
        self.selected = None;
        self.hover = Hover::None;
        self.compose = None;
        self.laid = None;
        self.ask_level(cx);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Return, a double press, and Look inside: into the part selected, or on level two into a feature's path.
    pub(super) fn look_inside(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.selected.clone() else { return };
        let Some(part) = self.level().and_then(|map| map.parts.iter().find(|one| one.id == id)).cloned() else { return };
        if !part.zooms || !self.trail.zoom_part(&part.id, &part.name) {
            return;
        }
        self.grow(true, cx);
        self.level_changed(window, cx);
    }

    /// A feature of level two: its path is drawn.
    pub(super) fn follow_feature(&mut self, slug: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(title) = self.level().and_then(|map| map.features.iter().find(|one| one.slug == slug)).map(|one| one.title.clone()) else { return };
        if self.trail.zoom_feature(slug, &title) {
            self.level_changed(window, cx);
        }
    }

    /// Esc: one level out, and at level one a block that grew goes back to its place.
    pub(super) fn step_out(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match trail::leave(&self.trail, self.grown) {
            Leave::Out => {
                self.trail.up();
                self.level_changed(window, cx);
                true
            }
            Leave::Shrink => {
                self.grow(false, cx);
                cx.notify();
                true
            }
            Leave::Nothing => false,
        }
    }

    /// A crumb: out to its level.
    pub(super) fn out_to(&mut self, depth: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.trail.out_to(depth) {
            self.level_changed(window, cx);
        }
    }

    fn grow(&mut self, growing: bool, cx: &mut Context<Self>) {
        if self.grown == growing {
            return;
        }
        self.grown = growing;
        self.gen += 1;
        self.motion = Some(Motion { growing, gen: self.gen });
        let wait = cx.look().duration(token::DURATION_TRAVEL);
        self.tasks.insert(
            "motion",
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(wait + Duration::from_millis(40)).await;
                let _ = this.update(cx, |this, cx| {
                    this.motion = None;
                    cx.notify();
                });
            }),
        );
    }

    pub fn is_grown(&self) -> bool {
        self.grown
    }

    /// Whether the block is drawn over the whole board, now or while it moves.
    pub fn over_board(&self) -> bool {
        self.grown || self.motion.is_some()
    }

    pub fn set_view(&mut self, view: View) {
        self.view = view;
    }

    /// Where the block's frame stands, as the board places it: `placed` is its cells in the grid. None where it stands in them.
    pub fn overlay(&self, placed: Rect, fallback: (f32, f32)) -> Option<Overlay> {
        if !self.over_board() {
            return None;
        }
        let root = self.root_seen.get();
        let full = root.map_or(Rect { x: 0., y: 0., w: fallback.0, h: fallback.1 }, |bounds| Rect { x: 0., y: 0., w: f32::from(bounds.size.width), h: f32::from(bounds.size.height) });
        let origin = match (self.grid_seen.get(), root) {
            (Some(grid), Some(root)) => (f32::from(grid.origin.x - root.origin.x), f32::from(grid.origin.y - root.origin.y)),
            _ => (0., 0.),
        };
        let home = Rect { x: placed.x + origin.0, y: placed.y + origin.1, ..placed };
        Some(match self.motion {
            Some(Motion { growing: true, gen }) => Overlay { from: home, to: full, gen, moving: true },
            Some(Motion { growing: false, gen }) => Overlay { from: full, to: home, gen, moving: true },
            None => Overlay { from: full, to: full, gen: self.gen, moving: false },
        })
    }

    /// A canvas the board lays over its root, which records how big the room the block grows into is.
    pub fn measure_root(&self) -> AnyElement {
        let seen = self.root_seen.clone();
        canvas(move |bounds, _, _| seen.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full().into_any_element()
    }

    /// The same over the grid the block stands in, so its place can be told in the board's own coordinates.
    pub fn measure_grid(&self) -> AnyElement {
        let seen = self.grid_seen.clone();
        canvas(move |bounds, _, _| seen.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full().into_any_element()
    }

    // ---- the keys ------------------------------------------------------------------------------

    pub(super) fn on_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let plain = !event.keystroke.modifiers.platform && !event.keystroke.modifiers.control && !event.keystroke.modifiers.alt && !event.keystroke.modifiers.shift;
        let on_graph = self.focus.is_focused(window);
        let handled = match key {
            "escape" if plain => self.step_out(window, cx),
            "enter" if plain && on_graph => {
                self.look_inside(window, cx);
                self.selected.is_some()
            }
            "left" | "right" | "up" | "down" if plain && on_graph && self.view == View::Graph => {
                let dir = match key {
                    "left" => Dir::Left,
                    "right" => Dir::Right,
                    "up" => Dir::Up,
                    _ => Dir::Down,
                };
                self.move_selection(dir, cx);
                true
            }
            _ => false,
        };
        if handled {
            cx.stop_propagation();
        }
    }

    // ---- the tree -------------------------------------------------------------------------------

    /// A folder row of the Code section was pressed: it opens or shuts, and a folder not yet listed is asked for.
    pub(super) fn toggle_folder(&mut self, part: &str, row: &tree::Row, cx: &mut Context<Self>) {
        let want = !row.open;
        self.toggled.entry(part.to_string()).or_default().insert(row.path.clone(), want);
        let known = self.answers.get(part).and_then(Read::ready).is_some_and(|answers| answers.code.iter().any(|path| path.trim_end_matches('/').starts_with(&format!("{}/", row.path)) && path.trim_end_matches('/') != row.path));
        if want && !known && !self.listings.contains_key(&row.path) {
            self.ask_folder(&row.path, cx);
        }
        cx.notify();
    }

    fn ask_folder(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else { return };
        match &self.source {
            Source::Mock => {
                self.listings.insert(path.to_string(), Listing::Names(mock::folder(&project, path).entries));
            }
            Source::Live(_) => {
                let Some(port) = self.port else { return };
                self.listings.insert(path.to_string(), Listing::Reading);
                let task = wire::system_folder(port, &project, &format!("{path}/"), cx);
                let asked = path.to_string();
                // One task per folder, so a second press does not cancel the first; each ends by writing its own answer.
                cx.spawn(async move |this, cx| {
                    let answer = task.await;
                    let _ = this.update(cx, |this, cx| {
                        this.listings.insert(asked, match answer {
                            Ok(folder) => Listing::Names(folder.entries),
                            Err(why) => Listing::Failed(why.said),
                        });
                        cx.notify();
                    });
                })
                .detach();
            }
        }
    }

    // ---- the buttons of the panel ----------------------------------------------------------------

    /// The root of the repository a part is in, where Fleet has said where it is.
    fn root_of(&self, part: Option<&ipc::SystemPart>, cx: &App) -> Option<String> {
        let Source::Live(fleet) = &self.source else { return None };
        let held = fleet.read(cx).held();
        let repositories = held.repositories.as_deref().unwrap_or_default();
        let id_of = |one: &ipc::RepositorySummary| one.manifest.as_ref().map(|manifest| manifest.id.as_str().to_string()).or_else(|| one.implicit_id.as_ref().map(|id| id.as_str().to_string()));
        let wanted = part.and_then(|part| part.manifest_id.as_ref()).map(|id| id.as_str().to_string()).or_else(|| self.manifests.first().cloned());
        repositories.iter().find(|one| id_of(one) == wanted).or_else(|| repositories.first()).map(|one| one.root.clone())
    }

    /// Open folder: the part's first path, in Finder.
    pub(super) fn open_folder(&mut self, id: &str, cx: &mut Context<Self>) {
        let part = self.level().and_then(|map| map.parts.iter().find(|one| one.id == id)).or_else(|| self.top().and_then(|map| map.parts.iter().find(|one| one.id == id))).cloned();
        let Some(path) = part.as_ref().and_then(|part| part.paths.first()).cloned() else { return };
        self.open_path(&path, part.as_ref(), cx);
    }

    /// A repository-relative path opens with the app macOS names for it.
    pub(super) fn open_path(&mut self, path: &str, part: Option<&ipc::SystemPart>, cx: &mut Context<Self>) {
        if matches!(self.source, Source::Mock) {
            self.opened.push(path.to_string());
            return;
        }
        let Some(root) = self.root_of(part, cx) else { return };
        let full = std::path::Path::new(&root).join(path.trim_end_matches('/'));
        cx.background_executor()
            .spawn(async move {
                let _ = std::process::Command::new("open").arg(full).status();
            })
            .detach();
    }

    pub(super) fn open_feature(&mut self, slug: &str, cx: &mut Context<Self>) {
        cx.emit(SystemEvent::Feature(slug.to_string()));
    }

    pub(super) fn open_job(&mut self, id: &str, cx: &mut Context<Self>) {
        cx.emit(SystemEvent::Job(id.to_string()));
    }

    // ---- the boxes ------------------------------------------------------------------------------

    pub(super) fn close_compose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.compose.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    /// New feature…: a box opens for a sentence, with the draft the last one left.
    pub(super) fn open_feature_box(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(Compose::Feature { input, .. }) = &self.compose {
            input.update(cx, |input, cx| input.focus(window, cx));
            return;
        }
        let key = format!("{KEEP_FEATURE}.{}.{}", self.project.clone().unwrap_or_default(), self.panel_part().unwrap_or_default());
        let (input, typed) = self.box_for(key, "What should the feature do?", window, cx);
        self.compose = Some(Compose::Feature { input, _typed: typed });
        cx.notify();
    }

    /// Work with the librarian: a box opens for the note, with the draft the last one left.
    pub(super) fn open_rework_box(&mut self, addition: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(Compose::Rework { input, .. }) = &self.compose {
            input.update(cx, |input, cx| input.focus(window, cx));
            return;
        }
        let key = format!("{KEEP_REWORK}.{}.{addition}", self.project.clone().unwrap_or_default());
        let (input, typed) = self.box_for(key, "What should the librarian change?", window, cx);
        self.compose = Some(Compose::Rework { addition: addition.to_string(), input, _typed: typed });
        cx.notify();
    }

    /// A text box that keeps what is typed in it under `key`, and has the keys.
    fn box_for(&mut self, key: String, hint: &'static str, window: &mut Window, cx: &mut Context<Self>) -> (Entity<TextareaState>, Subscription) {
        let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(2, 6).placeholder(hint));
        if let Some(words) = crate::drafts::kept(&key, cx) {
            input.update(cx, |input, cx| input.set_value(words, window, cx));
        }
        input.update(cx, |input, cx| input.focus(window, cx));
        let typed = cx.subscribe_in(&input, window, move |_, input, event: &InputEvent, _, cx| {
            if let InputEvent::Change = event {
                let text = input.read(cx).value().to_string();
                if text.trim().is_empty() {
                    crate::drafts::clear(&key, cx);
                } else {
                    crate::drafts::keep(&key, &text, cx);
                }
                cx.notify();
            }
        });
        (input, typed)
    }

    /// The words in the open box.
    pub(super) fn words(&self, cx: &App) -> String {
        match &self.compose {
            Some(Compose::Feature { input, .. } | Compose::Rework { input, .. }) => input.read(cx).value().to_string(),
            None => String::new(),
        }
    }

    // ---- what a press sends ----------------------------------------------------------------------

    /// Start a Job on the sentence in the box, in the repository the part is in. The Job opens, and a refusal is a toast.
    pub(super) fn start_job(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let words = self.words(cx).trim().to_string();
        if words.is_empty() || self.starting || !matches!(self.compose, Some(Compose::Feature { .. })) {
            return;
        }
        let key = format!("{KEEP_FEATURE}.{}.{}", self.project.clone().unwrap_or_default(), self.panel_part().unwrap_or_default());
        let at = self.panel_part().and_then(|id| self.level().and_then(|map| map.parts.iter().find(|one| one.id == id)).or_else(|| self.top().and_then(|map| map.parts.iter().find(|one| one.id == id))).cloned());
        let place = at.as_ref().map_or_else(|| self.name.clone(), |part| part.name.clone());
        let request = format!("New feature, beginning at {place}: {words}");
        if matches!(self.source, Source::Mock) {
            self.started.push(request);
            crate::drafts::clear(&key, cx);
            self.compose = None;
            window.focus(&self.focus, cx);
            cx.notify();
            return;
        }
        let Some(port) = self.port else { return };
        let manifest = at.as_ref().and_then(|part| part.manifest_id.clone()).or_else(|| self.manifests.first().map(|id| ipc::ManifestId::carried(id.clone())));
        let Some(manifest) = manifest else { return };
        let project = self.project.clone();
        self.starting = true;
        let task = cx.background_executor().spawn(async move {
            let start = ipc::StartSession {
                manifest_id: manifest,
                title: None,
                model: None,
                effort: None,
                mode: None,
                fork: None,
                root: None,
                request: Some(request.clone()),
                after: None,
                held: false,
                project_rows: Vec::new(),
                on_your_words: None,
                project,
            };
            let job = ipc::DispatchJob { start, kind: None, then: Vec::new() };
            let body = ipc::encode(&job).map_err(|why| crate::refusal::Failed::from(why.to_string()))?;
            let bytes = crate::dispatch::live::ask(port, "/sessions/dispatch".into(), Some(body))?;
            let done = ipc::decode::<ipc::JobDispatched>("JobDispatched", &bytes).map_err(|why| crate::refusal::Failed::from(why.to_string()))?;
            Ok::<_, crate::refusal::Failed>((done.session.map(|record| record.id.as_str().to_string()), done.operator.or(done.parked), request))
        });
        self.tasks.insert(
            "start",
            cx.spawn_in(window, async move |this, cx| {
                let answer = task.await;
                let _ = this.update_in(cx, |this, window, cx| {
                    this.starting = false;
                    match answer {
                        Ok((job, held, words)) => {
                            crate::drafts::clear(&key, cx);
                            this.compose = None;
                            window.focus(&this.focus, cx);
                            match (job, held) {
                                (Some(id), _) => cx.emit(SystemEvent::Job(id)),
                                (None, Some(id)) => cx.emit(SystemEvent::Held { id, words }),
                                (None, None) => {}
                            }
                        }
                        Err(failed) => failed.tell("The Job was not started", cx),
                    }
                    cx.notify();
                });
            }),
        );
        cx.notify();
    }

    /// Ask the librarian: it reads the project again and its additions come back as dashed parts.
    pub(super) fn ask_librarian(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else { return };
        if self.asking_librarian {
            return;
        }
        if matches!(self.source, Source::Mock) {
            self.maps.insert((None, None), Read::Ready(mock::librarian_adds(&project)));
            self.laid = None;
            cx.notify();
            return;
        }
        let Some(port) = self.port else { return };
        self.asking_librarian = true;
        let task = wire::system_librarian(port, &project, cx);
        self.take_level_one("librarian", task, project, cx);
        cx.notify();
    }

    /// Keep, Remove, and the note of Work with the librarian: what he chose for the part it added.
    pub(super) fn answer_addition(&mut self, addition: &str, answer: AdditionAnswer, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else { return };
        let sent = AnswerSystemAddition { addition: addition.to_string(), answer };
        let rework = matches!(sent.answer, AdditionAnswer::Rework { .. });
        if let (true, Some(Compose::Rework { addition, .. })) = (rework, &self.compose) {
            crate::drafts::clear(&format!("{KEEP_REWORK}.{project}.{addition}"), cx);
        }
        if matches!(self.source, Source::Mock) {
            if let (Some(Read::Ready(map)), AdditionAnswer::Keep) = (self.maps.get_mut(&(None, None)), &sent.answer) {
                for part in &mut map.parts {
                    if matches!(&part.origin, ipc::PartOrigin::Librarian { addition, .. } if addition == &sent.addition) {
                        part.origin = ipc::PartOrigin::Detected { from: "the librarian".into() };
                    }
                }
            }
            if let (Some(Read::Ready(map)), AdditionAnswer::Remove) = (self.maps.get_mut(&(None, None)), &sent.answer) {
                let gone: Vec<String> = map.parts.iter().filter(|part| matches!(&part.origin, ipc::PartOrigin::Librarian { addition, .. } if addition == &sent.addition)).map(|part| part.id.clone()).collect();
                map.parts.retain(|part| !gone.contains(&part.id));
                map.links.retain(|link| !gone.contains(&link.from) && !gone.contains(&link.to));
                if self.selected.as_ref().is_some_and(|id| gone.contains(id)) {
                    self.selected = None;
                }
            }
            self.answered.push(sent);
            self.compose = None;
            self.laid = None;
            window.focus(&self.focus, cx);
            cx.notify();
            return;
        }
        let Some(port) = self.port else { return };
        let task = wire::system_addition(port, &project, &sent, cx);
        self.take_level_one("addition", task, project, cx);
        self.compose = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// An answer that is the whole of level one again: it replaces what was held, and what was held under it is read again.
    fn take_level_one(&mut self, slot: &'static str, task: Task<Result<SystemMap, crate::settings::wire::Refused>>, project: String, cx: &mut Context<Self>) {
        self.tasks.insert(
            slot,
            cx.spawn(async move |this, cx| {
                let answer = task.await;
                let _ = this.update(cx, |this, cx| {
                    this.asking_librarian = false;
                    if this.project.as_deref() != Some(project.as_str()) {
                        return;
                    }
                    match answer {
                        Ok(map) => {
                            this.maps.retain(|asked, _| *asked == (None, None));
                            this.maps.insert((None, None), Read::Ready(map));
                            this.answers.clear();
                            this.laid = None;
                            this.ask_level(cx);
                        }
                        Err(why) => crate::toasts::tell(why.said, cx),
                    }
                    cx.notify();
                });
            }),
        );
    }

    // ---- what the graph is laid out for ------------------------------------------------------------

    /// The arrangement of the level on screen, made again when the level or the room changes.
    pub(super) fn arranged(&mut self, look: &Look, size: (f32, f32)) -> Option<&Arranged> {
        let asked = self.asked();
        let key = LaidFor { asked: asked.clone(), size: (size.0 as i32, size.1 as i32), gen: self.gen };
        if self.laid.as_ref().is_none_or(|(held, _)| *held != key) {
            let map = self.maps.get(&asked).and_then(Read::ready)?;
            let top = self.maps.get(&(None, None)).and_then(Read::ready);
            let names: HashMap<&str, &str> = top.iter().flat_map(|top| top.parts.iter()).chain(map.parts.iter()).map(|part| (part.id.as_str(), part.name.as_str())).collect();
            let title_of = |id: &str| names.get(id).map(|name| name.to_string());
            let arranged = arrange::arrange(map, self.trail.depth(), &self.name, &title_of, &graph::metrics(look), size.0, size.1);
            self.laid = Some((key, arranged));
        }
        self.laid.as_ref().map(|(_, arranged)| arranged)
    }

    pub(super) fn gen(&self) -> u64 {
        self.gen
    }

    pub(super) fn stage_seen(&self) -> Rc<Cell<Option<Bounds<Pixels>>>> {
        self.stage.clone()
    }

    /// The part the keys and the panel are on, as the level draws it.
    pub(super) fn part_selected(&self) -> Option<&ipc::SystemPart> {
        let id = self.selected.as_deref()?;
        self.level()?.parts.iter().find(|part| part.id == id)
    }

    pub(super) fn here(&self) -> &Level {
        self.trail.here()
    }

    pub(super) fn project_name(&self) -> &str {
        &self.name
    }
}

impl Render for SystemBlock {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        graph::draw(self, window, cx)
    }
}

/// A frame the board has made for the block, set at the place the overlay says, and moved there from where it was.
pub fn posed(frame: AnyElement, overlay: Option<Overlay>, look: &Look) -> AnyElement {
    let Some(overlay) = overlay else { return frame };
    let place = |element: gpui::Div, rect: Rect| element.absolute().left(gpui::px(rect.x)).top(gpui::px(rect.y)).w(gpui::px(rect.w)).h(gpui::px(rect.h));
    let outer = place(div(), overlay.to).child(frame);
    if !overlay.moving {
        return outer.into_any_element();
    }
    let (from, to) = (overlay.from, overlay.to);
    let lerp = move |a: f32, b: f32, t: f32| a + (b - a) * t;
    outer
        .with_animation(
            gpui::ElementId::from(("system-pose", overlay.gen as usize)),
            gpui::Animation::new(look.duration(token::DURATION_TRAVEL)).with_easing(gpui::ease_out_quint()),
            move |element, t| {
                place(element, Rect { x: lerp(from.x, to.x, t), y: lerp(from.y, to.y, t), w: lerp(from.w, to.w, t), h: lerp(from.h, to.h, t) })
            },
        )
        .into_any_element()
}

use gpui::AnimationExt as _;
