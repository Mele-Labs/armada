//! Cloning one build directory onto another, copy-on-write where the volume does it.
//!
//! The seam a slot's lease clones through: [`CopyOnWrite`] is a seam so a fixture has no volume
//! to clone on, and [`TheVolume`] is the shipped answer. #1064.

mod cloning;

pub use cloning::{CopyOnWrite, NotCloned, TheVolume};
