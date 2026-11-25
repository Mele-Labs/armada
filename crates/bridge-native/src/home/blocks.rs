//! The kinds of block the Cockpit's home can hold, shaped as the Project Dashboard's `BlockKind` is so the one grid can take either.

use crate::look::{token, Token};

/// What a home block shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HomeBlockKind {
    NeedsCards,
    FloorPlan,
    SystemOverview,
    SystemStats,
}

impl HomeBlockKind {
    pub const ALL: [HomeBlockKind; 4] = [HomeBlockKind::NeedsCards, HomeBlockKind::FloorPlan, HomeBlockKind::SystemOverview, HomeBlockKind::SystemStats];

    /// The words on the block's header and the name VoiceOver reads.
    pub fn title(self) -> &'static str {
        match self {
            HomeBlockKind::NeedsCards => "Needs you",
            HomeBlockKind::FloorPlan => "Floor plan",
            HomeBlockKind::SystemOverview => "System overview",
            HomeBlockKind::SystemStats => "System stats",
        }
    }

    /// What its element id is made of, and the word Fleet keeps for it in `HomeLayout`.
    pub fn slug(self) -> &'static str {
        match self {
            HomeBlockKind::NeedsCards => "needs-cards",
            HomeBlockKind::FloorPlan => "floor-plan",
            HomeBlockKind::SystemOverview => "system-overview",
            HomeBlockKind::SystemStats => "system-stats",
        }
    }

    /// The kind Fleet's word names, or none for one a later Bridge adds.
    pub fn from_slug(slug: &str) -> Option<HomeBlockKind> {
        HomeBlockKind::ALL.into_iter().find(|kind| kind.slug() == slug)
    }

    /// The glyph beside the title on the block's header.
    pub fn mark(self) -> &'static str {
        match self {
            HomeBlockKind::NeedsCards => "bell",
            HomeBlockKind::FloorPlan => "library",
            HomeBlockKind::SystemOverview => "layout-dashboard",
            HomeBlockKind::SystemStats => "activity",
        }
    }

    /// The hue the header band and the glyph wear.
    pub fn hue(self) -> Token {
        match self {
            HomeBlockKind::NeedsCards => token::YOU,
            HomeBlockKind::FloorPlan => token::ASK_FEATURES,
            HomeBlockKind::SystemOverview => token::SKETCH_TEAL,
            HomeBlockKind::SystemStats => token::SKETCH_GREY,
        }
    }
}
