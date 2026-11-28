//! What a block of a project's board shows. Where the blocks stand is the shared grid (`crate::grid`); the kinds are the
//! Dashboard's own.

use std::collections::BTreeSet;
use std::sync::Mutex;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::grid::GridKind;
pub use crate::grid::{View, COLUMNS};

pub type Layout = crate::grid::Layout<BlockKind>;
pub type Placed = crate::grid::Placed<BlockKind>;

/// What a block shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockKind {
    System,
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
    pub const ALL: [BlockKind; 7] = [BlockKind::System, BlockKind::Needs, BlockKind::Run, BlockKind::Reviews, BlockKind::Issues, BlockKind::Features, BlockKind::Checks];

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
            BlockKind::System => "System",
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
            BlockKind::System => "system".to_string(),
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
            BlockKind::System => &[View::Graph, View::List],
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

impl GridKind for BlockKind {
    fn all() -> &'static [BlockKind] {
        &BlockKind::ALL
    }

    fn slug(self) -> String {
        BlockKind::slug(self)
    }

    fn from_slug(slug: &str) -> Option<BlockKind> {
        BlockKind::from_slug(slug)
    }

    fn views(self) -> &'static [View] {
        BlockKind::views(self)
    }

    fn title(self) -> &'static str {
        BlockKind::title(self)
    }
}
