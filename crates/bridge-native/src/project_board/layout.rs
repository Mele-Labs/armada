//! Where each block of a project's board stands: a plain struct a project keeps, so Fleet can hold it later and Bridge
//! send it as it is. Nothing here touches the screen.
//!
//! The grid is six columns wide and as tall as its blocks. A move or a resize pins the block it is about and settles the
//! rest around it: every other block rises as far as it can and drops below whatever stands in its way.

use std::collections::BTreeSet;
use std::sync::Mutex;

use ipc::{BoardBlock, BoardLayout};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The columns of the grid.
pub const COLUMNS: u8 = 6;

/// The least a block is drawn at, in columns and rows.
pub const LEAST_W: u8 = 2;
pub const LEAST_H: u8 = 1;

/// The most rows a block takes.
pub const MOST_H: u8 = 6;

/// What a block shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockKind {
    Needs,
    Run,
    Reviews,
    Issues,
    Features,
    Checks,
    /// One live metric of the project, by its recipe's id. The id is interned, so the kind stays `Copy` like the others.
    Metric(&'static str),
}

/// What Fleet's word for a metric block starts with: `metric:<id>`.
const METRIC_PREFIX: &str = "metric:";

/// A recipe's id as a string that lives as long as the program. Each id is leaked once, however often it is read.
fn interned(id: &str) -> &'static str {
    static HELD: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());
    let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(known) = held.get(id) {
        return known;
    }
    let leaked: &'static str = Box::leak(id.to_string().into_boxed_str());
    held.insert(leaked);
    leaked
}

impl Serialize for BlockKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.slug())
    }
}

impl<'de> Deserialize<'de> for BlockKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let slug = String::deserialize(deserializer)?;
        BlockKind::from_slug(&slug).ok_or_else(|| serde::de::Error::custom(format!("no block kind is spelled {slug}")))
    }
}

impl BlockKind {
    /// The kinds Bridge draws whatever the project keeps. A metric block stands on a recipe, so it is not among them.
    pub const ALL: [BlockKind; 6] = [BlockKind::Needs, BlockKind::Run, BlockKind::Reviews, BlockKind::Issues, BlockKind::Features, BlockKind::Checks];

    /// The metric block of the recipe `id`.
    pub fn metric(id: &str) -> BlockKind {
        BlockKind::Metric(interned(id))
    }

    /// The recipe a metric block shows.
    pub fn metric_id(self) -> Option<&'static str> {
        match self {
            BlockKind::Metric(id) => Some(id),
            _ => None,
        }
    }

    /// The words on the block's header and the name VoiceOver reads. A metric block is named by its recipe's title, which the
    /// board holds (`Board::title_of`); the id stands here for one it does not hold.
    pub fn title(self) -> &'static str {
        match self {
            BlockKind::Needs => "Needs you",
            BlockKind::Run => "Run",
            BlockKind::Reviews => "Reviews",
            BlockKind::Issues => "Issues",
            BlockKind::Features => "Features",
            BlockKind::Checks => "Checks and commands",
            BlockKind::Metric(id) => id,
        }
    }

    /// What its element id is made of, and the word Fleet keeps for it.
    pub fn slug(self) -> String {
        match self {
            BlockKind::Needs => "needs-you".to_string(),
            BlockKind::Run => "run".to_string(),
            BlockKind::Reviews => "reviews".to_string(),
            BlockKind::Issues => "issues".to_string(),
            BlockKind::Features => "features".to_string(),
            BlockKind::Checks => "checks".to_string(),
            BlockKind::Metric(id) => format!("{METRIC_PREFIX}{id}"),
        }
    }

    /// The kind Fleet's word names, or none for one a later Bridge adds. `metric:<id>` is a kind whether or not the project
    /// still keeps a recipe by that id: the board decides what to draw, and the block goes back as it came.
    pub fn from_slug(slug: &str) -> Option<BlockKind> {
        if let Some(id) = slug.strip_prefix(METRIC_PREFIX) {
            return (!id.is_empty()).then(|| BlockKind::metric(id));
        }
        BlockKind::ALL.into_iter().find(|kind| kind.slug() == slug)
    }

    /// The views its data fits, the first being the one it starts in.
    pub fn views(self) -> &'static [View] {
        match self {
            BlockKind::Needs => &[View::List, View::Cards],
            BlockKind::Run => &[View::List, View::Table, View::Cards],
            BlockKind::Reviews => &[View::Cards, View::List, View::Table],
            BlockKind::Issues => &[View::Table, View::List, View::Cards],
            BlockKind::Features => &[View::Cards, View::List, View::Picture],
            BlockKind::Checks => &[View::List, View::Table],
            // Picture is offered only where the reading carries one, which the kind cannot tell (`metric::offered`).
            BlockKind::Metric(_) => &[View::List, View::Table, View::Cards, View::Graph, View::Picture],
        }
    }
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

/// One block of the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placed {
    pub kind: BlockKind,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
    pub view: View,
}

impl Placed {
    pub const fn new(kind: BlockKind, x: u8, y: u8, w: u8, h: u8, view: View) -> Placed {
        Placed { kind, x, y, w, h, view }
    }

    pub(super) fn overlaps(&self, other: &Placed) -> bool {
        self.x < other.x + other.w && other.x < self.x + self.w && self.y < other.y + other.h && other.y < self.y + self.h
    }
}

/// A project's board.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    pub blocks: Vec<Placed>,
}

/// How big a block is drawn when it is added, in columns and rows: the size the presets give each.
const ADDED_W: u8 = 3;
const ADDED_H: u8 = 2;

impl Layout {
    /// The board Fleet kept, and the blocks of it this build has no kind for, which go back as they came.
    /// A block of a kind that stands twice keeps its first, one of a view its kind does not draw takes the kind's first, and
    /// a size or place outside the grid is brought inside it. The blocks are then settled, as every edit leaves them.
    pub fn from_wire(kept: &BoardLayout) -> (Layout, Vec<BoardBlock>) {
        let mut layout = Layout { blocks: Vec::new() };
        let mut unknown = Vec::new();
        for block in &kept.blocks {
            let Some(kind) = BlockKind::from_slug(&block.kind) else {
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

    /// What Fleet keeps for this board: its blocks, then the ones of kinds this build does not know.
    pub fn to_wire(&self, unknown: &[BoardBlock]) -> BoardLayout {
        let known = self.blocks.iter().map(|one| BoardBlock { kind: one.kind.slug(), x: one.x, y: one.y, w: one.w, h: one.h, view: one.view.slug().to_string() });
        BoardLayout { blocks: known.chain(unknown.iter().cloned()).collect() }
    }

    pub fn block(&self, kind: BlockKind) -> Option<&Placed> {
        self.blocks.iter().find(|one| one.kind == kind)
    }

    /// The kinds that are not on the board, in the order the board lists them.
    pub fn missing(&self) -> Vec<BlockKind> {
        BlockKind::ALL.into_iter().filter(|kind| self.block(*kind).is_none()).collect()
    }

    /// The block is taken off the board, and the rest rise into the room it leaves.
    pub fn hide(&mut self, kind: BlockKind) {
        self.blocks.retain(|one| one.kind != kind);
        self.settle(None);
    }

    /// The block goes on the board in the first cells, reading along each row from the top, that are free for its size.
    pub fn add(&mut self, kind: BlockKind) {
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

    /// The kinds that are not on the board go below everything that is, in the order given, side by side where they fit.
    pub fn append_below(&mut self, kinds: &[BlockKind]) {
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

    /// The rows the board is as tall as.
    pub fn rows(&self) -> u8 {
        self.blocks.iter().map(|one| one.y + one.h).max().unwrap_or(0)
    }

    /// The block goes to `(x, y)`, held inside the grid, and the rest settle round it.
    pub fn move_to(&mut self, kind: BlockKind, x: i16, y: i16) {
        let Some(at) = self.blocks.iter().position(|one| one.kind == kind) else { return };
        let block = &mut self.blocks[at];
        block.x = x.clamp(0, i16::from(COLUMNS - block.w)) as u8;
        block.y = y.max(0) as u8;
        self.settle(Some(kind));
    }

    /// The block takes `w` columns and `h` rows, between the least and the most and inside the grid.
    pub fn resize_to(&mut self, kind: BlockKind, w: i16, h: i16) {
        let Some(at) = self.blocks.iter().position(|one| one.kind == kind) else { return };
        let block = &mut self.blocks[at];
        block.w = w.clamp(i16::from(LEAST_W), i16::from(COLUMNS - block.x)) as u8;
        block.h = h.clamp(i16::from(LEAST_H), i16::from(MOST_H)) as u8;
        self.settle(Some(kind));
    }

    /// One step with the arrows: the block moves by whole cells.
    pub fn nudge(&mut self, kind: BlockKind, dx: i16, dy: i16) {
        if let Some(one) = self.block(kind).copied() {
            self.move_to(kind, i16::from(one.x) + dx, i16::from(one.y) + dy);
        }
    }

    /// One step with Shift and the arrows: the block grows or shrinks by whole cells.
    pub fn grow(&mut self, kind: BlockKind, dw: i16, dh: i16) {
        if let Some(one) = self.block(kind).copied() {
            self.resize_to(kind, i16::from(one.w) + dw, i16::from(one.h) + dh);
        }
    }

    pub fn set_view(&mut self, kind: BlockKind, view: View) {
        if let Some(one) = self.blocks.iter_mut().find(|one| one.kind == kind) {
            if kind.views().contains(&view) {
                one.view = view;
            }
        }
    }

    /// No two blocks overlap and none has room above it: the pinned block keeps its place, and the others rise to the
    /// first row where they fit, in the order they stood.
    pub fn settle(&mut self, pinned: Option<BlockKind>) {
        let mut order: Vec<usize> = (0..self.blocks.len()).collect();
        order.sort_by_key(|&at| (Some(self.blocks[at].kind) != pinned, self.blocks[at].y, self.blocks[at].x));
        let mut standing: Vec<Placed> = Vec::new();
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
