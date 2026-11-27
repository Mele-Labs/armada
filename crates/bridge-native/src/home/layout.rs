//! Where the home's blocks stand when Fleet kept no layout, and how a Needs block's scope rides in the one it did keep.
//!
//! **The scope is held in the block's `view` word**, as `<view>:<scope>`, such as `cards:all`; `selected`, the scope a block
//! has unless told otherwise, is left off, so a block never moved off it is saved as plainly as any other. The wire's block is
//! `{ kind, x, y, w, h, view }` and `view` is text Fleet keeps as written, so this crosses with no wire change. A kind of its
//! own (`needs-cards:all`) would make a build that did not read the suffix treat the block as a stranger and draw none; the
//! word on `view` is, to such a build, a view it does not draw, which `Layout::from_wire` swaps for the kind's first.

use ipc::BoardLayout;

use super::{HomeBlockKind, NeedsScope};
use crate::grid::{Layout, Placed, View};

pub type HomeLayout = Layout<HomeBlockKind>;
pub type HomePlaced = Placed<HomeBlockKind>;

/// What the home draws until Fleet has one he saved: the Needs cards across the top, then the System overview and the floor
/// plan each across the whole width. The System overview keeps its panel beside its graph only where it has the room, as on
/// the Project board, where it is full width too.
pub fn preset() -> HomeLayout {
    let mut layout = Layout {
        blocks: vec![
            Placed::new(HomeBlockKind::NeedsCards, 0, 0, 6, 2, View::Cards),
            Placed::new(HomeBlockKind::SystemOverview, 0, 2, 6, 4, View::Graph),
            Placed::new(HomeBlockKind::FloorPlan, 0, 6, 6, 4, View::Picture),
        ],
    };
    layout.settle(None);
    layout
}

/// The layout Fleet kept with the scope taken off each Needs block's `view`, and the scope it named: the first Needs block's.
/// Words this build does not read stay as written, so `Layout::from_wire` treats them as it treats any.
pub fn split(kept: &BoardLayout) -> (BoardLayout, NeedsScope) {
    let mut scope = NeedsScope::default();
    let mut found = false;
    let mut plain = kept.clone();
    for block in plain.blocks.iter_mut().filter(|block| block.kind == HomeBlockKind::NeedsCards.slug()) {
        if let Some((view, word)) = block.view.split_once(':') {
            if !found {
                scope = NeedsScope::from_slug(word).unwrap_or_default();
                found = true;
            }
            block.view = view.to_string();
        }
    }
    (plain, scope)
}

/// What Fleet keeps: the layout as the grid writes it, with a scope that is not the default put on the Needs block's `view`.
pub fn join(mut wire: BoardLayout, scope: NeedsScope) -> BoardLayout {
    if scope != NeedsScope::default() {
        for block in wire.blocks.iter_mut().filter(|block| block.kind == HomeBlockKind::NeedsCards.slug()) {
            block.view = format!("{}:{}", block.view, scope.slug());
        }
    }
    wire
}
