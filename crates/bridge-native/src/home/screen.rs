//! The Cockpit's home: the grid of blocks on the Cockpit's right side while no Job is open and the Operator is off. It is the
//! Dashboard's grid (`crate::grid`) with its own kinds, so Edit, Add block, a header that lifts a block, a corner that resizes
//! it and the view switch are the Dashboard's, drawn by the same code. What is the home's own is where the layout lives (a
//! per-user layout on Fleet, `ipc::HomeLayout`), the preset it draws until he has kept one, the Needs block's scope, and the
//! slim strip above the grid that holds the Operator's Switch on.
//!
//! Fleet keeps the layout whole, so a change is sent whole, after the grid has stood still for [`SAVE_AFTER`]; a read that comes
//! while a change is still to be sent is left to the change. Fleet tells every window a setting changed (`settings.changed`),
//! and the home reads the layout again then, which is how a layout saved in another window arrives.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{canvas, deferred, div, prelude::*, AnyElement, App, Bounds, Context, DragMoveEvent, Entity, EventEmitter, FocusHandle, KeyDownEvent, Pixels, Role, SharedString, Subscription, Task, WeakEntity, Window};
use ipc::{BoardBlock, BoardLayout};

use super::layout::{self, HomeLayout, HomePlaced};
use super::wire;
use super::{FloorPlan, HomeBlockKind, NeedsCards, NeedsScope, PlaceholderBlock, ScopeChanged};
use crate::features::FeaturesScreen;
use crate::fleet::FleetConnection;
use crate::generated::actions::ActionScope;
use crate::grid::chrome::{self, BlockDrag, Cells, Pressed};
use crate::grid::View;
use crate::jobs::JobsList;
use crate::keymap::Act;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::operator::page::OperatorPageView;
use crate::operator::view::{chip, press, pressed, Weight};
use crate::project_board::system::{posed, PoseRect, SystemBlock, SystemEvent};
use crate::settings::view::{bounds_of, ring_colour};
use crate::titlebar::parts::{card, glyph, sans, spinner, tip};

/// A change to the layout is sent when no other has come for this long: the Dashboard's own wait.
pub const SAVE_AFTER: Duration = Duration::from_millis(600);

/// What the home tells the screen it stands in.
pub enum HomeEvent {
    /// The Operator was switched off or on, so the home takes the right side or gives it back.
    Moved,
    /// A feature was pressed in the System overview: the Features place opens it.
    Feature(String),
}

impl EventEmitter<HomeEvent> for HomeScreen {}

enum Source {
    Live(Entity<FleetConnection>),
    Mock,
}

/// The layout as it stands here: what Fleet kept, else the preset, and the scope the Needs block holds.
struct Stored {
    layout: HomeLayout,
    /// He changed it, or Fleet kept one: it is sent as a layout. Until then it is the preset, and a save sends none.
    edited: bool,
    scope: NeedsScope,
    /// The blocks Fleet kept of kinds this build does not draw, sent back as they came.
    unknown: Vec<BoardBlock>,
    /// Counts the changes, so a save that was on its way when another was made does not clear the new one.
    rev: u32,
}

impl Stored {
    fn preset(rev: u32) -> Stored {
        Stored { layout: layout::preset(), edited: false, scope: NeedsScope::default(), unknown: Vec::new(), rev }
    }

    /// What Fleet keeps: the layout with the scope in the Needs block's view word, or none where it is the preset.
    fn to_wire(&self) -> Option<BoardLayout> {
        self.edited.then(|| layout::join(self.layout.to_wire(&self.unknown), self.scope))
    }
}

/// The menu beside Add block: the kinds that are not on the grid, and the row the keys are on.
struct AddMenu {
    choices: Vec<HomeBlockKind>,
    at: usize,
    focus: FocusHandle,
}

pub struct HomeScreen {
    source: Source,
    /// The Operator's page, whose Switch on the strip presses and whose being off puts the home on the right side.
    page: Entity<OperatorPageView>,
    features: Entity<FeaturesScreen>,
    needs: Entity<NeedsCards>,
    floor: Entity<FloorPlan>,
    system: Entity<SystemBlock>,
    stats: Entity<PlaceholderBlock>,
    stored: Stored,
    /// Fleet has said what it kept, or could not; until then the grid draws its blocks as bones.
    layout_read: bool,
    /// A change is waiting to be sent, or on its way.
    unsaved: bool,
    /// What a walk's or a test's Fleet keeps, where there is no Fleet.
    kept: Option<BoardLayout>,
    /// The port the layout was last read on, and the count of `settings.changed` it was read after.
    read_on: Option<u16>,
    settings_seen: u64,
    edit: bool,
    adding: Option<AddMenu>,
    /// The key handle each block holds, for the keys that move it.
    handles: Vec<(HomeBlockKind, FocusHandle)>,
    /// The page's own key handle, which the home's root tracks so the window's keys reach the Cockpit while it is drawn.
    keys: FocusHandle,
    /// Where a block was lifted, as an offset in cells.
    grab: Option<(i16, i16)>,
    /// How big the grid was drawn, which is what its cells are cut from.
    seen: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Whether the home stood on the right side when last looked at, to say when that moves.
    up: bool,
    saves: Option<Task<()>>,
    sending: Option<Task<()>>,
    reading: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl HomeScreen {
    /// On Fleet.
    pub fn live(fleet: &Entity<FleetConnection>, page: Entity<OperatorPageView>, features: Entity<FeaturesScreen>, list: Entity<JobsList>, cx: &mut Context<Self>) -> Self {
        let system = cx.new(|cx| SystemBlock::live(fleet, cx));
        let mut this = Self::new(Source::Live(fleet.clone()), page, features, list, system, cx);
        this.settings_seen = fleet.read(cx).held().settings_stamp;
        let watch = cx.observe(fleet, |this, fleet, cx| {
            let stamp = fleet.read(cx).held().settings_stamp;
            let changed = stamp != this.settings_seen;
            this.settings_seen = stamp;
            this.moved(changed, cx);
        });
        this._subs.push(watch);
        this.moved(false, cx);
        this
    }

    /// On the walk's Fleet, which keeps the layout in memory.
    pub fn mock(page: Entity<OperatorPageView>, features: Entity<FeaturesScreen>, list: Entity<JobsList>, cx: &mut Context<Self>) -> Self {
        let system = cx.new(SystemBlock::mock);
        let mut this = Self::new(Source::Mock, page, features, list, system, cx);
        this.layout_read = true;
        this.sync_blocks(cx);
        this
    }

    fn new(source: Source, page: Entity<OperatorPageView>, features: Entity<FeaturesScreen>, list: Entity<JobsList>, system: Entity<SystemBlock>, cx: &mut Context<Self>) -> Self {
        let needs = cx.new(|cx| NeedsCards::new(list.clone(), NeedsScope::default(), cx));
        let floor = cx.new(|cx| FloorPlan::plan(features.clone(), list, cx));
        let stats = cx.new(|_| PlaceholderBlock::headless(HomeBlockKind::SystemStats));
        let keys = page.read(cx).focus.clone();
        let mut subs = vec![
            // The Operator switched off or on: the right side is the home or the page.
            cx.observe(&page, |this, _, cx| this.follow_page(cx)),
            cx.observe(&system, |_, _, cx| cx.notify()),
            cx.subscribe(&system, |this, _, event: &SystemEvent, cx| match event {
                SystemEvent::Feature(slug) => cx.emit(HomeEvent::Feature(slug.clone())),
                SystemEvent::Job(id) => {
                    if let Some(sink) = cx.try_global::<crate::titlebar::AskedSink>().cloned() {
                        (sink.0)(&crate::titlebar::Asked::OpenJob(id.clone()), cx);
                    }
                    crate::palette::show_jobs(cx);
                    let _ = this;
                }
                SystemEvent::Held { id, words } => crate::toasts::dispatch_held(id, words, cx),
            }),
            // The scope he picked on the Needs block is kept with the layout.
            cx.subscribe(&needs, |this, _, scope: &ScopeChanged, cx| this.scope_picked(scope.0, cx)),
        ];
        // The title bar's project pick moves the System overview and the Needs scope.
        if let Some(bar) = crate::titlebar::live(cx) {
            subs.push(cx.observe(&bar, |this, _, cx| this.follow_project(cx)));
        }
        let up = page.read(cx).is_off();
        let handles = HomeBlockKind::ALL.into_iter().map(|kind| (kind, cx.focus_handle().tab_stop(true))).collect();
        let mut this = HomeScreen {
            source,
            page,
            features,
            needs,
            floor,
            system,
            stats,
            stored: Stored::preset(0),
            layout_read: false,
            unsaved: false,
            kept: None,
            read_on: None,
            settings_seen: 0,
            edit: false,
            adding: None,
            handles,
            keys,
            grab: None,
            seen: Rc::default(),
            up,
            saves: None,
            sending: None,
            reading: None,
            _subs: subs,
        };
        this.needs.update(cx, |needs, cx| needs.set_hosted(true, cx));
        this.follow_project(cx);
        this.sync_blocks(cx);
        this
    }

    // ---- whether it is the right side ------------------------------------------------------

    /// Whether the home is what the right side shows: the Operator reads off.
    pub fn is_up(&self, cx: &App) -> bool {
        self.page.read(cx).is_off()
    }

    fn follow_page(&mut self, cx: &mut Context<Self>) {
        let up = self.is_up(cx);
        if up != self.up {
            self.up = up;
            cx.emit(HomeEvent::Moved);
        }
        cx.notify();
    }

    /// Brought to the front: the floor plan reads the library again where Features was in front and left, and the layout is read
    /// again, since another window may have kept one.
    pub fn follow(&mut self, cx: &mut Context<Self>) {
        self.floor.update(cx, |floor, cx| floor.follow(cx));
        self.reread_layout(cx);
    }

    // ---- which project ---------------------------------------------------------------------

    /// The project the System overview is of: the one the title bar is scoped to, else the one he chose last, else the first.
    /// Its id, name and the repositories that belong to it.
    fn project(&self, cx: &App) -> Option<(String, String, Vec<String>)> {
        let scoped = crate::titlebar::live(cx).and_then(|bar| bar.read(cx).face.scoped_project().map(str::to_string));
        match &self.source {
            Source::Mock => {
                let chips = crate::features::mock::projects();
                let listed = |id: &String| chips.iter().any(|(one, _)| one == id);
                let id = scoped.filter(listed).or_else(|| crate::features::chosen::get(cx).filter(listed)).or_else(|| chips.first().map(|(id, _)| id.clone()))?;
                let name = chips.into_iter().find(|(one, _)| *one == id).map(|(_, name)| name)?;
                Some((id, name, Vec::new()))
            }
            Source::Live(fleet) => {
                let held = fleet.read(cx).held();
                let projects = held.projects.as_ref()?;
                let listed = |id: &String| projects.projects.iter().any(|one| one.id == *id);
                let id = scoped.filter(listed).or_else(|| crate::features::chosen::get(cx).filter(listed)).or_else(|| projects.projects.first().map(|one| one.id.clone()))?;
                let project = projects.projects.iter().find(|one| one.id == id)?;
                let mut manifests: Vec<String> = Vec::new();
                for member in &project.members {
                    let id = member.manifest_id.as_str().to_string();
                    if !manifests.contains(&id) {
                        manifests.push(id);
                    }
                }
                Some((project.id.clone(), project.name.clone(), manifests))
            }
        }
    }

    /// The System overview is of the project on screen, and reads for it.
    fn follow_project(&mut self, cx: &mut Context<Self>) {
        let Some((id, name, manifests)) = self.project(cx) else { return };
        self.system.update(cx, |system, cx| system.follow(&id, &name, manifests, cx));
        cx.notify();
    }

    // ---- reading and keeping the layout ----------------------------------------------------

    fn port(&self, cx: &App) -> Option<u16> {
        match &self.source {
            Source::Live(fleet) => fleet.read(cx).held().port(),
            Source::Mock => None,
        }
    }

    /// Fleet's state moved: the layout is read the first time there is a port, and again when a setting changed.
    fn moved(&mut self, settings_changed: bool, cx: &mut Context<Self>) {
        let port = self.port(cx);
        if port.is_some() && (port != self.read_on || settings_changed) {
            self.read_on = port;
            self.read_layout(cx);
        }
        self.follow_project(cx);
    }

    fn read_layout(&mut self, cx: &mut Context<Self>) {
        let Some(port) = self.port(cx) else { return };
        let task = wire::read(port, cx);
        self.reading = Some(cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| match answer {
                Ok(home) => this.apply_kept(home.layout, cx),
                Err(why) => {
                    crate::log::Note::warn("the home layout was not read").field("why", why.said).emit();
                    this.apply_kept(None, cx);
                }
            });
        }));
    }

    /// Read what Fleet keeps again: a walk's memory, or Fleet.
    pub fn reread_layout(&mut self, cx: &mut Context<Self>) {
        match &self.source {
            Source::Mock => {
                let kept = self.kept.clone();
                self.apply_kept(kept, cx);
            }
            Source::Live(_) => self.read_layout(cx),
        }
    }

    /// Fleet said what it keeps, or that it keeps none. A change made here and not yet sent is left as it is.
    pub(crate) fn apply_kept(&mut self, kept: Option<BoardLayout>, cx: &mut Context<Self>) {
        self.layout_read = true;
        if self.unsaved {
            return;
        }
        let rev = self.stored.rev;
        self.stored = match kept {
            Some(kept) => {
                let (plain, scope) = layout::split(&kept);
                let (layout, unknown) = HomeLayout::from_wire(&plain);
                Stored { layout, edited: true, scope, unknown, rev }
            }
            None => Stored::preset(rev),
        };
        self.sync_blocks(cx);
        cx.notify();
    }

    /// Put what the layout says into the blocks that hold it: the Needs block's view and scope.
    fn sync_blocks(&mut self, cx: &mut Context<Self>) {
        let view = self.stored.layout.block(HomeBlockKind::NeedsCards).map_or(View::Cards, |placed| placed.view);
        let scope = self.stored.scope;
        self.needs.update(cx, |needs, cx| {
            needs.set_view(view, cx);
            needs.set_scope(scope, cx);
        });
        let view = self.stored.layout.block(HomeBlockKind::SystemOverview).map_or(View::Graph, |placed| placed.view);
        self.system.update(cx, |system, _| system.set_view(view));
    }

    /// The layout as it stands: what Fleet kept, else the preset.
    pub fn layout(&self) -> &HomeLayout {
        &self.stored.layout
    }

    /// The scope the Needs block holds.
    pub fn scope(&self) -> NeedsScope {
        self.stored.scope
    }

    /// Whether he has kept a layout, or changed this one: false while the preset is what is drawn.
    pub fn is_edited(&self) -> bool {
        self.stored.edited
    }

    /// Change the layout; he has edited it from here on, so the preset no longer applies, and Fleet is told once it has stood still.
    pub(crate) fn edit_layout(&mut self, change: impl FnOnce(&mut HomeLayout), cx: &mut Context<Self>) {
        let before = self.stored.layout.clone();
        change(&mut self.stored.layout);
        if self.stored.layout != before {
            self.changed(cx);
            self.sync_blocks(cx);
        }
    }

    fn scope_picked(&mut self, scope: NeedsScope, cx: &mut Context<Self>) {
        if scope != self.stored.scope {
            self.stored.scope = scope;
            self.changed(cx);
        }
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        self.stored.edited = true;
        self.stored.rev += 1;
        self.keep_soon(cx);
        cx.notify();
    }

    /// A change is waiting to be sent: it goes when no other has come for [`SAVE_AFTER`].
    fn keep_soon(&mut self, cx: &mut Context<Self>) {
        self.unsaved = true;
        self.saves = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_AFTER).await;
            let _ = this.update(cx, |this, cx| this.keep_now(cx));
        }));
    }

    /// Send the layout as it stands: it whole, or none where it is the preset, which has Fleet forget what it kept.
    fn keep_now(&mut self, cx: &mut Context<Self>) {
        let (layout, sent) = (self.stored.to_wire(), self.stored.rev);
        let Source::Live(_) = &self.source else {
            self.kept = layout;
            self.unsaved = false;
            return;
        };
        let Some(port) = self.port(cx) else { return };
        let task = wire::save(port, layout, cx);
        self.sending = Some(cx.spawn(async move |this, cx| {
            let answer = task.await;
            let _ = this.update(cx, |this, cx| match answer {
                // A change made while this one was on its way is still to be sent.
                Ok(_) => {
                    if this.stored.rev == sent {
                        this.unsaved = false;
                    }
                }
                Err(why) => crate::toasts::tell(why.said, cx),
            });
        }));
    }

    /// What a walk's or a test's Fleet keeps, where there is no Fleet.
    pub fn kept(&self) -> Option<&BoardLayout> {
        self.kept.as_ref()
    }

    // ---- editing ---------------------------------------------------------------------------

    pub fn toggle_edit(&mut self, cx: &mut Context<Self>) {
        self.edit = !self.edit;
        self.grab = None;
        cx.notify();
    }

    pub fn editing(&self) -> bool {
        self.edit
    }

    /// A kind that was off the grid goes on it, in the first free cells.
    pub fn add_block(&mut self, kind: HomeBlockKind, cx: &mut Context<Self>) {
        self.edit_layout(|layout| layout.add(kind), cx);
    }

    /// The block comes off the grid, the rest rise into its room, and the keys go back to the home.
    pub fn hide_block(&mut self, kind: HomeBlockKind, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_layout(|layout| layout.hide(kind), cx);
        window.focus(&self.keys, cx);
    }

    // ---- keys ------------------------------------------------------------------------------

    /// The block the keys are on.
    fn focused(&self, window: &Window) -> Option<HomeBlockKind> {
        self.handles.iter().find(|(_, handle)| handle.is_focused(window)).map(|(kind, _)| *kind)
    }

    /// The registry's `home` acts, pressed. The arrows move and resize only while the grid is edited, and only the block that has
    /// the keys; with none they go on to whatever else answers them.
    fn on_act(&mut self, act: &Act, window: &mut Window, cx: &mut Context<Self>) {
        if !act.id.starts_with("home_") {
            cx.propagate();
            return;
        }
        let (dx, dy) = chrome::arrow(act.slot);
        match (act.id, self.focused(window)) {
            ("home_edit", _) => self.toggle_edit(cx),
            ("home_move", Some(kind)) if self.edit => self.edit_layout(|layout| layout.nudge(kind, dx, dy), cx),
            ("home_resize", Some(kind)) if self.edit => self.edit_layout(|layout| layout.grow(kind, dx, dy), cx),
            ("home_block_view", Some(kind)) => {
                let views = kind.views();
                let now = self.stored.layout.block(kind).map(|one| one.view).unwrap_or(views[0]);
                let next = views[(views.iter().position(|view| *view == now).unwrap_or(0) + 1) % views.len()];
                self.edit_layout(|layout| layout.set_view(kind, next), cx);
            }
            _ => cx.propagate(),
        }
    }

    /// The menu's keys while it is open; Tab and Shift+Tab walk the tab stops otherwise.
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
        if crate::keymap::traverse(event, window, cx) {
            cx.stop_propagation();
        }
    }

    /// The pointer moved with a block lifted: the block follows to the cell under it, or takes the size that reaches it.
    fn dragged(&mut self, event: &DragMoveEvent<BlockDrag<HomeBlockKind>>, pitch: (Pixels, Pixels), cx: &mut Context<Self>) {
        let drag = event.drag(cx).clone();
        let at = event.event.position - event.bounds.origin;
        let Some(now) = self.stored.layout.block(drag.kind).copied() else { return };
        let step = chrome::drag_step(&now, drag.resize, at, pitch, &mut self.grab);
        self.edit_layout(|layout| step.apply(layout, drag.kind), cx);
    }

    // ---- the Add block menu ----------------------------------------------------------------

    pub fn open_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let choices = self.stored.layout.missing();
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
            window.focus(&self.keys, cx);
            cx.notify();
        }
    }

    /// The block goes on the grid in the first free cells, and the keys are on the home.
    fn pick_block(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(kind) = self.adding.as_ref().and_then(|menu| menu.choices.get(at).copied()) else { return };
        self.close_add(window, cx);
        self.add_block(kind, cx);
    }

    // ---- drawn: the strip ------------------------------------------------------------------

    /// The slim strip over the grid: that the Operator is off, why a Switch on was refused, Add block and Edit, and the page's own
    /// Switch on. One height across, as the buttons are all the one size.
    fn strip(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let page = self.page.read(cx);
        let state = labelled(
            chip("Operator off", look.color(token::FG_MUTED), look.color(token::BG_HOVER), look).id("home-operator-off").tooltip(tip("Nothing is handed to the Operator and it makes no calls")),
            Role::Status,
            "Operator off",
        );
        let refused = page.switch_refused().map(|why| {
            let said = format!("Not switched on: {why}");
            labelled(
                div().id("home-refused").min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(SharedString::from(said.clone())),
                Role::Alert,
                said,
            )
        });
        let switch = match page.switching() {
            Some(on) => div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL_LG))
                .text_color(look.color(token::FG_MUTED))
                .child(spinner("home-switch-spin", look.length(token::SPACE_4), look.color(token::OPERATOR), cx))
                .child(if on { "Switching on" } else { "Switching off" })
                .into_any_element(),
            None => {
                // The press goes as the page reads on: the keys go to the page, where Tab starts at the pills.
                let button = press("home-switch", "Switch on".into(), Some("power"), None, Weight::Pick, look).tooltip(tip("Queue what needs you here"));
                labelled(
                    pressed(button, cx, |this, window, cx| {
                        let page = this.page.clone();
                        page.update(cx, |page, cx| page.switch_on(cx));
                        let keys = page.read(cx).focus.clone();
                        window.focus(&keys, cx);
                    }),
                    Role::Button,
                    "Switch the Operator on",
                )
                .into_any_element()
            }
        };
        let editing = self.edit;
        let empty = self.stored.layout.blocks.is_empty();
        let free = self.stored.layout.missing().len();
        let edit = labelled(
            pressed(
                press("home-edit", if editing { "Done" } else { "Edit" }.into(), Some(if editing { "check" } else { "pencil" }), None, Weight::Plain, look)
                    .tooltip(tip(if editing { "Stop moving and resizing blocks" } else { "Move and resize the blocks of the home" })),
                cx,
                |this, _, cx| this.toggle_edit(cx),
            ),
            Role::Button,
            if editing { "Done" } else { "Edit" },
        );
        let add = (editing && !empty).then(|| {
            let button = press("home-add-block", "Add block".into(), Some("plus"), None, Weight::Plain, look).tooltip(tip("Put a block that is not on the home back on it"));
            let button = if free == 0 { button.opacity(0.5) } else { button };
            labelled(if free == 0 { button } else { pressed(button, cx, |this, window, cx| this.open_add(window, cx)) }, Role::Button, "Add block")
        });
        div()
            .id("home-strip")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3))
            .pt(look.length(token::SPACE_3))
            .child(state)
            .children(refused)
            .child(div().flex_1().min_w_0())
            .children(add)
            .child(edit)
            .child(switch)
            .into_any_element()
    }

    // ---- drawn: a block --------------------------------------------------------------------

    /// What a block shows in its box. The scope control of the Needs block is not here but in the header.
    fn body(&self, kind: HomeBlockKind) -> AnyElement {
        match kind {
            HomeBlockKind::NeedsCards => self.needs.clone().into_any_element(),
            HomeBlockKind::FloorPlan => self.floor.clone().into_any_element(),
            HomeBlockKind::SystemOverview => self.system.clone().into_any_element(),
            HomeBlockKind::SystemStats => self.stats.clone().into_any_element(),
        }
    }

    fn block(&self, placed: &HomePlaced, grid: Cells, look: &Look, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let kind = placed.kind;
        let (slug, title, hue) = (kind.slug(), kind.title(), kind.hue());
        let handle = self.handles.iter().find(|(one, _)| *one == kind).map(|(_, handle)| handle.clone());
        let views = kind.views();
        let switch = (views.len() > 1).then(|| {
            chrome::view_switch(slug, title, views, placed.view, look, |view| -> Pressed { Box::new(cx.listener(move |this, _, _, cx| this.edit_layout(|layout| layout.set_view(kind, view), cx))) })
        });
        let scope = (kind == HomeBlockKind::NeedsCards).then(|| self.needs.update(cx, |needs, cx| needs.scope_control(look, cx)));
        let editing = self.edit;
        let hide = editing.then(|| chrome::hide_button(slug, title, look, cx.listener(move |this, _, window, cx| this.hide_block(kind, window, cx))));
        let header = chrome::head(slug, hue, look)
            .when(editing, |head| chrome::lift(head, kind, cx.listener(|this, _, _, _| this.grab = None)))
            .child(chrome::mark(kind.mark(), hue, look))
            .child(chrome::title_text(title, look))
            .child(div().flex_1())
            .children(scope)
            .children(switch)
            .children(hide);
        let header = if editing { labelled(header, Role::Group, format!("Move {}", title)).into_any_element() } else { header.into_any_element() };
        let body = div().flex_1().min_h_0().relative().child(div().absolute().size_full().child(self.body(kind)));
        // The System overview grows over the home, and its frame is then set by the home's size and not by its cells.
        let overlay = (kind == HomeBlockKind::SystemOverview)
            .then(|| {
                let home = PoseRect { x: f32::from(grid.left(placed.x)), y: f32::from(grid.top(placed.y)), w: f32::from(grid.wide(placed.w)), h: f32::from(grid.tall(placed.h)) };
                let seen = window.viewport_size();
                self.system.read(cx).overlay(home, (f32::from(seen.width), f32::from(seen.height)))
            })
            .flatten();
        let over = overlay.is_some();
        let corner = editing.then(|| chrome::corner(slug, title, kind, look, cx.listener(|this, _, _, _| this.grab = None)));
        let edge = if editing { look.color(token::BORDER_STRONG) } else { look.color(token::BORDER_DEFAULT) };
        let frame = chrome::frame(slug, look, edge)
            .when(!over, |frame| grid.put(frame, placed))
            .when(over, |frame| frame.size_full())
            .when_some(handle, |frame, handle| frame.track_focus(&handle))
            .child(header)
            .child(body)
            .children(corner);
        let framed = labelled(frame, Role::Region, title).into_any_element();
        if kind == HomeBlockKind::SystemOverview {
            posed(framed, overlay, look)
        } else {
            framed
        }
    }

    /// Nothing is on the grid: the words that say so, and the way to put a block back.
    fn empty(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let free = self.stored.layout.missing().len();
        let button = press("home-add-block", "Add block".into(), Some("plus"), None, Weight::Plain, look).tooltip(tip("Put a block on the home"));
        let button = if free == 0 { button.opacity(0.5) } else { button };
        div()
            .id("home-empty")
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(look.length(token::SPACE_3))
            .child(labelled(
                div()
                    .id("home-empty-words")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .child(glyph("layout-dashboard", look.length(token::SPACE_4), look.color(token::FG_SUBTLE)))
                    .child("Nothing here yet"),
                Role::Status,
                "Nothing here yet",
            ))
            .child(labelled(if free == 0 { button } else { pressed(button, cx, |this, window, cx| this.open_add(window, cx)) }, Role::Button, "Add block"))
            .into_any_element()
    }

    /// The menu of kinds not on the grid, hung under the Add block button.
    fn add_menu(&self, look: &Look, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.adding.as_ref()?;
        let ring = ring_colour(look);
        let from = bounds_of(window, cx, Role::Button, "Add block").map(|bounds| bounds.bottom_left()).unwrap_or_default();
        let rows = menu.choices.iter().enumerate().map(|(at, kind)| {
            let (kind, on) = (*kind, at == menu.at);
            labelled(
                div()
                    .id(SharedString::from(format!("home-add-{}", kind.slug())))
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
                    .child(glyph(kind.mark(), look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                    .child(div().min_w_0().truncate().child(kind.title())),
                Role::MenuItem,
                kind.title(),
            )
        });
        Some(
            deferred(
                gpui::anchored().position(from).anchor(gpui::Anchor::TopLeft).child(
                    labelled(
                        card(look).id("home-add-menu").track_focus(&menu.focus).mt(look.length(token::SPACE_1)).w(look.length(token::SPACE_12) * 4.0).p(look.length(token::SPACE_1)).children(rows),
                        Role::Menu,
                        "Blocks to add",
                    ),
                ),
            )
            .with_priority(3)
            .into_any_element(),
        )
    }
}

/// Records the room it is given and asks the home to draw again where that changed, so the cells are cut from the room the grid
/// has and not from the window.
fn measure(seen: Rc<Cell<Option<Bounds<Pixels>>>>, home: WeakEntity<HomeScreen>) -> AnyElement {
    canvas(
        move |bounds, _, cx| {
            let before = seen.get().map(|held| held.size);
            seen.set(Some(bounds));
            if before != Some(bounds.size) {
                let home = home.clone();
                cx.defer(move |cx| {
                    home.update(cx, |_, cx| cx.notify()).ok();
                });
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
    .into_any_element()
}

impl Render for HomeScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let gap = look.length(token::SPACE_3);
        let width = self.seen.get().map_or(window.viewport_size().width, |bounds| bounds.size.width) - gap * 2.0;
        let grid = Cells::across(width, gap, look.length(token::H_ROW_UNIT) * 3.5);
        let layout = self.stored.layout.clone();
        let height = grid.height(layout.rows());
        let pitch = grid.pitch();
        // The System overview, grown, is drawn over the home and not in its cells.
        let grown = self.system.read(cx).over_board();
        let in_grid = |placed: &&HomePlaced| !(grown && placed.kind == HomeBlockKind::SystemOverview);
        let over: Option<AnyElement> = layout.block(HomeBlockKind::SystemOverview).filter(|_| self.layout_read && grown).map(|placed| self.block(placed, grid, &look, window, cx));
        let blocks: Vec<AnyElement> = if self.layout_read {
            layout.blocks.iter().filter(in_grid).map(|placed| self.block(placed, grid, &look, window, cx)).collect()
        } else {
            layout.blocks.iter().map(|placed| chrome::skeleton(placed, grid, &look)).collect()
        };
        let area: AnyElement = if layout.blocks.is_empty() && self.layout_read {
            self.empty(&look, cx)
        } else {
            let board = div()
                .id("home-grid")
                .relative()
                .w_full()
                .h(height)
                .on_drag_move(cx.listener(move |this, event: &DragMoveEvent<BlockDrag<HomeBlockKind>>, _, cx| this.dragged(event, pitch, cx)))
                .child(measure(self.seen.clone(), cx.entity().downgrade()))
                .child(self.system.read(cx).measure_grid())
                .children(blocks);
            labelled(div().id("home-scroll").size_full().overflow_y_scroll().px(gap).py(gap).child(board), Role::Group, "Home").into_any_element()
        };
        let strip = self.strip(&look, cx);
        div()
            .id("cockpit-home")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .key_context(ActionScope::CockpitHome.key_context().unwrap_or_default())
            .track_focus(&self.keys)
            .on_action(cx.listener(Self::on_act))
            .on_key_down(cx.listener(Self::on_key))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(strip)
            .child(div().flex_1().min_h_0().relative().child(self.system.read(cx).measure_root()).child(area).children(over))
            .children(self.add_menu(&look, window, cx))
    }
}

#[cfg(test)]
#[path = "screen_tests.rs"]
mod tests;
