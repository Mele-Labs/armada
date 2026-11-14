//! The open Job's area as a tree of panes, ported from Classic's `job-panes/tree.ts` and the undo,
//! minimum-size and drop rules in its `store.ts`. A split holds two nodes side by side (`row`) or one
//! over the other (`column`) at a ratio; a pane holds tabs, one in front. Every operation is pure:
//! an arrangement in, the next one out.
//!
//! **A change that does nothing hands back its own input** (`Cow::Borrowed`), as Classic returns the
//! same object, because the store's `next !== was` reads that. Everything else is `Cow::Owned`.
//!
//! The serde shape is Classic's own JSON: `{ root, focus, next, threadWas? }`, nodes tagged by
//! `type`, a tab's kind its string name, and an absent field never written as `null`.
#![allow(dead_code)] // The drag, layout and kept-shape parts are drawn by the pane PRs after this one.


use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// What an operation hands back: the input itself when nothing changed, else a new arrangement.
pub(crate) type Out<'a> = Cow<'a, Arrangement>;

fn same(arrangement: &Arrangement) -> Out<'_> {
    Cow::Borrowed(arrangement)
}

/// Kinds a Job's tab can be (Classic's `JOB_TAB_KINDS`).
pub(crate) const JOB_TAB_KINDS: [&str; 27] = [
    "thread", "browser", "terminal", "done", "now", "open", "deferred", "plan", "changes", "pulls", "issues", "jobs", "ledger", "files", "request",
    "subagents", "checks", "workflow", "system", "bays", "spine", "judge", "custom", "draft", "walk", "document", "subagent",
];
/// A pull request's tabs on the Reviews page (Classic's `PR_TAB_KINDS`).
pub(crate) const PR_TAB_KINDS: [&str; 8] = ["pr-overview", "pr-files", "pr-checks", "pr-commits", "pr-review", "pr-tool", "pr-comments", "pr-ask"];
/// Kinds a kept layout may still name, and the kind each opens now (Classic's `RENAMED_PR_KINDS`).
pub(crate) const RENAMED_PR_KINDS: [(&str, &str); 1] = [("pr-focus", "pr-review")];

/// Whether this is a kind Classic's `TAB_KINDS` names, a Job's or a pull request's.
pub(crate) fn is_tab_kind(kind: &str) -> bool {
    JOB_TAB_KINDS.contains(&kind) || PR_TAB_KINDS.contains(&kind)
}

/// The least a pane keeps, in px: wide enough for its strip (Classic's `MIN_PANE_W`).
pub(crate) const MIN_PANE_W: f64 = 160.0;
/// The least a pane keeps, in px: tall enough for a few lines (Classic's `MIN_PANE_H`).
pub(crate) const MIN_PANE_H: f64 = 120.0;
/// The least a side of a split keeps, as a share of the split (Classic's `RATIO_MIN`).
pub(crate) const RATIO_MIN: f64 = 0.12;

/// The most steps back the undo key keeps (Classic's `HISTORY`).
pub(crate) const HISTORY: usize = 50;
/// A run of one kind of change inside this many ms is one step back (Classic's `MERGE_MS`).
pub(crate) const MERGE_MS: u64 = 600;

fn clamp_ratio(ratio: f64) -> f64 {
    (1.0 - RATIO_MIN).min(RATIO_MIN.max(ratio))
}

// ---- Types ------------------------------------------------------------------------------------------

/// Which way a split lays its two sides: `row` side by side, `column` one over the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Dir {
    Row,
    Column,
}

/// A tab (Classic's `Tab`). `label` reads where the kind alone would not tell two apart; `page` names the
/// page a browser tab shows; `mod_` is a Custom tab's mod by folder name; `row` a Draft tab's card;
/// `terminal` a Terminal tab's shell by the ID Fleet minted for it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Tab {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    #[serde(default, rename = "mod", skip_serializing_if = "Option::is_none")]
    pub mod_: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
}

/// The fields a new tab can carry beside its id and kind, as `addTab` and `arrive` take them.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Extras {
    pub label: Option<String>,
    pub page: Option<String>,
    pub mod_: Option<String>,
    pub row: Option<String>,
    pub terminal: Option<String>,
}

impl Extras {
    /// A tab that reads `label`.
    pub fn label(label: &str) -> Self {
        Extras { label: Some(label.to_string()), ..Extras::default() }
    }

    /// A tab for the shell Fleet minted `terminal` for.
    pub fn terminal(terminal: Option<&str>) -> Self {
        Extras { terminal: terminal.map(str::to_string), ..Extras::default() }
    }
}

impl Tab {
    /// A tab with only the fields it has (Classic's `tabOf`).
    fn of(id: String, kind: &str, extras: Extras) -> Tab {
        Tab { id, kind: kind.to_string(), label: extras.label, page: extras.page, mod_: extras.mod_, row: extras.row, terminal: extras.terminal }
    }
}

/// Tabs, one of them in front (Classic's `Pane`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Pane {
    pub id: String,
    pub tabs: Vec<Tab>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
}

/// Two nodes at a ratio (Classic's `Split`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Split {
    pub id: String,
    pub dir: Dir,
    pub ratio: f64,
    pub a: Box<Node>,
    pub b: Box<Node>,
}

/// A pane or a split (Classic's `Node`), tagged `type` as Classic writes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum Node {
    Pane(Pane),
    Split(Split),
}

impl Node {
    /// The node's id, a pane's or a split's.
    pub fn id(&self) -> &str {
        match self {
            Node::Pane(pane) => &pane.id,
            Node::Split(split) => &split.id,
        }
    }
}

/// Where a closed thread's pane met the node beside it, for a pane that went with it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Beside {
    pub node: String,
    pub dir: Dir,
    pub ratio: f64,
    pub first: bool,
}

/// Where the thread stood when it was closed (Classic's `ThreadPlace`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ThreadPlace {
    pub pane: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beside: Option<Beside>,
}

/// The tree, the focused pane and the counter ids come from (Classic's `Arrangement`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Arrangement {
    pub root: Node,
    /// The pane keys and the menus act on.
    pub focus: String,
    /// The next number for an id.
    pub next: u64,
    /// Absent: the thread has not been closed.
    #[serde(default, rename = "threadWas", skip_serializing_if = "Option::is_none")]
    pub thread_was: Option<ThreadPlace>,
}

/// A box as shares of the whole, or in px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// An area's size in px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Size {
    pub w: f64,
    pub h: f64,
}

/// A side of a pane or of the whole area (Classic's `Side`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    fn first(self) -> bool {
        matches!(self, Side::Left | Side::Top)
    }

    fn dir(self) -> Dir {
        if matches!(self, Side::Left | Side::Right) {
            Dir::Row
        } else {
            Dir::Column
        }
    }
}

/// A direction between panes (Classic's `Toward`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Toward {
    Left,
    Right,
    Up,
    Down,
}

/// Where a pane splits to: its right, or below it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SplitInto {
    Right,
    Down,
}

/// Where a dragged tab is over a pane: its middle, or an edge (Classic's `Side | "middle"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Zone {
    Middle,
    Edge(Side),
}

// ---- Reading ----------------------------------------------------------------------------------------

/// The least a node needs along an axis (Classic's `minExtent`).
pub(crate) fn min_extent(node: &Node, axis: Dir) -> f64 {
    match node {
        Node::Pane(_) => match axis {
            Dir::Row => MIN_PANE_W,
            Dir::Column => MIN_PANE_H,
        },
        Node::Split(split) => {
            let (a, b) = (min_extent(&split.a, axis), min_extent(&split.b, axis));
            if split.dir == axis {
                a + b
            } else {
                a.max(b)
            }
        }
    }
}

/// How far the panes fall short of their minimums in an area of this size, in px (Classic's `shortfall`).
pub(crate) fn shortfall(arrangement: &Arrangement, size: Size) -> f64 {
    rects_of(&arrangement.root)
        .panes
        .iter()
        .map(|(_, rect)| (MIN_PANE_W - rect.w * size.w - 0.5).max(0.0) + (MIN_PANE_H - rect.h * size.h - 0.5).max(0.0))
        .sum()
}

/// Every pane in reading order (Classic's `panesOf`).
pub(crate) fn panes_of(node: &Node) -> Vec<&Pane> {
    fn collect<'a>(node: &'a Node, out: &mut Vec<&'a Pane>) {
        match node {
            Node::Pane(pane) => out.push(pane),
            Node::Split(split) => {
                collect(&split.a, out);
                collect(&split.b, out);
            }
        }
    }
    let mut out = Vec::new();
    collect(node, &mut out);
    out
}

/// The pane with this id (Classic's `paneById`).
pub(crate) fn pane_by_id<'a>(arrangement: &'a Arrangement, id: &str) -> Option<&'a Pane> {
    panes_of(&arrangement.root).into_iter().find(|one| one.id == id)
}

/// The pane that holds this tab (Classic's `paneOfTab`).
pub(crate) fn pane_of_tab<'a>(arrangement: &'a Arrangement, tab_id: &str) -> Option<&'a Pane> {
    panes_of(&arrangement.root).into_iter().find(|one| one.tabs.iter().any(|tab| tab.id == tab_id))
}

/// The tab with this id (Classic's `tabById`).
pub(crate) fn tab_by_id<'a>(arrangement: &'a Arrangement, tab_id: &str) -> Option<&'a Tab> {
    panes_of(&arrangement.root).into_iter().flat_map(|one| one.tabs.iter()).find(|tab| tab.id == tab_id)
}

/// The pane that holds a tab of this kind, the focused one first (Classic's `holderOf`).
pub(crate) fn holder_of<'a>(arrangement: &'a Arrangement, kind: &str) -> Option<&'a Pane> {
    let holding: Vec<&Pane> = panes_of(&arrangement.root).into_iter().filter(|one| one.tabs.iter().any(|tab| tab.kind == kind)).collect();
    holding.iter().copied().find(|one| one.id == arrangement.focus).or_else(|| holding.first().copied())
}

/// Whether a pane holds a tab of this kind (Classic's `hasKind`).
pub(crate) fn has_kind(arrangement: &Arrangement, kind: &str) -> bool {
    holder_of(arrangement, kind).is_some()
}

/// One split's box and its two sides, for the handle on the line between them.
#[derive(Debug)]
pub(crate) struct SplitRect<'a> {
    pub id: &'a str,
    pub dir: Dir,
    pub ratio: f64,
    pub rect: Rect,
    pub a: &'a Node,
    pub b: &'a Node,
}

/// Each pane's box and each split's, in tree order (Classic's `rectsOf` result).
#[derive(Debug)]
pub(crate) struct Rects<'a> {
    pub panes: Vec<(&'a str, Rect)>,
    pub splits: Vec<SplitRect<'a>>,
}

impl Rects<'_> {
    /// One pane's box.
    pub fn pane(&self, id: &str) -> Option<Rect> {
        self.panes.iter().find(|(one, _)| *one == id).map(|(_, rect)| *rect)
    }
}

/// Each pane's box as shares of the whole (Classic's `rectsOf`).
pub(crate) fn rects_of(root: &Node) -> Rects<'_> {
    rects_within(root, Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 })
}

/// Each pane's box inside `within` (Classic's `rectsOf` with its second argument).
pub(crate) fn rects_within(root: &Node, within: Rect) -> Rects<'_> {
    fn walk<'a>(node: &'a Node, rect: Rect, out: &mut Rects<'a>) {
        match node {
            Node::Pane(pane) => out.panes.push((&pane.id, rect)),
            Node::Split(split) => {
                out.splits.push(SplitRect { id: &split.id, dir: split.dir, ratio: split.ratio, rect, a: &split.a, b: &split.b });
                if split.dir == Dir::Row {
                    let first = rect.w * split.ratio;
                    walk(&split.a, Rect { w: first, ..rect }, out);
                    walk(&split.b, Rect { x: rect.x + first, w: rect.w - first, ..rect }, out);
                } else {
                    let first = rect.h * split.ratio;
                    walk(&split.a, Rect { h: first, ..rect }, out);
                    walk(&split.b, Rect { y: rect.y + first, h: rect.h - first, ..rect }, out);
                }
            }
        }
    }
    let mut out = Rects { panes: Vec::new(), splits: Vec::new() };
    walk(root, within, &mut out);
    out
}

/// The pane next to this one in a direction: the nearest that shares some of its edge (Classic's `neighbourOf`).
pub(crate) fn neighbour_of(arrangement: &Arrangement, pane_id: &str, toward: Toward) -> Option<String> {
    let rects = rects_of(&arrangement.root);
    let from = rects.pane(pane_id)?;
    let eps = 1e-6;
    let overlap = |a0: f64, a1: f64, b0: f64, b1: f64| a1.min(b1) - a0.max(b0);
    // The id, its gap and how much of the edge it shares.
    let mut best: Option<(&str, f64, f64)> = None;
    for (id, rect) in &rects.panes {
        if *id == pane_id {
            continue;
        }
        let gap = match toward {
            Toward::Left => from.x - (rect.x + rect.w),
            Toward::Right => rect.x - (from.x + from.w),
            Toward::Up => from.y - (rect.y + rect.h),
            Toward::Down => rect.y - (from.y + from.h),
        };
        let shared = match toward {
            Toward::Left | Toward::Right => overlap(from.y, from.y + from.h, rect.y, rect.y + rect.h),
            Toward::Up | Toward::Down => overlap(from.x, from.x + from.w, rect.x, rect.x + rect.w),
        };
        if gap < -eps || shared <= eps {
            continue;
        }
        let better = match best {
            None => true,
            Some((_, best_gap, best_shared)) => gap < best_gap - eps || ((gap - best_gap).abs() <= eps && shared > best_shared),
        };
        if better {
            best = Some((id, gap, shared));
        }
    }
    best.map(|(id, _, _)| id.to_string())
}

// ---- Building ---------------------------------------------------------------------------------------

/// A new id from the arrangement's own counter (Classic's `mint`), moving the counter on.
fn mint(arrangement: &mut Arrangement, prefix: char) -> String {
    let id = format!("{prefix}{}", arrangement.next);
    arrangement.next += 1;
    id
}

/// A plain shape for `build`: a list of kinds is a pane.
#[derive(Clone, Debug)]
pub(crate) enum Shape {
    Pane(Vec<String>),
    Split { dir: Dir, ratio: f64, a: Box<Shape>, b: Box<Shape> },
}

impl Shape {
    /// A pane of these kinds, in order.
    pub fn pane(kinds: &[&str]) -> Shape {
        Shape::Pane(kinds.iter().map(|kind| kind.to_string()).collect())
    }

    /// Two shapes at a ratio.
    pub fn split(dir: Dir, ratio: f64, a: Shape, b: Shape) -> Shape {
        Shape::Split { dir, ratio, a: Box::new(a), b: Box::new(b) }
    }
}

/// An arrangement from a plain shape, `front` the kind shown in each pane that has one (Classic's `build`).
pub(crate) fn build(shape: &Shape, front: Option<&str>, labels: &[(&str, &str)]) -> Arrangement {
    fn make(at: &mut Arrangement, shape: &Shape, front: Option<&str>, labels: &[(&str, &str)]) -> Node {
        match shape {
            Shape::Pane(kinds) => {
                let id = mint(at, 'p');
                let mut tabs = Vec::new();
                for kind in kinds {
                    let tab_id = mint(at, 't');
                    let label = labels.iter().find(|(one, _)| one == kind).map(|(_, label)| label.to_string());
                    tabs.push(Tab::of(tab_id, kind, Extras { label, ..Extras::default() }));
                }
                let active = front.and_then(|front| tabs.iter().find(|tab| tab.kind == front)).or_else(|| tabs.first()).map(|tab| tab.id.clone());
                Node::Pane(Pane { id, tabs, active })
            }
            Shape::Split { dir, ratio, a, b } => {
                let id = mint(at, 's');
                let a = make(at, a, front, labels);
                let b = make(at, b, front, labels);
                Node::Split(Split { id, dir: *dir, ratio: clamp_ratio(*ratio), a: Box::new(a), b: Box::new(b) })
            }
        }
    }
    let mut at = Arrangement { root: Node::Pane(Pane { id: "p0".into(), tabs: Vec::new(), active: None }), focus: "p0".into(), next: 1, thread_was: None };
    let root = make(&mut at, shape, front, labels);
    at.focus = panes_of(&root).first().map(|pane| pane.id.clone()).unwrap_or_default();
    at.root = root;
    at
}

/// Today's Job arrangement: the thread, and to its right Open, Done and Deferred as tabs of one pane over Now (Classic's `defaultArrangement`).
pub(crate) fn default_arrangement() -> Arrangement {
    build(
        &Shape::split(
            Dir::Row,
            0.62,
            Shape::pane(&["thread"]),
            Shape::split(Dir::Column, 0.4, Shape::pane(&["open", "done", "deferred"]), Shape::pane(&["now"])),
        ),
        None,
        &[],
    )
}

// ---- Changing ---------------------------------------------------------------------------------------

/// The node with the pane `id` replaced by what `change` makes of it (Classic's `mapPane`).
fn map_pane(node: &Node, id: &str, change: &dyn Fn(&Pane) -> Node) -> Node {
    match node {
        Node::Pane(pane) if pane.id == id => change(pane),
        Node::Pane(_) => node.clone(),
        Node::Split(split) => Node::Split(Split {
            id: split.id.clone(),
            dir: split.dir,
            ratio: split.ratio,
            a: Box::new(map_pane(&split.a, id, change)),
            b: Box::new(map_pane(&split.b, id, change)),
        }),
    }
}

/// The pane `id` with `change` done to a copy of it, in place in the node.
fn edit_pane(node: &Node, id: &str, change: &dyn Fn(&mut Pane)) -> Node {
    map_pane(node, id, &|one| {
        let mut pane = one.clone();
        change(&mut pane);
        Node::Pane(pane)
    })
}

/// A node without a pane: the sibling takes the split's place. None when the pane was the whole tree (Classic's `without`).
fn without(node: &Node, id: &str) -> Option<Node> {
    match node {
        Node::Pane(pane) => (pane.id != id).then(|| node.clone()),
        Node::Split(split) => match (without(&split.a, id), without(&split.b, id)) {
            (None, b) => b,
            (a, None) => a,
            (Some(a), Some(b)) => Some(Node::Split(Split { id: split.id.clone(), dir: split.dir, ratio: split.ratio, a: Box::new(a), b: Box::new(b) })),
        },
    }
}

/// Where the focus goes when its pane is gone: the first pane of what is left (Classic's `refocus`).
fn refocus(mut arrangement: Arrangement) -> Arrangement {
    if pane_by_id(&arrangement, &arrangement.focus).is_none() {
        arrangement.focus = panes_of(&arrangement.root).first().map(|pane| pane.id.clone()).unwrap_or_default();
    }
    arrangement
}

/// The pane takes the focus (Classic's `focusPane`).
pub(crate) fn focus_pane<'a>(arrangement: &'a Arrangement, pane_id: &str) -> Out<'a> {
    if pane_by_id(arrangement, pane_id).is_none() || arrangement.focus == pane_id {
        return same(arrangement);
    }
    Cow::Owned(Arrangement { focus: pane_id.to_string(), ..arrangement.clone() })
}

/// The tab comes to the front of its pane and the pane takes the focus (Classic's `activate`).
pub(crate) fn activate<'a>(arrangement: &'a Arrangement, tab_id: &str) -> Out<'a> {
    let Some(pane) = pane_of_tab(arrangement, tab_id) else { return same(arrangement) };
    let mut next = arrangement.clone();
    next.focus = pane.id.clone();
    next.root = edit_pane(&arrangement.root, &pane.id, &|one| one.active = Some(tab_id.to_string()));
    Cow::Owned(next)
}

/// A tab already open shows another page, and comes to the front (Classic's `showPage`).
pub(crate) fn show_page<'a>(arrangement: &'a Arrangement, tab_id: &str, page: &str) -> Out<'a> {
    let Some(pane) = pane_of_tab(arrangement, tab_id) else { return same(arrangement) };
    let mut shown = arrangement.clone();
    shown.root = edit_pane(&arrangement.root, &pane.id, &|one| {
        for tab in one.tabs.iter_mut().filter(|tab| tab.id == tab_id) {
            tab.page = Some(page.to_string());
        }
    });
    Cow::Owned(activate(&shown, tab_id).into_owned())
}

/// A tab for a shell the Job opened itself joins the pane that holds Terminal tabs, behind the one in front and without taking the focus,
/// or opens as a split on the right of everything (Classic's `arriveTerminalBehind`).
pub(crate) fn arrive_terminal_behind(arrangement: &Arrangement, terminal: Option<&str>) -> Arrangement {
    let holder = holder_of(arrangement, "terminal").map(|pane| (pane.id.clone(), pane.active.clone()));
    let mut arrived = arrive(arrangement, "terminal", Extras::terminal(terminal));
    arrived.focus = arrangement.focus.clone();
    if let Some((id, active)) = holder {
        arrived.root = edit_pane(&arrived.root, &id, &|one| one.active = active.clone());
    }
    arrived
}

/// A terminal tab reads this name, or its default again where `label` is empty (Classic's `renameTab`).
pub(crate) fn rename_tab<'a>(arrangement: &'a Arrangement, tab_id: &str, label: &str) -> Out<'a> {
    let named = label.trim();
    let Some(pane) = pane_of_tab(arrangement, tab_id) else { return same(arrangement) };
    if tab_by_id(arrangement, tab_id).and_then(|tab| tab.label.as_deref()).unwrap_or("") == named {
        return same(arrangement);
    }
    let mut next = arrangement.clone();
    next.root = edit_pane(&arrangement.root, &pane.id, &|one| {
        for tab in one.tabs.iter_mut().filter(|tab| tab.id == tab_id) {
            tab.label = if named.is_empty() { None } else { Some(named.to_string()) };
        }
    });
    Cow::Owned(next)
}

/// A Custom tab's mod, kept on the tab so it comes back bound to its mod after a restart (Classic's `markTabMod`).
pub(crate) fn mark_tab_mod<'a>(arrangement: &'a Arrangement, tab_id: &str, mod_: &str) -> Out<'a> {
    let Some(pane) = pane_of_tab(arrangement, tab_id) else { return same(arrangement) };
    if tab_by_id(arrangement, tab_id).and_then(|tab| tab.mod_.as_deref()) == Some(mod_) {
        return same(arrangement);
    }
    let mut next = arrangement.clone();
    next.root = edit_pane(&arrangement.root, &pane.id, &|one| {
        for tab in one.tabs.iter_mut().filter(|tab| tab.id == tab_id) {
            tab.mod_ = Some(mod_.to_string());
        }
    });
    Cow::Owned(next)
}

/// A label no other tab has: the same host or shell a second time is numbered (Classic's `distinct`).
fn distinct(arrangement: &Arrangement, label: Option<&str>) -> Option<String> {
    let label = label?;
    let taken: HashSet<&str> = panes_of(&arrangement.root).into_iter().flat_map(|one| one.tabs.iter()).filter_map(|tab| tab.label.as_deref()).collect();
    let mut at = 1;
    let named = |at: u32| if at == 1 { label.to_string() } else { format!("{label} {at}") };
    while taken.contains(named(at).as_str()) {
        at += 1;
    }
    Some(named(at))
}

/// A pane holding one new tab of `kind`, under the whole arrangement and as wide as it, taking `share` of its height.
/// The focus stays where it was (Classic's `dockAtBottom`).
pub(crate) fn dock_at_bottom(arrangement: &Arrangement, kind: &str, share: f64) -> Arrangement {
    let mut next = arrangement.clone();
    let split_id = mint(&mut next, 's');
    let pane_id = mint(&mut next, 'p');
    let tab_id = mint(&mut next, 't');
    next.root = Node::Split(Split {
        id: split_id,
        dir: Dir::Column,
        ratio: clamp_ratio(1.0 - share),
        a: Box::new(arrangement.root.clone()),
        b: Box::new(Node::Pane(Pane { id: pane_id, tabs: vec![Tab::of(tab_id.clone(), kind, Extras::default())], active: Some(tab_id) })),
    });
    next
}

/// Each tab of a kind `renamed` names, as the kind it opens now. Where that leaves two tabs of one kind, the first in the tree
/// stays and the other goes, its pane fronting the one that stayed or its first tab (Classic's `renameKinds`).
pub(crate) fn rename_kinds(arrangement: &Arrangement, renamed: &[(&str, &str)]) -> Arrangement {
    let now = |kind: &str| renamed.iter().find(|(from, _)| *from == kind).map(|(_, to)| to.to_string()).unwrap_or_else(|| kind.to_string());
    let onto: HashSet<&str> = renamed.iter().map(|(_, to)| *to).collect();
    let mut seen: HashMap<String, String> = HashMap::new();
    fn walk(node: &Node, now: &dyn Fn(&str) -> String, onto: &HashSet<&str>, seen: &mut HashMap<String, String>) -> Node {
        let pane = match node {
            Node::Split(split) => {
                let a = walk(&split.a, now, onto, seen);
                let b = walk(&split.b, now, onto, seen);
                return Node::Split(Split { id: split.id.clone(), dir: split.dir, ratio: split.ratio, a: Box::new(a), b: Box::new(b) });
            }
            Node::Pane(pane) => pane,
        };
        let mut tabs: Vec<Tab> = Vec::new();
        for tab in &pane.tabs {
            let kind = now(&tab.kind);
            if onto.contains(kind.as_str()) && seen.contains_key(&kind) {
                continue;
            }
            if onto.contains(kind.as_str()) {
                seen.insert(kind.clone(), tab.id.clone());
            }
            tabs.push(if kind == tab.kind { tab.clone() } else { Tab { kind, ..tab.clone() } });
        }
        let has = |tabs: &[Tab], id: &str| tabs.iter().any(|tab| tab.id == id);
        let gone = pane.active.as_deref().is_some_and(|active| !has(&tabs, active));
        let kept = if gone { pane.tabs.iter().find(|tab| Some(&tab.id) == pane.active.as_ref()) } else { None };
        let instead = kept.and_then(|kept| seen.get(&now(&kept.kind)).cloned());
        let active = if !gone {
            pane.active.clone()
        } else if instead.as_deref().is_some_and(|id| has(&tabs, id)) {
            instead
        } else {
            tabs.first().map(|tab| tab.id.clone())
        };
        Node::Pane(Pane { id: pane.id.clone(), tabs, active })
    }
    let root = walk(&arrangement.root, &now, &onto, &mut seen);
    Arrangement { root, ..arrangement.clone() }
}

/// A new tab at the end of a pane, in front (Classic's `addTab`).
pub(crate) fn add_tab<'a>(arrangement: &'a Arrangement, pane_id: &str, kind: &str, extras: Extras) -> Out<'a> {
    if pane_by_id(arrangement, pane_id).is_none() {
        return same(arrangement);
    }
    let mut next = arrangement.clone();
    let id = mint(&mut next, 't');
    let label = distinct(arrangement, extras.label.as_deref());
    let made = Tab::of(id.clone(), kind, Extras { label, ..extras });
    next.focus = pane_id.to_string();
    next.root = edit_pane(&next.root, pane_id, &|one| {
        one.tabs.push(made.clone());
        one.active = Some(id.clone());
    });
    Cow::Owned(next)
}

/// Which tab comes to the front when this one leaves a pane: the one after it, else the one before (Classic's `afterLeaving`).
fn after_leaving(pane: &Pane, tab_id: &str) -> Option<String> {
    let at = pane.tabs.iter().position(|tab| tab.id == tab_id)?;
    let rest: Vec<&Tab> = pane.tabs.iter().filter(|tab| tab.id != tab_id).collect();
    if pane.active.as_deref() != Some(tab_id) {
        return pane.active.clone();
    }
    rest.get(at).or_else(|| at.checked_sub(1).and_then(|before| rest.get(before))).map(|tab| tab.id.clone())
}

/// The tab leaves its pane; a pane left with none is removed, and the last pane of all stays, empty (Classic's `detach`).
fn detach<'a>(arrangement: &'a Arrangement, tab_id: &str) -> Out<'a> {
    let Some(pane) = pane_of_tab(arrangement, tab_id) else { return same(arrangement) };
    let rest: Vec<Tab> = pane.tabs.iter().filter(|tab| tab.id != tab_id).cloned().collect();
    let mut next = arrangement.clone();
    if !rest.is_empty() {
        let active = after_leaving(pane, tab_id);
        next.root = edit_pane(&arrangement.root, &pane.id, &|one| {
            one.tabs = rest.clone();
            one.active = active.clone();
        });
        return Cow::Owned(next);
    }
    match without(&arrangement.root, &pane.id) {
        None => {
            next.root = Node::Pane(Pane { id: pane.id.clone(), tabs: Vec::new(), active: None });
            Cow::Owned(next)
        }
        Some(root) => {
            next.root = root;
            Cow::Owned(refocus(next))
        }
    }
}

/// Where a pane stands in the tree, for the thread to come back to (Classic's `placeOf`).
fn place_of(root: &Node, pane_id: &str) -> ThreadPlace {
    fn parent<'a>(node: &'a Node, id: &str) -> Option<&'a Split> {
        match node {
            Node::Pane(_) => None,
            Node::Split(split) if split.a.id() == id || split.b.id() == id => Some(split),
            Node::Split(split) => parent(&split.a, id).or_else(|| parent(&split.b, id)),
        }
    }
    let Some(split) = parent(root, pane_id) else { return ThreadPlace { pane: pane_id.to_string(), beside: None } };
    let first = split.a.id() == pane_id;
    let other = if first { &split.b } else { &split.a };
    ThreadPlace { pane: pane_id.to_string(), beside: Some(Beside { node: other.id().to_string(), dir: split.dir, ratio: split.ratio, first }) }
}

/// A tab closes; a thread's remembers where it stood (Classic's `closeTab`).
pub(crate) fn close_tab<'a>(arrangement: &'a Arrangement, tab_id: &str) -> Out<'a> {
    let pane = pane_of_tab(arrangement, tab_id);
    let closed = detach(arrangement, tab_id);
    match pane {
        Some(pane) if pane.tabs.iter().any(|tab| tab.id == tab_id && tab.kind == "thread") => {
            let mut closed = closed.into_owned();
            closed.thread_was = Some(place_of(&arrangement.root, &pane.id));
            Cow::Owned(closed)
        }
        _ => closed,
    }
}

/// The pane goes with its tabs, empty or not; the last pane of all stays (Classic's `closePane`).
pub(crate) fn close_pane<'a>(arrangement: &'a Arrangement, pane_id: &str) -> Out<'a> {
    let Some(pane) = pane_by_id(arrangement, pane_id) else { return same(arrangement) };
    let Some(root) = without(&arrangement.root, pane_id) else { return same(arrangement) };
    let mut next = arrangement.clone();
    next.root = root;
    if pane.tabs.iter().any(|tab| tab.kind == "thread") {
        next.thread_was = Some(place_of(&arrangement.root, pane_id));
    }
    Cow::Owned(refocus(next))
}

/// A closed thread back where it stood, in front of its pane, with the focus where it was: its pane if that is still there; a pane
/// that went with it, made again beside the node it met; else the focused pane. One that stands is left (Classic's `reopenThread`).
pub(crate) fn reopen_thread(arrangement: &Arrangement) -> Out<'_> {
    if has_kind(arrangement, "thread") {
        return same(arrangement);
    }
    let mut rest = arrangement.clone();
    let was = rest.thread_was.take();
    let focus = rest.focus.clone();
    let in_pane = |pane_id: &str, from: &Arrangement| {
        let mut next = add_tab(from, pane_id, "thread", Extras::default()).into_owned();
        next.focus = focus.clone();
        Cow::Owned(next)
    };
    if let Some(was) = &was {
        if pane_by_id(&rest, &was.pane).is_some() {
            return in_pane(&was.pane, &rest);
        }
    }
    fn stands(node: &Node, id: &str) -> bool {
        node.id() == id || matches!(node, Node::Split(split) if stands(&split.a, id) || stands(&split.b, id))
    }
    let beside = was.and_then(|was| was.beside);
    let Some(beside) = beside.filter(|beside| stands(&rest.root, &beside.node)) else { return in_pane(&focus, &rest) };
    let split_id = mint(&mut rest, 's');
    let fresh = mint(&mut rest, 'p');
    let made = Node::Pane(Pane { id: fresh.clone(), tabs: Vec::new(), active: None });
    fn wrap(node: &Node, beside: &Beside, split_id: &str, made: &Node) -> Node {
        if node.id() == beside.node {
            let (a, b) = if beside.first { (made.clone(), node.clone()) } else { (node.clone(), made.clone()) };
            return Node::Split(Split { id: split_id.to_string(), dir: beside.dir, ratio: beside.ratio, a: Box::new(a), b: Box::new(b) });
        }
        match node {
            Node::Pane(_) => node.clone(),
            Node::Split(split) => Node::Split(Split {
                id: split.id.clone(),
                dir: split.dir,
                ratio: split.ratio,
                a: Box::new(wrap(&split.a, beside, split_id, made)),
                b: Box::new(wrap(&split.b, beside, split_id, made)),
            }),
        }
    }
    rest.root = wrap(&rest.root, &beside, &split_id, &made);
    in_pane(&fresh, &rest)
}

/// The pane splits, and an empty pane takes the new side: to its right, or below it. Focus goes to the new pane (Classic's `splitPane`).
pub(crate) fn split_pane<'a>(arrangement: &'a Arrangement, pane_id: &str, into: SplitInto) -> Out<'a> {
    if pane_by_id(arrangement, pane_id).is_none() {
        return same(arrangement);
    }
    let mut next = arrangement.clone();
    let split_id = mint(&mut next, 's');
    let fresh = mint(&mut next, 'p');
    let empty = Node::Pane(Pane { id: fresh.clone(), tabs: Vec::new(), active: None });
    let dir = if into == SplitInto::Right { Dir::Row } else { Dir::Column };
    next.root = map_pane(&next.root, pane_id, &|one| {
        Node::Split(Split { id: split_id.clone(), dir, ratio: 0.5, a: Box::new(Node::Pane(one.clone())), b: Box::new(empty.clone()) })
    });
    next.focus = fresh;
    Cow::Owned(next)
}

/// The tab goes into a pane, at an index or at the end (Classic's `moveTabToPane`).
pub(crate) fn move_tab_to_pane<'a>(arrangement: &'a Arrangement, tab_id: &str, pane_id: &str, index: Option<usize>) -> Out<'a> {
    let (Some(from), Some(tab)) = (pane_of_tab(arrangement, tab_id), tab_by_id(arrangement, tab_id)) else { return same(arrangement) };
    if pane_by_id(arrangement, pane_id).is_none() {
        return same(arrangement);
    }
    let put = |tabs: &[Tab]| -> Vec<Tab> {
        let at = index.unwrap_or(tabs.len()).min(tabs.len());
        let mut out = tabs[..at].to_vec();
        out.push(tab.clone());
        out.extend_from_slice(&tabs[at..]);
        out
    };
    // Dropped back where it is: only its place among the pane's tabs can change.
    if from.id == pane_id {
        let others: Vec<Tab> = from.tabs.iter().filter(|one| one.id != tab_id).cloned().collect();
        let tabs = put(&others);
        let mut next = arrangement.clone();
        next.focus = pane_id.to_string();
        next.root = edit_pane(&arrangement.root, pane_id, &|one| {
            one.tabs = tabs.clone();
            one.active = Some(tab_id.to_string());
        });
        return Cow::Owned(next);
    }
    let left = detach(arrangement, tab_id).into_owned();
    let Some(target) = pane_by_id(&left, pane_id) else { return same(arrangement) };
    let tabs = put(&target.tabs);
    let mut next = left.clone();
    next.focus = pane_id.to_string();
    next.root = edit_pane(&left.root, pane_id, &|one| {
        one.tabs = tabs.clone();
        one.active = Some(tab_id.to_string());
    });
    Cow::Owned(next)
}

/// A pane holding only this tab, in front.
fn alone_with(tab: &Tab, id: String) -> Node {
    Node::Pane(Pane { id, tabs: vec![tab.clone()], active: Some(tab.id.clone()) })
}

/// The tab goes to a new pane on one side of a pane. A pane's only tab dropped on its own edge stays as it is (Classic's `moveTabToEdge`).
pub(crate) fn move_tab_to_edge<'a>(arrangement: &'a Arrangement, tab_id: &str, pane_id: &str, side: Side) -> Out<'a> {
    let (Some(from), Some(tab)) = (pane_of_tab(arrangement, tab_id), tab_by_id(arrangement, tab_id)) else { return same(arrangement) };
    if pane_by_id(arrangement, pane_id).is_none() || (from.id == pane_id && from.tabs.len() == 1) {
        return same(arrangement);
    }
    let mut next = detach(arrangement, tab_id).into_owned();
    let split_id = mint(&mut next, 's');
    let fresh = mint(&mut next, 'p');
    let made = alone_with(tab, fresh.clone());
    next.root = map_pane(&next.root, pane_id, &|one| {
        let one = Node::Pane(one.clone());
        let (a, b) = if side.first() { (made.clone(), one) } else { (one, made.clone()) };
        Node::Split(Split { id: split_id.clone(), dir: side.dir(), ratio: 0.5, a: Box::new(a), b: Box::new(b) })
    });
    next.focus = fresh;
    Cow::Owned(next)
}

/// The tab goes to a new pane in the gap between the two sides of a split, along the split's own axis, the split shared in thirds.
/// A pane's only tab dropped on a gap beside it stays as it is (Classic's `moveTabToGap`).
pub(crate) fn move_tab_to_gap<'a>(arrangement: &'a Arrangement, tab_id: &str, split_id: &str) -> Out<'a> {
    let (Some(from), Some(tab)) = (pane_of_tab(arrangement, tab_id), tab_by_id(arrangement, tab_id)) else { return same(arrangement) };
    let rects = rects_of(&arrangement.root);
    let Some(split) = rects.splits.iter().find(|one| one.id == split_id) else { return same(arrangement) };
    let alone = |node: &Node| matches!(node, Node::Pane(pane) if pane.id == from.id && from.tabs.len() == 1);
    if alone(split.a) || alone(split.b) {
        return same(arrangement);
    }
    let mut next = detach(arrangement, tab_id).into_owned();
    let inner_id = mint(&mut next, 's');
    let fresh = mint(&mut next, 'p');
    let made = alone_with(tab, fresh.clone());
    fn put(node: &Node, split_id: &str, inner_id: &str, made: &Node) -> Node {
        let Node::Split(split) = node else { return node.clone() };
        if split.id != split_id {
            return Node::Split(Split {
                id: split.id.clone(),
                dir: split.dir,
                ratio: split.ratio,
                a: Box::new(put(&split.a, split_id, inner_id, made)),
                b: Box::new(put(&split.b, split_id, inner_id, made)),
            });
        }
        let inner = Node::Split(Split { id: inner_id.to_string(), dir: split.dir, ratio: 0.5, a: Box::new(made.clone()), b: split.b.clone() });
        Node::Split(Split { id: split.id.clone(), dir: split.dir, ratio: 1.0 / 3.0, a: split.a.clone(), b: Box::new(inner) })
    }
    next.root = put(&next.root, split_id, &inner_id, &made);
    next.focus = fresh;
    Cow::Owned(next)
}

/// The panes in reading order with each split flattened into its own axis, so two layouts that read the same compare equal
/// whatever their ids and shares (Classic's `shapeOf`).
fn shape_of(node: &Node) -> String {
    let split = match node {
        Node::Pane(pane) => return format!("[{}]", pane.tabs.iter().map(|tab| tab.id.as_str()).collect::<Vec<_>>().join(",")),
        Node::Split(split) => split,
    };
    fn parts(node: &Node, dir: Dir, out: &mut Vec<String>) {
        match node {
            Node::Split(one) if one.dir == dir => {
                parts(&one.a, dir, out);
                parts(&one.b, dir, out);
            }
            _ => out.push(shape_of(node)),
        }
    }
    let mut out = Vec::new();
    parts(node, split.dir, &mut out);
    let dir = if split.dir == Dir::Row { "row" } else { "column" };
    format!("{dir}({})", out.join(" "))
}

/// The tab goes to a new pane along one edge of the whole arrangement, a new root split that gives it a third. A drop that leaves
/// the panes as they were is nothing (Classic's `moveTabToEnd`).
pub(crate) fn move_tab_to_end<'a>(arrangement: &'a Arrangement, tab_id: &str, side: Side) -> Out<'a> {
    let Some(tab) = tab_by_id(arrangement, tab_id) else { return same(arrangement) };
    let left = detach(arrangement, tab_id).into_owned();
    // The only tab of the only pane has nowhere to go.
    if matches!(&left.root, Node::Pane(pane) if pane.tabs.is_empty()) {
        return same(arrangement);
    }
    let mut next = left.clone();
    let split_id = mint(&mut next, 's');
    let fresh = mint(&mut next, 'p');
    let made = alone_with(tab, fresh.clone());
    let first = side.first();
    let root = Node::Split(Split {
        id: split_id,
        dir: side.dir(),
        ratio: if first { 1.0 / 3.0 } else { 2.0 / 3.0 },
        a: Box::new(if first { made.clone() } else { left.root.clone() }),
        b: Box::new(if first { left.root.clone() } else { made }),
    });
    if shape_of(&root) == shape_of(&arrangement.root) {
        return same(arrangement);
    }
    next.root = root;
    next.focus = fresh;
    Cow::Owned(next)
}

/// A split's share is set, held to the least each side keeps (Classic's `resizeSplit`).
pub(crate) fn resize_split(arrangement: &Arrangement, split_id: &str, ratio: f64) -> Arrangement {
    fn put(node: &Node, split_id: &str, ratio: f64) -> Node {
        match node {
            Node::Pane(_) => node.clone(),
            Node::Split(split) if split.id == split_id => Node::Split(Split { ratio: clamp_ratio(ratio), ..split.clone() }),
            Node::Split(split) => Node::Split(Split {
                id: split.id.clone(),
                dir: split.dir,
                ratio: split.ratio,
                a: Box::new(put(&split.a, split_id, ratio)),
                b: Box::new(put(&split.b, split_id, ratio)),
            }),
        }
    }
    Arrangement { root: put(&arrangement.root, split_id, ratio), ..arrangement.clone() }
}

/// The next (`by` 1) or the previous (`by` -1) tab of a pane comes to the front, wrapping round (Classic's `cycleTab`).
pub(crate) fn cycle_tab<'a>(arrangement: &'a Arrangement, pane_id: &str, by: isize) -> Out<'a> {
    let Some(pane) = pane_by_id(arrangement, pane_id).filter(|pane| pane.tabs.len() >= 2) else { return same(arrangement) };
    let at = pane.tabs.iter().position(|tab| Some(&tab.id) == pane.active.as_ref()).map_or(-1, |at| at as isize);
    let len = pane.tabs.len() as isize;
    let to = &pane.tabs[(at + by).rem_euclid(len) as usize];
    Cow::Owned(activate(arrangement, &to.id).into_owned())
}

/// A tab a Job opens itself joins the pane that holds that kind; with none, it opens as a split on the right of everything (Classic's `arrive`).
pub(crate) fn arrive(arrangement: &Arrangement, kind: &str, extras: Extras) -> Arrangement {
    if let Some(holder) = holder_of(arrangement, kind) {
        return add_tab(arrangement, &holder.id, kind, extras).into_owned();
    }
    let mut next = arrangement.clone();
    let split_id = mint(&mut next, 's');
    let fresh = mint(&mut next, 'p');
    let tab_id = mint(&mut next, 't');
    let label = distinct(arrangement, extras.label.as_deref());
    let made = Node::Pane(Pane { id: fresh, tabs: vec![Tab::of(tab_id.clone(), kind, Extras { label, ..extras })], active: Some(tab_id) });
    next.root = Node::Split(Split { id: split_id, dir: Dir::Row, ratio: 0.62, a: Box::new(arrangement.root.clone()), b: Box::new(made) });
    next
}

/// A walk the Job opened: its tab joins the pane that holds a walk tab, else the pane that holds the thread. `behind` leaves the
/// front tab and the focus where they were (Classic's `arriveWalk`).
pub(crate) fn arrive_walk(arrangement: &Arrangement, label: &str, page: &str, behind: bool) -> Arrangement {
    let extras = Extras { label: Some(label.to_string()), page: Some(page.to_string()), ..Extras::default() };
    let Some(holder) = holder_of(arrangement, "walk").or_else(|| holder_of(arrangement, "thread")) else { return arrive(arrangement, "walk", extras) };
    let (id, active) = (holder.id.clone(), holder.active.clone());
    let mut arrived = add_tab(arrangement, &id, "walk", extras).into_owned();
    if behind {
        arrived.focus = arrangement.focus.clone();
        arrived.root = edit_pane(&arrived.root, &id, &|one| one.active = active.clone());
    }
    arrived
}

// ---- Kept shapes ------------------------------------------------------------------------------------

/// A kept arrangement as a Job's, or None where it is not one (Classic's `isArrangement` over `JOB_TAB_KINDS`).
pub(crate) fn revive(value: &serde_json::Value) -> Option<Arrangement> {
    revive_with(value, &JOB_TAB_KINDS, &[])
}

/// A kept arrangement of a family with these kinds, the kinds `renamed` names put under the kinds they open now first (Classic's `renamedIn` then `isArrangement`).
pub(crate) fn revive_with(value: &serde_json::Value, kinds: &[&str], renamed: &[(&str, &str)]) -> Option<Arrangement> {
    let mut arrangement = Arrangement::deserialize(value).ok()?;
    if !renamed.is_empty() {
        arrangement = rename_kinds(&arrangement, renamed);
    }
    let kinds_known = panes_of(&arrangement.root).iter().all(|pane| pane.tabs.iter().all(|tab| kinds.contains(&tab.kind.as_str())));
    (kinds_known && pane_by_id(&arrangement, &arrangement.focus).is_some()).then_some(arrangement)
}

// ---- Where the person has been, for the undo key ---------------------------------------------------------

/// One owner's undo: what was before each change and what undo took back (Classic's `History` in `store.ts`). In memory only.
#[derive(Debug, Default)]
pub(crate) struct History {
    past: Vec<Arrangement>,
    future: Vec<Arrangement>,
    merge: Option<String>,
    at: u64,
}

impl History {
    /// What was there before a change is kept. A run of one `merge` kind inside `MERGE_MS` joins the step before it (Classic's `remember`).
    pub fn remember(&mut self, was: &Arrangement, merge: Option<&str>, now_ms: u64) {
        let joins = merge.is_some() && self.merge.as_deref() == merge && now_ms.saturating_sub(self.at) < MERGE_MS;
        self.at = now_ms;
        self.merge = merge.map(str::to_string);
        if !joins {
            let keep = self.past.len().saturating_sub(HISTORY - 1);
            self.past.drain(..keep);
            self.past.push(was.clone());
        }
        self.future.clear();
    }

    /// The change before the last, with `current` kept for redo; None with nothing to take back (Classic's `undo`).
    pub fn undo(&mut self, current: &Arrangement) -> Option<Arrangement> {
        let target = self.past.pop()?;
        self.future.push(current.clone());
        self.merge = None;
        Some(target)
    }

    /// The change undo took back, with `current` kept for undo again; None with nothing to put back (Classic's `redo`).
    pub fn redo(&mut self, current: &Arrangement) -> Option<Arrangement> {
        let target = self.future.pop()?;
        self.past.push(current.clone());
        self.merge = None;
        Some(target)
    }
}

/// Whether a change leaves every pane its minimum, or in an area too small for that, no pane further short than before.
/// No measured area, or one with no width, allows everything (Classic's `allowed`).
pub(crate) fn allowed(size: Option<Size>, was: &Arrangement, next: &Arrangement) -> bool {
    size.is_none_or(|size| size.w == 0.0 || shortfall(next, size) <= shortfall(was, size))
}

/// A change of arrangement the person made, or None where it changes nothing or would squeeze a pane below its minimum, unless
/// `force`, as a Job opening a tab is (Classic's `rearrange`).
pub(crate) fn rearrange(was: &Arrangement, change: impl for<'a> FnOnce(&'a Arrangement) -> Out<'a>, size: Option<Size>, force: bool) -> Option<Arrangement> {
    match change(was) {
        Cow::Borrowed(_) => None,
        Cow::Owned(next) => (force || allowed(size, was, &next)).then_some(next),
    }
}

/// Whether what a change made is a new arrangement that leaves every pane its minimum.
fn lands(was: &Arrangement, next: &Out<'_>, size: Option<Size>) -> bool {
    matches!(next, Cow::Owned(next) if allowed(size, was, next))
}

/// Where a tab dragged over a pane would land, and whether the pane can take it (Classic's `droppable`).
pub(crate) fn droppable(was: &Arrangement, size: Option<Size>, tab_id: &str, pane_id: &str, zone: Zone) -> bool {
    let next = match zone {
        Zone::Middle => move_tab_to_pane(was, tab_id, pane_id, None),
        Zone::Edge(side) => move_tab_to_edge(was, tab_id, pane_id, side),
    };
    lands(was, &next, size)
}

/// Whether a tab dragged over the gap of a split can be dropped there (Classic's `droppableGap`).
pub(crate) fn droppable_gap(was: &Arrangement, size: Option<Size>, tab_id: &str, split_id: &str) -> bool {
    lands(was, &move_tab_to_gap(was, tab_id, split_id), size)
}

/// Whether a tab dragged over an outer edge of the area can be dropped there (Classic's `droppableEdge`).
pub(crate) fn droppable_edge(was: &Arrangement, size: Option<Size>, tab_id: &str, side: Side) -> bool {
    lands(was, &move_tab_to_end(was, tab_id, side), size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn kinds_of(arrangement: &Arrangement) -> Vec<Vec<&str>> {
        panes_of(&arrangement.root).iter().map(|pane| pane.tabs.iter().map(|tab| tab.kind.as_str()).collect()).collect()
    }

    fn tab_of_kind<'a>(arrangement: &'a Arrangement, kind: &str) -> &'a Tab {
        panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == kind).expect("a tab of that kind")
    }

    fn pane_at(arrangement: &Arrangement, at: usize) -> &Pane {
        panes_of(&arrangement.root)[at]
    }

    /// Classic's `toBe(arrangement)`: the very arrangement handed in came back.
    fn unchanged(out: &Out<'_>) -> bool {
        matches!(out, Cow::Borrowed(_))
    }

    fn split_of(arrangement: &Arrangement) -> &Split {
        match &arrangement.root {
            Node::Split(split) => split,
            Node::Pane(_) => panic!("the root is a pane"),
        }
    }

    fn row() -> Arrangement {
        build(&Shape::split(Dir::Row, 0.5, Shape::pane(&["thread"]), Shape::pane(&["terminal", "browser"])), None, &[])
    }

    mod split_and_close {
        use super::*;

        #[test]
        fn a_pane_splits_to_its_right_with_an_empty_pane_that_takes_the_focus() {
            let one = build(&Shape::pane(&["thread"]), None, &[]);
            let next = split_pane(&one, &one.focus, SplitInto::Right).into_owned();
            assert_eq!(kinds_of(&next), vec![vec!["thread"], vec![]]);
            assert_eq!((split_of(&next).dir, split_of(&next).ratio), (Dir::Row, 0.5));
            assert_eq!(next.focus, pane_at(&next, 1).id);
        }

        #[test]
        fn a_split_below_stacks_and_an_unknown_pane_changes_nothing() {
            let one = build(&Shape::pane(&["thread"]), None, &[]);
            assert_eq!(split_of(&split_pane(&one, &one.focus, SplitInto::Down)).dir, Dir::Column);
            assert!(unchanged(&split_pane(&one, "nope", SplitInto::Down)));
        }

        #[test]
        fn closing_a_panes_last_tab_removes_the_pane_and_the_sibling_takes_the_splits_place() {
            let arrangement = build(&Shape::split(Dir::Row, 0.5, Shape::pane(&["thread"]), Shape::pane(&["terminal"])), None, &[]);
            let next = close_tab(&arrangement, &tab_of_kind(&arrangement, "terminal").id);
            assert!(matches!(next.root, Node::Pane(_)));
            assert_eq!(kinds_of(&next), vec![vec!["thread"]]);
        }

        #[test]
        fn the_last_tab_of_the_last_pane_leaves_an_empty_pane_and_the_last_pane_is_never_closed() {
            let arrangement = build(&Shape::pane(&["thread"]), None, &[]);
            let next = close_tab(&arrangement, &tab_of_kind(&arrangement, "thread").id);
            assert_eq!(kinds_of(&next), vec![Vec::<&str>::new()]);
            assert!(unchanged(&close_pane(&arrangement, &arrangement.focus)));
        }

        #[test]
        fn closing_the_front_tab_brings_the_next_one_forward_and_the_last_brings_the_one_before() {
            let arrangement = build(&Shape::pane(&["thread", "terminal", "browser"]), Some("terminal"), &[]);
            let closed = close_tab(&arrangement, &tab_of_kind(&arrangement, "terminal").id);
            assert_eq!(pane_at(&closed, 0).active.as_ref(), Some(&tab_of_kind(&arrangement, "browser").id));
            let two = build(&Shape::pane(&["thread", "terminal"]), Some("terminal"), &[]);
            let last = close_tab(&two, &tab_of_kind(&two, "terminal").id);
            assert_eq!(pane_at(&last, 0).active.as_ref(), Some(&tab_of_kind(&two, "thread").id));
        }

        #[test]
        fn closing_the_focused_pane_puts_the_focus_on_the_first_pane_left() {
            let arrangement = row();
            let second = pane_at(&arrangement, 1).id.clone();
            let focused = Arrangement { focus: second.clone(), ..arrangement.clone() };
            assert_eq!(close_pane(&focused, &second).focus, pane_at(&arrangement, 0).id);
        }
    }

    mod moving_a_tab {
        use super::*;

        #[test]
        fn into_another_pane_at_an_index() {
            let arrangement = row();
            let thread = tab_of_kind(&arrangement, "thread");
            let target = pane_at(&arrangement, 1);
            let next = move_tab_to_pane(&arrangement, &thread.id, &target.id, Some(1));
            assert_eq!(kinds_of(&next), vec![vec!["terminal", "thread", "browser"]]);
            assert!(matches!(next.root, Node::Pane(_)));
        }

        #[test]
        fn within_its_own_pane_it_only_changes_its_place() {
            let arrangement = row();
            let pane = pane_at(&arrangement, 1);
            let next = move_tab_to_pane(&arrangement, &tab_of_kind(&arrangement, "terminal").id, &pane.id, None);
            assert_eq!(kinds_of(&next), vec![vec!["thread"], vec!["browser", "terminal"]]);
        }

        #[test]
        fn onto_an_edge_of_a_pane_it_makes_a_pane_there_and_a_lone_tab_dropped_on_its_own_edge_stays() {
            let arrangement = row();
            let next = move_tab_to_edge(&arrangement, &tab_of_kind(&arrangement, "browser").id, &pane_at(&arrangement, 0).id, Side::Left);
            assert_eq!(kinds_of(&next), vec![vec!["browser"], vec!["thread"], vec!["terminal"]]);
            let thread = tab_of_kind(&arrangement, "thread");
            assert!(unchanged(&move_tab_to_edge(&arrangement, &thread.id, &pane_at(&arrangement, 0).id, Side::Top)));
        }

        #[test]
        fn into_the_gap_of_a_split_it_makes_a_pane_between_the_two_sides_and_gives_each_a_third() {
            let arrangement = row();
            let split = rects_of(&arrangement.root).splits[0].id.to_string();
            let next = move_tab_to_gap(&arrangement, &tab_of_kind(&arrangement, "browser").id, &split);
            assert_eq!(kinds_of(&next), vec![vec!["thread"], vec!["browser"], vec!["terminal"]]);
            let widths: Vec<f64> = rects_of(&next.root).panes.iter().map(|(_, rect)| (rect.w * 100.0).round() / 100.0).collect();
            assert_eq!(widths, vec![0.33, 0.33, 0.33]);
        }

        #[test]
        fn a_gap_beside_a_panes_only_tab_changes_nothing() {
            let arrangement = row();
            let split = rects_of(&arrangement.root).splits[0].id.to_string();
            assert!(unchanged(&move_tab_to_gap(&arrangement, &tab_of_kind(&arrangement, "thread").id, &split)));
        }

        #[test]
        fn a_gap_in_a_column_stacks_the_new_pane() {
            let arrangement = build(&Shape::split(Dir::Column, 0.5, Shape::pane(&["thread"]), Shape::pane(&["terminal", "browser"])), None, &[]);
            let split = rects_of(&arrangement.root).splits[0].id.to_string();
            let next = move_tab_to_gap(&arrangement, &tab_of_kind(&arrangement, "browser").id, &split);
            assert!(rects_of(&next.root).splits.iter().all(|split| split.dir == Dir::Column));
        }

        #[test]
        fn to_an_outer_edge_it_makes_a_root_split_with_a_third_and_a_lone_tab_carried_to_the_far_edge_swaps_the_panes() {
            let arrangement = row();
            let next = move_tab_to_end(&arrangement, &tab_of_kind(&arrangement, "browser").id, Side::Right);
            assert_eq!(kinds_of(&next), vec![vec!["thread"], vec!["terminal"], vec!["browser"]]);
            let last = pane_at(&next, 2).id.clone();
            assert!((rects_of(&next.root).pane(&last).expect("the pane has a box").w - 1.0 / 3.0).abs() < 1e-9);
            let swapped = move_tab_to_end(&arrangement, &tab_of_kind(&arrangement, "thread").id, Side::Right);
            assert_eq!(kinds_of(&swapped), vec![vec!["terminal", "browser"], vec!["thread"]]);
        }

        #[test]
        fn a_drop_that_leaves_the_panes_as_they_were_is_nothing_and_the_only_tab_has_nowhere_to_go() {
            let arrangement = row();
            assert!(unchanged(&move_tab_to_end(&arrangement, &tab_of_kind(&arrangement, "thread").id, Side::Left)));
            let alone = build(&Shape::pane(&["thread"]), None, &[]);
            assert!(unchanged(&move_tab_to_end(&alone, &tab_of_kind(&alone, "thread").id, Side::Right)));
        }
    }

    mod minimum_sizes {
        use super::*;

        #[test]
        fn two_panes_in_an_area_as_wide_as_both_minimums_have_no_shortfall() {
            assert_eq!(shortfall(&row(), Size { w: MIN_PANE_W * 2.0 + 1.0, h: MIN_PANE_H }), 0.0);
        }

        #[test]
        fn the_shortfall_is_how_far_the_panes_fall_under_their_minimums() {
            let wide = shortfall(&row(), Size { w: MIN_PANE_W * 2.0 + 1.0, h: MIN_PANE_H });
            let narrow = shortfall(&row(), Size { w: MIN_PANE_W, h: MIN_PANE_H });
            assert!(narrow > wide);
        }

        #[test]
        fn a_ratio_is_held_to_the_least_a_side_keeps() {
            let arrangement = row();
            let split = rects_of(&arrangement.root).splits[0].id.to_string();
            let ratio = split_of(&resize_split(&arrangement, &split, 0.01)).ratio;
            assert!(ratio > 0.01);
            assert!(ratio < 0.2);
        }
    }

    mod arriving {
        use super::*;

        #[test]
        fn a_tab_a_job_opens_joins_the_pane_that_holds_that_kind() {
            let next = arrive(&row(), "browser", Extras::label("localhost:5173"));
            assert_eq!(kinds_of(&next), vec![vec!["thread"], vec!["terminal", "browser", "browser"]]);
            let made = holder_of(&next, "browser").and_then(|pane| pane.tabs.last()).expect("the holder has tabs");
            assert_eq!((made.kind.as_str(), made.label.as_deref()), ("browser", Some("localhost:5173")));
        }

        #[test]
        fn with_no_pane_holding_the_kind_it_opens_as_a_split_on_the_right_of_everything() {
            let arrangement = build(&Shape::pane(&["thread"]), None, &[]);
            let next = arrive(&arrangement, "terminal", Extras::label("dev"));
            assert_eq!(kinds_of(&next), vec![vec!["thread"], vec!["terminal"]]);
            assert_eq!(split_of(&next).dir, Dir::Row);
            assert_eq!(next.focus, arrangement.focus);
        }

        #[test]
        fn the_same_label_a_second_time_is_numbered() {
            let once = arrive(&build(&Shape::pane(&["thread"]), None, &[]), "terminal", Extras::label("dev"));
            let holder = holder_of(&once, "terminal").expect("a terminal pane").id.clone();
            let twice = add_tab(&once, &holder, "terminal", Extras::label("dev"));
            let labels: Vec<Option<&str>> = holder_of(&twice, "terminal").expect("a terminal pane").tabs.iter().map(|tab| tab.label.as_deref()).collect();
            assert_eq!(labels, vec![Some("dev"), Some("dev 2")]);
        }
    }

    mod a_custom_tabs_mod {
        use super::*;

        #[test]
        fn is_kept_on_the_tab_and_marking_it_again_changes_nothing() {
            let arrangement = build(&Shape::split(Dir::Row, 0.5, Shape::pane(&["thread"]), Shape::pane(&["custom"])), None, &[]);
            let tab = tab_of_kind(&arrangement, "custom");
            let marked = mark_tab_mod(&arrangement, &tab.id, "review-queue").into_owned();
            assert_eq!(tab_of_kind(&marked, "custom").mod_.as_deref(), Some("review-queue"));
            assert!(unchanged(&mark_tab_mod(&marked, &tab.id, "review-queue")));
            assert!(unchanged(&mark_tab_mod(&arrangement, "nope", "review-queue")));
        }
    }

    mod showing_a_page {
        use super::*;

        #[test]
        fn a_tab_open_shows_another_page_and_comes_to_the_front_and_an_unknown_tab_changes_nothing() {
            let arrangement = build(&Shape::split(Dir::Row, 0.5, Shape::pane(&["thread"]), Shape::pane(&["request", "terminal"])), None, &[]);
            let request = tab_of_kind(&arrangement, "request");
            let shown = show_page(&arrangement, &request.id, "page").into_owned();
            assert_eq!(tab_of_kind(&shown, "request").page.as_deref(), Some("page"));
            assert_eq!(holder_of(&shown, "request").and_then(|pane| pane.active.as_ref()), Some(&request.id));
            assert!(unchanged(&show_page(&arrangement, "nope", "page")));
        }
    }

    mod a_closed_thread_coming_back {
        use super::*;

        fn close_thread(arrangement: &Arrangement) -> Arrangement {
            close_tab(arrangement, &tab_of_kind(arrangement, "thread").id).into_owned()
        }

        #[test]
        fn into_its_pane_where_that_pane_is_still_there_in_front_with_the_focus_where_it_was() {
            let arrangement = build(&Shape::split(Dir::Row, 0.5, Shape::pane(&["thread", "files"]), Shape::pane(&["terminal"])), None, &[]);
            let focus = holder_of(&arrangement, "terminal").expect("a terminal pane").id.clone();
            let pane = holder_of(&arrangement, "thread").expect("a thread pane").id.clone();
            let back = reopen_thread(&close_thread(&Arrangement { focus: focus.clone(), ..arrangement.clone() })).into_owned();
            let holder = holder_of(&back, "thread").expect("the thread is back");
            assert_eq!(holder.id, pane);
            assert_eq!(holder.active.as_ref(), Some(&tab_of_kind(&back, "thread").id));
            assert_eq!(back.focus, focus);
            assert_eq!(back.thread_was, None);
        }

        #[test]
        fn a_pane_that_went_with_it_is_made_again_on_the_same_side_at_the_same_share() {
            let arrangement = build(&Shape::split(Dir::Row, 0.3, Shape::pane(&["thread"]), Shape::pane(&["terminal"])), None, &[]);
            let closed = close_thread(&arrangement);
            assert_eq!(panes_of(&closed.root).len(), 1);
            let back = reopen_thread(&closed).into_owned();
            assert_eq!(kinds_of(&back), vec![vec!["thread"], vec!["terminal"]]);
            assert_eq!((split_of(&back).dir, split_of(&back).ratio), (Dir::Row, 0.3));
        }

        #[test]
        fn with_its_place_gone_or_never_closed_it_comes_back_in_the_focused_pane_and_one_that_stands_is_left() {
            let arrangement = build(&Shape::pane(&["terminal"]), None, &[]);
            let back = reopen_thread(&arrangement).into_owned();
            assert_eq!(kinds_of(&back), vec![vec!["terminal", "thread"]]);
            assert!(unchanged(&reopen_thread(&back)));
        }
    }

    mod a_walk_arriving {
        use super::*;

        fn thread() -> Arrangement {
            build(&Shape::pane(&["thread"]), None, &[])
        }

        fn walk_tabs(arrangement: &Arrangement) -> Vec<&Tab> {
            panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).filter(|tab| tab.kind == "walk").collect()
        }

        #[test]
        fn the_first_one_joins_the_threads_pane_with_no_split_named_by_its_page_and_keeping_the_pages_address_and_takes_the_focus() {
            let arranged = arrive_walk(&thread(), "Billing", "http://x/billing", false);
            assert_eq!(kinds_of(&arranged), vec![vec!["thread", "walk"]]);
            assert!(matches!(arranged.root, Node::Pane(_)));
            let tab = walk_tabs(&arranged)[0];
            assert_eq!(tab.label.as_deref(), Some("Billing"));
            assert_eq!(tab.page.as_deref(), Some("http://x/billing"));
            let holder = holder_of(&arranged, "walk").expect("a walk pane");
            assert_eq!(holder.active.as_ref(), Some(&tab.id));
            assert_eq!(arranged.focus, holder.id);
        }

        #[test]
        fn a_second_one_joins_the_same_pane_and_comes_to_the_front() {
            let first = arrive_walk(&thread(), "Billing", "http://x/billing", false);
            let second = arrive_walk(&first, "Settings", "http://x/settings", false);
            assert_eq!(panes_of(&second.root).len(), 1);
            let labels: Vec<Option<&str>> = walk_tabs(&second).iter().map(|tab| tab.label.as_deref()).collect();
            assert_eq!(labels, vec![Some("Billing"), Some("Settings")]);
            assert_eq!(holder_of(&second, "walk").and_then(|pane| pane.active.as_ref()), Some(&walk_tabs(&second)[1].id));
        }

        #[test]
        fn one_that_arrives_behind_leaves_the_tab_in_front_and_the_focus_where_they_were() {
            let arrangement = build(&Shape::split(Dir::Row, 0.5, Shape::pane(&["thread"]), Shape::pane(&["terminal"])), None, &[]);
            let holder = holder_of(&arrangement, "thread").expect("a thread pane").clone();
            let focus = holder_of(&arrangement, "terminal").expect("a terminal pane").id.clone();
            let arranged = arrive_walk(&Arrangement { focus: focus.clone(), ..arrangement.clone() }, "Billing", "http://x/billing", true);
            assert_eq!(walk_tabs(&arranged).len(), 1);
            let now = holder_of(&arranged, "walk").expect("a walk pane");
            assert_eq!(now.id, holder.id);
            assert_eq!(now.active, holder.active);
            assert_eq!(arranged.focus, focus);
        }

        #[test]
        fn where_he_moved_walk_tabs_to_another_pane_that_pane_still_holds_them() {
            let arrangement = build(&Shape::split(Dir::Row, 0.5, Shape::pane(&["thread"]), Shape::pane(&["walk"])), None, &[]);
            let moved = holder_of(&arrangement, "walk").expect("a walk pane").id.clone();
            let arranged = arrive_walk(&arrangement, "Billing", "http://x/billing", false);
            assert_eq!(panes_of(&arranged.root).len(), 2);
            assert_eq!(holder_of(&arranged, "walk").expect("a walk pane").id, moved);
            assert_eq!(walk_tabs(&arranged).len(), 2);
            assert_eq!(holder_of(&arranged, "thread").expect("a thread pane").tabs.len(), 1);
        }

        #[test]
        fn with_no_thread_and_no_walk_pane_it_opens_by_the_ordinary_arrival_of_a_tab_named_by_its_page() {
            let arrangement = build(&Shape::pane(&["terminal"]), None, &[]);
            let arranged = arrive_walk(&arrangement, "Billing", "http://x/billing", false);
            let mut kinds: Vec<&str> = kinds_of(&arranged).into_iter().flatten().collect();
            kinds.sort();
            assert_eq!(kinds, vec!["terminal", "walk"]);
            let walk = walk_tabs(&arranged)[0];
            assert_eq!((walk.label.as_deref(), walk.page.as_deref()), (Some("Billing"), Some("http://x/billing")));
        }

        #[test]
        fn with_a_thread_that_shares_a_split_the_split_keeps_its_share_and_a_walk_of_that_name_already_open_is_named_apart() {
            let arrangement = build(&Shape::split(Dir::Row, 0.5, Shape::pane(&["thread"]), Shape::pane(&["terminal"])), None, &[]);
            let arranged = arrive_walk(&arrangement, "Billing", "http://x/billing", false);
            assert_eq!(split_of(&arranged).ratio, 0.5);
            assert_eq!(panes_of(&arranged.root).len(), 2);
            let again = arrive_walk(&arranged, "Billing", "http://x/billing2", false);
            let labels: HashSet<Option<&str>> = walk_tabs(&again).iter().map(|tab| tab.label.as_deref()).collect();
            assert_eq!(labels.len(), 2);
        }
    }

    /// What `store.test.ts` holds of undo.
    mod undo {
        use super::*;

        /// One owner as the store keeps it: its arrangement and its undo, with the clock held still.
        struct Owner {
            now: Arrangement,
            history: History,
            size: Option<Size>,
        }

        impl Owner {
            fn new() -> Self {
                Owner { now: default_arrangement(), history: History::default(), size: None }
            }

            fn rearrange(&mut self, change: impl for<'a> FnOnce(&'a Arrangement) -> Out<'a>, merge: Option<&str>, force: bool) {
                if let Some(next) = rearrange(&self.now, change, self.size, force) {
                    self.history.remember(&self.now, merge, 0);
                    self.now = next;
                }
            }

            fn undo(&mut self) {
                if let Some(target) = self.history.undo(&self.now) {
                    self.now = target;
                }
            }

            fn redo(&mut self) {
                if let Some(target) = self.history.redo(&self.now) {
                    self.now = target;
                }
            }
        }

        #[test]
        fn takes_back_the_last_change_and_puts_it_back_and_a_run_of_one_kind_of_change_is_one_step() {
            let mut owner = Owner::new();
            let before = owner.now.clone();
            owner.rearrange(|x| add_tab(x, &x.focus, "files", Extras::default()), None, false);
            owner.rearrange(|x| add_tab(x, &x.focus, "plan", Extras::default()), None, false);
            owner.undo();
            assert_eq!(kinds_of(&owner.now)[0], vec!["thread", "files"]);
            owner.undo();
            assert_eq!(owner.now, before);
            owner.redo();
            assert_eq!(kinds_of(&owner.now)[0], vec!["thread", "files"]);

            let split = rects_of(&owner.now.root).splits[0].id.to_string();
            owner.rearrange(|x| Cow::Owned(resize_split(x, &split, 0.3)), Some("resize"), false);
            owner.rearrange(|x| Cow::Owned(resize_split(x, &split, 0.4)), Some("resize"), false);
            owner.undo();
            assert!((rects_of(&owner.now.root).splits[0].ratio - 0.62).abs() < 1e-9);
        }

        #[test]
        fn is_this_windows_a_new_history_has_nothing_to_take_back() {
            let mut history = History::default();
            assert_eq!(history.undo(&default_arrangement()), None);
            assert_eq!(history.redo(&default_arrangement()), None);
        }

        #[test]
        fn keeps_the_last_fifty_steps_and_a_run_older_than_the_merge_window_is_a_step_of_its_own() {
            let mut history = History::default();
            let mut at = default_arrangement();
            for step in 0..(HISTORY + 10) {
                history.remember(&at, None, step as u64);
                let split = rects_of(&at.root).splits[0].id.to_string();
                at = resize_split(&at, &split, 0.2 + step as f64 / 200.0);
            }
            let mut taken = 0;
            while let Some(target) = history.undo(&at) {
                at = target;
                taken += 1;
            }
            assert_eq!(taken, HISTORY);

            history.remember(&at, Some("resize"), 0);
            history.remember(&at, Some("resize"), MERGE_MS - 1);
            history.remember(&at, Some("resize"), MERGE_MS * 3);
            assert_eq!(history.past.len(), 2);
        }
    }

    /// What `store.test.ts` holds of minimum sizes.
    mod minimum_sizes_in_the_store {
        use super::*;

        #[test]
        fn a_change_that_squeezes_a_pane_below_its_minimum_in_a_measured_area_is_refused_and_one_forced_as_a_job_opening_a_tab_is_is_not() {
            // Room for the thread and one pane beside it, and no more.
            let size = Some(Size { w: MIN_PANE_W * 2.0 + 1.0, h: MIN_PANE_H * 8.0 });
            let mut at = default_arrangement();
            at = rearrange(&at, |x| add_tab(x, &x.focus, "files", Extras::default()), size, false).unwrap_or(at);
            let moved = pane_at(&at, 0).tabs[1].id.clone();
            at = rearrange(&at, |x| move_tab_to_edge(x, &moved, &x.focus, Side::Right), size, false).unwrap_or(at);
            let crowded = panes_of(&at.root).len();
            let split = |x: &Arrangement| split_pane(x, &x.focus, SplitInto::Right).into_owned();
            assert_eq!(rearrange(&at, |x| Cow::Owned(split(x)), size, false), None);
            let forced = rearrange(&at, |x| Cow::Owned(split(x)), size, true).expect("a forced change is made");
            assert_eq!(panes_of(&forced.root).len(), crowded + 1);
        }

        #[test]
        fn whether_a_drop_can_be_made_reads_the_same_rule() {
            let arrangement = default_arrangement();
            let tab = pane_at(&arrangement, 1).tabs[0].id.clone();
            let tight = Some(Size { w: MIN_PANE_W * 2.0 + 1.0, h: MIN_PANE_H * 8.0 });
            assert!(!droppable_edge(&arrangement, tight, &tab, Side::Right));
            let wide = Some(Size { w: MIN_PANE_W * 20.0, h: MIN_PANE_H * 8.0 });
            assert!(droppable_edge(&arrangement, wide, &tab, Side::Right));
        }

        #[test]
        fn an_area_not_measured_yet_allows_every_change() {
            let arrangement = default_arrangement();
            let tab = pane_at(&arrangement, 1).tabs[0].id.clone();
            assert!(droppable_edge(&arrangement, None, &tab, Side::Right));
            assert!(droppable_edge(&arrangement, Some(Size { w: 0.0, h: 0.0 }), &tab, Side::Right));
        }

        #[test]
        fn a_gap_and_a_pane_read_the_same_rule() {
            let arrangement = default_arrangement();
            let tab = pane_at(&arrangement, 1).tabs[0].id.clone();
            let split = rects_of(&arrangement.root).splits[0].id.to_string();
            let tight = Some(Size { w: MIN_PANE_W * 2.0 + 1.0, h: MIN_PANE_H * 8.0 });
            assert!(!droppable_gap(&arrangement, tight, &tab, &split));
            assert!(droppable_gap(&arrangement, None, &tab, &split));
            let other = pane_at(&arrangement, 2).id.clone();
            assert!(droppable(&arrangement, None, &tab, &other, Zone::Edge(Side::Left)));
            assert!(droppable(&arrangement, None, &tab, &other, Zone::Middle));
        }
    }

    /// Operations `tree.test.ts` does not reach, held to what `tree.ts` does.
    mod beside_classics_tests {
        use super::*;

        #[test]
        fn the_nearest_pane_that_shares_an_edge_is_the_neighbour() {
            let arrangement = default_arrangement();
            let (thread, open, now) = (pane_at(&arrangement, 0).id.clone(), pane_at(&arrangement, 1).id.clone(), pane_at(&arrangement, 2).id.clone());
            // Both panes at its right touch it; the one sharing more of its edge (Now, 60%) is nearer than Open (40%).
            assert_eq!(neighbour_of(&arrangement, &thread, Toward::Right), Some(now.clone()));
            assert_eq!(neighbour_of(&arrangement, &open, Toward::Down), Some(now.clone()));
            assert_eq!(neighbour_of(&arrangement, &now, Toward::Left), Some(thread));
            assert_eq!(neighbour_of(&arrangement, &open, Toward::Up), None);
            assert_eq!(neighbour_of(&arrangement, "nope", Toward::Up), None);
        }

        #[test]
        fn cycling_wraps_round_in_both_directions_and_a_lone_tab_changes_nothing() {
            let arrangement = build(&Shape::pane(&["thread", "terminal", "browser"]), None, &[]);
            let pane = pane_at(&arrangement, 0).id.clone();
            let front = |a: &Arrangement| pane_at(a, 0).active.clone();
            let back = cycle_tab(&arrangement, &pane, -1).into_owned();
            assert_eq!(front(&back), Some(tab_of_kind(&arrangement, "browser").id.clone()));
            let round = cycle_tab(&back, &pane, 1).into_owned();
            assert_eq!(front(&round), front(&arrangement));
            let lone = build(&Shape::pane(&["thread"]), None, &[]);
            assert!(unchanged(&cycle_tab(&lone, &pane_at(&lone, 0).id, 1)));
        }

        #[test]
        fn a_kind_renamed_onto_one_already_there_leaves_the_first_and_the_pane_fronts_it() {
            let arrangement = build(&Shape::pane(&["pr-focus", "pr-review", "pr-files"]), Some("pr-review"), &[]);
            let renamed = rename_kinds(&arrangement, &RENAMED_PR_KINDS);
            assert_eq!(kinds_of(&renamed), vec![vec!["pr-review", "pr-files"]]);
            assert_eq!(pane_at(&renamed, 0).active.as_ref(), Some(&tab_of_kind(&arrangement, "pr-focus").id));
        }

        #[test]
        fn a_pane_docked_at_the_bottom_takes_its_share_and_leaves_the_focus() {
            let arrangement = row();
            let docked = dock_at_bottom(&arrangement, "pr-ask", 0.22);
            assert_eq!(kinds_of(&docked), vec![vec!["thread"], vec!["terminal", "browser"], vec!["pr-ask"]]);
            assert_eq!((split_of(&docked).dir, split_of(&docked).ratio), (Dir::Column, 1.0 - 0.22));
            assert_eq!(docked.focus, arrangement.focus);
        }

        #[test]
        fn a_shell_the_job_opened_joins_the_terminal_pane_behind_the_tab_in_front() {
            let arrangement = row();
            let front = pane_at(&arrangement, 1).active.clone();
            let arrived = arrive_terminal_behind(&arrangement, Some("sh-2"));
            assert_eq!(kinds_of(&arrived)[1], vec!["terminal", "browser", "terminal"]);
            assert_eq!(pane_at(&arrived, 1).active, front);
            assert_eq!(arrived.focus, arrangement.focus);
            assert_eq!(pane_at(&arrived, 1).tabs[2].terminal.as_deref(), Some("sh-2"));
        }

        #[test]
        fn a_tab_is_renamed_and_an_empty_name_gives_its_default_back() {
            let arrangement = row();
            let tab = tab_of_kind(&arrangement, "terminal").id.clone();
            let named = rename_tab(&arrangement, &tab, "  build  ").into_owned();
            assert_eq!(tab_of_kind(&named, "terminal").label.as_deref(), Some("build"));
            assert!(unchanged(&rename_tab(&named, &tab, "build")));
            assert_eq!(tab_of_kind(&rename_tab(&named, &tab, " ").into_owned(), "terminal").label, None);
        }

        #[test]
        fn a_nodes_least_size_adds_along_its_split_and_takes_the_larger_across_it() {
            let arrangement = default_arrangement();
            assert_eq!(min_extent(&arrangement.root, Dir::Row), MIN_PANE_W * 2.0);
            assert_eq!(min_extent(&arrangement.root, Dir::Column), MIN_PANE_H * 2.0);
        }
    }

    // What `store.test.ts` builds for the thread's Default, written as Classic's `JSON.stringify` writes it.
    fn classic_arrangement() -> serde_json::Value {
        json!({
            "root": {
                "type": "split", "id": "s1", "dir": "row", "ratio": 0.62,
                "a": { "type": "pane", "id": "p2", "tabs": [{ "id": "t3", "kind": "thread" }], "active": "t3" },
                "b": {
                    "type": "split", "id": "s4", "dir": "column", "ratio": 0.4,
                    "a": {
                        "type": "pane", "id": "p5",
                        "tabs": [
                            { "id": "t6", "kind": "open" },
                            { "id": "t7", "kind": "done" },
                            { "id": "t8", "kind": "custom", "label": "Queue", "mod": "review-queue" },
                            { "id": "t11", "kind": "terminal", "terminal": "sh-1" }
                        ],
                        "active": "t6"
                    },
                    "b": { "type": "pane", "id": "p9", "tabs": [] }
                }
            },
            "focus": "p2",
            "next": 12,
            "threadWas": { "pane": "p2", "beside": { "node": "s4", "dir": "row", "ratio": 0.62, "first": true } }
        })
    }

    mod revived {
        use super::*;

        #[test]
        fn a_classic_arrangement_deserializes_unchanged_and_revives() {
            let stored = classic_arrangement();
            let arrangement = revive(&stored).expect("Classic's own arrangement revives");
            assert_eq!(kinds_of(&arrangement), vec![vec!["thread"], vec!["open", "done", "custom", "terminal"], vec![]]);
            assert_eq!(tab_of_kind(&arrangement, "custom").mod_.as_deref(), Some("review-queue"));
            assert_eq!(pane_at(&arrangement, 2).active, None);
            assert_eq!(arrangement.thread_was.as_ref().and_then(|place| place.beside.as_ref()).map(|beside| beside.first), Some(true));
            // The same names and tags go back out: nothing added, an absent field not written as null.
            assert_eq!(serde_json::to_value(&arrangement).expect("an arrangement serializes"), stored);
            let text = serde_json::to_string(&stored).expect("json is text");
            assert_eq!(serde_json::from_str::<Arrangement>(&text).expect("text reads back"), arrangement);
        }

        #[test]
        fn what_is_not_an_arrangement_is_not_used() {
            let nonsense = json!({
                "root": { "type": "pane", "id": "p1", "tabs": [{ "id": "t1", "kind": "nonsense" }] },
                "focus": "p1",
                "next": 2
            });
            assert_eq!(revive(&nonsense), None);
            let mut off_focus = classic_arrangement();
            off_focus["focus"] = json!("p99");
            assert_eq!(revive(&off_focus), None);
            let mut no_counter = classic_arrangement();
            no_counter.as_object_mut().expect("an object").remove("next");
            assert_eq!(revive(&no_counter), None);
            let mut bad_dir = classic_arrangement();
            bad_dir["root"]["dir"] = json!("diagonal");
            assert_eq!(revive(&bad_dir), None);
            for other in [json!("not json"), json!(null), json!(7), json!([])] {
                assert_eq!(revive(&other), None);
            }
        }

        #[test]
        fn a_kind_outside_the_family_is_not_used_and_a_renamed_one_is_put_under_the_kind_it_opens_now() {
            assert_eq!(revive_with(&classic_arrangement(), &PR_TAB_KINDS, &RENAMED_PR_KINDS), None);
            let kept = json!({
                "root": { "type": "pane", "id": "p1", "tabs": [{ "id": "t1", "kind": "pr-focus" }, { "id": "t2", "kind": "pr-files" }], "active": "t1" },
                "focus": "p1",
                "next": 3
            });
            assert_eq!(revive_with(&kept, &PR_TAB_KINDS, &[]), None);
            let revived = revive_with(&kept, &PR_TAB_KINDS, &RENAMED_PR_KINDS).expect("pr-focus opens as pr-review");
            assert_eq!(kinds_of(&revived), vec![vec!["pr-review", "pr-files"]]);
        }

        #[test]
        fn every_kind_a_job_can_open_is_known_and_none_is_listed_twice() {
            let all: HashSet<&str> = JOB_TAB_KINDS.iter().chain(PR_TAB_KINDS.iter()).copied().collect();
            assert_eq!(all.len(), JOB_TAB_KINDS.len() + PR_TAB_KINDS.len());
            assert!(is_tab_kind("thread") && is_tab_kind("pr-ask") && !is_tab_kind("pr-focus"));
        }
    }
}
