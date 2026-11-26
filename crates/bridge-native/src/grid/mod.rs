//! The block grid the Project Dashboard and the Cockpit home share: six columns of blocks a person adds, moves, resizes and
//! sets a view on. `layout` is the geometry and what Fleet keeps; a screen names its own kinds by implementing `GridKind`.

pub mod layout;

pub use layout::{GridKind, Layout, Placed, View, ADDED_H, ADDED_W, COLUMNS, LEAST_H, LEAST_W, MOST_H};

#[cfg(test)]
mod tests;
