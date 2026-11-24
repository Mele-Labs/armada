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
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Instant;

use gpui::{Bounds, Context, FocusHandle, Focusable, Pixels, ScrollHandle, SharedString, Window};

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
    /// The store has answered, with what it kept or with nothing, so a tab the Job puts in is not lost to it.
    answered: bool,
}

impl Placed {
    fn new(arrangement: Arrangement) -> Self {
        Placed { arrangement, history: History::default(), changed: false, answered: false }
    }
}

/// Where the Jobs' panes stand between draws.
pub(super) struct Arranged {
    jobs: HashMap<SharedString, Placed>,
    /// The pane covering the Job's area, if one is: where he is looking, not how the Job is laid out, so it is not kept.
    pub zoom: Option<String>,
    /// The Job's area as last drawn, which a change is held against (`allowed`).
    pub area: Rc<Cell<Option<Size>>>,
    /// The split whose divider is being dragged.
    pub dragging: Option<String>,
    /// Focus handles by `{job}/{tab}`, `{job}/add:{pane}`.
    pub(super) handles: HashMap<String, FocusHandle>,
    /// Each strip's scroll, by `{job}/{pane}`, and the tab that was in front when it was last drawn.
    pub scrolls: HashMap<String, (ScrollHandle, Option<String>)>,
    /// The pane a + was pressed in, for the tab its source opens next on that Job, while that is still recent.
    aimed: Option<(String, String, Instant)>,
    started: Instant,
    /// The tab being dragged and where it is over (`place_drag.rs`).
    pub drop: Option<super::place_drag::Hover>,
    /// Each strip, tab, body and gap as last laid out, by `strip:{pane}`, `tab:{tab}`, `body:{pane}`, `gap:{split}`.
    pub drawn_at: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
}

impl Default for Arranged {
    fn default() -> Self {
        Arranged {
            jobs: HashMap::new(),
            zoom: None,
            area: Rc::default(),
            dragging: None,
            handles: HashMap::new(),
            scrolls: HashMap::new(),
            aimed: None,
            started: Instant::now(),
            drop: None,
            drawn_at: Rc::default(),
        }
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
    // The column is one pane now; a Job kept with the old column of tabs over Now has its two panes named as they were.
    static IDS: OnceLock<([String; 2], [String; 3])> = OnceLock::new();
    let (now, legacy) = IDS.get_or_init(|| {
        let ids = |arrangement: tree::Arrangement| tree::panes_of(&arrangement.root).into_iter().map(|pane| pane.id.clone()).collect::<Vec<_>>();
        let (now, legacy) = (ids(tree::default_arrangement()), ids(tree::legacy_arrangement()));
        ([now[0].clone(), now[1].clone()], [legacy[0].clone(), legacy[1].clone(), legacy[2].clone()])
    });
    if pane == now[0] || pane == legacy[0] {
        Slot::Thread
    } else if pane == now[1] || pane == legacy[1] {
        Slot::Top
    } else if pane == legacy[2] {
        Slot::Now
    } else {
        Slot::Other
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
        RowKind::Artifact => "artifact",
        RowKind::Prototype => "prototype",
    }
}

pub(super) fn row_kind_of(kind: &str) -> Option<RowKind> {
    match kind {
        "draft" => Some(RowKind::Draft),
        "document" => Some(RowKind::Document),
        "walk" => Some(RowKind::Walk),
        "subagent" => Some(RowKind::Subagent),
        "artifact" => Some(RowKind::Artifact),
        "prototype" => Some(RowKind::Prototype),
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
/// column beside it. None where there is none, and the tab then opens as a split. Where the thread's neighbour is the
/// side chat's own pane, the column is beside the split of the two.
pub(super) fn pane_for(arrangement: &Arrangement, place: ViewPane) -> Option<String> {
    let thread = tree::holder_of(arrangement, "thread");
    if place == ViewPane::Thread {
        return thread.map(|pane| pane.id.clone());
    }
    let mut beside = thread.and_then(|pane| sibling_of(&arrangement.root, &pane.id));
    if beside.is_some_and(side_pane) {
        let split = thread.and_then(|pane| parent_split_of(&arrangement.root, &pane.id)).map(|(split, _)| split);
        beside = split.and_then(|split| sibling_of(&arrangement.root, &split));
    }
    let column = beside.is_some();
    let panes: Vec<&Pane> = tree::panes_of(beside.unwrap_or(&arrangement.root)).into_iter().filter(|pane| !is_side(pane)).collect();
    let at = usize::from(place == ViewPane::Bottom);
    // A column of one pane is the foot as much as the top.
    let held = if at == 1 && column && panes.len() == 1 { panes.first() } else { panes.get(at) };
    held.map(|pane| pane.id.clone())
}

/// Whether a pane holds the side chat's tab and nothing else.
fn is_side(pane: &tree::Pane) -> bool {
    !pane.tabs.is_empty() && pane.tabs.iter().all(|tab| tab.kind == "terminal" && tab.terminal.as_deref().is_some_and(|key| key.starts_with("aside:")))
}

fn side_pane(node: &Node) -> bool {
    matches!(node, Node::Pane(pane) if is_side(pane))
}

/// Which place a pane is, as a view tab names its places.
pub(super) fn view_pane_of(arrangement: &Arrangement, pane: &str) -> ViewPane {
    if tree::holder_of(arrangement, "thread").is_some_and(|holder| holder.id == pane) {
        ViewPane::Thread
    } else if tree::pane_by_id(arrangement, pane).is_some_and(is_side) {
        ViewPane::Beside
    } else if pane_for(arrangement, ViewPane::Top).is_some_and(|first| first == pane) {
        ViewPane::Top
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

/// A tab the Job puts in once on its own, behind: right after Now in the pane that holds Now, else at the end of the
/// column's foot, else in a split of its own. The front tab of every pane and the focus stay where they were. A Job it
/// was offered to before gets nothing, so one he closed stays closed; one already open is only marked offered.
pub(super) fn offer_beside_now<'a>(was: &'a Arrangement, kind: &str) -> Out<'a> {
    if was.offered.iter().any(|one| one == kind) {
        return Cow::Borrowed(was);
    }
    let mut next = if tree::has_kind(was, kind) {
        was.clone()
    } else {
        let after_now = tree::holder_of(was, "now").map(|pane| (pane.id.clone(), pane.tabs.iter().position(|tab| tab.kind == "now").map(|at| at + 1)));
        put_behind(was, kind, after_now.or_else(|| pane_for(was, ViewPane::Bottom).map(|pane| (pane, None))))
    };
    next.offered.push(kind.to_string());
    Cow::Owned(next)
}

/// Story is a fixed kind of a Job's panes. One a Job lacks is put in once on its own, behind, right after Thread in the
/// pane that holds Thread, and the front tab of every pane and the focus stay. Where no pane holds Thread, none is put in
/// and nothing is marked, so it comes when Thread does. A Story tab the view-tab API kept (a shell named `story:<job>`)
/// becomes the kind in its place. Once placed it is marked offered, so one he closed stays closed, and one he already has
/// keeps its place.
pub(super) fn offer_story(was: &Arrangement) -> Out<'_> {
    if was.offered.iter().any(|one| one == "story") {
        return Cow::Borrowed(was);
    }
    let mut next = was.clone();
    retag(&mut next.root, &mut |tab| {
        if tab.kind == "terminal" && tab.terminal.as_deref().is_some_and(|key| key.starts_with("story:")) {
            tab.kind = "story".to_string();
            tab.terminal = None;
        }
    });
    if !tree::has_kind(&next, "story") {
        let Some(after_thread) = tree::holder_of(&next, "thread").map(|pane| (pane.id.clone(), pane.tabs.iter().position(|tab| tab.kind == "thread").map(|at| at + 1))) else {
            return Cow::Borrowed(was);
        };
        next = put_behind(&next, "story", Some(after_thread));
    }
    next.offered.push("story".to_string());
    Cow::Owned(next)
}

/// The tabs the column's bands took the place of: Needs you holds what For you, Open, Done and Deferred listed.
const BAND_TABS: [&str; 4] = ["for_you", "open", "done", "deferred"];

/// A Job kept with those four as tabs shows them twice, as tabs and as the band. They go out of its panes once, a pane
/// left empty goes with them and the rest of the layout stays; they are marked, so one he opens again from + stays.
pub(super) fn leave_bands(was: &Arrangement) -> Out<'_> {
    if was.offered.iter().any(|one| one == "bands") {
        return Cow::Borrowed(was);
    }
    let held: Vec<String> = tree::panes_of(&was.root).into_iter().flat_map(|pane| pane.tabs.iter()).filter(|tab| BAND_TABS.contains(&tab.kind.as_str())).map(|tab| tab.id.clone()).collect();
    let mut next = held.iter().fold(was.clone(), |at, tab| tree::close_tab(&at, tab).into_owned());
    next.offered.push("bands".to_string());
    Cow::Owned(next)
}

/// Child Jobs is put in once on its own, behind, at the end of the column's top pane, and the front tab of every pane and the
/// focus stay. A Job it was offered to before gets nothing, so one he closed stays closed; one already open is only marked.
pub(super) fn offer_children_tab(was: &Arrangement) -> Out<'_> {
    if was.offered.iter().any(|one| one == "children") {
        return Cow::Borrowed(was);
    }
    let mut next = if tree::has_kind(was, "children") { was.clone() } else { put_behind(was, "children", pane_for(was, ViewPane::Top).map(|pane| (pane, None))) };
    next.offered.push("children".to_string());
    Cow::Owned(next)
}

/// A Job kept with the column on Now, alone or with Artifacts (what a new Job opened on before the column showed what the Job
/// produced), has Now taken out and Artifacts, Pull Requests and Issues put first; a tab he opened himself stays behind them.
/// Child Jobs comes by `offer_children` where the Job started any. A column he arranged otherwise, split or without Now, is
/// left, and the move is marked in both cases, so Now opened again from + stays.
pub(super) fn leave_now(was: &Arrangement) -> Out<'_> {
    if was.offered.iter().any(|one| one == "made") {
        return Cow::Borrowed(was);
    }
    let mut next = was.clone();
    let column = pane_for(was, ViewPane::Top).filter(|top| pane_for(was, ViewPane::Bottom).as_deref() == Some(top.as_str()));
    let now = tree::holder_of(was, "now").map(|pane| pane.id.clone());
    if let Some(pane) = column.filter(|column| now.as_ref() == Some(column)) {
        let front_was_now = tree::pane_by_id(was, &pane).and_then(front_of).is_some_and(|tab| tab.kind == "now");
        for kind in &tree::PRODUCED[..3] {
            if !tree::has_kind(&next, kind) {
                next = tree::add_tab(&next, &pane, kind, Extras::default()).into_owned();
            }
        }
        for (at, kind) in tree::PRODUCED[..3].iter().enumerate() {
            let tab = tree::pane_by_id(&next, &pane).and_then(|one| one.tabs.iter().find(|tab| tab.kind == *kind)).map(|tab| tab.id.clone());
            if let Some(tab) = tab {
                next = tree::move_tab_to_pane(&next, &tab, &pane, Some(at)).into_owned();
            }
        }
        let held: Vec<String> = tree::pane_by_id(&next, &pane).map(|one| one.tabs.iter().filter(|tab| tab.kind == "now").map(|tab| tab.id.clone()).collect()).unwrap_or_default();
        next = held.iter().fold(next, |at, tab| tree::close_tab(&at, tab).into_owned());
        // Where Now was in front, what he had is Artifacts, the first of what the Job produced.
        let artifacts = tree::holder_of(&next, "artifacts").filter(|_| front_was_now).and_then(|holder| holder.tabs.iter().find(|tab| tab.kind == "artifacts")).map(|tab| tab.id.clone());
        if let Some(tab) = artifacts {
            next = tree::activate(&next, &tab).into_owned();
        }
        if !next.offered.iter().any(|one| one == "artifacts") {
            next.offered.push("artifacts".to_string());
        }
    }
    next.offered.push("made".to_string());
    Cow::Owned(next)
}

fn retag(node: &mut Node, change: &mut impl FnMut(&mut Tab)) {
    match node {
        Node::Pane(pane) => pane.tabs.iter_mut().for_each(change),
        Node::Split(split) => {
            retag(&mut split.a, change);
            retag(&mut split.b, change);
        }
    }
}

/// A new tab of `kind` at the end of the pane, or at an index in it, else in a split of its own, behind: each pane keeps
/// the tab it had in front, and the focus stays.
pub(super) fn put_behind(was: &Arrangement, kind: &str, target: Option<(String, Option<usize>)>) -> Arrangement {
    let mut next = match &target {
        Some((pane, _)) => tree::add_tab(was, pane, kind, Extras::default()).into_owned(),
        None => tree::arrive(was, kind, Extras::default()),
    };
    if let Some((pane, Some(at))) = &target {
        let made = tree::pane_by_id(&next, pane).and_then(|one| one.tabs.last()).map(|tab| tab.id.clone());
        if let Some(made) = made {
            next = tree::move_tab_to_pane(&next, &made, pane, Some(*at)).into_owned();
        }
    }
    // What each pane showed stays in front. A pane kept with no front tab shows its first, and `add_tab` would have put the
    // new one there, so a Job opened on a tab it put in itself.
    for pane in tree::panes_of(&was.root) {
        let shown = pane.active.clone().or_else(|| front_of(pane).map(|tab| tab.id.clone()));
        if let Some(shown) = shown {
            next = tree::activate(&next, &shown).into_owned();
        }
    }
    if tree::pane_by_id(&next, &was.focus).is_some() {
        next.focus = was.focus.clone();
    }
    next
}

impl JobView {
    // ---- the arrangement ---------------------------------------------------------------------

    /// The shown Job's arrangement; the default one before the Job has been placed.
    pub(super) fn arrangement(&self) -> Cow<'_, Arrangement> {
        match self.shown.as_ref().and_then(|id| self.arranged.jobs.get(id)) {
            Some(placed) => Cow::Borrowed(&placed.arrangement),
            None => Cow::Owned(tree::default_arrangement()),
        }
    }

    pub(super) fn arrangement_of(&self, job: &str) -> Option<&Arrangement> {
        self.arranged.jobs.get(job).map(|placed| &placed.arrangement)
    }

    /// A change the Job's arrival puts back, which is neither a step back nor kept: an arrangement the store answers with still replaces it.
    pub(super) fn restore_in(&mut self, job: &SharedString, change: impl for<'a> FnOnce(&'a Arrangement) -> Out<'a>) {
        if let Some(placed) = self.arranged.jobs.get_mut(job) {
            if let Cow::Owned(next) = change(&placed.arrangement) {
                placed.arrangement = next;
            }
        }
    }

    /// The Job has an arrangement: the default until the store's answers. A change made meanwhile is kept.
    pub(super) fn ensure_placed(&mut self, job: &SharedString, cx: &mut Context<Self>) {
        if self.arranged.jobs.contains_key(job) {
            return;
        }
        self.arranged.jobs.insert(job.clone(), Placed::new(tree::default_arrangement()));
        let read = pane_store::read(&pane_store::job_key(job), cx);
        let job = job.clone();
        cx.spawn(async move |this, cx| {
            let kept = read.await.map(|kept| {
                // A side panel's tab holds what this run read, so a kept one comes back with nothing to draw: it goes.
                let panels: Vec<String> = tree::panes_of(&kept.root).into_iter().flat_map(|pane| pane.tabs.iter()).filter(|tab| tab.kind == "terminal" && tab.terminal.as_deref().is_some_and(|key| key.starts_with("panel:"))).map(|tab| tab.id.clone()).collect();
                panels.iter().fold(kept, |was, tab| tree::close_tab(&was, tab).into_owned())
            });
            let _ = this.update(cx, |this, cx| {
                let Some(placed) = this.arranged.jobs.get_mut(&job) else { return };
                placed.answered = true;
                if let Some(kept) = kept.filter(|_| !placed.changed) {
                    // The thread is what a Job opens on, whichever tab of its pane was in front when it was left.
                    placed.arrangement = tree::thread_in_front(&kept);
                    cx.notify();
                }
                // With Needs you hidden the tabs are all he has of those lists, so they stay.
                if this.band_shown(super::column_bands::Band::Column) {
                    this.look_in(&job, leave_bands, cx);
                }
                this.look_in(&job, leave_now, cx);
                this.offer_artifacts(&job, cx);
                this.offer_story_tab(&job, cx);
            });
        })
        .detach();
    }

    /// The Job has started a Job, and has none of its Child Jobs tab yet: it opens, behind, once. It waits for the store's answer.
    pub(super) fn offer_children(&mut self, job: &SharedString, started: bool, cx: &mut Context<Self>) {
        if started && self.panes_answered(job) {
            self.look_in(job, offer_children_tab, cx);
        }
    }

    /// The store has answered: Story is put in the Job's panes once, and a Story link left for the Job opens it.
    fn offer_story_tab(&mut self, job: &SharedString, cx: &mut Context<Self>) {
        self.look_in(job, offer_story, cx);
        self.open_for_link(job, cx);
    }

    /// Whether the store has answered for the Job's panes.
    pub(super) fn panes_answered(&self, job: &str) -> bool {
        self.arranged.jobs.get(job).is_some_and(|placed| placed.answered)
    }

    fn now_ms(&self) -> u64 {
        self.arranged.started.elapsed().as_millis() as u64
    }

    /// A change he made, or the Job's own that is his to take back: held against the panes' least sizes unless
    /// `force`, kept for ⌘Z (a run of one `merge` is one step), and written to the store. True where it changed.
    pub(super) fn rearrange_in(&mut self, job: &SharedString, merge: Option<&str>, force: bool, change: impl for<'a> FnOnce(&'a Arrangement) -> Out<'a>, cx: &mut Context<Self>) -> bool {
        self.ensure_placed(job, cx);
        let size = if self.shown.as_ref() == Some(job) { self.arranged.area.get() } else { None };
        let now = self.now_ms();
        let Some(placed) = self.arranged.jobs.get_mut(job) else { return false };
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
        let Some(placed) = self.arranged.jobs.get_mut(job) else { return false };
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
        let Some(placed) = self.arranged.jobs.get_mut(&job) else { return false };
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
        // The Side chat tab among the thread's own opens its split.
        if self.side_tab_pressed(tab, window, cx) {
            return;
        }
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
        if let Some(placed) = self.arranged.jobs.get_mut(&job) {
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
        self.arranged.aimed = Some((job.to_string(), pane.to_string(), Instant::now()));
    }

    /// The pane the last + aimed at, once: where it is still held and was pressed not long ago.
    pub(super) fn take_aim(&mut self, job: &str) -> Option<String> {
        let (aimed_job, pane, at) = self.arranged.aimed.take()?;
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
        self.arranged.handles.entry(format!("{job}/{tab}")).or_insert_with(|| cx.focus_handle()).clone()
    }

    pub(super) fn add_handle(&mut self, job: &SharedString, pane: &str, cx: &mut Context<Self>) -> FocusHandle {
        self.arranged.handles.entry(format!("{job}/add:{pane}")).or_insert_with(|| cx.focus_handle()).clone()
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
            || self.pictures.viewing.is_some()
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
            // Back, Forward and Reload are the Job browser's where the pane the keys are in has its tab in front.
            "history" | "browser_reload" => return self.browser_act(act, window, cx),
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
        if self.arranged.zoom.as_deref() == Some(pane) {
            self.arranged.zoom = None;
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
        self.arranged.zoom = if self.arranged.zoom.as_deref() == Some(pane) { None } else { Some(pane.to_string()) };
        cx.notify();
    }

    /// Another Job on screen ends a zoom.
    pub(super) fn end_zoom(&mut self) {
        self.arranged.zoom = None;
    }

    /// The pane that is zoomed, where it is still one.
    pub(super) fn zoomed(&self) -> Option<String> {
        let zoom = self.arranged.zoom.as_ref()?;
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

    /// The Job as it was started before the column's bands: For you, Open, Done and Deferred as tabs over Now, with Story offered.
    fn two_panes() -> Arrangement {
        let mut arrangement = tree::legacy_arrangement();
        arrangement.offered.push("story".to_string());
        arrangement
    }

    /// A new Job: the thread with Story behind it, then one column pane that holds what the Job produced. The strip and Needs you are bands over it, not tabs.
    #[test]
    fn a_new_job_starts_with_a_column_of_what_it_produced() {
        let arrangement = tree::default_arrangement();
        let Node::Split(root) = &arrangement.root else { panic!("the root is a pane") };
        assert_eq!(root.dir, Dir::Row);
        assert!((root.ratio - 0.62).abs() < 1e-9);
        let (Node::Pane(thread), Node::Pane(column)) = (&*root.a, &*root.b) else { panic!("the column is not one pane") };
        assert_eq!(kinds(thread), ["thread", "story"]);
        assert_eq!(kinds(column), ["artifacts", "pulls", "issues"], "Child Jobs comes with the first Job it starts");
        for gone in ["now", "children", "for_you", "open", "done", "deferred"] {
            assert!(!tree::has_kind(&arrangement, gone), "{gone} is opened from +, not placed");
        }
        assert_eq!(arrangement.offered, ["story", "artifacts", "made"], "none of the three is put back once closed");
        assert_eq!(arrangement.focus, thread.id);
        assert_eq!([slot_of(&thread.id), slot_of(&column.id)], [Slot::Thread, Slot::Top]);
        // The one pane is the top and the foot of the column, so a view tab asking for either lands in it.
        assert_eq!(pane_for(&arrangement, ViewPane::Top), Some(column.id.clone()));
        assert_eq!(pane_for(&arrangement, ViewPane::Bottom), Some(column.id.clone()));
        assert_eq!(view_pane_of(&arrangement, &column.id), ViewPane::Top);
    }

    /// Today's screen: the thread with Story behind it, then a column of For you, Open, Done and Deferred over Now, 0.62 across and 2:3 down.
    #[test]
    fn a_job_kept_with_the_old_column_reads_as_it_did() {
        let arrangement = two_panes();
        let (root, top, foot) = column(&arrangement);
        assert_eq!(root.dir, Dir::Row);
        assert!((root.ratio - 0.62).abs() < 1e-9);
        let Node::Pane(thread) = &*root.a else { panic!("the thread's side is a split") };
        assert_eq!(kinds(thread), ["thread", "story"]);
        let Node::Split(inner) = &*root.b else { unreachable!() };
        assert_eq!(inner.dir, Dir::Column);
        // Top to Now is two parts to three.
        assert!((inner.ratio - 0.4).abs() < 1e-9);
        assert_eq!(kinds(top), ["for_you", "open", "done", "deferred"]);
        assert_eq!(kinds(foot), ["now"]);
        assert_eq!(arrangement.focus, thread.id);
        // The three panes read by the names their strips and regions have always had.
        assert_eq!([slot_of(&thread.id), slot_of(&top.id), slot_of(&foot.id)], [Slot::Thread, Slot::Top, Slot::Now]);
    }

    #[test]
    fn a_pane_split_off_reads_as_no_slot_of_the_default() {
        let arrangement = two_panes();
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
        let arrangement = two_panes();
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

    /// The default screen with the side chat split off the thread's pane: the thread, the side chat's pane, then the column.
    fn with_side_chat(arrangement: &Arrangement) -> (Arrangement, String) {
        let thread = tree::holder_of(arrangement, "thread").expect("a thread").id.clone();
        let held = put_terminal(arrangement, "aside:mock", Some(&thread), false);
        let split = tree::split_pane(&held, &thread, SplitInto::Right).into_owned();
        let beside = split.focus.clone();
        let tab = tree::panes_of(&split.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.terminal.as_deref() == Some("aside:mock")).expect("the tab").id.clone();
        (tree::move_tab_to_pane(&split, &tab, &beside, None).into_owned(), beside)
    }

    /// With the side chat split open, the column is still Top and Bottom, and the side chat's pane is neither.
    #[test]
    fn with_the_side_chat_split_open_top_and_bottom_are_still_the_column() {
        let was = two_panes();
        let (_, top, foot) = column(&was);
        let (top, foot) = (top.id.clone(), foot.id.clone());
        let (arrangement, beside) = with_side_chat(&was);
        assert_eq!(pane_for(&arrangement, ViewPane::Top), Some(top.clone()), "Top is the column's first pane");
        assert_eq!(pane_for(&arrangement, ViewPane::Bottom), Some(foot.clone()), "Bottom is its second");
        assert_eq!(view_pane_of(&arrangement, &beside), ViewPane::Beside, "the side chat's pane is neither");
        assert_eq!(view_pane_of(&arrangement, &top), ViewPane::Top);
        assert_eq!(view_pane_of(&arrangement, &foot), ViewPane::Bottom);
        let thread = tree::holder_of(&arrangement, "thread").expect("a thread").id.clone();
        assert_eq!(pane_for(&arrangement, ViewPane::Thread), Some(thread.clone()));
        assert_eq!(view_pane_of(&arrangement, &thread), ViewPane::Thread);
        // With no column beside the two, nothing is handed out as the column.
        let alone = tree::close_pane(&tree::close_pane(&arrangement, &top).into_owned(), &foot).into_owned();
        assert_eq!(pane_for(&alone, ViewPane::Bottom), None);
    }

    /// Where the thread stands alone there is no column: the foot has no pane, and the tab opens as a split.
    #[test]
    fn a_place_with_no_column_has_no_foot() {
        let arrangement = two_panes();
        let (_, top, foot) = column(&arrangement);
        let (top, foot) = (top.id.clone(), foot.id.clone());
        let alone = tree::close_pane(&tree::close_pane(&arrangement, &top).into_owned(), &foot).into_owned();
        assert_eq!(pane_for(&alone, ViewPane::Bottom), None);
    }

    /// A Job's first artifact puts Artifacts right after Now, behind it, with the focus where it was.
    #[test]
    fn a_first_artifact_puts_artifacts_after_now_behind_it() {
        let mut arrangement = two_panes();
        let (_, _, foot) = column(&arrangement);
        let foot = foot.id.clone();
        arrangement = put_terminal(&arrangement, "t1", Some(&foot), false);
        let now = tree::holder_of(&arrangement, "now").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "now")).map(|tab| tab.id.clone());
        let offered = offer_beside_now(&arrangement, "artifacts").into_owned();
        let pane = tree::pane_by_id(&offered, &foot).expect("the foot");
        assert_eq!(kinds(pane), ["now", "artifacts", "terminal"]);
        assert_eq!(pane.active, now, "Now stays in front");
        assert_eq!(offered.focus, arrangement.focus);
        assert_eq!(offered.offered, ["story", "artifacts"], "the default carries Story, and Artifacts is marked after it");
    }

    /// Read again, the Job is not given a second Artifacts tab.
    #[test]
    fn a_second_read_adds_no_second_artifacts_tab() {
        let once = offer_beside_now(&two_panes(), "artifacts").into_owned();
        assert!(matches!(offer_beside_now(&once, "artifacts"), Cow::Borrowed(_)));
        // Already open where he put it, it is only marked offered.
        let fresh = two_panes();
        let (_, top, _) = column(&fresh);
        let opened = tree::add_tab(&fresh, &top.id, "artifacts", Extras::default()).into_owned();
        let marked = offer_beside_now(&opened, "artifacts").into_owned();
        assert_eq!(marked.root, opened.root);
        assert_eq!(marked.offered, ["story", "artifacts"]);
    }

    /// Closed, the tab stays closed, through a restart too.
    #[test]
    fn a_closed_artifacts_tab_is_not_put_back() {
        let once = offer_beside_now(&two_panes(), "artifacts").into_owned();
        let tab = tree::holder_of(&once, "artifacts").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "artifacts")).map(|tab| tab.id.clone()).expect("the tab");
        let closed = tree::close_tab(&once, &tab).into_owned();
        let kept = tree::revive(&serde_json::to_value(&closed).expect("json")).expect("an arrangement");
        let again = offer_beside_now(&kept, "artifacts");
        assert!(matches!(again, Cow::Borrowed(_)));
        assert!(!tree::has_kind(&again, "artifacts"));
    }

    /// The Story tab of an arrangement, by id.
    fn story_tab(arrangement: &Arrangement) -> String {
        tree::holder_of(arrangement, "story").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "story")).map(|tab| tab.id.clone()).expect("a Story tab")
    }

    /// The default arrangement as one kept before Story was a kind has it: no Story, and Plan and Files in the column's top.
    fn kept_without_story() -> (Arrangement, String) {
        let arrangement = two_panes();
        let closed = tree::close_tab(&arrangement, &story_tab(&arrangement)).into_owned();
        let top = column(&closed).1.id.clone();
        let planned = tree::add_tab(&closed, &top, "plan", Extras::default()).into_owned();
        let mut old = tree::add_tab(&planned, &top, "files", Extras::default()).into_owned();
        // Kept before Story was a kind, so it was never offered.
        old.offered.clear();
        (tree::revive(&serde_json::to_value(&old).expect("json")).expect("an arrangement"), top)
    }

    fn thread_pane(arrangement: &Arrangement) -> &Pane {
        tree::holder_of(arrangement, "thread").expect("a pane holds the thread")
    }

    /// A new Job's panes have Story right after Thread in the thread's pane, with Thread in front.
    #[test]
    fn the_default_has_thread_then_story_with_thread_in_front() {
        let arrangement = two_panes();
        let pane = thread_pane(&arrangement);
        assert_eq!(kinds(pane), ["thread", "story"]);
        let front = pane.active.as_ref().and_then(|id| pane.tabs.iter().find(|tab| &tab.id == id)).expect("a front tab");
        assert_eq!(front.kind, "thread");
        assert_eq!(arrangement.focus, pane.id);
        assert!(offer_story(&arrangement).into_owned().offered == ["story"], "it is only marked offered");
    }

    /// A pane kept with no front tab shows its first one, so a tab put in behind must leave that one in front: a Job whose
    /// thread pane was kept that way opened on Story, which `add_tab` brought to the front.
    #[test]
    fn a_pane_kept_with_no_front_tab_keeps_showing_its_first_when_story_is_put_in() {
        let (mut old, _) = kept_without_story();
        let left = thread_pane(&old).id.clone();
        let Node::Split(root) = &mut old.root else { panic!("the root is a split") };
        let Node::Pane(pane) = &mut *root.a else { panic!("the thread's pane is the left") };
        assert_eq!(pane.id, left);
        pane.active = None;
        assert_eq!(front_of(thread_pane(&old)).map(|tab| tab.kind.as_str()), Some("thread"));
        let opened = offer_story(&old).into_owned();
        let pane = tree::pane_by_id(&opened, &left).expect("the thread's pane");
        assert_eq!(kinds(pane), ["thread", "story"]);
        assert_eq!(front_of(pane).map(|tab| tab.kind.as_str()), Some("thread"), "Thread stays in front");
        let beside = offer_beside_now(&old, "plan").into_owned();
        for pane in tree::panes_of(&old.root) {
            assert_eq!(front_of(tree::pane_by_id(&beside, &pane.id).expect("the pane")).map(|tab| tab.id.clone()), front_of(pane).map(|tab| tab.id.clone()), "the front tab of {} stays", pane.id);
        }
    }

    /// An arrangement kept before Story was a kind has none: it gains Story right after Thread, behind, with the front tab
    /// of every pane and the focus where they were, and what else the pane holds stays after it.
    #[test]
    fn an_old_arrangement_gains_story_right_after_thread() {
        let (old, top) = kept_without_story();
        let left = thread_pane(&old).id.clone();
        let opened = offer_story(&old).into_owned();
        assert_eq!(kinds(tree::pane_by_id(&opened, &left).expect("the thread's pane")), ["thread", "story"]);
        assert_eq!(kinds(tree::pane_by_id(&opened, &top).expect("the top")), ["for_you", "open", "done", "deferred", "plan", "files"], "no other pane gains it");
        for pane in tree::panes_of(&old.root) {
            assert_eq!(tree::pane_by_id(&opened, &pane.id).and_then(|one| one.active.clone()), pane.active, "the front tab of {} stays", pane.id);
        }
        assert_eq!(opened.focus, old.focus);
        assert_eq!(opened.offered, ["story"]);
        // With another tab after Thread, Story goes between them.
        let beside = tree::add_tab(&old, &left, "browser", Extras::default()).into_owned();
        assert_eq!(kinds(tree::pane_by_id(&offer_story(&beside).into_owned(), &left).expect("the thread's pane")), ["thread", "story", "browser"]);
    }

    /// Where Thread is not in the left pane, Story follows it into the pane that holds it.
    #[test]
    fn story_goes_after_thread_in_whichever_pane_holds_it() {
        let (old, top) = kept_without_story();
        let thread = tree::holder_of(&old, "thread").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "thread")).map(|tab| tab.id.clone()).expect("the thread");
        let moved = tree::move_tab_to_pane(&old, &thread, &top, Some(2)).into_owned();
        assert_eq!(kinds(tree::pane_by_id(&moved, &top).expect("the top")), ["for_you", "open", "thread", "done", "deferred", "plan", "files"]);
        let opened = offer_story(&moved).into_owned();
        assert_eq!(kinds(tree::pane_by_id(&opened, &top).expect("the top")), ["for_you", "open", "thread", "story", "done", "deferred", "plan", "files"]);
        assert_eq!(tree::pane_by_id(&opened, &top).and_then(|pane| pane.active.clone()), tree::pane_by_id(&moved, &top).and_then(|pane| pane.active.clone()), "the front tab stays");
        assert_eq!(opened.focus, moved.focus);
        assert_eq!(opened.offered, ["story"]);
    }

    /// With no pane holding Thread there is nowhere to put Story, so it is not put in and nothing is marked offered.
    #[test]
    fn an_arrangement_with_no_thread_pane_is_not_given_story() {
        let (old, _) = kept_without_story();
        let thread = tree::holder_of(&old, "thread").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "thread")).map(|tab| tab.id.clone()).expect("the thread");
        let closed = tree::close_tab(&old, &thread).into_owned();
        assert!(tree::holder_of(&closed, "thread").is_none());
        let offered = offer_story(&closed);
        assert!(matches!(offered, Cow::Borrowed(_)));
        assert!(!tree::has_kind(&offered, "story"));
        assert!(offered.offered.is_empty());
    }

    /// Closed, Story stays closed through a load; read again, it is not put in twice.
    #[test]
    fn a_closed_story_stays_closed_across_loads() {
        let (old, _) = kept_without_story();
        let once = offer_story(&old).into_owned();
        assert!(matches!(offer_story(&once), Cow::Borrowed(_)));
        let closed = tree::close_tab(&once, &story_tab(&once)).into_owned();
        let mut kept = closed;
        for _ in 0..2 {
            kept = tree::revive(&serde_json::to_value(&kept).expect("json")).expect("an arrangement");
            let again = offer_story(&kept);
            assert!(matches!(again, Cow::Borrowed(_)));
            assert!(!tree::has_kind(&again, "story"));
            assert_eq!(again.offered, ["story"]);
        }
    }

    /// A new Job's default is already marked offered, so a Story he closes before the store answers is not put back.
    #[test]
    fn a_story_closed_on_a_new_job_before_the_store_answers_stays_closed() {
        let fresh = two_panes();
        assert_eq!(fresh.offered, ["story"], "the default carries it");
        let closed = tree::close_tab(&fresh, &story_tab(&fresh)).into_owned();
        assert!(!tree::has_kind(&closed, "story"));
        let again = offer_story(&closed);
        assert!(matches!(again, Cow::Borrowed(_)), "nothing is put in");
        assert!(!tree::has_kind(&again, "story"));
        assert_eq!(again.offered, ["story"]);
    }

    /// A Job that has Story already keeps its own place for it, in whichever pane, and it is only marked offered.
    #[test]
    fn an_arrangement_that_has_story_keeps_its_place_for_it() {
        let (old, top) = kept_without_story();
        let at_the_end = tree::add_tab(&old, &top, "story", Extras::default()).into_owned();
        let kept = offer_story(&at_the_end).into_owned();
        assert_eq!(kinds(tree::pane_by_id(&kept, &top).expect("the top")), ["for_you", "open", "done", "deferred", "plan", "files", "story"]);
        assert_eq!(kinds(thread_pane(&kept)), ["thread"], "it is not put in a second time");
        assert_eq!(kept.root, at_the_end.root);
        assert_eq!(kept.offered, ["story"]);
        let default = two_panes();
        let moved = tree::move_tab_to_pane(&default, &story_tab(&default), &column(&default).1.id, Some(1)).into_owned();
        let kept = offer_story(&moved).into_owned();
        assert_eq!(kinds(tree::pane_by_id(&kept, &column(&moved).1.id).expect("the top")), ["for_you", "story", "open", "done", "deferred"]);
        assert_eq!(kept.root, moved.root);
    }

    /// The Story tab the view-tab API kept, a shell named `story:<job>`, becomes the kind where it stood.
    #[test]
    fn a_story_tab_kept_as_a_view_tab_becomes_the_kind_in_place() {
        let (planned, top) = kept_without_story();
        let old = tree::add_tab(&planned, &top, "terminal", Extras::terminal(Some("story:job-a"))).into_owned();
        let shell = tree::add_tab(&old, &top, "terminal", Extras::terminal(Some("t1"))).into_owned();
        let opened = offer_story(&shell).into_owned();
        let pane = tree::pane_by_id(&opened, &top).expect("the top");
        assert_eq!(kinds(pane), ["for_you", "open", "done", "deferred", "plan", "files", "story", "terminal"]);
        assert!(pane.tabs.iter().all(|tab| tab.terminal.as_deref() != Some("story:job-a")));
        assert_eq!(pane.tabs.iter().filter(|tab| tab.kind == "terminal").count(), 1, "a real shell is left alone");
        assert_eq!(kinds(thread_pane(&opened)), ["thread"], "none is put in beside Thread as well");
    }

    #[test]
    fn story_is_a_kind_a_pane_can_hold_and_a_zone_that_draws_it() {
        assert!(tree::is_tab_kind("story"));
        assert_eq!(zone_of("story").map(Zone::id), Some("story"));
        assert_eq!(zone_of("story").map(Zone::look), Some(("book-open", "Story")));
        assert!(super::super::tabs::KINDS.iter().any(|one| one.0 == "story"), "+ lists Story");
    }

    /// With no Now, it goes where Now would: the end of the column's foot.
    #[test]
    fn with_no_now_artifacts_goes_to_the_columns_foot() {
        let arrangement = two_panes();
        let (_, _, foot) = column(&arrangement);
        let foot = foot.id.clone();
        let with_shell = put_terminal(&arrangement, "t1", Some(&foot), false);
        let now = tree::holder_of(&with_shell, "now").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "now")).map(|tab| tab.id.clone()).expect("now");
        let no_now = tree::close_tab(&with_shell, &now).into_owned();
        let offered = offer_beside_now(&no_now, "artifacts").into_owned();
        let pane = tree::pane_by_id(&offered, &foot).expect("the foot");
        assert_eq!(kinds(pane), ["terminal", "artifacts"]);
        assert_eq!(front_of(pane).map(|tab| tab.kind.as_str()), Some("terminal"));
    }

    /// A shell arrives in the column, or in the pane its + was pressed in; the same shell is not opened twice.
    #[test]
    fn a_shell_goes_where_it_was_asked_and_once() {
        let arrangement = two_panes();
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
        assert_eq!(kinds(tree::pane_by_id(&moved, &thread).expect("the thread's pane")), ["thread", "story", "terminal"]);
    }

    /// Split right, then ⌘Z: the arrangement is the one before the split, and ⇧⌘Z puts the split back.
    #[test]
    fn undo_takes_a_split_back_and_redo_puts_it_back() {
        let was = two_panes();
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
        let was = two_panes();
        let (_, _, foot) = column(&was);
        let split = |size: Option<Size>| tree::rearrange(&was, |arrangement| tree::split_pane(arrangement, &foot.id, SplitInto::Right), size, false);
        assert!(split(Some(Size { w: 1400., h: 900. })).is_some());
        assert!(split(Some(Size { w: 500., h: 900. })).is_none());
    }

    /// The thread's only tab closed takes its pane; opening it again makes a pane where it stood, beside the column, in front.
    #[test]
    fn a_closed_thread_comes_back_where_it_stood() {
        let default = two_panes();
        let was = tree::close_tab(&default, &story_tab(&default)).into_owned();
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
        let was = two_panes();
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
        let arrangement = two_panes();
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
        let was = two_panes();
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

    /// A Job kept with For you, Open, Done and Deferred as tabs loses them once, since the band is their home: the pane they
    /// filled goes, the thread, Story and Now stay, and the arrangement says it ran.
    #[test]
    fn a_kept_column_of_the_four_tabs_loses_them_once() {
        let kept = two_panes();
        let left = leave_bands(&kept).into_owned();
        for gone in BAND_TABS {
            assert!(!tree::has_kind(&left, gone), "{gone} is the band's now");
        }
        let panes = tree::panes_of(&left.root);
        assert_eq!(panes.iter().map(|pane| kinds(pane)).collect::<Vec<_>>(), [vec!["thread", "story"], vec!["now"]], "the emptied pane is dropped, the rest stays");
        assert!(left.offered.iter().any(|one| one == "bands"));
        assert!(tree::pane_by_id(&left, &left.focus).is_some(), "the focus is on a pane that is there");
        assert!(matches!(leave_bands(&left), Cow::Borrowed(_)), "it ran once, so it changes nothing again");
    }

    /// One he opens from + afterwards stays: the arrangement was marked when the four went.
    #[test]
    fn a_tab_opened_again_from_plus_is_not_taken_out_again() {
        let left = leave_bands(&two_panes()).into_owned();
        let foot = tree::holder_of(&left, "now").map(|pane| pane.id.clone()).expect("Now stays");
        let again = tree::add_tab(&left, &foot, "open", Extras::default()).into_owned();
        assert!(matches!(leave_bands(&again), Cow::Borrowed(_)));
        assert!(tree::has_kind(&again, "open"));
    }

    /// A pane that holds his own tabs beside one of the four keeps those, and keeps its place.
    #[test]
    fn a_pane_with_his_own_tabs_beside_the_four_keeps_them() {
        let kept = two_panes();
        let (_, top, _) = column(&kept);
        let top = top.id.clone();
        let kept = tree::add_tab(&kept, &top, "files", Extras::default()).into_owned();
        let left = leave_bands(&kept).into_owned();
        let panes = tree::panes_of(&left.root);
        assert_eq!(panes.iter().map(|pane| kinds(pane)).collect::<Vec<_>>(), [vec!["thread", "story"], vec!["files"], vec!["now"]]);
    }

    fn column_kinds(arrangement: &Arrangement) -> Vec<String> {
        tree::holder_of(arrangement, "artifacts").or_else(|| tree::holder_of(arrangement, "now")).map(|pane| pane.tabs.iter().map(|tab| tab.kind.clone()).collect()).unwrap_or_default()
    }

    /// The column this branch's earlier default opened on, Now alone, has Now taken out for the three the Job produces.
    #[test]
    fn a_kept_column_of_now_alone_becomes_what_the_job_produced() {
        let was = leave_bands(&two_panes()).into_owned();
        assert_eq!(column_kinds(&was), ["now"]);
        let made = leave_now(&was).into_owned();
        assert_eq!(column_kinds(&made), ["artifacts", "pulls", "issues"]);
        assert!(!tree::has_kind(&made, "now"));
        let (thread, column) = (tree::holder_of(&made, "thread").expect("the thread"), tree::holder_of(&made, "artifacts").expect("the column"));
        assert_eq!(front_of(column).map(|tab| tab.kind.as_str()), Some("artifacts"), "Now was in front, so Artifacts is");
        assert_eq!(kinds(thread), ["thread", "story"]);
        assert!(made.offered.iter().any(|one| one == "made") && made.offered.iter().any(|one| one == "artifacts"));
        assert!(matches!(leave_now(&made), Cow::Borrowed(_)), "it ran once");
    }

    /// Now with Artifacts, which the first artifact put there, keeps that Artifacts tab, first.
    #[test]
    fn a_kept_column_of_now_with_artifacts_keeps_the_artifacts_tab_and_loses_now() {
        let was = leave_bands(&two_panes()).into_owned();
        let was = offer_beside_now(&was, "artifacts").into_owned();
        assert_eq!(column_kinds(&was), ["now", "artifacts"]);
        let artifacts = tree::holder_of(&was, "artifacts").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "artifacts")).map(|tab| tab.id.clone()).expect("the tab");
        let made = leave_now(&was).into_owned();
        assert_eq!(column_kinds(&made), ["artifacts", "pulls", "issues"]);
        assert_eq!(tree::holder_of(&made, "artifacts").and_then(|pane| pane.tabs.first()).map(|tab| tab.id.clone()), Some(artifacts), "the same tab");
        assert_eq!(made.offered.iter().filter(|one| *one == "artifacts").count(), 1);
    }

    /// A tab he opened himself stays, behind the three.
    #[test]
    fn a_tab_he_opened_in_the_column_stays_behind_the_three() {
        let was = leave_bands(&two_panes()).into_owned();
        let column = tree::holder_of(&was, "now").map(|pane| pane.id.clone()).expect("the column");
        let was = tree::add_tab(&was, &column, "files", Extras::default()).into_owned();
        let made = leave_now(&was).into_owned();
        assert_eq!(column_kinds(&made), ["artifacts", "pulls", "issues", "files"]);
    }

    /// A layout he arranged differently is left, and marked so it is not looked at again; Now opened again stays.
    #[test]
    fn a_column_he_arranged_otherwise_is_left_alone() {
        // Now in a pane of its own below others: not the column's one pane.
        let split = two_panes();
        let made = leave_now(&split).into_owned();
        assert_eq!(made.root, split.root, "the old two-pane column is left as it is");
        assert!(made.offered.iter().any(|one| one == "made"));
        // Now closed: nothing to take out.
        let was = leave_bands(&two_panes()).into_owned();
        let now = tree::holder_of(&was, "now").and_then(|pane| pane.tabs.first()).map(|tab| tab.id.clone()).expect("Now");
        let kept = tree::add_tab(&was, &tree::holder_of(&was, "now").expect("the column").id, "files", Extras::default()).into_owned();
        let kept = tree::close_tab(&kept, &now).into_owned();
        assert_eq!(leave_now(&kept).root, kept.root);
        // Opened again from +, after the move: it stays.
        let made = leave_now(&was).into_owned();
        let column = tree::holder_of(&made, "artifacts").map(|pane| pane.id.clone()).expect("the column");
        let again = tree::add_tab(&made, &column, "now", Extras::default()).into_owned();
        assert!(matches!(leave_now(&again), Cow::Borrowed(_)));
    }

    /// The first Job a Job starts opens its Child Jobs tab at the end of the column, once.
    #[test]
    fn the_first_child_job_opens_child_jobs_after_the_three_once() {
        let was = tree::default_arrangement();
        let opened = offer_children_tab(&was).into_owned();
        assert_eq!(column_kinds(&opened), ["artifacts", "pulls", "issues", "children"]);
        let first = tree::holder_of(&opened, "artifacts").and_then(front_of).map(|tab| tab.kind.clone());
        assert_eq!(first.as_deref(), Some("artifacts"), "what he had in front stays");
        assert!(matches!(offer_children_tab(&opened), Cow::Borrowed(_)));
        let tab = tree::holder_of(&opened, "children").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "children")).map(|tab| tab.id.clone()).expect("the tab");
        let closed = tree::close_tab(&opened, &tab).into_owned();
        assert!(matches!(offer_children_tab(&closed), Cow::Borrowed(_)) && !tree::has_kind(&closed, "children"), "closed, it stays closed");
    }
}
