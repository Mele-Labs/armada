//! The title bar and what stands under it: the project switcher and its Settings, search, the
//! Operator, the plan rings with the step-down offer, the Fleet dot with its panel and build
//! switcher, the Keep and Revert bar and the Fleet notice (`TitleBar.tsx`, `FleetBuild.tsx`,
//! `FleetNotice.tsx`, `trying-bar.tsx`).
//!
//! **One entity holds all of it**, because the pieces share one thing: which popover is open and
//! what held the keys before it opened. The data is a [`Face`]; nothing in a view works out a band
//! or a sentence. A press changes the face at once (a pending state, or the thing it asked for) and
//! emits an [`Asked`] for the wiring to send; [`TitleBar::refused`] puts the face back with the reason.
//!
//! The Job slot in the end region is empty and takes no room until the Jobs screens fill it with
//! the open Job's Layout control and view icons ([`TitleBar::fill_job_slot`]).

#[cfg(test)]
mod centred;
mod disk;
mod face;
mod fleet_down;
mod link_layer;
mod link_panel;
pub mod offers;
mod origin;
pub mod parts;
mod projects;
pub mod scenes;
mod swap_hold;
mod switcher;
mod usage;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    canvas, div, point, prelude::*, px, Anchor, AnyElement, AnyView, App, Bounds, Context, DismissEvent, Entity,
    EventEmitter, FocusHandle, FontWeight, Focusable, Global, MouseButton, PathBuilder, Pixels, Point, Role, SharedString,
    Subscription, Task, WeakEntity, Window,
};
use ipc::{BuildSource, ModelDownshift};

pub use face::{repositories_of, Connection, Face, LiveLink, Remedy, RestartWhy, Restarting};
pub use link_layer::StandIn;
/// The Operator's page adds its place to the strip and reads the week's ring.
pub use face::Place;

use crate::assets::MARK;
use crate::dropdown::{Dropdown, Entry, Picked, Row};
use crate::generated::actions::act;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use parts::{glyph, mono, no_drag, sans, tip};

/// What a press on the bar asks of the rest of the app. The wiring sends these; the walk reads none.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    /// The command palette: ⌘K and the search trigger. The Jobs screen's sink opens its palette.
    Palette,
    Dispatch,
    Operator,
    Settings,
    Account,
    Doctor,
    Repository(Option<usize>),
    /// Scope the window to a project.
    Project(usize),
    /// Settings, scoped to a project or to a repository in none: the switcher's Settings, and ⌘, in it.
    ProjectSettings(usize),
    RepositorySettings(usize),
    ProjectMembers(usize),
    NewProject,
    ChangeBuild { build: BuildSource, adopt: bool },
    Revert,
    Keep,
    Swap,
    Unpick(String),
    /// Pick a branch on origin that no Job here holds into the next build.
    PickOrigin(String),
    /// Skip the Refresh row with this id.
    SkipOffers(String),
    RestartFleet,
    /// The Fleet popover opened: read the disk again, so the figure in it is fresh.
    ReadDisk,
    Remedy(Remedy),
    CopyDebugInfo,
    /// Go to a place of the strip (⌘1 to ⌘9, or a press on it), by its id.
    Place(&'static str),
    /// The layout shows or hides a place (`TabBar.tsx`'s `show`): the wiring writes the person's own layout.
    PlaceShown { id: &'static str, shown: bool },
    /// ⇧⌘R: refresh what the window shows.
    Refresh,
    /// Teach, from the menu a right press on a thing offers; the target is what the menu's heading names.
    Teach(&'static str),
    /// The interface's press sound: the snapshot turned ready.
    Chime,
    /// Noted on the dropped-events alert: this many are said to have been seen.
    EventsNoted(u32),
}

impl EventEmitter<Asked> for TitleBar {}

/// Where the window in front hears what the bar asks: set by the screen that answers it, such as
/// the one that opens the dispatch sheet. The bar hands every [`Asked`] to it after its own change.
#[derive(Clone)]
pub struct AskedSink(pub std::rc::Rc<dyn Fn(&Asked, &mut App)>);

impl Global for AskedSink {}

/// Which popover hangs off the bar.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Popup {
    /// The plan card: hover opens it, a press pins it.
    Usage { pinned: bool },
    Fleet,
    /// The project switcher, under its trigger.
    Projects,
}

/// The act a Drones dialog holds until the person says yes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Held {
    Choose(BuildSource),
    Restart,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialog {
    Revert,
    Drones(Held),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuKind {
    Build,
    Account,
    /// The places strip's: Hide, then View.
    Places,
    /// The one a right press on the bar offers: Teach.
    Teach,
}

struct Menu {
    dropdown: Entity<Dropdown>,
    kind: MenuKind,
    _picked: Subscription,
    _dismissed: Subscription,
}

/// The row is narrow below this width, as Electron's container query is (`TitleBar.css` 746). The
/// stylesheet states it off-contract because a container condition cannot read a token.
const NARROW_BELOW: f32 = 900.;

/// An id a menu row answers to, kept for the life of the app: a row's id is a `&'static str` and the
/// picker's ids name a project and a repository by place. Each distinct id is made once.
pub fn id_of(text: String) -> &'static str {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static KNOWN: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut known = KNOWN.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(found) = known.get(text.as_str()) {
        return found;
    }
    let kept: &'static str = Box::leak(text.into_boxed_str());
    known.insert(kept);
    kept
}

/// The ids the repository menu's entries answer to: an entry's id lives as long as the app.
const REPO: [&str; 12] = ["repo-1", "repo-2", "repo-3", "repo-4", "repo-5", "repo-6", "repo-7", "repo-8", "repo-9", "repo-10", "repo-11", "repo-12"];

/// Where each trigger was drawn last frame, so a popover can hang off it.
#[derive(Default)]
struct Anchors {
    repository: Rc<Cell<Option<Bounds<Pixels>>>>,
    meter: Rc<Cell<Option<Bounds<Pixels>>>>,
    fleet: Rc<Cell<Option<Bounds<Pixels>>>>,
    select: Rc<Cell<Option<Bounds<Pixels>>>>,
    menu: Rc<Cell<Option<Bounds<Pixels>>>>,
    row: Rc<Cell<Option<Bounds<Pixels>>>>,
    centre: Rc<Cell<Option<Bounds<Pixels>>>>,
    start: Rc<Cell<Option<Bounds<Pixels>>>>,
    end: Rc<Cell<Option<Bounds<Pixels>>>>,
}

/// Where the title row was drawn last frame, for what sits below it to start under it.
#[derive(Clone)]
pub struct TitleRow(pub Rc<Cell<Option<Bounds<Pixels>>>>);

impl Global for TitleRow {}

pub struct TitleBar {
    pub face: Face,
    job: Option<AnyView>,
    popup: Option<Popup>,
    /// The pointer is over the open card, so leaving the meter does not close it.
    over_card: bool,
    _leaving: Option<Task<()>>,
    dialog: Option<Dialog>,
    menu: Option<Menu>,
    /// What had the keys before a popover, dialog or menu took them, so they go back there.
    keys_were: Option<FocusHandle>,
    popup_focus: FocusHandle,
    dialog_focus: FocusHandle,
    switcher: projects::Switcher,
    /// The keys of each place of the strip, kept by its id.
    place_focus: Vec<(&'static str, FocusHandle)>,
    /// The plan meter's keys: they open its card as a hover does.
    meter_focus: FocusHandle,
    /// The Menu button's keys: a menu it opened gives them back to it, as Radix gives them to its trigger.
    menu_focus: FocusHandle,
    /// The card the keys opened, which they shut as they leave.
    meter_by_keys: bool,
    /// Esc shut the card while the keys stayed on the meter: it does not open again until they leave.
    meter_quiet: bool,
    _meter_focus: [Subscription; 2],
    at: Anchors,
    /// The face as it was before the last optimistic change, for [`TitleBar::refused`].
    before: Option<Face>,
    /// The Fleet states last handed to the toasts.
    notices_sent: Vec<crate::toasts::FleetNotice>,
    /// A window width the bar draws at in place of the window's own: a walk's way of narrowing it.
    pub width: Option<Pixels>,
    _asked: Subscription,
    // The window held through a swap, and what the chime needs to know of the snapshot (`swap_hold.rs`).
    swap_since: Option<std::time::Instant>,
    swap_dismissed: Option<String>,
    snapshot_was: Option<Option<ipc::SnapshotState>>,
    _swap_tick: Option<Task<()>>,
    _swap_keys: Option<Subscription>,
    /// Fleet's build report as last handed in: a press's answer in the frame stands until Fleet's report moves.
    build_read: Option<ipc::FleetBuildReport>,
    /// The offers as last handed in, which an optimistic skip stands over until Fleet answers.
    offers_read: Option<ipc::PreviewOffers>,
    /// The live link as last handed in, read again by the clock while the face moves with time alone.
    link_read: Option<face::LiveLink>,
    /// When the window opened: a Fleet down at launch has the same grace as one that just dropped.
    opened: std::time::Instant,
    _link_tick: Option<Task<()>>,
    /// The pid a Restart Fleet was pressed under, so a new Fleet that still does not match is told apart.
    restart_from: Option<u32>,
    _restart_wait: Option<Task<()>>,
    /// The centred Fleet panel, while Fleet is stopped, restarting or on another build (`link_layer.rs`).
    link: link_layer::LinkPanel,
    /// The search over the switcher's On origin row.
    origin: origin::OriginSearch,
}

/// How long a restart may take to bring back a new Fleet before it is called unanswered (`FleetNotice.tsx`).
const RESTART_WAIT: Duration = Duration::from_secs(30);

/// The one bar of the window, for the keys that work from anywhere.
struct Live(WeakEntity<TitleBar>);

impl Global for Live {}

/// The bar's keys that work from anywhere: ⌘K, ⌘J, ⌘, , ⇧⌘R and ⌘1 to ⌘9 (`actions.toml`, scope `anywhere`). A
/// screen's own handler runs first; what it does not take reaches here.
pub fn bind(cx: &mut App) {
    cx.on_action(|key: &Act, cx: &mut App| {
        let asked = match key.id {
            "command_palette" => Asked::Palette,
            "operator" => Asked::Operator,
            "open_settings" => Asked::Settings,
            "refresh" => Asked::Refresh,
            "deck_tabs" => {
                let Some(n) = key.digit else { return cx.propagate() };
                if let Some(bar) = cx.try_global::<Live>().and_then(|live| live.0.upgrade()) {
                    bar.update(cx, |bar, cx| bar.go_to_digit(n, cx));
                }
                return;
            }
            // Not the bar's: the key goes on to whatever else listens, such as a menu's Esc.
            _ => return cx.propagate(),
        };
        if let Some(bar) = cx.try_global::<Live>().and_then(|live| live.0.upgrade()) {
            bar.update(cx, |_, cx| cx.emit(asked));
        }
    });
}

/// Fleet refused a skip: the bar of the window in front draws the Refresh row as Fleet last said it.
pub fn offers_refused(cx: &mut App) {
    if let Some(bar) = live(cx) {
        bar.update(cx, |bar, cx| bar.offers_refused(cx));
    }
}

/// The bar of the window in front, for a walk to move between scenes.
pub fn live(cx: &App) -> Option<Entity<TitleBar>> {
    cx.try_global::<Live>().and_then(|live| live.0.upgrade())
}

impl TitleBar {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let face = cx.try_global::<scenes::Opening>().map_or_else(Face::bare, |opening| opening.0.clone());
        cx.set_global(Live(cx.entity().downgrade()));
        let at = Anchors::default();
        cx.set_global(TitleRow(Rc::clone(&at.row)));
        let me = cx.entity();
        let asked = cx.subscribe(&me, |_, _, asked: &Asked, cx| {
            if let Some(sink) = cx.try_global::<AskedSink>().cloned() {
                (sink.0)(asked, cx);
            }
        });
        let meter_focus = cx.focus_handle().tab_stop(true);
        let arrived = cx.on_focus_in(&meter_focus, window, |this, window, cx| this.meter_focused(true, window, cx));
        let left = cx.on_focus_out(&meter_focus, window, |this, _, window, cx| this.meter_focused(false, window, cx));
        TitleBar {
            face,
            job: None,
            popup: None,
            over_card: false,
            _leaving: None,
            dialog: None,
            menu: None,
            keys_were: None,
            popup_focus: cx.focus_handle(),
            dialog_focus: cx.focus_handle(),
            switcher: projects::Switcher::default(),
            place_focus: Vec::new(),
            meter_focus,
            menu_focus: cx.focus_handle().tab_stop(true),
            meter_by_keys: false,
            meter_quiet: false,
            _meter_focus: [arrived, left],
            at,
            before: None,
            notices_sent: Vec::new(),
            width: None,
            _asked: asked,
            swap_since: None,
            swap_dismissed: None,
            snapshot_was: None,
            _swap_tick: None,
            _swap_keys: None,
            build_read: None,
            origin: Default::default(),
            offers_read: None,
            link_read: None,
            opened: std::time::Instant::now(),
            _link_tick: None,
            restart_from: None,
            _restart_wait: None,
            link: link_layer::LinkPanel::new(window, cx),
        }
    }

    /// Back to a first face with nothing open, as a walk does when it starts a screen over.
    pub fn reset(&mut self, face: Face) {
        self.face = face;
        self.popup = None;
        self.menu = None;
        self.dialog = None;
        self.before = None;
        self.over_card = false;
        self.keys_were = None;
        self.width = None;
        self.swap_since = None;
        self.swap_dismissed = None;
        self.snapshot_was = None;
        self._swap_tick = None;
        self._swap_keys = None;
        self.link.clear();
    }

    /// The plan's rings and the step down, as Fleet last said them. A Fleet that dropped keeps what it said.
    pub fn set_usage(&mut self, usage: Option<&ipc::PlanUsage>, downshift: Option<&ipc::ModelDownshiftState>, cx: &mut Context<Self>) {
        let rings = face::usage_of(usage, chrono::Utc::now());
        let shift = face::downshift_of(downshift, usage);
        if rings == self.face.usage && shift == self.face.downshift {
            return;
        }
        self.face.usage = rings;
        self.face.downshift = shift;
        cx.notify();
    }

    /// Fleet's build report, as the switcher draws it. A press already changed the face in its frame; that
    /// stands until Fleet's report is not the one last handed in.
    pub fn set_build(&mut self, report: Option<&ipc::FleetBuildReport>, cx: &mut Context<Self>) {
        if self.build_read.as_ref() == report {
            return;
        }
        self.build_read = report.cloned();
        self.face.build = report.map(face::build_of);
        cx.notify();
    }

    /// The branches Jobs offered for the preview and the Refresh row, as Fleet last said them. A skip already
    /// cleared the row in its frame; that stands until Fleet's offers are not the ones last handed in.
    pub fn set_offers(&mut self, offers: Option<&ipc::PreviewOffers>, cx: &mut Context<Self>) {
        if self.offers_read.as_ref() == offers {
            return;
        }
        self.offers_read = offers.cloned();
        self.face.offers = offers.cloned();
        cx.notify();
    }

    /// The disk Fleet last reported. Its section of the Fleet panel draws it, and nothing where there is none.
    pub fn set_disk(&mut self, disk: Option<&ipc::DiskHealth>, cx: &mut Context<Self>) {
        if self.face.fleet.disk.as_ref() == disk {
            return;
        }
        self.face.fleet.disk = disk.cloned();
        cx.notify();
    }

    /// Fleet refused a skip: the row is back as Fleet last said it.
    pub fn offers_refused(&mut self, cx: &mut Context<Self>) {
        self.face.offers = self.offers_read.clone();
        cx.notify();
    }

    /// The live link, as the dot, the panel and the notice draw it.
    pub fn set_fleet(&mut self, link: &face::LiveLink, cx: &mut Context<Self>) {
        if self.link_read.as_ref() == Some(link) {
            return;
        }
        self.link_read = Some(link.clone());
        self.read_link(cx);
    }

    /// Read the link at this instant, and again at the next instant the clock alone moves it. A restart
    /// ends where Fleet answers, and is apart where a new Fleet still does not match.
    fn read_link(&mut self, cx: &mut Context<Self>) {
        let Some(link) = &self.link_read else { return };
        let now = std::time::Instant::now();
        let connection = link.connection(now);
        let power = link.power(self.opened, now);
        let restarting = match (&self.face.fleet.restarting, &connection) {
            (_, Connection::Connected { .. }) => Restarting::Idle,
            (Restarting::Working, Connection::Mismatch { pid, .. }) if self.restart_from.is_some_and(|from| from != *pid) => Restarting::Apart,
            (held, _) => held.clone(),
        };
        let fleet = face::FleetFace { connection, power, restarting, doctor: self.face.fleet.doctor.clone(), disk: self.face.fleet.disk.clone() };
        crate::merge_strip::set_standby(power == face::Power::Standby, cx);
        if fleet != self.face.fleet {
            self.face.fleet = fleet;
            cx.notify();
        }
        self._link_tick = link.next_at(self.opened, now).map(|at| {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(at.saturating_duration_since(std::time::Instant::now())).await;
                let _ = this.update(cx, |bar, cx| bar.read_link(cx));
            })
        });
        self.settle_panel(cx);
    }

    /// launchd would not restart Fleet: the notice says what it answered.
    pub fn restart_failed(&mut self, why: face::RestartWhy, detail: String, cx: &mut Context<Self>) {
        self._restart_wait = None;
        self.face.fleet.restarting = Restarting::Failed { why, detail };
        cx.notify();
    }

    /// The repositories Fleet serves, as the picker lists them: each by its directory's name, `Not set
    /// up` where no Manifest has loaded. A pick that no longer names one goes back to All.
    pub fn set_repositories(&mut self, served: &[ipc::RepositorySummary], cx: &mut Context<Self>) {
        let listed = face::repositories_of(served, &self.face.repositories);
        if listed == self.face.repositories {
            return;
        }
        self.face.chosen = self.face.chosen.filter(|&at| at < listed.len());
        self.face.repositories = listed;
        cx.notify();
    }

    /// The projects Fleet serves, by the repositories it serves. The window stays on the project it is
    /// scoped to by its id, and goes back to All where Fleet no longer lists it.
    pub fn set_projects(&mut self, served: &[ipc::RepositorySummary], read: &ipc::Projects, cx: &mut Context<Self>) {
        let listed = face::projects_of(served, read);
        let here = self.face.scoped.and_then(|at| self.face.projects.get(at)).map(|one| one.id.clone());
        let scoped = here.and_then(|id| listed.iter().position(|one| one.id == id));
        if listed == self.face.projects && scoped == self.face.scoped {
            return;
        }
        self.face.projects = listed;
        self.face.scoped = scoped;
        cx.notify();
    }

    /// Whether the window is narrower than the row can lay out: the start, the centre and the
    /// Operator button go behind the menu.
    fn narrow(&self, window: &Window) -> bool {
        self.width.unwrap_or(window.viewport_size().width) < px(NARROW_BELOW)
    }

    /// Put the open Job's Layout control and view icons in the end region, or take them out. Empty,
    /// the slot takes no room.
    pub fn fill_job_slot(&mut self, view: Option<AnyView>, cx: &mut Context<Self>) {
        self.job = view;
        cx.notify();
    }

    /// Whether the end region holds a screen's own control.
    pub fn job_slot_filled(&self) -> bool {
        self.job.is_some()
    }

    /// A press asked for something and Fleet refused: the face goes back to what it was, and the
    /// build's failed line says why.
    #[allow(dead_code)]
    pub fn refused(&mut self, why: &str, cx: &mut Context<Self>) {
        if let Some(before) = self.before.take() {
            self.face = before;
        }
        if let Some(build) = self.face.build.as_mut() {
            build.report.failed = Some(why.to_string());
        }
        self.faced(cx);
    }

    /// Change the face for an optimistic answer, keeping the one it replaces.
    fn answer(&mut self, change: impl FnOnce(&mut Face), cx: &mut Context<Self>) {
        self.before = Some(self.face.clone());
        change(&mut self.face);
        self.faced(cx);
    }

    // -- keys and focus ----------------------------------------------------------------------

    /// Remember what held the keys, once, before a popover or dialog takes them.
    fn keep_keys(&mut self, window: &Window, cx: &App) {
        if self.keys_were.is_none() {
            self.keys_were = window.focused(cx);
        }
    }

    fn give_keys_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(was) = self.keys_were.take() {
            window.focus(&was, cx);
        }
    }

    fn shut(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.meter_by_keys = false;
        self.popup = None;
        self.over_card = false;
        self.menu = None;
        self.dialog = None;
        self.give_keys_back(window, cx);
        cx.notify();
    }

    /// Esc: the innermost thing open shuts first.
    fn escape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dialog.is_some() {
            self.dialog = None;
            let focus = self.popup_focus.clone();
            if self.popup.is_some() {
                window.focus(&focus, cx);
            } else {
                self.give_keys_back(window, cx);
            }
            cx.notify();
        } else if self.menu.is_some() {
            self.menu = None;
            let focus = self.popup_focus.clone();
            if self.popup.is_some() {
                window.focus(&focus, cx);
            } else {
                self.give_keys_back(window, cx);
            }
            cx.notify();
        } else if self.popup.is_some() {
            self.shut(window, cx);
        }
    }

    fn open_popup(&mut self, popup: Popup, window: &mut Window, cx: &mut Context<Self>) {
        self.keep_keys(window, cx);
        self.popup = Some(popup);
        let focus = self.popup_focus.clone();
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
        cx.notify();
    }

    // -- the popovers --------------------------------------------------------------------------

    /// Hover or focus opens the plan card; leaving shuts it unless pinned, after the time it takes
    /// to cross to the card.
    fn meter_hovered(&mut self, over: bool, window: &mut Window, cx: &mut Context<Self>) {
        if over {
            self._leaving = None;
            if self.popup.is_none() && self.dialog.is_none() && self.menu.is_none() {
                self.keep_keys(window, cx);
                self.popup = Some(Popup::Usage { pinned: false });
                cx.notify();
            }
        } else if self.popup == Some(Popup::Usage { pinned: false }) {
            self._leaving = Some(cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(Duration::from_millis(150)).await;
                let _ = this.update_in(cx, |this, window, cx| {
                    if !this.over_card && this.popup == Some(Popup::Usage { pinned: false }) {
                        this.popup = None;
                        this.give_keys_back(window, cx);
                        cx.notify();
                    }
                });
            }));
        }
    }

    /// The keys arrived on the meter or left it: from the keyboard the card opens as a hover opens it, and
    /// goes as the keys go.
    fn meter_focused(&mut self, arrived: bool, window: &mut Window, cx: &mut Context<Self>) {
        if arrived {
            if window.last_input_was_keyboard() && !self.meter_quiet && self.popup.is_none() && self.dialog.is_none() && self.menu.is_none() {
                self.meter_by_keys = true;
                self.meter_hovered(true, window, cx);
            }
        } else {
            self.meter_quiet = false;
            if self.meter_by_keys && self.popup == Some(Popup::Usage { pinned: false }) && !self.over_card {
                self.meter_by_keys = false;
                self.popup = None;
                self.keys_were = None;
                cx.notify();
            }
        }
    }

    /// A press on the meter pins its card open, or shuts a pinned one.
    fn meter_pressed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.meter_by_keys = false;
        if self.popup == Some(Popup::Usage { pinned: true }) {
            self.shut(window, cx);
        } else {
            self.keep_keys(window, cx);
            self.open_popup(Popup::Usage { pinned: true }, window, cx);
        }
    }

    fn fleet_pressed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup == Some(Popup::Fleet) {
            self.shut(window, cx);
        } else {
            self.menu = None;
            self.make_origin_search(window, cx);
            self.open_popup(Popup::Fleet, window, cx);
            cx.emit(Asked::ReadDisk);
        }
    }

    /// The menu a trigger opens: the entries, under the trigger.
    fn open_menu(&mut self, kind: MenuKind, entries: Vec<Entry>, name: &'static str, from: Bounds<Pixels>, end: bool, window: &mut Window, cx: &mut Context<Self>) {
        let gap = cx.look().length(token::SPACE_1);
        let at = point(if end { from.origin.x + from.size.width } else { from.origin.x }, from.origin.y + from.size.height + gap);
        let corner = if end { Anchor::TopRight } else { Anchor::TopLeft };
        self.open_menu_on(kind, entries, name, at, corner, false, false, window, cx);
    }

    /// A menu with its corner at `at`: under a trigger, or at the pointer for a right press. The keys go
    /// on its first entry when `from_keys`, as a menu opened from the keyboard has them.
    #[allow(clippy::too_many_arguments)]
    fn open_menu_at(&mut self, kind: MenuKind, entries: Vec<Entry>, name: &'static str, at: Point<Pixels>, corner: Anchor, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.open_menu_on(kind, entries, name, at, corner, from_keys, true, window, cx);
    }

    /// `pointed`: opened at a point, so Mod and Report follow its entries (`point-acts.tsx`).
    #[allow(clippy::too_many_arguments)]
    fn open_menu_on(&mut self, kind: MenuKind, entries: Vec<Entry>, name: &'static str, at: Point<Pixels>, corner: Anchor, from_keys: bool, pointed: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.keep_keys(window, cx);
        let dropdown = cx.new(|cx| {
            let menu = Dropdown::new(name, entries, at, corner, from_keys, cx);
            if pointed { menu.at_point(window, cx) } else { menu }
        });
        let picked = cx.subscribe_in(&dropdown, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id, keep } = picked;
            // A pick that keeps the menu leaves it where it is, with the keys in it.
            if !*keep {
                this.menu = None;
            }
            this.picked(kind, id, window, cx);
        });
        let dismissed = cx.subscribe_in(&dropdown, window, |this, _, _: &DismissEvent, window, cx| {
            this.menu = None;
            if this.popup.is_some() {
                let focus = this.popup_focus.clone();
                window.focus(&focus, cx);
            } else {
                this.give_keys_back(window, cx);
            }
            cx.notify();
        });
        let keys = dropdown.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.menu = Some(Menu { dropdown, kind, _picked: picked, _dismissed: dismissed });
        cx.notify();
    }

    fn picked(&mut self, kind: MenuKind, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        match (kind, id) {
            (MenuKind::Account, "all") => self.choose_repository(None, window, cx),
            (MenuKind::Account, id) if id.starts_with("project-") => {
                let at = id["project-".len()..].parse::<usize>().unwrap_or(1).saturating_sub(1);
                self.give_keys_back(window, cx);
                self.answer(|face| {
                    face.scoped = Some(at);
                    face.chosen = None;
                }, cx);
                cx.emit(Asked::Project(at));
            }
            (MenuKind::Account, id) if REPO.contains(&id) => {
                if let Some(at) = REPO.iter().position(|one| *one == id) {
                    self.choose_repository(Some(at), window, cx);
                }
            }
            (MenuKind::Account, "scoped-settings") => {
                let asked = self.settings_here();
                self.leave_with(asked, window, cx);
            }
            (MenuKind::Account, "search") => self.leave_with(Asked::Palette, window, cx),
            (MenuKind::Account, "dispatch") => self.leave_with(Asked::Dispatch, window, cx),
            (MenuKind::Account, "new-project") => self.leave_with(Asked::NewProject, window, cx),
            (MenuKind::Account, "operator") => self.leave_with(Asked::Operator, window, cx),
            (MenuKind::Build, "main") => self.change_build(BuildSource::Main, window, cx),
            (MenuKind::Build, "preview") => self.change_build(BuildSource::Preview, window, cx),
            (MenuKind::Account, "account") => self.leave_with(Asked::Account, window, cx),
            (MenuKind::Account, "settings") => self.leave_with(Asked::Settings, window, cx),
            (MenuKind::Account, "doctor") => self.leave_with(Asked::Doctor, window, cx),
            (MenuKind::Teach, "teach") => self.leave_with(Asked::Teach("Title bar"), window, cx),
            (MenuKind::Places, id) => {
                let Some((act, place)) = id.split_once(':') else { return };
                if act == "hide" {
                    self.give_keys_back(window, cx);
                }
                self.place_shown(place, act == "view" && !self.face.places.iter().any(|one| one.id == place && one.shown), cx);
            }
            _ => {}
        }
    }

    /// An act that leaves the bar: the keys go back where they were and the app is asked.
    fn leave_with(&mut self, asked: Asked, window: &mut Window, cx: &mut Context<Self>) {
        self.popup = None;
        self.give_keys_back(window, cx);
        cx.emit(asked);
        cx.notify();
    }

    fn choose_repository(&mut self, at: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        self.give_keys_back(window, cx);
        self.answer(
            |face| {
                face.chosen = at;
                face.scoped = None;
            },
            cx,
        );
        cx.emit(Asked::Repository(at));
    }

    // -- the presses ---------------------------------------------------------------------------

    /// Restart onto `build`. Any Jobs a restart would adopt are named first; none restarts at once.
    fn change_build(&mut self, build: BuildSource, window: &mut Window, cx: &mut Context<Self>) {
        let adopts = self.face.build.as_ref().is_some_and(|held| !held.adopts.is_empty());
        if adopts {
            self.keep_keys(window, cx);
            self.dialog = Some(Dialog::Drones(Held::Choose(build)));
            let focus = self.dialog_focus.clone();
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
            cx.notify();
        } else {
            self.restart_onto(build, false, cx);
        }
    }

    /// The restart button: merge the preview again, or move to the latest main.
    fn restart_pressed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(on) = self.face.build.as_ref().map(|held| held.report.on) else { return };
        let adopts = self.face.build.as_ref().is_some_and(|held| !held.adopts.is_empty());
        if adopts {
            self.keep_keys(window, cx);
            self.dialog = Some(Dialog::Drones(Held::Restart));
            let focus = self.dialog_focus.clone();
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
            cx.notify();
        } else {
            self.restart_onto(on, false, cx);
        }
    }

    /// The restart is under way in this frame: the button turns to its pending word and the select goes off.
    fn restart_onto(&mut self, build: BuildSource, adopt: bool, cx: &mut Context<Self>) {
        self.answer(
            |face| {
                if let Some(held) = face.build.as_mut() {
                    held.report.restarting = Some(build);
                    held.report.stage = None;
                    held.report.failed = None;
                }
            },
            cx,
        );
        cx.emit(Asked::ChangeBuild { build, adopt });
    }

    fn drones_confirmed(&mut self, held: Held, window: &mut Window, cx: &mut Context<Self>) {
        let Some(on) = self.face.build.as_ref().map(|held| held.report.on) else { return };
        self.dialog = None;
        let focus = self.popup_focus.clone();
        if self.popup.is_some() {
            window.focus(&focus, cx);
        } else {
            self.give_keys_back(window, cx);
        }
        let build = match held {
            Held::Choose(build) => build,
            Held::Restart => on,
        };
        self.restart_onto(build, true, cx);
    }

    fn revert_pressed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.keep_keys(window, cx);
        self.dialog = Some(Dialog::Revert);
        let focus = self.dialog_focus.clone();
        window.defer(cx, move |window, cx| window.focus(&focus, cx));
        cx.notify();
    }

    /// Revert, confirmed or from the trying bar: under way in this frame.
    fn revert_confirmed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        if self.popup.is_some() {
            let focus = self.popup_focus.clone();
            window.focus(&focus, cx);
        } else {
            self.give_keys_back(window, cx);
        }
        self.answer(
            |face| {
                if let Some(held) = face.build.as_mut() {
                    held.report.reverting = true;
                    held.report.failed = None;
                }
            },
            cx,
        );
        cx.emit(Asked::Revert);
    }

    fn keep_pressed(&mut self, cx: &mut Context<Self>) {
        self.answer(
            |face| {
                if let Some(held) = face.build.as_mut() {
                    held.report.trying.clear();
                }
            },
            cx,
        );
        cx.emit(Asked::Keep);
    }

    fn swap_pressed(&mut self, cx: &mut Context<Self>) {
        self.answer(
            |face| {
                if let Some(held) = face.build.as_mut() {
                    held.report.swapping = true;
                }
            },
            cx,
        );
        cx.emit(Asked::Swap);
    }

    fn unpick_pressed(&mut self, branch: String, cx: &mut Context<Self>) {
        let gone = branch.clone();
        self.answer(
            move |face| {
                if let Some(held) = face.build.as_mut() {
                    held.report.picked.retain(|one| one.branch != gone);
                }
            },
            cx,
        );
        cx.emit(Asked::Unpick(branch));
    }

    fn restart_fleet_pressed(&mut self, cx: &mut Context<Self>) {
        self.restart_from = match self.face.fleet.connection {
            Connection::Mismatch { pid, .. } => Some(pid),
            _ => None,
        };
        self.answer(|face| face.fleet.restarting = Restarting::Working, cx);
        // No new Fleet inside the wait is a failure of its own; a Fleet that answered or came back apart ended it.
        self._restart_wait = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(RESTART_WAIT).await;
            let _ = this.update(cx, |bar, cx| {
                if bar.face.fleet.restarting == Restarting::Working {
                    bar.restart_failed(face::RestartWhy::NoAnswer, format!("no new runtime file in {}s", RESTART_WAIT.as_secs()), cx);
                }
            });
        }));
        cx.emit(Asked::RestartFleet);
    }

    /// One of the step-down acts: the card or the notification changes in this frame, and Fleet is asked.
    fn remedy_pressed(&mut self, remedy: Remedy, window: &mut Window, cx: &mut Context<Self>) {
        self.answer(
            |face| {
                face.downshift = match (remedy, face.downshift.take()) {
                    (Remedy::Accept, Some((ModelDownshift::Offered { was, to, .. }, until))) => {
                        Some((ModelDownshift::Shifted { was, to, until: ipc::Instant::carried(until.to_string()) }, until))
                    }
                    _ => None,
                };
            },
            cx,
        );
        // Choose goes on to the Session model setting, which this app does not draw yet.
        let stays = remedy == Remedy::Accept;
        if !stays {
            self.shut(window, cx);
        }
        cx.emit(Asked::Remedy(remedy));
    }

    // -- drawing -------------------------------------------------------------------------------

    fn focus_ring(look: &Look) -> gpui::Hsla {
        look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour)
    }

    /// The first region: the traffic-light gutter, Armada's mark, the project switcher and its Settings.
    fn start(&self, look: &Look, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let mark = look.length(token::SPACE_4) + look.length(token::SPACE_1);
        let ring = Self::focus_ring(look);
        let glyph_side = look.length(token::SPACE_4);
        let chip_at = Rc::clone(&self.at.repository);
        let opened = self.popup == Some(Popup::Projects);
        let (name, members) = self.face.chosen_said();
        // Short of room the repositories beside the name go whole, so nothing short reads "…", then the name
        // truncates. The project's Settings is in the switcher, so the start holds no button of its own. The
        // start's share was measured last frame; it does not hang on what it holds.
        let room = self.at.start.get().map(|start| start.size.width);
        let space = |at| look.length(at);
        let border = look.length(token::BORDER_WIDTH);
        let wide = |text: &SharedString, size, weight| {
            let font = gpui::Font { weight: FontWeight(look.number(weight)), ..gpui::font(sans(look)) };
            window.text_system().shape_line(text.clone(), look.length(size), &[gpui::TextRun { len: text.len(), font, ..Default::default() }], None).width
        };
        let lead = space(token::SPACE_12) * 2. + mark + space(token::SPACE_6) + border + space(token::SPACE_3) * 2.;
        let chip = space(token::SPACE_3) * 3. + glyph_side + space(token::SPACE_1) + space(token::SPACE_2) + border * 2. + wide(&name, token::TEXT_SM, token::WEIGHT_MEDIUM);
        let fits = |width| room.is_some_and(|room| lead + chip + width <= room);
        let members = members.filter(|said| fits(space(token::SPACE_2) + wide(said, token::TEXT_XS, token::WEIGHT_BODY)));
        let chip = labelled(
            no_drag(div()
                .id("repository")
                .relative()
                .flex_initial()
                // Its floor keeps the glyph, a few letters of the name and the chevron inside the border.
                .min_w(look.length(token::SPACE_3) * 2. + glyph_side + look.length(token::SPACE_1) + look.length(token::SPACE_12) + look.length(token::SPACE_2) + look.length(token::SPACE_3))
                .max_w(look.length(token::W_MENU))
                .h(look.length(token::H_CONTROL_LG))
                .flex()
                .items_center()
                .pl(look.length(token::SPACE_3) + glyph_side + look.length(token::SPACE_1))
                .pr(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(if opened { token::ACCENT } else { token::BORDER_DEFAULT }))
                .bg(look.color(if opened { token::ACCENT_MUTED } else { token::BG_SUNKEN }))
                .when(!opened, |chip| chip.hover(|style| style.bg(look.color(token::BG_HOVER)).border_color(look.color(token::BORDER_STRONG))))
                .focus_visible(move |style| style.border_color(ring))
                .focusable()
                .tab_stop(true)
                .cursor_pointer()
                .font_family(sans(look))
                .text_size(look.length(token::TEXT_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(canvas(move |bounds, _, _| chip_at.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
                .child(
                    div()
                        .absolute()
                        .left(look.length(token::SPACE_3))
                        .top_0()
                        .bottom_0()
                        .flex()
                        .items_center()
                        .child(glyph("layers", glyph_side, look.color(token::ACCENT))),
                )
                // Short of room the members truncate, then the name, and the chevron stays inside the border.
                .overflow_hidden()
                .child(div().min_w(look.length(token::SPACE_12)).truncate().child(name))
                .children(members.map(|said| {
                    // Many times the name's give: the members go before the name starts to truncate.
                    div().flex_shrink(100.).min_w_0().truncate().ml(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_BODY))).text_color(look.color(token::FG_MUTED)).child(said)
                }))
                .child(div().flex_none().ml(look.length(token::SPACE_2)).child(glyph("chevron-down", look.length(token::SPACE_3), look.color(token::FG_MUTED))))
                .on_click(cx.listener(|this, _, window, cx| this.repository_pressed(window, cx)))),
            Role::Button,
            "Project",
        );
        let start_at = Rc::clone(&self.at.start);
        div()
            // `minmax(floor, 1fr)`: the start and the end take equal shares from nothing, so the centre sits
            // on the window's middle whatever the picker reads. Short of room the start yields first, the
            // project's name truncating, and it never draws under the centre.
            .relative()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .flex()
            .items_center()
            // A new share is drawn again, so what fits is read from it.
            .child(
                canvas(
                    move |bounds, window, _| {
                        if start_at.replace(Some(bounds)).map(|was| was.size.width) != Some(bounds.size.width) {
                            window.refresh();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            // `padding-inline-start: calc(var(--space-12) * 2)` clears the traffic lights. A spacer and not padding: a
            // box sized from nothing still keeps its padding, which would make the start's share wider than the end's.
            .child(div().flex_none().w(look.length(token::SPACE_12) * 2.))
            .child(labelled(
                div().id("mark").flex_none().child(glyph_mark(mark, look.color(token::FG_DEFAULT))),
                Role::Image,
                "Armada",
            ))
            .child(
                div()
                    .min_w_0()
                    .ml(look.length(token::SPACE_6))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .child(div().flex_none().w(look.length(token::BORDER_WIDTH)).h(look.length(token::SPACE_4)).bg(look.color(token::BORDER_DEFAULT)))
                    .child(chip),
            )
    }

    fn repository_pressed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup == Some(Popup::Projects) {
            self.shut(window, cx);
            return;
        }
        self.open_switcher(window, cx);
    }

    /// The switcher's rows inside the menu of a narrow window: All, each project over its
    /// repositories, then those in no project, those not set up with their detail.
    fn repository_entries(&self) -> Vec<Entry> {
        let face = &self.face;
        let mut entries = vec![Entry::Search { label: "Find a project", to_rule: true }];
        entries.push(Entry::Row(Row::new("all", "All").icon("globe").ticked(face.chosen.is_none() && face.scoped.is_none())));
        for (p, project) in face.projects.iter().enumerate() {
            entries.push(Entry::Row(Row::new(id_of(format!("project-{}", p + 1)), project.name.clone()).icon("layers").ticked(face.scoped == Some(p))));
        }
        for at in face.lone().into_iter().take(REPO.len()) {
            let one = &face.repositories[at];
            let row = Row::new(REPO[at], one.name.clone()).icon("git-branch").member().ticked(face.chosen == Some(at) && face.scoped.is_none());
            entries.push(Entry::Row(if one.not_set_up { row.detail("Not set up") } else { row }));
        }
        // Last, as the switcher's left pane ends on it.
        entries.push(Entry::Row(Row::new("new-project", "New project").icon("plus")));
        entries
    }

    /// The menu's rows: the account (Sign in until a profile is the active one), Settings and
    /// Doctor, and below a width of 900px everything the row above carries.
    fn menu_entries(&self, narrow: bool) -> Vec<Entry> {
        let signed_in = self.face.signed_in;
        let account = Row::new("account", if signed_in { "Account" } else { "Sign in" }).icon("user-check");
        let settings_key = act("open_settings").map(|row| row.shortcut).unwrap_or_default();
        let mut entries = vec![
            Entry::Row(if signed_in { account } else { account.attention() }),
            Entry::Row(Row::new("settings", "Settings").icon("settings").key(settings_key)),
            Entry::Row(Row::new("doctor", "Doctor").icon("stethoscope")),
        ];
        if narrow {
            entries.push(Entry::Rule);
            entries.extend(self.repository_entries());
            entries.push(Entry::Rule);
            entries.push(Entry::Row(Row::new("scoped-settings", self.face.settings_said()).icon("sliders-horizontal")));
            let key = |id: &str| act(id).map(|row| row.shortcut).unwrap_or_default();
            entries.push(Entry::Row(Row::new("search", "Search").icon("search").key(key("command_palette"))));
            entries.push(Entry::Row(Row::new("dispatch", "Dispatch").icon("plus").key(key("new_job")).off_when(!self.face.fleet.connection.is_live(), "Fleet is not connected")));
            entries.push(Entry::Row(Row::new("operator", "Operator").icon("bot").key(key("operator"))));
        }
        entries
    }

    /// The search trigger's name. Drawn as an icon and its key beside the places, the key joins the
    /// name (`TitleBar.tsx`'s `SEARCH_ONLY`); alone, its label is its name.
    pub fn search_name(with_places: bool) -> String {
        let key = act("command_palette").map(|row| row.shortcut).unwrap_or_default();
        match (with_places, key.is_empty()) {
            (true, false) => format!("Search, or describe work to dispatch ({key})"),
            _ => "Search, or describe work to dispatch".into(),
        }
    }

    /// The centre. With no place to draw it is the search trigger at Electron's width; with places it
    /// is the notch of glass hanging from the window's top edge, the places and then the trigger as an
    /// icon and its key (`TitleBar.css` 262-350).
    fn centre(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let at = Rc::clone(&self.at.centre);
        if self.face.drawn_places().next().is_none() {
            return self
                .search_field(look, cx)
                .relative()
                .child(canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
                .into_any_element();
        }
        // The room above and below a place, which is also the room at the notch's side.
        let gap = (look.length(token::H_TITLE_BAR) - look.length(token::H_CONTROL_LG)) / 2.;
        let radius = look.length(token::RADIUS_LG) * 2.;
        let ground = look.color(token::BG_GLASS);
        // The flare: a corner square with a quarter disc cut from its lower outer corner, so the notch runs into the edge.
        let flare = move |left: bool| {
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let (l, r, top, bottom) = (bounds.origin.x, bounds.origin.x + bounds.size.width, bounds.origin.y, bounds.origin.y + bounds.size.height);
                    let radii = point(bounds.size.width, bounds.size.height);
                    let mut path = PathBuilder::fill();
                    if left {
                        path.move_to(point(l, top));
                        path.line_to(point(r, top));
                        path.line_to(point(r, bottom));
                        path.arc_to(radii, px(0.), false, false, point(l, top));
                    } else {
                        path.move_to(point(r, top));
                        path.line_to(point(l, top));
                        path.line_to(point(l, bottom));
                        path.arc_to(radii, px(0.), false, true, point(r, top));
                    }
                    path.close();
                    if let Ok(path) = path.build() {
                        window.paint_path(path, ground);
                    }
                },
            )
            .absolute()
            .top_0()
            .size(radius)
        };
        div()
            .id("centre")
            .relative()
            .flex_none()
            .self_stretch()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(gap - look.length(token::BORDER_WIDTH))
            .rounded_b(radius)
            .bg(ground)
            .shadow(look.shadows(token::SHADOW_CARD))
            .child(canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            .child(flare(true).left(-radius))
            .child(flare(false).right(-radius))
            .child(self.places_strip(gap, radius, look, cx))
            // A hairline parts the places from the search beside them.
            .child(
                div()
                    .flex_none()
                    .w(look.length(token::BORDER_WIDTH))
                    .h(look.length(token::SPACE_4) + look.length(token::SPACE_1))
                    .bg(look.color(token::BORDER_DEFAULT)),
            )
            .child(self.search_icon(look, cx))
            .into_any_element()
    }

    /// The strip of places, one line: each a glyph and its name, the one the window is in brighter and on
    /// a faint fill. **Only the places the native app has a screen for are in `face.places`** (Jobs); the
    /// Cockpit, Operator and the rest of Electron's roster come with their screens.
    fn places_strip(&self, gap: Pixels, radius: Pixels, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let ring = Self::focus_ring(look);
        let round = radius - gap;
        let tabs = self.face.drawn_places().map(|place| {
            let (id, label, glyph_name) = (place.id, place.label, place.glyph);
            let here = self.face.here == Some(id);
            let ink = look.color(if here { token::FG_DEFAULT } else { token::FG_MUTED });
            let keys = self.place_focus.iter().find(|(one, _)| *one == id).map(|(_, handle)| handle.clone());
            labelled(
                div()
                    .id(SharedString::from(format!("place-{id}")))
                    .when_some(keys.clone(), |tab, keys| tab.track_focus(&keys))
                    // Not `no_drag`: a press on a place takes the keys, which `no_drag` keeps from it.
                    .on_mouse_down(MouseButton::Left, {
                        let keys = keys.clone();
                        move |_, window, cx| {
                            if let Some(keys) = &keys {
                                window.focus(keys, cx);
                            }
                            cx.stop_propagation();
                        }
                    })
                    .flex_none()
                    .h(look.length(token::H_CONTROL_LG))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .px(look.length(token::SPACE_2))
                    .rounded(round)
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(gpui::transparent_black())
                    .font_family(sans(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .text_color(ink)
                    .when(here, |tab| {
                        tab.font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).bg(look.color(token::FG_DEFAULT).opacity(0.08))
                    })
                    .cursor_pointer()
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_color(ring))
                    .hover(move |style| style.text_color(look.color(token::FG_DEFAULT)))
                    .child(glyph(glyph_name, look.length(token::SPACE_4), ink))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| this.place_pressed(id, cx)))
                    .on_mouse_down(MouseButton::Right, cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.open_places_menu(Some(id), event.position, false, window, cx);
                    }))
                    .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                        if !Self::asks_for_menu(event) {
                            return;
                        }
                        cx.stop_propagation();
                        let tab = crate::labels::role_named("tab").and_then(|role| {
                            cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, label)).map(|drawn| drawn.bounds)
                        });
                        let at = tab.map_or_else(Point::default, |tab| point(tab.origin.x + tab.size.width / 2., tab.origin.y + tab.size.height));
                        this.open_places_menu(Some(id), at, true, window, cx);
                    })),
                Role::Tab,
                label,
            )
        });
        labelled(
            no_drag(div()
                .id("places")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .children(tabs)
                .on_mouse_down(MouseButton::Right, cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_places_menu(None, event.position, false, window, cx);
                }))),
            Role::TabList,
            "Places",
        )
    }

    /// Shift+F10: Electron also takes the ContextMenu key (`TabBar.tsx:149`), which gpui does not
    /// deliver on macOS, so a menu from the keyboard is Shift+F10 alone.
    fn asks_for_menu(event: &gpui::KeyDownEvent) -> bool {
        event.keystroke.key == "f10" && event.keystroke.modifiers.shift && !event.keystroke.modifiers.platform && !event.keystroke.modifiers.alt
    }

    /// A press on a place, or ⌘ and its number: the window goes there.
    fn place_pressed(&mut self, id: &'static str, cx: &mut Context<Self>) {
        self.face.here = Some(id);
        cx.emit(Asked::Place(id));
        cx.notify();
    }

    /// ⌘1 to ⌘9: the place drawn at that number.
    fn go_to_digit(&mut self, n: u8, cx: &mut Context<Self>) {
        let id = self.face.drawn_places().nth(usize::from(n).saturating_sub(1)).map(|place| place.id);
        if let Some(id) = id {
            self.place_pressed(id, cx);
        }
    }

    /// The layout shows or hides a place, at once; the wiring writes the person's own layout.
    fn place_shown(&mut self, id: &str, shown: bool, cx: &mut Context<Self>) {
        let Some(place) = self.face.places.iter().position(|one| one.id == id) else { return };
        let id = self.face.places[place].id;
        if !shown && !self.face.may_hide(id) {
            return;
        }
        self.answer(|face| face.places[place].shown = shown, cx);
        cx.emit(Asked::PlaceShown { id, shown });
    }

    /// The strip's menu: Hide for the place pressed, then View with every place and whether it is drawn.
    /// A place that may not be hidden is off and says why; a View pick leaves the menu open.
    fn open_places_menu(&mut self, tab: Option<&'static str>, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.dialog.is_some() || self.menu.is_some() {
            return;
        }
        const ALWAYS: &str = "Always shown";
        let mut entries = Vec::new();
        if let Some(place) = tab.and_then(|id| self.face.places.iter().find(|one| one.id == id)) {
            let free = self.face.may_hide(place.id);
            let mut hide = Row::new(id_of(format!("hide:{}", place.id)), format!("Hide {}", place.label)).off_when(!free, ALWAYS);
            if !free {
                hide = hide.detail(ALWAYS);
            }
            entries.push(Entry::Row(hide));
            entries.push(Entry::Rule);
        }
        entries.push(Entry::Label("View"));
        for place in &self.face.places {
            // A hidden place may always come back; a drawn one goes only where the layout lets it.
            let free = !place.shown || self.face.may_hide(place.id);
            let mut row = Row::new(id_of(format!("view:{}", place.id)), place.label).ticked(place.shown).keeps_open().off_when(!free, ALWAYS);
            if !free {
                row = row.detail(ALWAYS);
            }
            entries.push(Entry::Row(row));
        }
        self.open_menu_at(MenuKind::Places, entries, "Places", at, Anchor::TopLeft, from_keys, window, cx);
    }

    /// The menu a right press on the bar offers: its heading and Teach.
    fn open_teach_menu(&mut self, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.dialog.is_some() || self.menu.is_some() {
            return;
        }
        let entries = vec![Entry::Label("Title bar"), Entry::Row(Row::new("teach", "Teach").icon("graduation-cap"))];
        self.open_menu_at(MenuKind::Teach, entries, "Title bar", at, Anchor::TopLeft, from_keys, window, cx);
    }

    /// The trigger beside the places: an icon and its key, with the label hidden and its tooltip naming it.
    fn search_icon(&self, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let ring = Self::focus_ring(look);
        let key = act("command_palette").map(|row| row.shortcut).unwrap_or_default();
        let name = Self::search_name(true);
        labelled(
            no_drag(div()
                .id("search")
                .flex_none()
                .h(look.length(token::H_CONTROL_LG))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .text_color(look.color(token::FG_MUTED))
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .cursor_pointer()
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .hover(move |style| style.text_color(look.color(token::FG_DEFAULT)))
                .tooltip(tip(name.clone()))
                .child(glyph("search", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                .when(!key.is_empty(), |trigger| trigger.child(parts::kbd(key, look)))
                .on_click(cx.listener(|_, _, _, cx| cx.emit(Asked::Palette)))),
            Role::Button,
            name,
        )
    }

    /// The search trigger alone, at Electron's width, its key drawn from the registry.
    fn search_field(&self, look: &Look, cx: &Context<Self>) -> gpui::Div {
        let ring = Self::focus_ring(look);
        let key = act("command_palette").map(|row| row.shortcut).unwrap_or_default();
        // `minmax(0, auto)`: the centre gives way first, down to nothing.
        div().w(look.length(token::PALETTE_WIDTH)).flex_shrink(1.).min_w_0().flex().items_center().child(labelled(
            no_drag(div()
                .id("search")
                .w_full()
                .min_w_0()
                .h(look.length(token::H_CONTROL_LG))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .text_color(look.color(token::FG_SUBTLE))
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .cursor_pointer()
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(glyph("search", look.length(token::SPACE_4), look.color(token::FG_SUBTLE)))
                .child(div().flex_1().min_w_0().truncate().child("Search, or describe work to dispatch"))
                .when(!key.is_empty(), |trigger| trigger.child(parts::kbd(key, look)))
                .on_click(cx.listener(|_, _, _, cx| cx.emit(Asked::Palette)))),
            Role::Button,
            "Search, or describe work to dispatch",
        ))
    }

    /// The end region: the Job slot, the rings, the toasts' bell, the Operator, the Fleet dot and the Menu.
    fn end(&self, narrow: bool, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let ring = Self::focus_ring(look);
        let operator = {
            let key = act("operator").map(|row| row.shortcut).unwrap_or_default();
            labelled(
                no_drag(div()
                    .id("operator")
                    .flex_none()
                    .h(look.length(token::H_CONTROL_LG))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_RAISED))
                    .text_size(look.length(token::TEXT_XS))
                    .font_family(sans(look))
                    .text_color(look.color(token::FG_DEFAULT))
                    .cursor_pointer()
                    .focusable()
                    .tab_stop(true)
                    .focus_visible(move |style| style.border_color(ring))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(glyph("bot", look.length(token::SPACE_4), look.color(token::OPERATOR)))
                    .tooltip(tip(format!("Open Operator — {key}")))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Asked::Operator)))),
                Role::Button,
                "Operator",
            )
        };
        let menu_at = Rc::clone(&self.at.menu);
        let signed_out = !self.face.signed_in;
        let menu = labelled(
            no_drag(parts::icon_button("menu", "menu", look.color(token::FG_MUTED), look)
                .track_focus(&self.menu_focus)
                .child(canvas(move |bounds, _, _| menu_at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .relative()
                // Nothing signed in: an amber dot at the button's bottom-right, read without the menu opened.
                .when(signed_out, |button| {
                    button.child(
                        div()
                            .absolute()
                            .right(look.length(token::SPACE_1))
                            .bottom(look.length(token::SPACE_1))
                            .size(look.length(token::SPACE_2))
                            .rounded_full()
                            .bg(look.color(token::STATUS_AWAITING_REVIEW)),
                    )
                })
                .tooltip(tip(if signed_out { "Menu, signed out" } else { "Menu" }))
                .on_click(cx.listener(|this, _, window, cx| this.menu_pressed(window, cx)))),
            Role::Button,
            "Menu",
        );
        let end_at = Rc::clone(&self.at.end);
        div()
            // `minmax(max-content, 1fr)`: the same share as the start, and never less than what it holds.
            .relative()
            .flex_1()
            .flex()
            .items_center()
            .justify_end()
            .gap(look.length(token::SPACE_4))
            .child(canvas(move |bounds, _, _| end_at.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            // Empty, it takes no room: a flex child with nothing in it and no gap of its own.
            .children(self.job.clone())
            .children(self.meter(look, cx))
            .children(cx.try_global::<crate::toasts::BellSlot>().map(|bell| bell.0.clone()))
            .when(!narrow, |end| end.child(operator))
            .child(self.dot(look, cx))
            .child(menu)
    }

    fn menu_pressed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.as_ref().is_some_and(|menu| menu.kind == MenuKind::Account) {
            self.menu = None;
            self.give_keys_back(window, cx);
            cx.notify();
            return;
        }
        let Some(from) = self.at.menu.get() else { return };
        let narrow = self.narrow(window);
        let entries = self.menu_entries(narrow);
        self.keys_were = Some(self.menu_focus.clone());
        self.open_menu(MenuKind::Account, entries, "Menu", from, true, window, cx);
    }
}

/// Armada's mark at `side`.
fn glyph_mark(side: Pixels, ink: gpui::Hsla) -> gpui::Svg {
    gpui::svg().path(MARK).size(side).text_color(ink)
}

impl Render for TitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Fleet's states go to the toasts when they change; the bar draws no band for them.
        let notices = self.fleet_notices();
        if notices != self.notices_sent {
            self.notices_sent = notices.clone();
            cx.defer(move |cx| crate::toasts::fleet(notices, cx));
        }
        let look = cx.look();
        let viewport = window.viewport_size();
        let narrow = self.narrow(window);
        let has_places = self.face.drawn_places().next().is_some();
        for place in self.face.drawn_places() {
            if !self.place_focus.iter().any(|(id, _)| *id == place.id) {
                self.place_focus.push((place.id, cx.focus_handle().tab_stop(true)));
            }
        }
        let row_at = Rc::clone(&self.at.row);
        let row = div()
            .id("title-row")
            .relative()
            .flex_none()
            .flex()
            .items_center()
            .h(look.length(token::H_TITLE_BAR))
            .child(canvas(move |bounds, _, _| row_at.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            .px(look.length(token::SPACE_3))
            .gap(look.length(token::SPACE_4))
            .on_mouse_down(gpui::MouseButton::Left, |event, window, _cx| {
                if event.click_count == 2 {
                    window.titlebar_double_click();
                } else {
                    window.start_window_move();
                }
            })
            // A right press on the bar, and Shift+F10 in it, offer Teach. A thing inside with its own menu
            // stops the press first.
            .on_mouse_down(MouseButton::Right, cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                this.open_teach_menu(event.position, false, window, cx);
            }))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if !Self::asks_for_menu(event) {
                    return;
                }
                let at = this.at.row.get().map_or_else(Point::default, |row| point(row.origin.x + row.size.width / 2., row.origin.y + row.size.height));
                this.open_teach_menu(at, true, window, cx);
            }))
            // Narrow, the start goes behind the menu and the end takes the rest of the row. The places stay:
            // they are how the window is moved about (`TitleBar.css` 765).
            .when(!narrow, |row| row.child(self.start(look, window, cx)))
            .when(!narrow || has_places, |row| row.child(self.centre(look, cx)))
            .child(self.end(narrow, look, cx));
        div()
            .flex_none()
            .when_some(self.width, |bar, width| bar.w(width))
            .flex()
            .flex_col()
            .child(row)
            .children(self.trying_bar(look, cx))
            .children(self.dropped_alert(look, cx))
            .children(self.link_layer(look, crate::room::of(window, cx), cx))
            .children(self.popover(look, window, cx))
            .children(self.dialog_layer(look, crate::room::of(window, cx), cx))
            .children(self.toast(look, viewport, cx))
            .children(self.swap_layer(look, crate::room::of(window, cx), cx))
            .children(self.menu.as_ref().map(|menu| menu.dropdown.clone()))
    }
}
