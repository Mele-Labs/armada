//! What `merge_by: push` may land: only a head whose tree the Job's Checks
//! passed on. `docs/concepts/manifest.md`, *How work lands*.
//!
//! **A tree and not a commit**, because the gate reads a worktree before Fleet
//! commits it: the commit comes later, and carries the same tree.

use alloc::string::String;
use alloc::vec::Vec;

/// Which head [`Delivery::merge_by_push`](crate::Delivery::merge_by_push) may
/// land. Any other is [`NotMerged::Unchecked`](crate::NotMerged::Unchecked),
/// and nothing is pushed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Landable<'a> {
    /// A head carrying this tree, the one the Job's Checks last passed on.
    Checked(&'a str),
    /// None: the Checks have passed on nothing this branch could carry.
    Unchecked,
    /// Any head: the Job's workflow declares no Manifest Check to have read it.
    NothingToCheck,
}

/// A Job's branch as it stands, waiting for its Checks to read it before it is
/// pushed. [`Delivery::the_unchecked_head`](crate::Delivery::the_unchecked_head).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UncheckedHead {
    /// The commit the branch is on, which is what the Checks read.
    pub head: String,
    /// Its tree, which is what is landable once they pass.
    pub tree: String,
    /// Every path changed since the tree the Checks last passed on, and every
    /// path the branch carries over the base, for each Check's `when`.
    pub touched: Vec<String>,
}
