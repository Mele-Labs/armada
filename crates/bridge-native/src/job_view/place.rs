//! Each Job's panes as an [`Arrangement`], and every act on them. The tree in `pane_tree.rs` says
//! what is where; this keeps one per Job, writes it to the installed [`pane_store`] on every
//! change, and keeps a [`History`] per Job for ⌘Z. What was a fixed three (the thread, Open, Done
//! and Deferred over Now) is the tree `default_arrangement` builds, and what drew a fixed pane draws
//! whichever pane a tab is in (`place_draw.rs`).
//!
//! **Contents and places are kept apart.** A view tab's body (`view_tabs.rs`) and a row's tab
//! (`row_tabs.rs`) are held by what opened them; the tree holds the tab's place, its order and
//! whether it is in front. The tree takes a tab when one opens and gives it up when it closes.
//!
//! **A pane keeps the name its strip had.** A pane the default arrangement gave a name (the thread's,
//! the column's top and its foot) reads as it did; a pane he splits off reads as Classic's does.

use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Instant;

use gpui::{Context, FocusHandle, Focusable, ScrollHandle, SharedString, Window};

use super::pane_store;
use super::pane_tree::{self as tree, Arrangement, Extras, History, Node, Out, Pane, Size, SplitInto, Tab, Toward};
use super::row_tabs::RowKind;
use super::side::Zone;
use super::view_tabs::ViewPane;
use super::JobView;

/// One Job's panes and the steps back from them.
pub(super) struct Placed {
    pub arrangement: Arrangement,
    history: History,
    /// A change has been made here, so what the store held when this was read no longer replaces it.
    changed: bool,
}

impl Placed {
    fn new(arrangement: Arrangement) -> Self {
        Placed { arrangement, history: History::default(), changed: false }
    }
}

/// Where the Jobs' panes stand between draws.
pub(super) struct Place {
    jobs: HashMap<SharedString, Placed>,
    /// The pane covering the Job's area, if one is: where he is looking, not how the Job is laid out, so it is not kept.
    pub zoom: Option<String>,
    /// The Job's area as last drawn, which a change is held against (`allowed`).
    pub area: Rc<Cell<Option<Size>>>,
    /// The split whose divider is being dragged.
    pub dragging: Option<String>,
    /// Focus handles by `{job}/{tab}`, `{job}/add:{pane}`.
    handles: HashMap<String, FocusHandle>,
    /// Each strip's scroll, by `{job}/{pane}`, and the tab that was in front when it was last drawn.
    pub scrolls: HashMap<String, (ScrollHandle, Option<String>)>,
    /// The pane a + was pressed in, for the tab its source opens next on that Job, while that is still recent.
    aimed: Option<(String, String, Instant)>,
    started: Instant,
}

impl Default for Place {
    fn default() -> Self {
        Place { jobs: HashMap::new(), zoom: None, area: Rc::default(), dragging: None, handles: HashMap::new(), scrolls: HashMap::new(), aimed: None, started: Instant::now() }
    }
}

/// Which of the default arrangement's panes this is, by the name its strip and region have always had.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Slot {
    Thread,
    Top,
    Now,
    Other,
}

pub(super) fn slot_of(pane: &str) -> Slot {
    static IDS: OnceLock<[String; 3]> = OnceLock::new();
    let ids = IDS.get_or_init(|| {
        let default = tree::default_arrangement();
        let panes = tree::panes_of(&default.root);
        [panes[0].id.clone(), panes[1].id.clone(), panes[2].id.clone()]
    });
    match ids.iter().position(|id| id == pane) {
        Some(0) => Slot::Thread,
        Some(1) => Slot::Top,
        Some(2) => Slot::Now,
        _ => Slot::Other,
    }
}

/// The zone a kind of tab draws, for the kinds the right column's top pane holds.
pub(super) fn zone_of(kind: &str) -> Option<Zone> {
    Zone::ALL.into_iter().find(|zone| zone.id() == kind)
}

/// The tab's terminal as the tree names it: the view tab's id without its `terminal:` head.
pub(super) fn terminal_key(view_id: &str) -> &str {
    view_id.strip_prefix("terminal:").unwrap_or(view_id)
}

/// The kind a row's tab has in the tree.
pub(super) fn row_kind_name(kind: RowKind) -> &'static str {
    match kind {
        RowKind::Draft => "draft",
        RowKind::Document => "document",
        RowKind::Walk => "walk",
        RowKind::Subagent => "subagent",
    }
}

pub(super) fn row_kind_of(kind: &str) -> Option<RowKind> {
    match kind {
        "draft" => Some(RowKind::Draft),
        "document" => Some(RowKind::Document),
        "walk" => Some(RowKind::Walk),
        "subagent" => Some(RowKind::Subagent),
        _ => None,
    }
}

/// The node beside a pane: what shares its split.
fn sibling_of<'a>(node: &'a Node, pane: &str) -> Option<&'a Node> {
    let Node::Split(split) = node else { return None };
    if split.a.id() == pane {
        return Some(&split.b);
    }
    if split.b.id() == pane {
        return Some(&split.a);
    }
    sibling_of(&split.a, pane).or_else(|| sibling_of(&split.b, pane))
}

/// The pane a view tab asked for stands in: the one that holds the thread, or the first or second pane of the
/// column beside it. None where there is none, and the tab then opens as a split.
pub(super) fn pane_for(arrangement: &Arrangement, place: ViewPane) -> Option<String> {
    let thread = tree::holder_of(arrangement, "thread");
    if place == ViewPane::Thread {
        return thread.map(|pane| pane.id.clone());
    }
    let beside = thread.and_then(|pane| sibling_of(&arrangement.root, &pane.id));
    let panes = tree::panes_of(beside.unwrap_or(&arrangement.root));
    let at = usize::from(place == ViewPane::Bottom);
    panes.get(at).map(|pane| pane.id.clone())
}

/// Which place a pane is, as a view tab names its places.
pub(super) fn view_pane_of(arrangement: &Arrangement, pane: &str) -> ViewPane {
    if tree::holder_of(arrangement, "thread").is_some_and(|holder| holder.id == pane) {
        ViewPane::Thread
    } else if pane_for(arrangement, ViewPane::Bottom).is_some_and(|second| second == pane) {
        ViewPane::Bottom
    } else {
        ViewPane::Top
    }
}

/// A shell's tab put in `target`, or in a split of its own where there is none. In front where `front`, else behind
/// the tab that was, with the focus where it was.
pub(super) fn put_terminal(was: &Arrangement, terminal: &str, target: Option<&str>, front: bool) -> Arrangement {
    let held = |arrangement: &Arrangement| tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == "terminal" && tab.terminal.as_deref() == Some(terminal)).map(|tab| tab.id.clone());
    let existing = held(was);
    let mut next = match (&existing, target) {
        (Some(tab), Some(pane)) if tree::pane_of_tab(was, tab).is_some_and(|holder| holder.id != pane) => tree::move_tab_to_pane(was, tab, pane, None).into_owned(),
        (Some(_), _) => was.clone(),
        (None, Some(pane)) => tree::add_tab(was, pane, "terminal", Extras::terminal(Some(terminal))).into_owned(),
        (None, None) => tree::arrive(was, "terminal", Extras::terminal(Some(terminal))),
    };
    let Some(tab) = held(&next) else { return next };
    if front {
        return tree::activate(&next, &tab).into_owned();
    }
    // Behind: the pane it is in keeps the front tab it had.
    let holder = tree::pane_of_tab(&next, &tab).map(|pane| pane.id.clone());
    let was_front = holder.as_deref().and_then(|pane| tree::pane_by_id(was, pane)).and_then(|pane| pane.active.clone());
    if let (Some(pane), Some(active)) = (holder, was_front) {
        next = tree::activate(&next, &active).into_owned();
        next.focus = was.focus.clone();
        if tree::pane_by_id(&next, &next.focus).is_none() {
            next.focus = pane;
        }
    }
    next
}

impl JobView {
    // ---- the arrangement ---------------------------------------------------------------------

    /// The shown Job's arrangement; the default one before the Job has been placed.
    pub(super) fn arrangement(&self) -> Cow<'_, Arrangement> {
        match self.shown.as_ref().and_then(|id| self.place.jobs.get(id)) {
            Some(placed) => Cow::Borrowed(&placed.arrangement),
            None => Cow::Owned(tree::default_arrangement()),
        }
    }

    pub(super) fn arrangement_of(&self, job: &str) -> Option<&Arrangement> {
        self.place.jobs.get(job).map(|placed| &placed.arrangement)
    }

    /// The Job has an arrangement: the default until the store's answers. A change made meanwhile is kept.
    pub(super) fn ensure_placed(&mut self, job: &SharedString, cx: &mut Context<Self>) {
        if self.place.jobs.contains_key(job) {
            return;
        }
        self.place.jobs.insert(job.clone(), Placed::new(tree::default_arrangement()));
        let read = pane_store::read(&pane_store::job_key(job), cx);
        let job = job.clone();
        cx.spawn(async move |this, cx| {
            if let Some(kept) = read.await {
                let _ = this.update(cx, |this, cx| {
                    if let Some(placed) = this.place.jobs.get_mut(&job).filter(|placed| !placed.changed) {
                        placed.arrangement = kept;
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    fn now_ms(&self) -> u64 {
        self.place.started.elapsed().as_millis() as u64
    }

    /// A change he made, or the Job's own that is his to take back: held against the panes' least sizes unless
    /// `force`, kept for ⌘Z (a run of one `merge` is one step), and written to the store. True where it changed.
    pub(super) fn rearrange_in(&mut self, job: &SharedString, merge: Option<&str>, force: bool, change: impl for<'a> FnOnce(&'a Arrangement) -> Out<'a>, cx: &mut Context<Self>) -> bool {
        self.ensure_placed(job, cx);
        let size = if self.shown.as_ref() == Some(job) { self.place.area.get() } else { None };
        let now = self.now_ms();
        let Some(placed) = self.place.jobs.get_mut(job) else { return false };
        let Some(next) = tree::rearrange(&placed.arrangement, change, size, force) else { return false };
        placed.history.remember(&placed.arrangement, merge, now);
        placed.arrangement = next;
        placed.changed = true;
        pane_store::write(&pane_store::job_key(job), &placed.arrangement, cx);
        cx.notify();
        true
    }

    pub(super) fn rearrange(&mut self, merge: Option<&str>, force: bool, change: impl for<'a> FnOnce(&'a Arrangement) -> Out<'a>, cx: &mut Context<Self>) -> bool {
        let Some(job) = self.shown.clone() else { return false };
        self.rearrange_in(&job, merge, force, change, cx)
    }

    /// Which tab is in front and which pane has the focus: where he is, not how the area is laid out. No step back.
    pub(super) fn look_in(&mut self, job: &SharedString, change: impl for<'a> FnOnce(&'a Arrangement) -> Out<'a>, cx: &mut Context<Self>) -> bool {
        self.ensure_placed(job, cx);
        let Some(placed) = self.place.jobs.get_mut(job) else { return false };
        let Cow::Owned(next) = change(&placed.arrangement) else { return false };
        placed.arrangement = next;
        placed.changed = true;
        pane_store::write(&pane_store::job_key(job), &placed.arrangement, cx);
        cx.notify();
        true
    }

    pub(super) fn look(&mut self, change: impl for<'a> FnOnce(&'a Arrangement) -> Out<'a>, cx: &mut Context<Self>) -> bool {
        let Some(job) = self.shown.clone() else { return false };
        self.look_in(&job, change, cx)
    }

    /// ⌘Z, or ⇧⌘Z with `redo`: the shown Job's panes as they were before the last change, or as that change left them.
    pub(super) fn step_panes(&mut self, redo: bool, cx: &mut Context<Self>) -> bool {
        let Some(job) = self.shown.clone() else { return false };
        let Some(placed) = self.place.jobs.get_mut(&job) else { return false };
        let target = if redo { placed.history.redo(&placed.arrangement) } else { placed.history.undo(&placed.arrangement) };
        let Some(target) = target else { return false };
        placed.arrangement = target;
        placed.changed = true;
        pane_store::write(&pane_store::job_key(&job), &placed.arrangement, cx);
        cx.notify();
        true
    }

    // ---- what is in the panes ----------------------------------------------------------------

    /// Whether no pane holds a tab of this kind, or Settings › Layout has it off.
    pub(super) fn tab_closed(&self, kind: &str) -> bool {
        super::tabs::laid_off(kind) || !tree::has_kind(&self.arrangement(), kind)
    }

    /// The front tab of every pane that has a zone in front, the focused pane's first.
    pub(super) fn zones_shown(&self) -> Vec<Zone> {
        let arrangement = self.arrangement();
        let mut panes = tree::panes_of(&arrangement.root);
        panes.sort_by_key(|pane| pane.id != arrangement.focus);
        panes.into_iter().filter_map(|pane| front_of(pane)).filter_map(|tab| zone_of(&tab.kind)).collect()
    }

    /// The zone in front in the pane that has the focus, else in the first pane that has one.
    pub(super) fn zone_shown(&self) -> Option<Zone> {
        self.zones_shown().into_iter().next()
    }

    /// The zone's tab in front, in the pane that holds it.
    pub(super) fn show_zone(&mut self, zone: Zone, cx: &mut Context<Self>) {
        let kind = zone.id();
        let found = tree::holder_of(&self.arrangement(), kind).and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == kind).map(|tab| tab.id.clone()));
        if let Some(tab) = found {
            self.look(|arrangement| tree::activate(arrangement, &tab), cx);
        }
    }

    /// The shown Job's tab, by id.
    pub(super) fn tab_of(&self, tab: &str) -> Option<Tab> {
        tree::tab_by_id(&self.arrangement(), tab).cloned()
    }

    /// A kind of tab comes to the front where one is open, else opens where the Job keeps such tabs: Now at the
    /// column's foot, the thread where it stood, anything else in the column's top. Where there is no such pane it opens as a split.
    pub(super) fn open_kind(&mut self, kind: &str, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        self.ensure_placed(&job, cx);
        let arrangement = self.arrangement().into_owned();
        if kind == "thread" {
            self.rearrange_in(&job, None, true, tree::reopen_thread, cx);
            return;
        }
        if let Some(tab) = tree::holder_of(&arrangement, kind).and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == kind)) {
            let id = tab.id.clone();
            self.look_in(&job, |arrangement| tree::activate(arrangement, &id), cx);
            return;
        }
        let place = if kind == "now" { ViewPane::Bottom } else { ViewPane::Top };
        let kind = kind.to_string();
        match pane_for(&arrangement, place) {
            Some(pane) => self.rearrange_in(&job, None, true, move |arrangement| tree::add_tab(arrangement, &pane, &kind, Extras::default()), cx),
            None => self.rearrange_in(&job, None, true, move |arrangement| Cow::Owned(tree::arrive(arrangement, &kind, Extras::default())), cx),
        };
    }

    /// The tab at the front of its pane, and the pane takes the focus.
    pub(super) fn activate_tab(&mut self, tab: &str, window: &mut Window, cx: &mut Context<Self>) {
        let tab = tab.to_string();
        self.look(|arrangement| tree::activate(arrangement, &tab), cx);
        self.want_pane_reads(window, cx);
    }

    /// The thread's tab comes to the front of its pane.
    pub(super) fn front_thread(&mut self) {
        let Some(job) = self.shown.clone() else { return };
        let arrangement = self.arrangement();
        let Some(tab) = tree::holder_of(&arrangement, "thread").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "thread")).map(|tab| tab.id.clone()) else { return };
        drop(arrangement);
        if let Some(placed) = self.place.jobs.get_mut(&job) {
            if let Cow::Owned(next) = tree::activate(&placed.arrangement, &tab) {
                placed.arrangement = next;
                placed.changed = true;
            }
        }
    }

    /// A closed thread comes back, as a walk arriving brings it (`reopenThread`).
    pub(super) fn reopen_thread(&mut self, job: &SharedString, cx: &mut Context<Self>) {
        self.rearrange_in(job, None, true, tree::reopen_thread, cx);
    }

    /// The tab a source opens next on this Job goes to this pane, as the + pressed there asked.
    pub(super) fn aim_at(&mut self, job: &str, pane: &str) {
        self.place.aimed = Some((job.to_string(), pane.to_string(), Instant::now()));
    }

    /// The pane the last + aimed at, once: where it is still held and was pressed not long ago.
    pub(super) fn take_aim(&mut self, job: &str) -> Option<String> {
        let (aimed_job, pane, at) = self.place.aimed.take()?;
        let live = aimed_job == job && at.elapsed().as_secs() < 10;
        live.then_some(pane).filter(|pane| self.arrangement_of(job).is_some_and(|arrangement| tree::pane_by_id(arrangement, pane).is_some()))
    }

    // ---- pane acts ---------------------------------------------------------------------------

    /// Whether the thread is what its pane shows, and that pane is drawn.
    pub(super) fn thread_open(&self) -> bool {
        let arrangement = self.arrangement();
        let Some(pane) = tree::holder_of(&arrangement, "thread") else { return false };
        front_of(pane).is_some_and(|tab| tab.kind == "thread") && self.zoomed().is_none_or(|zoomed| zoomed == pane.id)
    }

    /// The pane the keys are in: the thread's pane while the message box or the thread's tab has them, else the focused one.
    pub(super) fn here(&self, window: &Window, cx: &gpui::App) -> String {
        let arrangement = self.arrangement();
        let in_thread = self.focus.contains_focused(window, cx) || self.head_tab_focused(window, cx) || self.composer.input.read(cx).focus_handle(cx).contains_focused(window, cx);
        if in_thread {
            if let Some(pane) = tree::holder_of(&arrangement, "thread") {
                return pane.id.clone();
            }
        }
        arrangement.focus.clone()
    }

    /// The pane splits, and the new, empty one takes the focus and the keys.
    pub(super) fn split(&mut self, pane: &str, into: SplitInto, window: &mut Window, cx: &mut Context<Self>) {
        let pane = pane.to_string();
        if self.rearrange(None, false, |arrangement| tree::split_pane(arrangement, &pane, into), cx) {
            let focus = self.arrangement().focus.clone();
            self.focus_in_pane(&focus, window, cx);
        }
    }

    /// Puts the keys in a pane: on its tab in front, or on its + where it has none.
    pub(super) fn focus_in_pane(&mut self, pane: &str, window: &mut Window, cx: &mut Context<Self>) {
        let arrangement = self.arrangement().into_owned();
        let Some(job) = self.shown.clone() else { return };
        let front = tree::pane_by_id(&arrangement, pane).and_then(front_of).map(|tab| (tab.id.clone(), tab.kind.clone()));
        let handle = match front {
            Some((tab, kind)) => self.tab_handle(&job, &tab, &kind, cx),
            None => self.add_handle(&job, pane, cx),
        };
        window.defer(cx, move |window, cx| window.focus(&handle, cx));
    }

    /// The focus handle of a tab on a Job's strip. The thread's tab keeps the one the head has always had.
    pub(super) fn tab_handle(&mut self, job: &SharedString, tab: &str, kind: &str, cx: &mut Context<Self>) -> FocusHandle {
        if kind == "thread" {
            return self.head.tab.clone();
        }
        self.place.handles.entry(format!("{job}/{tab}")).or_insert_with(|| cx.focus_handle()).clone()
    }

    pub(super) fn add_handle(&mut self, job: &SharedString, pane: &str, cx: &mut Context<Self>) -> FocusHandle {
        self.place.handles.entry(format!("{job}/add:{pane}")).or_insert_with(|| cx.focus_handle()).clone()
    }

    /// Another pane has the focus: the nearest one that shares some of this one's edge (`neighbourOf`).
    pub(super) fn move_to_pane(&mut self, toward: Toward, window: &mut Window, cx: &mut Context<Self>) {
        let here = self.here(window, cx);
        let Some(to) = tree::neighbour_of(&self.arrangement(), &here, toward) else { return };
        let target = to.clone();
        self.look(|arrangement| tree::focus_pane(arrangement, &target), cx);
        self.focus_in_pane(&to, window, cx);
    }

    /// The next or the previous tab of the pane comes to the front, and has the keys.
    pub(super) fn cycle(&mut self, by: isize, window: &mut Window, cx: &mut Context<Self>) {
        let here = self.here(window, cx);
        let pane = here.clone();
        if self.look(|arrangement| tree::cycle_tab(arrangement, &pane, by), cx) {
            self.want_pane_reads(window, cx);
            let focus = self.arrangement().focus.clone();
            self.focus_in_pane(&focus, window, cx);
        }
    }

    /// Whether a sheet or a dialog stands over the Job: no pane key is heard then.
    pub(super) fn sheet_over(&self) -> bool {
        self.confirm.is_some()
            || self.head_confirming()
            || self.bar.editor.is_some()
            || self.reporting.is_some()
            || self.sketch_open
            || self.teach.is_some()
            || self.review_open()
            || self.sketch_reading.is_some()
            || self.subagent_reading.is_some()
            || self.pictures.viewing.is_some()
            || self.pane_panel_open()
    }

    /// A key of the registry's that arranges the panes. True where it was taken.
    pub(crate) fn pane_act(&mut self, act: &crate::keymap::Act, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.shown.is_none() || self.sheet_over() {
            return false;
        }
        let in_text = window.context_stack().iter().any(|context| context.contains(crate::keymap::TEXT_BOX));
        match act.id {
            "pane_split_right" | "pane_split_down" => {
                let here = self.here(window, cx);
                self.split(&here, if act.is("pane_split_right") { SplitInto::Right } else { SplitInto::Down }, window, cx);
            }
            // A text box keeps ⌘Z for what is typed in it.
            "pane_undo" | "pane_redo" if !in_text => {
                self.step_panes(act.is("pane_redo"), cx);
            }
            "next_tab" => self.cycle(1, window, cx),
            "previous_tab" => self.cycle(-1, window, cx),
            "focus_pane" => {
                let toward = [Toward::Left, Toward::Up, Toward::Right, Toward::Down][usize::from(act.slot).min(3)];
                self.move_to_pane(toward, window, cx);
            }
            _ => return false,
        }
        true
    }

    /// The pane is where the next press acts.
    pub(super) fn pane_pressed(&mut self, pane: &str, cx: &mut Context<Self>) {
        if self.arrangement().focus != pane {
            let pane = pane.to_string();
            self.look(|arrangement| tree::focus_pane(arrangement, &pane), cx);
        }
    }

    // ---- closing -----------------------------------------------------------------------------

    /// A tab's ×: what the tab holds is let go of, and the tab leaves its pane.
    pub(super) fn close_tab_id(&mut self, tab: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(held) = self.tab_of(tab) else { return };
        match held.kind.as_str() {
            "terminal" => {
                if let Some(view) = self.view_id_of(&job, &held) {
                    self.press_close_view_tab(&job, &view, window, cx);
                    return;
                }
            }
            kind if row_kind_of(kind).is_some() => {
                if let (Some(kind), Some(key)) = (row_kind_of(kind), held.row.as_deref()) {
                    self.row_tab_let_go(&job, kind, key);
                }
            }
            "judge" => self.close_judge_tab(),
            "system" => self.panes.systems.stop(),
            _ => {}
        }
        let id = tab.to_string();
        self.rearrange_in(&job, None, true, move |arrangement| tree::close_tab(arrangement, &id), cx);
        self.want_pane_reads(window, cx);
    }

    /// Close pane: the pane goes with what it holds, and the panes beside it take its room.
    pub(super) fn close_pane_id(&mut self, pane: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.place.zoom.as_deref() == Some(pane) {
            self.place.zoom = None;
        }
        let pane = pane.to_string();
        self.rearrange(None, true, move |arrangement| tree::close_pane(arrangement, &pane), cx);
        self.want_pane_reads(window, cx);
    }

    // ---- zoom --------------------------------------------------------------------------------

    pub(super) fn toggle_zoom(&mut self, pane: &str, cx: &mut Context<Self>) {
        self.place.zoom = if self.place.zoom.as_deref() == Some(pane) { None } else { Some(pane.to_string()) };
        cx.notify();
    }

    /// Another Job on screen ends a zoom.
    pub(super) fn end_zoom(&mut self) {
        self.place.zoom = None;
    }

    /// The pane that is zoomed, where it is still one.
    pub(super) fn zoomed(&self) -> Option<String> {
        let zoom = self.place.zoom.as_ref()?;
        tree::pane_by_id(&self.arrangement(), zoom).map(|pane| pane.id.clone())
    }
}

/// The tab in front of a pane: the one it names, or its first.
pub(super) fn front_of(pane: &Pane) -> Option<&Tab> {
    visible(pane).find(|tab| pane.active.as_deref() == Some(tab.id.as_str())).or_else(|| visible(pane).next())
}

/// A pane's tabs that Settings › Layout has not turned off.
pub(super) fn visible(pane: &Pane) -> impl Iterator<Item = &Tab> {
    pane.tabs.iter().filter(|tab| !super::tabs::laid_off(&tab.kind))
}
