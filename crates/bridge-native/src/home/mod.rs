//! The blocks of the Cockpit's home: what it draws on its right side while no Job is open. A block is an entity that fills the
//! box it is given and holds what is its own (the Needs cards' scope); the grid that places them, and Fleet's saved layout
//! (`ipc::HomeLayout`, whose kinds are the slugs of [`HomeBlockKind`]), are not here.

mod blocks;
mod needs;
mod placeholder;
#[cfg(test)]
mod tests;

pub use blocks::HomeBlockKind;
pub use needs::{NeedCard, NeedsCards, NeedsScope, ScopeChanged};
pub use placeholder::PlaceholderBlock;

/// The floor plan as a block: `Floor` made with `Floor::plan`, which draws the map or the code map alone, as many columns as its
/// box holds. The Operator's Switch on is not part of it.
pub use crate::features::floor_home::Floor as FloorPlan;
