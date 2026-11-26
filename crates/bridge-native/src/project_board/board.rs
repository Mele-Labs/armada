//! The Dashboard tab's state: which project, where its blocks stand, what each block has read, and what a press on a block asks.
//! What a block draws is worked out in `model.rs` and drawn in `blocks.rs`; this is the entity they read.
//!
//! Every read runs off the main thread and lands through one `took` of its own, so a batch of answers draws once each. The
//! Fleet connection is the screen's, never the board's: it reads Jobs from what that connection holds and asks the routes it
//! needs on its port.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;

use chrono::Utc;
use gpui::{AnyWindowHandle, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Subscription, Task, Window};
use gpui_component::input::{InputEvent, InputState};
use ipc::{AppRunDraft, AppRunList, AppRunState, BoardBlock, BoardLayout, ManifestConfig, MetricReading, MetricReadings, MetricState, ProjectOverview, RankedIssues, SinceYouLooked, TrackedIssue};

use super::layout::{BlockKind, Layout, View, COLUMNS};
use super::model::{self, Fill, Item, Press, Project, Repository, Shown};
use super::presets::{self, Kind};
use super::system::{SystemBlock, SystemEvent};
use super::{metric, mock, wire};
use crate::features::chosen;
use crate::fleet::FleetConnection;
use crate::reviews::ReviewsScreen;
use crate::titlebar;

/// How long the board stands still after a change before it is sent to Fleet, so a drag or a run of arrow keys is one save.
pub(super) const SAVE_AFTER: Duration = Duration::from_millis(600);

/// How many lines of a server's output the Run block keeps.
const TAIL_LINES: usize = 80;

/// How often the runs are read again while the board is in front: a process starts and stops without Bridge asking.
const RUNS_EVERY: Duration = Duration::from_secs(5);

/// How often the metrics are asked for while the dashboard is in front. Fleet decides whether a reading is stale; this is only
/// how often Bridge asks.
const METRICS_EVERY: Duration = Duration::from_secs(15);

/// The most repositories of one project the board reads for, so a project with many does not ask for every one.
const MOST_REPOSITORIES: usize = 4;

pub enum Source {
    Live(Entity<FleetConnection>),
    Mock,
}

/// What the board tells the screen it sits in.
pub enum BoardEvent {
    /// A feature was pressed: the Features tab opens it.
    Feature(String),
    /// "Fix with the librarian" was pressed on a metric block that is broken: the screen puts the words in the Ask box.
    FixMetric { id: String, title: String },
}

impl EventEmitter<BoardEvent> for Board {}

/// The checkout the Run block and Start act on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checkout {
    Main,
    /// A Job's worktree, by the Job and the repository it works in.
    Job { id: String, manifest: String, label: String },
}

impl Checkout {
    pub fn job(&self) -> Option<&str> {
        match self {
            Checkout::Main => None,
            Checkout::Job { id, .. } => Some(id),
        }
    }
}

/// A project's board as it stands on this Mac, and what of it Fleet has not been told.
struct Stored {
    layout: Layout,
    /// He changed it, or Fleet kept one: the preset no longer applies when the project's kind is read.
    edited: bool,
    /// The blocks Fleet kept of kinds this build does not draw, sent back as they came.
    unknown: Vec<BoardBlock>,
    /// Counts the changes, so an answer to a save tells whether a later change is still to go.
    rev: u32,
}

impl Stored {
    fn preset(kind: Kind, rev: u32) -> Stored {
        Stored { layout: presets::preset(kind), edited: false, unknown: Vec::new(), rev }
    }
}

/// The issues the Issues block draws from: his own, and the ones Fleet ranked with the pull requests of others.
pub struct IssuesRead {
    pub mine: Vec<TrackedIssue>,
    pub ranked: RankedIssues,
}

/// Where Rank with the librarian stands: asked, or refused with Fleet's line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Ranking {
    #[default]
    Idle,
    Waiting,
    Refused(String),
}

/// A Command flagged `confirm: true` that has been pressed and waits for Run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asking {
    pub manifest: String,
    pub name: String,
    /// The repository it is in, said only where the project has more than one.
    pub repository: Option<String>,
}

/// The tail of one server's output, under the Run block's rows.
pub struct Tail {
    pub title: String,
    pub server: String,
    pub lines: Option<Result<Vec<String>, String>>,
}

/// The field a token is pasted in, for a recipe whose credential this Mac does not hold. **It is never kept as a draft**: a
/// secret is not persisted by Bridge, so nothing here writes it anywhere, and the field is dropped once the token is sent.
pub(super) struct TokenField {
    pub input: Entity<InputState>,
    _sub: Subscription,
}

/// What a read was made for.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Key {
    port: Option<u16>,
    project: String,
    manifests: Vec<String>,
    checkout: Checkout,
}

pub struct Board {
    pub(super) source: Source,
    reviews: Option<Entity<ReviewsScreen>>,
    window: AnyWindowHandle,
    /// The project the board was last made ready for, so a change of project is told from a redraw.
    shown: Option<String>,
    layouts: HashMap<String, Stored>,
    /// The projects whose kept layout Fleet has answered for, or could not.
    layout_read: HashSet<String>,
    /// The projects whose layout changed here and has not been answered by a save.
    unsaved: HashSet<String>,
    /// A change waits for the board to stand still, then goes: one task for each project.
    saves: HashMap<String, Task<()>>,
    sending: HashMap<String, Task<()>>,
    /// What the mock Fleet keeps of each project's layout, for a board that has none of its own.
    pub(super) kept: HashMap<String, BoardLayout>,
    pub(super) edit: bool,
    /// A Command that asks before it runs, pressed and not answered; its keys are Run, then Cancel.
    pub(super) asking: Option<Asking>,
    pub(super) run_stop: FocusHandle,
    pub(super) cancel_stop: FocusHandle,
    /// What a mock board was told to run.
    pub(super) ran: Vec<(String, String)>,
    /// A focus handle for each block, so every block is a stop of the keys.
    pub(super) handles: Vec<(BlockKind, FocusHandle)>,
    /// Where each block's body is scrolled, by its element id, so a body that has rows to scroll keeps the wheel.
    pub(super) bodies: std::cell::RefCell<HashMap<String, gpui::ScrollHandle>>,
    pub(super) root: FocusHandle,
    /// The System block, which reads and draws its own map.
    pub(super) system: Entity<SystemBlock>,
    /// The cell the pointer was in, relative to the block's own corner, when the block was lifted.
    pub(super) grab: Option<(i16, i16)>,
    entered: bool,
    read_for: Option<Key>,
    pub(super) overview: Option<Result<ProjectOverview, String>>,
    pub(super) since: Option<Result<SinceYouLooked, String>>,
    pub(super) runs: BTreeMap<String, Result<AppRunList, String>>,
    pub(super) drafts: BTreeMap<String, Result<AppRunDraft, String>>,
    pub(super) configs: Option<Vec<Result<ManifestConfig, String>>>,
    pub(super) issues: Option<Result<IssuesRead, String>>,
    pub(super) ranking: Ranking,
    pub(super) checkout: Checkout,
    /// Presses in flight, so a second press on the same button waits for the first.
    pub(super) acting: HashSet<String>,
    pub(super) tail: Option<Tail>,
    /// What each recipe of the project on screen read, as Fleet last said.
    pub(super) metrics: Option<MetricReadings>,
    /// The paste field of each recipe that has no token, by recipe id.
    pub(super) token_fields: HashMap<String, TokenField>,
    /// The tokens on their way to Fleet, by recipe id.
    pub(super) saving_tokens: HashMap<String, Task<()>>,
    /// The Dashboard tab is the one in front of the Project place; the metrics are asked for only while it is.
    front: bool,
    tasks: HashMap<&'static str, Task<()>>,
    ticker: Option<Task<()>>,
    metric_ticker: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl Board {
    pub fn live(fleet: &Entity<FleetConnection>, reviews: Option<Entity<ReviewsScreen>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        chosen::persist(cx);
        let mut this = Self::new(Source::Live(fleet.clone()), reviews, window, cx);
        let watch = cx.observe(fleet, |this, _, cx| this.moved(cx));
        this._subs.push(watch);
        this
    }

    pub fn mock(reviews: Option<Entity<ReviewsScreen>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(Source::Mock, reviews, window, cx);
        this.fill_mock(cx);
        this
    }

    fn new(source: Source, reviews: Option<Entity<ReviewsScreen>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subs = Vec::new();
        if let Some(reviews) = &reviews {
            // The Reviews block draws what the Reviews screen holds, so it draws again when that does.
            subs.push(cx.observe(reviews, |_, _, cx| cx.notify()));
        }
        if let Some(page) = crate::operator::live(cx) {
            subs.push(cx.observe(&page, |_, _, cx| cx.notify()));
        }
        let system = match &source {
            Source::Live(fleet) => cx.new(|cx| SystemBlock::live(fleet, cx)),
            Source::Mock => cx.new(SystemBlock::mock),
        };
        subs.push(cx.observe(&system, |_, _, cx| cx.notify()));
        subs.push(cx.subscribe(&system, |this, _, event: &SystemEvent, cx| match event {
            SystemEvent::Feature(slug) => cx.emit(BoardEvent::Feature(slug.clone())),
            SystemEvent::Job(id) => this.press(Press::Job(id.clone()), cx),
            SystemEvent::Held { id, words } => crate::toasts::dispatch_held(id, words, cx),
        }));
        Board {
            source,
            reviews,
            window: window.window_handle(),
            shown: None,
            layouts: HashMap::new(),
            layout_read: HashSet::new(),
            unsaved: HashSet::new(),
            saves: HashMap::new(),
            sending: HashMap::new(),
            kept: HashMap::new(),
            edit: false,
            asking: None,
            run_stop: cx.focus_handle().tab_stop(true),
            cancel_stop: cx.focus_handle().tab_stop(true),
            ran: Vec::new(),
            handles: BlockKind::ALL.into_iter().map(|kind| (kind, cx.focus_handle().tab_stop(true))).collect(),
            bodies: Default::default(),
            root: cx.focus_handle(),
            system,
            grab: None,
            entered: false,
            read_for: None,
            overview: None,
            since: None,
            runs: BTreeMap::new(),
            drafts: BTreeMap::new(),
            configs: None,
            issues: None,
            ranking: Ranking::Idle,
            checkout: Checkout::Main,
            acting: HashSet::new(),
            tail: None,
            metrics: None,
            token_fields: HashMap::new(),
            saving_tokens: HashMap::new(),
            front: true,
            tasks: HashMap::new(),
            ticker: None,
            metric_ticker: None,
            _subs: subs,
        }
    }

    // ---- which project -----------------------------------------------------------------------

    /// Every project Fleet names, as `(id, name)` in Fleet's order.
    fn chips(&self, cx: &App) -> Vec<(String, String)> {
        match &self.source {
            Source::Live(fleet) => fleet.read(cx).held().projects.as_ref().map(|read| read.projects.iter().map(|one| (one.id.clone(), one.name.clone())).collect()).unwrap_or_default(),
            Source::Mock => crate::features::mock::projects(),
        }
    }

    /// The project on screen: the one the title bar is scoped to; on All, the one he chose last, else the first.
    pub fn project(&self, cx: &App) -> Option<Project> {
        match &self.source {
            Source::Mock => {
                let chips = self.chips(cx);
                let listed = |id: &String| chips.iter().any(|(one, _)| one == id);
                let scoped = titlebar::live(cx).and_then(|bar| bar.read(cx).face.scoped_project().map(str::to_string));
                let id = scoped.filter(listed).or_else(|| chosen::get(cx).filter(listed)).or_else(|| chips.first().map(|(id, _)| id.clone()))?;
                let name = chips.into_iter().find(|(one, _)| *one == id).map(|(_, name)| name)?;
                Some(mock::project(&id, &name))
            }
            Source::Live(fleet) => {
                let held = fleet.read(cx).held();
                let projects = held.projects.as_ref()?;
                let listed = |id: &String| projects.projects.iter().any(|one| one.id == *id);
                let scoped = titlebar::live(cx).and_then(|bar| bar.read(cx).face.scoped_project().map(str::to_string));
                let id = scoped.filter(listed).or_else(|| chosen::get(cx).filter(listed)).or_else(|| projects.projects.first().map(|one| one.id.clone()))?;
                let project = projects.projects.iter().find(|one| one.id == id)?;
                let served = held.repositories.as_deref().unwrap_or_default();
                let named = titlebar::repositories_of(served, &[]);
                let id_of = |one: &ipc::RepositorySummary| one.manifest.as_ref().map(|manifest| manifest.id.clone()).or_else(|| one.implicit_id.clone()).map(|id| id.as_str().to_string());
                let names: HashMap<String, String> = served.iter().zip(named).filter_map(|(one, named)| Some((id_of(one)?, named.name.to_string()))).collect();
                let mut repositories: Vec<Repository> = Vec::new();
                for member in &project.members {
                    let id = member.manifest_id.as_str().to_string();
                    if !repositories.iter().any(|one| one.id == id) {
                        repositories.push(Repository { name: names.get(&id).cloned().unwrap_or_else(|| id.clone()), id });
                    }
                }
                Some(Project { id: project.id.clone(), name: project.name.clone(), repositories })
            }
        }
    }

    /// The title bar's pick changed under the screen: the board is of the project now on screen and reads for it.
    pub fn follow(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx).map(|one| one.id) else { return };
        if self.shown.as_deref() == Some(project.as_str()) {
            return;
        }
        // The first project is the one the board opened on, so a checkout already chosen stays.
        let changed = self.shown.replace(project).is_some();
        if changed {
            self.checkout = Checkout::Main;
            self.tail = None;
        }
        if matches!(self.source, Source::Mock) {
            self.fill_mock(cx);
        } else if changed && self.entered {
            self.read_all(cx);
        }
        cx.notify();
    }

    // ---- the layout --------------------------------------------------------------------------

    /// The board of the project on screen: what Fleet kept, else the preset for its kind.
    /// On the preset, a recipe the project keeps is put at the bottom of the board. **A saved layout is left as it is**: Fleet puts
    /// a new recipe's block on it when the recipe is saved, so a recipe with no block there was taken off, and stays off.
    pub fn layout(&mut self, project: &str) -> &Layout {
        let kind = self.kind();
        let new: Vec<BlockKind> = match &self.metrics {
            Some(read) if read.project == project => read.readings.iter().map(|one| BlockKind::metric(&one.id)).collect(),
            _ => Vec::new(),
        };
        let stored = self.layouts.entry(project.to_string()).or_insert_with(|| Stored::preset(kind, 0));
        if !stored.edited {
            stored.layout.append_below(&new);
        }
        &stored.layout
    }

    /// The kinds that are not on the board of `project`: the fixed ones, then the recipes with no block.
    pub(super) fn addable(&mut self, project: &str) -> Vec<BlockKind> {
        let on: Vec<BlockKind> = self.layout(project).blocks.iter().map(|one| one.kind).collect();
        let mut all = self.layout(project).missing();
        if let Some(read) = self.metrics.as_ref().filter(|read| read.project == project) {
            all.extend(read.readings.iter().map(|one| BlockKind::metric(&one.id)).filter(|kind| !on.contains(kind)));
        }
        all
    }

    /// Whether the board of `project` is known: Fleet has said what it kept, so what is drawn is not a guess.
    pub(super) fn layout_ready(&self, project: &str) -> bool {
        matches!(self.source, Source::Mock) || self.layout_read.contains(project)
    }

    pub(super) fn kind(&self) -> Kind {
        match &self.overview {
            Some(Ok(read)) => presets::kind_of(read.rooms.iter().map(|room| room.kind.as_str())),
            _ => Kind::Unknown,
        }
    }

    /// Fleet said what `project` kept, or that it kept none. A board changed here and not yet sent is left as it is.
    pub(super) fn apply_kept(&mut self, project: &str, kept: Option<BoardLayout>, cx: &mut Context<Self>) {
        self.layout_read.insert(project.to_string());
        if self.unsaved.contains(project) {
            return;
        }
        let kind = self.kind();
        let rev = self.layouts.get(project).map_or(0, |stored| stored.rev);
        let stored = match kept {
            Some(kept) => {
                let (layout, unknown) = Layout::from_wire(&kept);
                Stored { layout, edited: true, unknown, rev }
            }
            None => Stored::preset(kind, rev),
        };
        self.layouts.insert(project.to_string(), stored);
        cx.notify();
    }

    /// Change the board of `project`; he has edited it from here on, so the preset no longer applies, and Fleet is told once
    /// the board has stood still.
    pub(super) fn edit_layout(&mut self, project: &str, change: impl FnOnce(&mut Layout), cx: &mut Context<Self>) {
        let kind = self.kind();
        let stored = self.layouts.entry(project.to_string()).or_insert_with(|| Stored::preset(kind, 0));
        let before = stored.layout.clone();
        change(&mut stored.layout);
        if stored.layout != before {
            stored.edited = true;
            stored.rev += 1;
            self.keep_soon(project, cx);
            cx.notify();
        }
    }

    /// Put the project's board back to the preset for its kind, and have Fleet forget the one it kept.
    pub fn reset_layout(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx) else { return };
        let kind = self.kind();
        let rev = self.layouts.get(&project.id).map_or(0, |stored| stored.rev) + 1;
        self.layouts.insert(project.id.clone(), Stored::preset(kind, rev));
        self.keep_soon(&project.id, cx);
        cx.notify();
    }

    /// A change is waiting to be sent: it goes when no other has come for `SAVE_AFTER`.
    fn keep_soon(&mut self, project: &str, cx: &mut Context<Self>) {
        self.unsaved.insert(project.to_string());
        let asked = project.to_string();
        self.saves.insert(
            project.to_string(),
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(SAVE_AFTER).await;
                let _ = this.update(cx, |this, cx| this.keep_now(&asked, cx));
            }),
        );
    }

    /// Send the board as it stands: its layout, or none where it is the preset, which has Fleet forget what it kept.
    fn keep_now(&mut self, project: &str, cx: &mut Context<Self>) {
        let Some(stored) = self.layouts.get(project) else { return };
        let (layout, sent) = (stored.edited.then(|| stored.layout.to_wire(&stored.unknown)), stored.rev);
        let Source::Live(_) = &self.source else {
            match layout {
                Some(layout) => self.kept.insert(project.to_string(), layout),
                None => self.kept.remove(project),
            };
            self.unsaved.remove(project);
            return;
        };
        let Some(port) = self.port(cx) else { return };
        let task = wire::save_board(port, project, layout, cx);
        let asked = project.to_string();
        self.sending.insert(
            project.to_string(),
            cx.spawn(async move |this, cx| {
                let answer = task.await;
                let _ = this.update(cx, |this, cx| match answer {
                    Ok(_) => {
                        // A change made while this one was on its way is still to be sent.
                        if this.layouts.get(&asked).is_some_and(|stored| stored.rev == sent) {
                            this.unsaved.remove(&asked);
                        }
                    }
                    Err(why) => crate::toasts::tell(why.said, cx),
                });
            }),
        );
    }

    fn read_layout(&mut self, cx: &mut Context<Self>) {
        let (Some(port), Some(project)) = (self.port(cx), self.project(cx)) else { return };
        let task = wire::board(port, &project.id, cx);
        let asked = project.id.clone();
        self.tasks.insert("board", cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| match answer {
                Ok(board) => this.apply_kept(&asked, board.layout, cx),
                Err(why) => {
                    crate::log::Note::warn("the board was not read").field("why", why.said).emit();
                    this.apply_kept(&asked, None, cx);
                }
            });
        }));
    }

    /// The board as an ask to the librarian sees it: the layout drawn, whether it is the preset, and every kind and view this
    /// build draws.
    pub fn asked_board(&mut self, project: &str) -> ipc::AskedBoard {
        self.layout(project);
        let stored = &self.layouts[project];
        let kinds = BlockKind::ALL.into_iter().map(|kind| ipc::AskedBlockKind { kind: kind.slug().to_string(), views: kind.views().iter().map(|view| view.slug().to_string()).collect() }).collect();
        ipc::AskedBoard { layout: stored.layout.to_wire(&stored.unknown), preset: !stored.edited, kinds, columns: COLUMNS }
    }

    /// An ask changed the board, or its Undo put one back, and Fleet already keeps it: it is drawn at once and sent nowhere. A
    /// change of his still waiting to be sent was on the board the ask saw, so it goes with it.
    pub fn take_from_fleet(&mut self, kept: Option<BoardLayout>, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx) else { return };
        self.saves.remove(&project.id);
        self.unsaved.remove(&project.id);
        if matches!(self.source, Source::Mock) {
            match &kept {
                Some(layout) => self.kept.insert(project.id.clone(), layout.clone()),
                None => self.kept.remove(&project.id),
            };
        }
        self.apply_kept(&project.id, kept, cx);
    }

    /// An ask's board change was taken back: the board is the one Fleet answers with now, or the preset where it keeps none.
    pub fn reread_layout(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx) else { return };
        self.saves.remove(&project.id);
        self.unsaved.remove(&project.id);
        match &self.source {
            Source::Mock => {
                let kept = self.kept.get(&project.id).cloned();
                self.apply_kept(&project.id, kept, cx);
            }
            Source::Live(_) => self.read_layout(cx),
        }
    }

    /// The might-lists changed in Fleet: the Issues block reads them again.
    pub fn reread_issues(&mut self, cx: &mut Context<Self>) {
        if matches!(self.source, Source::Live(_)) {
            self.read_issues(cx);
        }
    }

    // ---- ranking with the librarian -----------------------------------------------------------

    /// Whether the lists on the board are in the librarian's order, as Fleet last answered them.
    pub fn ranked_by_librarian(&self) -> bool {
        self.ranked_at().is_some()
    }

    /// When the librarian ranked the lists on the board, where it did.
    pub fn ranked_at(&self) -> Option<&ipc::Instant> {
        match &self.issues {
            Some(Ok(read)) => read.ranked.by_librarian.as_ref(),
            _ => None,
        }
    }

    /// Rank with the librarian, or Reset: the might-lists come back in the librarian's order with a why on each row, or by
    /// features again.
    pub fn rank(&mut self, forget: bool, cx: &mut Context<Self>) {
        if self.ranking == Ranking::Waiting {
            return;
        }
        let Some(project) = self.project(cx) else { return };
        let task = match (&self.source, self.port(cx)) {
            (Source::Live(_), Some(port)) if forget => crate::features::wire::forget_librarian_ranking(port, &project.id, cx),
            (Source::Live(_), Some(port)) => crate::features::wire::rank_with_librarian(port, &project.id, cx),
            (Source::Live(_), None) => crate::settings::wire::unreachable(),
            (Source::Mock, _) => Task::ready(Ok(if forget { mock::issues().ranked } else { mock::ranked_by_librarian() })),
        };
        self.ranking = Ranking::Waiting;
        let asked = project.id.clone();
        self.tasks.insert("rank", cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                match answer {
                    Ok(ranked) => {
                        this.ranking = Ranking::Idle;
                        if this.project(cx).is_some_and(|now| now.id == asked) {
                            match this.issues.as_mut() {
                                Some(Ok(read)) => read.ranked = ranked,
                                _ => this.issues = Some(Ok(IssuesRead { mine: Vec::new(), ranked })),
                            }
                        }
                    }
                    Err(why) => this.ranking = Ranking::Refused(crate::features::ask_box::refusal_line(&why)),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub fn toggle_edit(&mut self, cx: &mut Context<Self>) {
        self.edit = !self.edit;
        self.grab = None;
        cx.notify();
    }

    // ---- reading -----------------------------------------------------------------------------

    /// Brought to the front: the board follows Fleet, and reads what it draws.
    pub fn enter(&mut self, cx: &mut Context<Self>) {
        self.entered = true;
        self.read_all(cx);
        if self.ticker.is_none() && matches!(self.source, Source::Live(_)) {
            self.ticker = Some(cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(RUNS_EVERY).await;
                if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                    break;
                }
            }));
        }
        if self.metric_ticker.is_none() && matches!(self.source, Source::Live(_)) {
            self.metric_ticker = Some(cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(METRICS_EVERY).await;
                if this.update(cx, |this, cx| this.read_metrics(cx)).is_err() {
                    break;
                }
            }));
        }
    }

    pub fn leave(&mut self) {
        self.entered = false;
        self.ticker = None;
        self.metric_ticker = None;
        self.tasks.remove("metrics");
    }

    /// The Dashboard tab came to the front of the Project place, or another tab did. Metrics are asked for only while it is in front.
    pub fn set_front(&mut self, front: bool, cx: &mut Context<Self>) {
        if self.front == front {
            return;
        }
        self.front = front;
        if front {
            self.read_metrics(cx);
        } else {
            self.tasks.remove("metrics");
        }
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        if self.entered {
            self.read_runs(cx);
        }
    }

    fn port(&self, cx: &App) -> Option<u16> {
        match &self.source {
            Source::Live(fleet) => fleet.read(cx).held().port(),
            Source::Mock => None,
        }
    }

    fn key(&self, cx: &App) -> Option<Key> {
        let project = self.project(cx)?;
        Some(Key { port: self.port(cx), project: project.id.clone(), manifests: project.repositories.iter().map(|one| one.id.clone()).collect(), checkout: self.checkout.clone() })
    }

    /// Fleet said something: read again where what the board is made for has changed.
    fn moved(&mut self, cx: &mut Context<Self>) {
        if self.entered && self.key(cx) != self.read_for {
            self.read_all(cx);
        }
    }

    /// Every read the blocks draw from, for the project and checkout on screen.
    fn read_all(&mut self, cx: &mut Context<Self>) {
        if matches!(self.source, Source::Mock) {
            return;
        }
        let Some(key) = self.key(cx) else { return };
        if key.port.is_none() {
            return;
        }
        let project_changed = self.read_for.as_ref().is_none_or(|before| before.project != key.project);
        let manifests_changed = self.read_for.as_ref().is_none_or(|before| before.manifests != key.manifests);
        self.read_for = Some(key);
        if project_changed {
            self.overview = None;
            self.since = None;
            self.issues = None;
            self.metrics = None;
            self.token_fields.clear();
        }
        if manifests_changed || project_changed {
            self.configs = None;
            self.runs.clear();
            self.drafts.clear();
        }
        self.follow_system(cx);
        self.read_layout(cx);
        self.read_metrics(cx);
        self.read_features(cx);
        self.read_runs(cx);
        self.read_configs(cx);
        self.read_issues(cx);
        if let Some(reviews) = self.reviews.clone() {
            let window = self.window;
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, cx| reviews.update(cx, |reviews, cx| reviews.ensure_lists(window, cx)));
            });
        }
        cx.notify();
    }

    /// The System block reads the map of the project on screen.
    fn follow_system(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx) else { return };
        let manifests = project.repositories.iter().map(|one| one.id.clone()).collect();
        self.system.update(cx, |system, cx| system.follow(&project.id, &project.name, manifests, cx));
    }

    /// Ask Fleet what each recipe of the project reads. Fleet fetches again only where a reading is stale, so asking is cheap; the
    /// answer replaces what is drawn, and a failed ask leaves the last reading on screen.
    fn read_metrics(&mut self, cx: &mut Context<Self>) {
        if !self.entered || !self.front {
            return;
        }
        let (Some(port), Some(project)) = (self.port(cx), self.project(cx)) else { return };
        let task = wire::metrics(port, &project.id, cx);
        let asked = project.id.clone();
        self.tasks.insert("metrics", cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.project(cx).is_none_or(|now| now.id != asked) {
                    return;
                }
                match answer {
                    Ok(read) => this.apply_readings(read, cx),
                    Err(why) => crate::log::Note::warn("the metrics were not read").field("why", why.said).emit(),
                }
            });
        }));
    }

    /// What Fleet read: the blocks draw it, and a recipe with no token gets its paste field.
    pub(super) fn apply_readings(&mut self, read: MetricReadings, cx: &mut Context<Self>) {
        if self.project(cx).is_some_and(|now| now.id != read.project) {
            return;
        }
        self.metrics = Some(read);
        self.readings_changed(cx);
        cx.notify();
    }

    /// Each block of a recipe is a stop of the keys, and each recipe without a token has its field.
    fn readings_changed(&mut self, cx: &mut Context<Self>) {
        let kinds: Vec<BlockKind> = self.metrics.iter().flat_map(|read| &read.readings).map(|one| BlockKind::metric(&one.id)).collect();
        for kind in kinds {
            if !self.handles.iter().any(|(held, _)| *held == kind) {
                self.handles.push((kind, cx.focus_handle().tab_stop(true)));
            }
        }
        self.prepare_token_fields(cx);
    }

    pub(super) fn reading(&self, id: &str) -> Option<&MetricReading> {
        self.metrics.as_ref()?.readings.iter().find(|one| one.id == id)
    }

    /// The words on a block's header and the name VoiceOver reads: a metric block's is its recipe's title.
    pub(super) fn title_of(&self, kind: BlockKind) -> String {
        match kind {
            BlockKind::Metric(id) => self.reading(id).map_or_else(|| id.to_string(), |one| one.title.clone()),
            _ => kind.title().to_string(),
        }
    }

    /// The views a block is offered: Picture only on a metric whose answer carries one.
    pub(super) fn views_of(&self, kind: BlockKind) -> Vec<View> {
        match kind.metric_id().and_then(|id| self.reading(id)) {
            Some(reading) => metric::offered(reading),
            None => kind.views().to_vec(),
        }
    }

    // ---- tokens ------------------------------------------------------------------------------

    /// Every recipe that has no token has an empty paste field; one that has, or is being sent, has none. The field is made where
    /// a window is, which is after this update.
    fn prepare_token_fields(&mut self, cx: &mut Context<Self>) {
        let waiting: Vec<String> = self.metrics.iter().flat_map(|read| &read.readings).filter(|one| matches!(one.state, MetricState::NoToken { .. })).map(|one| one.id.clone()).collect();
        self.token_fields.retain(|id, _| waiting.contains(id));
        let missing: Vec<String> = waiting.into_iter().filter(|id| !self.token_fields.contains_key(id) && !self.saving_tokens.contains_key(id)).collect();
        if missing.is_empty() {
            return;
        }
        let (window, this) = (self.window, cx.weak_entity());
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                let _ = this.update(cx, |board, cx| {
                    for id in missing {
                        board.token_field(id, window, cx);
                    }
                });
            });
        });
    }

    fn token_field(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.token_fields.contains_key(&id) {
            return;
        }
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Paste the token");
            state.set_masked(true, window, cx);
            state
        });
        let held = id.clone();
        let sub = cx.subscribe_in(&input, window, move |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } => this.save_token(&held, window, cx),
            // Save is off while the field is empty.
            InputEvent::Change => cx.notify(),
            _ => {}
        });
        self.token_fields.insert(id, TokenField { input, _sub: sub });
    }

    /// Whether the paste field of `id` holds anything, which is all Save asks of it.
    pub(super) fn token_typed(&self, id: &str, cx: &App) -> bool {
        self.token_fields.get(id).is_some_and(|field| !field.input.read(cx).value().trim().is_empty())
    }

    /// Send the token typed for `id`. The field is dropped at once, so the token is held by the request alone and is on no
    /// screen, no draft and no log; a refusal brings back an empty field.
    pub(super) fn save_token(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving_tokens.contains_key(id) {
            return;
        }
        let Some(token) = self.token_fields.get(id).map(|field| field.input.read(cx).value().trim().to_string()).filter(|token| !token.is_empty()) else { return };
        let Some(credential) = self.reading(id).and_then(|one| match &one.state {
            MetricState::NoToken { credential } => Some(credential.name.clone()),
            _ => None,
        }) else {
            return;
        };
        let Some(project) = self.project(cx) else { return };
        self.token_fields.remove(id);
        window.focus(&self.root, cx);
        let held = id.to_string();
        let Some(port) = self.port(cx) else {
            if matches!(self.source, Source::Mock) {
                if let Some(read) = self.metrics.as_mut() {
                    mock::token_saved(read, &held);
                }
            }
            cx.notify();
            return;
        };
        let task = wire::save_metric_token(port, &project.id, &credential, token, cx);
        let id = held.clone();
        self.saving_tokens.insert(
            held,
            cx.spawn(async move |this, cx| {
                let answer = task.await;
                let _ = this.update(cx, |this, cx| {
                    this.saving_tokens.remove(&id);
                    match answer {
                        Ok(read) => this.apply_readings(read, cx),
                        Err(why) => {
                            crate::toasts::tell(why.said, cx);
                            this.prepare_token_fields(cx);
                        }
                    }
                    cx.notify();
                });
            }),
        );
        cx.notify();
    }

    /// "Fix with the librarian": the screen puts the words in the Ask box.
    pub(super) fn fix_metric(&mut self, id: &str, cx: &mut Context<Self>) {
        let title = self.title_of(BlockKind::metric(id));
        cx.emit(BoardEvent::FixMetric { id: id.to_string(), title });
    }

    fn read_features(&mut self, cx: &mut Context<Self>) {
        let (Some(port), Some(project)) = (self.port(cx), self.project(cx)) else { return };
        let since_at = crate::features::project_model::first_since(crate::features::looked::before_project(&project.id, cx), None, Utc::now());
        let overview = crate::features::wire::overview(port, &project.id, cx);
        let since = crate::features::wire::since(port, &project.id, since_at, cx);
        let asked = project.id.clone();
        self.tasks.insert("features", cx.spawn(async move |this, cx| {
            let (overview, since) = (overview.await, since.await);
            let _ = this.update(cx, |this, cx| {
                if this.project(cx).is_some_and(|now| now.id == asked) {
                    this.overview = Some(overview.map_err(|why| why.said));
                    this.since = Some(since.map_err(|why| why.said));
                    this.apply_preset(&asked);
                    cx.notify();
                }
            });
        }));
    }

    /// The project's kind is read: a board he has not edited takes the preset for it.
    fn apply_preset(&mut self, project: &str) {
        let kind = self.kind();
        if let Some(stored) = self.layouts.get_mut(project).filter(|stored| !stored.edited) {
            stored.layout = presets::preset(kind);
        }
    }

    fn read_runs(&mut self, cx: &mut Context<Self>) {
        let (Some(port), Some(project)) = (self.port(cx), self.project(cx)) else { return };
        let checkout = self.checkout.clone();
        let asked: Vec<(String, _)> = match &checkout {
            Checkout::Main => project.repositories.iter().take(MOST_REPOSITORIES).map(|one| (one.id.clone(), wire::runs(port, &one.id, None, cx))).collect(),
            Checkout::Job { id, manifest, .. } => vec![(manifest.clone(), wire::runs(port, manifest, Some(id), cx))],
        };
        let project_id = project.id.clone();
        self.tasks.insert("runs", cx.spawn(async move |this, cx| {
            let mut read = Vec::new();
            for (manifest, task) in asked {
                read.push((manifest, task.await));
            }
            let _ = this.update(cx, |this, cx| {
                if this.project(cx).is_some_and(|now| now.id == project_id) && this.checkout == checkout {
                    this.runs.clear();
                    for (manifest, answer) in read {
                        this.runs.insert(manifest, answer.map_err(|why| why.said));
                    }
                    cx.notify();
                }
            });
        }));
    }

    fn read_configs(&mut self, cx: &mut Context<Self>) {
        let (Some(port), Some(project)) = (self.port(cx), self.project(cx)) else { return };
        let asked: Vec<_> = project.repositories.iter().take(MOST_REPOSITORIES).map(|one| wire::manifest(port, &one.id, cx)).collect();
        let project_id = project.id.clone();
        self.tasks.insert("configs", cx.spawn(async move |this, cx| {
            let mut read = Vec::new();
            for task in asked {
                read.push(task.await.map_err(|why| why.said));
            }
            let _ = this.update(cx, |this, cx| {
                if this.project(cx).is_some_and(|now| now.id == project_id) {
                    this.configs = Some(read);
                    cx.notify();
                }
            });
        }));
    }

    /// The issues Fleet ranked for the project, and what is assigned to him, which is each tracker's own Assigned to me run as
    /// one query.
    fn read_issues(&mut self, cx: &mut Context<Self>) {
        let (Some(port), Some(project)) = (self.port(cx), self.project(cx)) else { return };
        let manifests: Vec<String> = project.repositories.iter().take(MOST_REPOSITORIES).map(|one| one.id.clone()).collect();
        let ranked = wire::ranked_issues(port, &project.id, cx);
        let queries: Vec<_> = manifests.iter().map(|id| (id.clone(), wire::issue_queries(port, id, cx))).collect();
        let project_id = project.id.clone();
        self.tasks.insert("issues", cx.spawn(async move |this, cx| {
            let ranked = ranked.await;
            let mut mine: Vec<TrackedIssue> = Vec::new();
            for (manifest, task) in queries {
                let Some(query) = task.await.ok().and_then(|read| read.guide).map(|guide| guide.assigned_to_me) else { continue };
                let search = cx.update(|cx| wire::search_issues(port, &manifest, &query, cx));
                if let Ok(list) = search.await {
                    mine.extend(list.issues);
                }
            }
            let _ = this.update(cx, |this, cx| {
                if this.project(cx).is_some_and(|now| now.id == project_id) {
                    this.issues = Some(ranked.map(|ranked| IssuesRead { mine, ranked }).map_err(|why| why.said));
                    cx.notify();
                }
            });
        }));
    }

    // ---- what each block draws ---------------------------------------------------------------

    pub fn fill(&self, kind: BlockKind, project: &Project, cx: &App) -> Fill {
        let now = Utc::now();
        match kind {
            // The block draws itself, from the map it reads.
            BlockKind::System => Fill::Reading,
            BlockKind::Needs => match &self.source {
                Source::Mock => Fill::items(mock::needs()),
                Source::Live(fleet) => {
                    let held = fleet.read(cx).held();
                    let cards = crate::operator::live(cx).map(|page| page.read(cx).turn().to_vec()).unwrap_or_default();
                    Fill::items(model::needs(project, &held.jobs, &cards, now))
                }
            },
            BlockKind::Run => self.run_fill(project),
            BlockKind::Reviews => match &self.reviews {
                Some(reviews) if reviews.read(cx).lists_read() || matches!(self.source, Source::Mock) => {
                    let mut items = model::reviews(&reviews.read(cx).listed_rows(), project);
                    // The pull requests of others Fleet ranked stand under his own, where there are any.
                    if let Some(Ok(read)) = &self.issues {
                        let more = model::interested(&read.ranked.might_be_interested, &items);
                        items.extend(more);
                    }
                    Fill::items(items)
                }
                _ => Fill::Reading,
            },
            BlockKind::Issues => match &self.issues {
                None => Fill::Reading,
                Some(Err(why)) => Fill::Failed(why.clone()),
                Some(Ok(read)) => Fill::items(model::issues(&read.mine, &read.ranked.might_pick_up)),
            },
            BlockKind::Features => match (&self.overview, &self.since) {
                (Some(overview), Some(since)) => match (overview, since) {
                    (Err(why), Err(_)) => Fill::Failed(why.clone()),
                    _ => Fill::Ready(model::features(since.as_ref().ok(), overview.as_ref().ok())),
                },
                _ => Fill::Reading,
            },
            // A metric block draws from its reading (`view.rs`, `metric_body`); it has no rows of the kind the others do.
            BlockKind::Metric(_) => Fill::Reading,
            BlockKind::Checks => match &self.source {
                Source::Mock => Fill::items(mock::checks()),
                Source::Live(_) => match &self.configs {
                    None => Fill::Reading,
                    Some(read) => {
                        let ok: Vec<ManifestConfig> = read.iter().filter_map(|one| one.as_ref().ok().cloned()).collect();
                        match (ok.is_empty(), read.iter().find_map(|one| one.as_ref().err())) {
                            (true, Some(why)) => Fill::Failed(why.clone()),
                            _ => Fill::items(model::checks(project, &ok)),
                        }
                    }
                },
            },
        }
    }

    fn run_fill(&self, project: &Project) -> Fill {
        if self.runs.is_empty() {
            return Fill::Reading;
        }
        let mut rows: Vec<Item> = Vec::new();
        let mut failed: Option<String> = None;
        for (manifest, read) in &self.runs {
            match read {
                Ok(list) if list.runs.is_empty() => match self.drafts.get(manifest) {
                    Some(Ok(draft)) => rows.extend(model::draft_rows(manifest, draft)),
                    _ => rows.push(model::offer_draft(project, manifest)),
                },
                Ok(list) => rows.extend(model::run_rows(project, manifest, &list.runs)),
                Err(why) => failed = Some(why.clone()),
            }
        }
        match (rows.is_empty(), failed) {
            (true, Some(why)) => Fill::Failed(why),
            _ => Fill::Ready(Shown { items: rows, tiles: Vec::new() }),
        }
    }

    // ---- the checkouts Start can run on -----------------------------------------------------------

    /// Main, then each live Job of the project that holds a worktree, by its branch.
    pub fn checkouts(&self, project: &Project, cx: &App) -> Vec<Checkout> {
        let mut all = vec![Checkout::Main];
        match &self.source {
            Source::Mock => all.extend(mock::worktrees()),
            Source::Live(fleet) => {
                for job in &fleet.read(cx).held().jobs {
                    if job.state != ipc::SessionState::Live || job.mac.is_some() {
                        continue;
                    }
                    let in_project = match job.hosted.as_ref().and_then(|hosted| hosted.project.as_deref()) {
                        Some(id) => id == project.id,
                        None => job.manifest_id.as_ref().is_some_and(|id| project.holds(id.as_str())),
                    };
                    let standing = |kind: &str| job.attachments.iter().find(|one| one.kind == kind && one.state == ipc::AttachmentState::Standing).map(|one| one.target.clone());
                    let (Some(manifest), Some(_slot)) = (job.manifest_id.as_ref(), standing("slot")) else { continue };
                    if in_project {
                        let label = standing("branch").unwrap_or_else(|| crate::jobs::titled(job.title.as_deref()));
                        all.push(Checkout::Job { id: job.id.as_str().to_string(), manifest: manifest.as_str().to_string(), label });
                    }
                }
            }
        }
        all
    }

    /// The Run block and Start act on `checkout` from here.
    pub fn use_checkout(&mut self, checkout: Checkout, cx: &mut Context<Self>) {
        if self.checkout == checkout {
            return;
        }
        self.checkout = checkout;
        self.tail = None;
        if matches!(self.source, Source::Mock) {
            self.fill_mock(cx);
        } else {
            self.runs.clear();
            self.read_runs(cx);
        }
        cx.notify();
    }

    // ---- presses -----------------------------------------------------------------------------

    /// Start the run he last used, else the first the project declares, on the checkout on screen.
    pub fn start(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx) else { return };
        let mut best: Option<(String, String, Option<String>)> = None;
        for (manifest, read) in &self.runs {
            for run in read.iter().flat_map(|list| &list.runs) {
                let used = run.last_used.as_ref().map(|at| at.as_str().to_string());
                if best.as_ref().is_none_or(|(_, _, held)| used > *held) {
                    best = Some((manifest.clone(), run.name.clone(), used));
                }
            }
        }
        match best {
            Some((manifest, name, _)) => self.press(Press::RunStart { manifest, name }, cx),
            None => {
                if let Some(first) = project.repositories.first() {
                    self.press(Press::Draft { manifest: first.id.clone() }, cx);
                }
            }
        }
    }

    pub fn press(&mut self, press: Press, cx: &mut Context<Self>) {
        match press {
            Press::Job(id) => {
                if let Some(sink) = cx.try_global::<titlebar::AskedSink>().cloned() {
                    (sink.0)(&titlebar::Asked::OpenJob(id), cx);
                }
                crate::palette::show_jobs(cx);
            }
            Press::Operator => {
                if let Some(sink) = cx.try_global::<titlebar::AskedSink>().cloned() {
                    (sink.0)(&titlebar::Asked::Place("operator"), cx);
                }
            }
            Press::Pull { repository, number } => {
                if let Some((owner, name)) = repository.split_once('/') {
                    crate::deep_link::open(&crate::deep_link::PullLink { owner: owner.to_string(), name: name.to_string(), number }, cx);
                }
            }
            Press::Url(url) => cx.open_url(&url),
            Press::Feature(slug) => cx.emit(BoardEvent::Feature(slug)),
            Press::RunStart { manifest, name } => self.run_act("/app_runs/start", manifest, name, cx),
            Press::RunStop { manifest, name } => self.run_act("/app_runs/stop", manifest, name, cx),
            Press::Logs { server, title } => self.read_tail(server, title, cx),
            Press::Draft { manifest } => self.read_draft(manifest, cx),
            Press::Command { manifest, name, confirm: true } => self.ask_run(manifest, name, cx),
            Press::Command { manifest, name, confirm: false } => self.run_command(manifest, name, cx),
        }
    }

    /// A Command flagged `confirm: true`: Run and Cancel are asked, the keys on Cancel, and nothing runs until Run.
    fn ask_run(&mut self, manifest: String, name: String, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx) else { return };
        let repository = (project.repositories.len() > 1).then(|| project.repository_name(&manifest).map(str::to_string)).flatten();
        self.asking = Some(Asking { manifest, name, repository });
        let (window, cancel) = (self.window, self.cancel_stop.clone());
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| window.focus(&cancel, cx));
        });
        cx.notify();
    }

    /// Run: the Command he was asked about runs in the main checkout.
    pub fn confirm_run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(asking) = self.asking.take() else { return };
        window.focus(&self.root, cx);
        self.run_command(asking.manifest, asking.name, cx);
        cx.notify();
    }

    /// Cancel, Esc and a press outside the card: nothing runs, and the keys go back to the board.
    pub fn cancel_run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.asking.take().is_some() {
            window.focus(&self.root, cx);
            cx.notify();
        }
    }

    fn run_act(&mut self, route: &'static str, manifest: String, name: String, cx: &mut Context<Self>) {
        let held = format!("{route}:{manifest}:{name}");
        if !self.acting.insert(held.clone()) {
            return;
        }
        let Source::Live(_) = &self.source else {
            self.acting.remove(&held);
            mock::flip(&mut self.runs, &manifest, &name, route == "/app_runs/start");
            cx.notify();
            return;
        };
        let Some(port) = self.port(cx) else {
            self.acting.remove(&held);
            return;
        };
        let task = wire::run_act(port, route, &manifest, &name, self.checkout.job(), cx);
        self.tasks.insert("act", cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                this.acting.remove(&held);
                match answer {
                    Ok(run) => {
                        if let Some(Ok(list)) = this.runs.get_mut(&manifest) {
                            match list.runs.iter_mut().find(|one| one.name == run.name) {
                                Some(one) => *one = run,
                                None => list.runs.push(run),
                            }
                        }
                    }
                    Err(why) => crate::toasts::tell(why.said, cx),
                }
                this.read_runs(cx);
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn read_tail(&mut self, server: String, title: String, cx: &mut Context<Self>) {
        self.tail = Some(Tail { title: title.clone(), server: server.clone(), lines: None });
        cx.notify();
        let Some(port) = self.port(cx) else {
            if matches!(self.source, Source::Mock) {
                if let Some(tail) = self.tail.as_mut() {
                    tail.lines = Some(Ok(mock::log()));
                }
            }
            return;
        };
        let task = wire::log_tail(port, server.clone(), TAIL_LINES, cx);
        self.tasks.insert("tail", cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(tail) = this.tail.as_mut().filter(|tail| tail.server == server) {
                    tail.lines = Some(answer.map_err(|why| why.said));
                    cx.notify();
                }
            });
        }));
    }

    /// Put the tail away.
    pub fn close_tail(&mut self, cx: &mut Context<Self>) {
        self.tail = None;
        cx.notify();
    }

    /// Read the tail of the server on screen again.
    pub fn refresh_tail(&mut self, cx: &mut Context<Self>) {
        if let Some((server, title)) = self.tail.as_ref().map(|tail| (tail.server.clone(), tail.title.clone())) {
            self.read_tail(server, title, cx);
        }
    }

    fn read_draft(&mut self, manifest: String, cx: &mut Context<Self>) {
        let Some(port) = self.port(cx) else {
            if matches!(self.source, Source::Mock) {
                self.drafts.insert(manifest, Ok(mock::draft()));
                cx.notify();
            }
            return;
        };
        let task = wire::draft(port, &manifest, cx);
        self.tasks.insert("draft", cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                this.drafts.insert(manifest, answer.map_err(|why| why.said));
                cx.notify();
            });
        }));
    }

    fn run_command(&mut self, manifest: String, name: String, cx: &mut Context<Self>) {
        let held = format!("command:{manifest}:{name}");
        let Some(port) = self.port(cx) else {
            if matches!(self.source, Source::Mock) {
                self.ran.push((manifest, name));
            }
            return;
        };
        if !self.acting.insert(held.clone()) {
            return;
        }
        let task = wire::run_command(port, &manifest, &name, cx);
        self.tasks.insert("command", cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| {
                this.acting.remove(&held);
                if let Err(why) = answer {
                    crate::toasts::tell(why.said, cx);
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    // ---- the mock ----------------------------------------------------------------------------

    pub(super) fn fill_mock(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project(cx) else { return };
        self.overview = Some(Ok(crate::features::mock::overview()));
        self.since = Some(Ok(crate::features::mock::since(Utc::now() - chrono::Duration::days(crate::features::mock::SINCE_DAYS_AGO))));
        self.runs = mock::runs(&project, self.checkout.job().is_some());
        self.drafts.clear();
        self.issues = Some(Ok(mock::issues()));
        self.metrics = Some(mock::readings(&project.id));
        self.token_fields.clear();
        self.saving_tokens.clear();
        self.readings_changed(cx);
        self.apply_kept(&project.id, self.kept.get(&project.id).cloned(), cx);
        self.apply_preset(&project.id);
        self.follow_system(cx);
        cx.notify();
    }

    /// The view a block starts in, for the keys that cycle it.
    pub(super) fn next_view(&self, kind: BlockKind, now: View) -> View {
        let views = self.views_of(kind);
        let at = views.iter().position(|one| *one == now).unwrap_or(0);
        views[(at + 1) % views.len()]
    }

    /// The key a press in flight is held under, for the presses that wait for their answer.
    pub(super) fn held(press: &Press) -> Option<String> {
        match press {
            Press::RunStart { manifest, name } => Some(format!("/app_runs/start:{manifest}:{name}")),
            Press::RunStop { manifest, name } => Some(format!("/app_runs/stop:{manifest}:{name}")),
            Press::Command { manifest, name, .. } => Some(format!("command:{manifest}:{name}")),
            _ => None,
        }
    }

    /// Whether any process the Run block draws is on its way up.
    pub fn starting(&self) -> bool {
        self.runs.values().filter_map(|read| read.as_ref().ok()).flat_map(|list| &list.runs).flat_map(|run| &run.processes).any(|one| one.state == AppRunState::Starting)
    }
}
