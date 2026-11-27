//! The block grid the Project Dashboard and the Cockpit home share: six columns of blocks a person adds, moves, resizes and
//! sets a view on. `layout` is the geometry and what Fleet keeps, `chrome` what a block is drawn in; a screen names its own kinds by implementing `GridKind`.

pub mod chrome;
pub mod layout;

pub use layout::{GridKind, Layout, Placed, View, COLUMNS};

#[cfg(test)]
mod tests;
