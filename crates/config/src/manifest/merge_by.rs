//! `merge_by:`, how Fleet lands a Job's work once a person or `auto_merge` has
//! said it may. `docs/capabilities/merge-line.md`, *The merge*.
//!
//! **Not a third policy.** `auto_merge` and `review_gate` decide *whether*;
//! this decides *how*, so no gate names it and nothing folds it across
//! Manifests: one Job lands in one repository, and that repository's word is
//! the answer.

use super::Manifest;
use crate::error::Refusal;
use crate::yaml::{self, Table};

/// How a repository's work reaches its base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeBy {
    /// The forge merges the pull request. **The default**, so a repository
    /// with branch protection or CI on the forge keeps both in the path.
    #[default]
    Forge,
    /// Fleet makes the `--no-ff` merge commit itself and pushes the base,
    /// never forced.
    Push,
}

const WORDS: &[(&str, MergeBy)] = &[("forge", MergeBy::Forge), ("push", MergeBy::Push)];
const LEGAL: &[&str] = &["forge", "push"];

impl MergeBy {
    /// Exactly as `armada.yml` writes it.
    pub fn as_written(&self) -> &'static str {
        match self {
            MergeBy::Forge => "forge",
            MergeBy::Push => "push",
        }
    }
}

/// What the key came to. A refused value reads as absent from here, for
/// `super::policies::read`'s reason: the refusal is already in `out`.
pub(super) fn read(top: &mut Table<'_>, out: &mut Vec<Refusal>) -> MergeBy {
    top.optional("merge_by")
        .and_then(|value| yaml::word("merge_by", value, WORDS, LEGAL, LEGAL, out))
        .unwrap_or_default()
}

impl Manifest {
    /// How this repository's work lands. **Read through the live cell**, for
    /// `Manifest::auto_merge`'s reason: it is asked at the merge.
    pub fn merge_by(&self) -> MergeBy {
        self.live.read().merge_by
    }
}
