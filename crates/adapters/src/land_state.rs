//! What `armada land` keeps on disk, under `armada-land/` in a clone's common
//! git directory: the queue, each branch's outcome, and the stamps beside them.
//! `docs/capabilities/merge-line.md`.
//!
//! **Here rather than in `armada`, so Fleet reads the same files with the same
//! types.** The binary writes them and Fleet only reads, through [`line`], which
//! creates nothing and starts no runner.

pub mod codec;
pub mod dir;
pub mod line;
pub mod outcome;
pub mod queue;
