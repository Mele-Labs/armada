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

/// The split a pane is a side of, and whether it is the first side: the one its Grow and Shrink move.
pub(super) fn parent_split_of(node: &Node, pane: &str) -> Option<(String, bool)> {
    let Node::Split(split) = node else { return None };
    if split.a.id() == pane {
        return Some((split.id.clone(), true));
    }
    if split.b.id() == pane {
        return Some((split.id.clone(), false));
    }
    parent_split_of(&split.a, pane).or_else(|| parent_split_of(&split.b, pane))
}

/// The pane after this one in the tree's order, round to the first from the last. None where there is no other.
pub(super) fn next_pane_of(arrangement: &Arrangement, pane: &str) -> Option<String> {
    let panes = tree::panes_of(&arrangement.root);
    let at = panes.iter().position(|one| one.id == pane)?;
    (panes.len() > 1).then(|| panes[(at + 1) % panes.len()].id.clone())
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
    /// The tab the keys are on, which is the front one of their pane, goes to the pane after it and comes to the front there.
    pub(super) fn move_tab_next_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let here = self.here(window, cx);
        let arrangement = self.arrangement().into_owned();
        let (Some(to), Some(tab)) = (next_pane_of(&arrangement, &here), tree::pane_by_id(&arrangement, &here).and_then(front_of).map(|tab| tab.id.clone())) else { return };
        if self.rearrange(None, false, move |arrangement| tree::move_tab_to_pane(arrangement, &tab, &to, None), cx) {
            self.want_pane_reads(window, cx);
            let focus = self.arrangement().focus.clone();
            self.focus_in_pane(&focus, window, cx);
        }
    }

    /// Grow pane or Shrink pane: the divider of the split the pane is a side of moves one step, toward its neighbour to grow.
    pub(super) fn resize_pane(&mut self, grow: bool, window: &mut Window, cx: &mut Context<Self>) {
        let here = self.here(window, cx);
        let Some((split, first)) = parent_split_of(&self.arrangement().root, &here) else { return };
        let toward = if grow == first { 1. } else { -1. };
        self.nudge_divider(&split, toward * super::place_draw::KEY_STEP, cx);
    }

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
                let was = self.here(window, cx);
                if self.step_panes(act.is("pane_redo"), cx) {
                    // The pane the keys were in may be gone: they go to the one the arrangement says has the focus.
                    let arrangement = self.arrangement().into_owned();
                    if tree::pane_by_id(&arrangement, &was).is_none() {
                        self.focus_in_pane(&arrangement.focus, window, cx);
                    }
                }
            }
            "move_tab_next_pane" => self.move_tab_next_pane(window, cx),
            "pane_grow" | "pane_shrink" => self.resize_pane(act.is("pane_grow"), window, cx),
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
        // The keys were on the tab that goes: they go to the pane it was in, on the tab that comes forward.
        let pane = tree::pane_of_tab(&self.arrangement(), tab).map(|pane| pane.id.clone());
        let held_keys = self.tab_handle(&job, tab, &held.kind, cx).contains_focused(window, cx);
        let id = tab.to_string();
        self.rearrange_in(&job, None, true, move |arrangement| tree::close_tab(arrangement, &id), cx);
        if held_keys {
            let arrangement = self.arrangement().into_owned();
            let to = pane.filter(|pane| tree::pane_by_id(&arrangement, pane).is_some()).unwrap_or(arrangement.focus);
            self.focus_in_pane(&to, window, cx);
        }
        self.want_pane_reads(window, cx);
    }

    /// Close pane: the pane goes with what it holds, and the panes beside it take its room.
    pub(super) fn close_pane_id(&mut self, pane: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.place.zoom.as_deref() == Some(pane) {
            self.place.zoom = None;
        }
        let (gone, here) = (pane.to_string(), self.here(window, cx));
        self.rearrange(None, true, move |arrangement| tree::close_pane(arrangement, &gone), cx);
        // The keys were in the pane that goes: they go to the pane the arrangement now says has the focus.
        if here == pane {
            let focus = self.arrangement().focus.clone();
            self.focus_in_pane(&focus, window, cx);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::tree::Dir;

    fn kinds(pane: &Pane) -> Vec<&str> {
        pane.tabs.iter().map(|tab| tab.kind.as_str()).collect()
    }

    fn column(arrangement: &Arrangement) -> (&tree::Split, &Pane, &Pane) {
        let Node::Split(root) = &arrangement.root else { panic!("the root is a pane") };
        let Node::Split(column) = &*root.b else { panic!("the right side is a pane") };
        let (Node::Pane(top), Node::Pane(foot)) = (&*column.a, &*column.b) else { panic!("the column holds a split") };
        (root, top, foot)
    }

    /// Today's screen: the thread, then a column of Open, Done and Deferred over Now, 0.62 across and 2:3 down.
    #[test]
    fn a_job_starts_with_the_screen_it_always_had() {
        let arrangement = tree::default_arrangement();
        let (root, top, foot) = column(&arrangement);
        assert_eq!(root.dir, Dir::Row);
        assert!((root.ratio - 0.62).abs() < 1e-9);
        let Node::Pane(thread) = &*root.a else { panic!("the thread's side is a split") };
        assert_eq!(kinds(thread), ["thread"]);
        let Node::Split(inner) = &*root.b else { unreachable!() };
        assert_eq!(inner.dir, Dir::Column);
        // Top to Now is two parts to three.
        assert!((inner.ratio - 0.4).abs() < 1e-9);
        assert_eq!(kinds(top), ["open", "done", "deferred"]);
        assert_eq!(kinds(foot), ["now"]);
        assert_eq!(arrangement.focus, thread.id);
        // The three panes read by the names their strips and regions have always had.
        assert_eq!([slot_of(&thread.id), slot_of(&top.id), slot_of(&foot.id)], [Slot::Thread, Slot::Top, Slot::Now]);
    }

    #[test]
    fn a_pane_split_off_reads_as_no_slot_of_the_default() {
        let arrangement = tree::default_arrangement();
        let (_, _, foot) = column(&arrangement);
        let split = tree::split_pane(&arrangement, &foot.id, SplitInto::Right).into_owned();
        let made = split.focus.clone();
        assert_ne!(made, foot.id);
        assert_eq!(slot_of(&made), Slot::Other);
        assert_eq!(slot_of(&foot.id), Slot::Now);
    }

    /// A view tab's three places map to the thread's pane and the column's first and second.
    #[test]
    fn a_view_tabs_places_map_onto_the_trees_panes() {
        let arrangement = tree::default_arrangement();
        let (_, top, foot) = column(&arrangement);
        let thread = tree::holder_of(&arrangement, "thread").expect("a thread").id.clone();
        assert_eq!(pane_for(&arrangement, ViewPane::Thread), Some(thread.clone()));
        assert_eq!(pane_for(&arrangement, ViewPane::Top), Some(top.id.clone()));
        assert_eq!(pane_for(&arrangement, ViewPane::Bottom), Some(foot.id.clone()));
        assert_eq!(view_pane_of(&arrangement, &thread), ViewPane::Thread);
        assert_eq!(view_pane_of(&arrangement, &top.id), ViewPane::Top);
        assert_eq!(view_pane_of(&arrangement, &foot.id), ViewPane::Bottom);
        // The foot stays the foot with a pane split off beside it.
        let split = tree::split_pane(&arrangement, &top.id, SplitInto::Right).into_owned();
        assert_eq!(pane_for(&split, ViewPane::Top).as_deref(), Some(top.id.as_str()));
    }

    /// Where the thread stands alone there is no column: the foot has no pane, and the tab opens as a split.
    #[test]
    fn a_place_with_no_column_has_no_foot() {
        let arrangement = tree::default_arrangement();
        let (_, top, foot) = column(&arrangement);
        let (top, foot) = (top.id.clone(), foot.id.clone());
        let alone = tree::close_pane(&tree::close_pane(&arrangement, &top).into_owned(), &foot).into_owned();
        assert_eq!(pane_for(&alone, ViewPane::Bottom), None);
    }

    /// A shell arrives in the column, or in the pane its + was pressed in; the same shell is not opened twice.
    #[test]
    fn a_shell_goes_where_it_was_asked_and_once() {
        let arrangement = tree::default_arrangement();
        let (_, _, foot) = column(&arrangement);
        let thread = tree::holder_of(&arrangement, "thread").expect("a thread").id.clone();
        let there = put_terminal(&arrangement, "t1", Some(&foot.id), true);
        let pane = tree::pane_by_id(&there, &foot.id).expect("the foot");
        assert_eq!(kinds(pane), ["now", "terminal"]);
        assert_eq!(front_of(pane).map(|tab| tab.kind.as_str()), Some("terminal"));
        // Asked again for another pane, it moves rather than opening a second.
        let moved = put_terminal(&there, "t1", Some(&thread), true);
        let count = tree::panes_of(&moved.root).iter().flat_map(|pane| pane.tabs.iter()).filter(|tab| tab.kind == "terminal").count();
        assert_eq!(count, 1);
        assert_eq!(kinds(tree::pane_by_id(&moved, &thread).expect("the thread's pane")), ["thread", "terminal"]);
    }

    /// Split right, then ⌘Z: the arrangement is the one before the split, and ⇧⌘Z puts the split back.
    #[test]
    fn undo_takes_a_split_back_and_redo_puts_it_back() {
        let was = tree::default_arrangement();
        let (_, _, foot) = column(&was);
        let size = Some(Size { w: 1400., h: 900. });
        let split = tree::rearrange(&was, |arrangement| tree::split_pane(arrangement, &foot.id, SplitInto::Right), size, false).expect("room for a split");
        assert_eq!(tree::panes_of(&split.root).len(), 4);
        let mut history = History::default();
        history.remember(&was, None, 0);
        assert_eq!(history.undo(&split).as_ref(), Some(&was));
        assert_eq!(history.undo(&was), None);
        assert_eq!(history.redo(&was).as_ref(), Some(&split));
    }

    /// A pane keeps its least size: the column at a narrow window has no room for two panes side by side.
    #[test]
    fn a_split_that_leaves_a_pane_too_small_is_refused() {
        let was = tree::default_arrangement();
        let (_, _, foot) = column(&was);
        let split = |size: Option<Size>| tree::rearrange(&was, |arrangement| tree::split_pane(arrangement, &foot.id, SplitInto::Right), size, false);
        assert!(split(Some(Size { w: 1400., h: 900. })).is_some());
        assert!(split(Some(Size { w: 500., h: 900. })).is_none());
    }

    /// The thread's only tab closed takes its pane; opening it again makes a pane where it stood, beside the column, in front.
    #[test]
    fn a_closed_thread_comes_back_where_it_stood() {
        let was = tree::default_arrangement();
        let thread = tree::holder_of(&was, "thread").expect("a thread");
        let tab = thread.tabs[0].id.clone();
        let closed = tree::close_tab(&was, &tab).into_owned();
        assert!(!tree::has_kind(&closed, "thread"));
        let back = tree::reopen_thread(&closed).into_owned();
        let (root, _, _) = column(&back);
        assert_eq!(root.dir, Dir::Row);
        let Node::Pane(first) = &*root.a else { panic!("the thread's side is a split") };
        assert_eq!(kinds(first), ["thread"]);
    }

    /// Close pane takes the pane and its tabs; the last pane of all stays.
    #[test]
    fn closing_a_pane_takes_it_and_the_last_one_stays() {
        let was = tree::default_arrangement();
        let (_, top, _) = column(&was);
        let closed = tree::close_pane(&was, &top.id).into_owned();
        assert_eq!(tree::panes_of(&closed.root).len(), 2);
        assert!(tree::pane_by_id(&closed, &top.id).is_none());
        let one = tree::panes_of(&closed.root)[0].id.clone();
        let only = tree::close_pane(&closed, &tree::panes_of(&closed.root)[1].id.clone()).into_owned();
        assert!(matches!(tree::close_pane(&only, &one), Cow::Borrowed(_)));
    }

    /// Grow and Shrink move the split the pane is a side of; the next pane runs round.
    #[test]
    fn a_pane_knows_its_split_and_the_pane_after_it() {
        let arrangement = tree::default_arrangement();
        let (root, top, foot) = column(&arrangement);
        let thread = tree::holder_of(&arrangement, "thread").expect("a thread").id.clone();
        let Node::Split(inner) = &*root.b else { unreachable!() };
        assert_eq!(parent_split_of(&arrangement.root, &thread), Some((root.id.clone(), true)));
        assert_eq!(parent_split_of(&arrangement.root, &top.id), Some((inner.id.clone(), true)));
        assert_eq!(parent_split_of(&arrangement.root, &foot.id), Some((inner.id.clone(), false)));
        assert_eq!(next_pane_of(&arrangement, &thread).as_deref(), Some(top.id.as_str()));
        assert_eq!(next_pane_of(&arrangement, &foot.id).as_deref(), Some(thread.as_str()));
        let alone = tree::close_pane(&tree::close_pane(&arrangement, &top.id).into_owned(), &foot.id).into_owned();
        assert_eq!(next_pane_of(&alone, &thread), None);
    }

    /// The tab goes to the pane after its own and the pane it left goes with its last tab.
    #[test]
    fn a_tab_moved_to_the_next_pane_leaves_an_emptied_pane_out() {
        let was = tree::default_arrangement();
        let (_, _, foot) = column(&was);
        let (foot, now) = (foot.id.clone(), foot.tabs[0].id.clone());
        let to = next_pane_of(&was, &foot).expect("a next pane");
        let moved = tree::move_tab_to_pane(&was, &now, &to, None).into_owned();
        assert_eq!(tree::panes_of(&moved.root).len(), 2);
        assert!(tree::pane_by_id(&moved, &foot).is_none());
        let held = tree::pane_by_id(&moved, &to).expect("the pane it went to");
        assert_eq!(front_of(held).map(|tab| tab.id.as_str()), Some(now.as_str()));
        assert_eq!(moved.focus, to);
    }

    /// Every pane key is a row of the registry with a chord, so the keys are read from there and never written in a view.
    #[test]
    fn every_pane_key_is_a_registry_row_with_a_chord() {
        for id in ["pane_split_right", "pane_split_down", "pane_undo", "pane_redo", "pane_zoom", "next_tab", "previous_tab", "move_tab_next_pane", "pane_grow", "pane_shrink"] {
            let row = crate::generated::actions::act(id).unwrap_or_else(|| panic!("no registry row {id}"));
            assert!(crate::keymap::parse_chord(row.shortcut).is_some(), "{id} has no chord: {}", row.shortcut);
        }
        // The four arrows are one row, bound once for each direction.
        assert!(crate::generated::actions::act("focus_pane").is_some());
    }
}
