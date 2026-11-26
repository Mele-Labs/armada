//! Where each block of a grid stands: a plain struct a screen keeps, so Fleet can hold it and Bridge send it as it is.
//! Nothing here touches the screen. The Project Dashboard and the Cockpit home each name their own kinds of block and share the rest.
//!
//! The grid is six columns wide and as tall as its blocks. A move or a resize pins the block it is about and settles the
//! rest around it: every other block rises as far as it can and drops below whatever stands in its way.

use std::hash::Hash;

use ipc::{BoardBlock, BoardLayout};
use serde::{Deserialize, Serialize};

/// The columns of the grid.
pub const COLUMNS: u8 = 6;

/// The least a block is drawn at, in columns and rows.
pub const LEAST_W: u8 = 2;
pub const LEAST_H: u8 = 1;

/// The most rows a block takes.
pub const MOST_H: u8 = 6;

/// How big a block is drawn when it is added, in columns and rows: the size the presets give each.
pub const ADDED_W: u8 = 3;
pub const ADDED_H: u8 = 2;

/// What a screen's blocks are: the words Fleet keeps for them and the views each can draw in.
pub trait GridKind: Copy + Eq + Hash + 'static {
    /// The kinds drawn whatever the screen keeps, in the order its add menu lists them.
    fn all() -> &'static [Self];

    /// What its element id is made of, and the word Fleet keeps for it.
    fn slug(self) -> String;

    /// The kind Fleet's word names, or none for one a later Bridge adds.
    fn from_slug(slug: &str) -> Option<Self>;

    /// The views its data fits, the first being the one it starts in.
    fn views(self) -> &'static [View];

    /// The words on the block's header and the name VoiceOver reads.
    fn title(self) -> &'static str;
}

/// How a block draws its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    List,
    Table,
    Cards,
    Graph,
    Picture,
}

impl View {
    pub fn word(self) -> &'static str {
        match self {
            View::List => "List",
            View::Table => "Table",
            View::Cards => "Cards",
            View::Graph => "Graph",
            View::Picture => "Picture",
        }
    }

    /// The word Fleet keeps for it.
    pub fn slug(self) -> &'static str {
        match self {
            View::List => "list",
            View::Table => "table",
            View::Cards => "cards",
            View::Graph => "graph",
            View::Picture => "picture",
        }
    }

    pub fn from_slug(slug: &str) -> Option<View> {
        [View::List, View::Table, View::Cards, View::Graph, View::Picture].into_iter().find(|view| view.slug() == slug)
    }

    /// What a press on it says, as its tooltip.
    pub fn said(self) -> &'static str {
        match self {
            View::List => "One line to a row",
            View::Table => "Columns, one row to a line",
            View::Cards => "A card to a row",
            View::Graph => "The rows as a graph",
            View::Picture => "The rows as a picture",
        }
    }
}

/// One block of the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placed<K> {
    pub kind: K,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
    pub view: View,
}

impl<K> Placed<K> {
    pub const fn new(kind: K, x: u8, y: u8, w: u8, h: u8, view: View) -> Placed<K> {
        Placed { kind, x, y, w, h, view }
    }

    pub fn overlaps(&self, other: &Placed<K>) -> bool {
        self.x < other.x + other.w && other.x < self.x + self.w && self.y < other.y + other.h && other.y < self.y + self.h
    }
}

/// A screen's grid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout<K> {
    pub blocks: Vec<Placed<K>>,
}

impl<K: GridKind> Layout<K> {
    /// The grid Fleet kept, and the blocks of it this build has no kind for, which go back as they came.
    /// A block of a kind that stands twice keeps its first, one of a view its kind does not draw takes the kind's first, and
    /// a size or place outside the grid is brought inside it. The blocks are then settled, as every edit leaves them.
    pub fn from_wire(kept: &BoardLayout) -> (Layout<K>, Vec<BoardBlock>) {
        let mut layout = Layout { blocks: Vec::new() };
        let mut unknown = Vec::new();
        for block in &kept.blocks {
            let Some(kind) = K::from_slug(&block.kind) else {
                unknown.push(block.clone());
                continue;
            };
            if layout.block(kind).is_some() {
                continue;
            }
            let w = block.w.clamp(LEAST_W, COLUMNS);
            let view = View::from_slug(&block.view).filter(|view| kind.views().contains(view)).unwrap_or(kind.views()[0]);
            layout.blocks.push(Placed { kind, x: block.x.min(COLUMNS - w), y: block.y, w, h: block.h.clamp(LEAST_H, MOST_H), view });
        }
        layout.settle(None);
        (layout, unknown)
    }

    /// What Fleet keeps for this grid: its blocks, then the ones of kinds this build does not know.
    pub fn to_wire(&self, unknown: &[BoardBlock]) -> BoardLayout {
        let known = self.blocks.iter().map(|one| BoardBlock { kind: one.kind.slug(), x: one.x, y: one.y, w: one.w, h: one.h, view: one.view.slug().to_string() });
        BoardLayout { blocks: known.chain(unknown.iter().cloned()).collect() }
    }

    pub fn block(&self, kind: K) -> Option<&Placed<K>> {
        self.blocks.iter().find(|one| one.kind == kind)
    }

    /// The kinds that are not on the grid, in the order the grid lists them.
    pub fn missing(&self) -> Vec<K> {
        K::all().iter().copied().filter(|kind| self.block(*kind).is_none()).collect()
    }

    /// The block is taken off the grid, and the rest rise into the room it leaves.
    pub fn hide(&mut self, kind: K) {
        self.blocks.retain(|one| one.kind != kind);
        self.settle(None);
    }

    /// The block goes on the grid in the first cells, reading along each row from the top, that are free for its size.
    pub fn add(&mut self, kind: K) {
        if self.block(kind).is_some() {
            return;
        }
        let mut placed = Placed::new(kind, 0, 0, ADDED_W, ADDED_H, kind.views()[0]);
        'rows: for y in 0..=self.rows() {
            for x in 0..=COLUMNS - ADDED_W {
                placed.x = x;
                placed.y = y;
                if !self.blocks.iter().any(|other| placed.overlaps(other)) {
                    break 'rows;
                }
            }
        }
        self.blocks.push(placed);
    }

    /// The kinds that are not on the grid go below everything that is, in the order given, side by side where they fit.
    pub fn append_below(&mut self, kinds: &[K]) {
        let floor = self.rows();
        for &kind in kinds {
            if self.block(kind).is_some() {
                continue;
            }
            let mut placed = Placed::new(kind, 0, floor, ADDED_W, ADDED_H, kind.views()[0]);
            'rows: for y in floor..=self.rows() {
                for x in 0..=COLUMNS - ADDED_W {
                    placed.x = x;
                    placed.y = y;
                    if !self.blocks.iter().any(|other| placed.overlaps(other)) {
                        break 'rows;
                    }
                }
            }
            self.blocks.push(placed);
        }
    }

    /// The rows the grid is as tall as.
    pub fn rows(&self) -> u8 {
        self.blocks.iter().map(|one| one.y + one.h).max().unwrap_or(0)
    }

    /// The block goes to `(x, y)`, held inside the grid, and the rest settle round it.
    pub fn move_to(&mut self, kind: K, x: i16, y: i16) {
        let Some(at) = self.blocks.iter().position(|one| one.kind == kind) else { return };
        let block = &mut self.blocks[at];
        block.x = x.clamp(0, i16::from(COLUMNS - block.w)) as u8;
        block.y = y.max(0) as u8;
        self.settle(Some(kind));
    }

    /// The block takes `w` columns and `h` rows, between the least and the most and inside the grid.
    pub fn resize_to(&mut self, kind: K, w: i16, h: i16) {
        let Some(at) = self.blocks.iter().position(|one| one.kind == kind) else { return };
        let block = &mut self.blocks[at];
        block.w = w.clamp(i16::from(LEAST_W), i16::from(COLUMNS - block.x)) as u8;
        block.h = h.clamp(i16::from(LEAST_H), i16::from(MOST_H)) as u8;
        self.settle(Some(kind));
    }

    /// One step with the arrows: the block moves by whole cells.
    pub fn nudge(&mut self, kind: K, dx: i16, dy: i16) {
        if let Some(one) = self.block(kind).copied() {
            self.move_to(kind, i16::from(one.x) + dx, i16::from(one.y) + dy);
        }
    }

    /// One step with Shift and the arrows: the block grows or shrinks by whole cells.
    pub fn grow(&mut self, kind: K, dw: i16, dh: i16) {
        if let Some(one) = self.block(kind).copied() {
            self.resize_to(kind, i16::from(one.w) + dw, i16::from(one.h) + dh);
        }
    }

    pub fn set_view(&mut self, kind: K, view: View) {
        if let Some(one) = self.blocks.iter_mut().find(|one| one.kind == kind) {
            if kind.views().contains(&view) {
                one.view = view;
            }
        }
    }

    /// No two blocks overlap and none has room above it: the pinned block keeps its place, and the others rise to the
    /// first row where they fit, in the order they stood.
    pub fn settle(&mut self, pinned: Option<K>) {
        let mut order: Vec<usize> = (0..self.blocks.len()).collect();
        order.sort_by_key(|&at| (Some(self.blocks[at].kind) != pinned, self.blocks[at].y, self.blocks[at].x));
        let mut standing: Vec<Placed<K>> = Vec::new();
        for at in order {
            let mut block = self.blocks[at];
            if Some(block.kind) != pinned {
                block.y = 0;
                while standing.iter().any(|other| block.overlaps(other)) {
                    block.y += 1;
                }
            }
            self.blocks[at] = block;
            standing.push(block);
        }
    }
}
