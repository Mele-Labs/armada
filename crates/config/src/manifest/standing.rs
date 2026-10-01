//! `standing_rules:`, the file a repository names as what every change in it
//! carries, which Fleet reads into every Judge brief. A path held to
//! [`artifact_target`]'s rules — one file, inside the checkout — and a file of
//! its own because `manifest.rs` is near the line count the gate refuses.

use super::Manifest;
use crate::error::Refusal;
use crate::workflow::artifact_target;
use crate::yaml::{self, Table};

/// The path, where the file names one. **Absent is `None`**, and every brief
/// is assembled as it was before the key existed.
pub(super) fn read(top: &mut Table<'_>, out: &mut Vec<Refusal>) -> Option<String> {
    top.optional("standing_rules")
        .and_then(|value| yaml::text("standing_rules", value, out))
        .and_then(|path| artifact_target("standing_rules", path, out))
}

impl Manifest {
    /// The repository-relative file every Judge brief carries as what this
    /// repository requires of every change, or `None` where the file names
    /// none. **Not read here**: whether it exists is a fact about a checkout,
    /// and Fleet reads it from the repository's own, never from a Job's
    /// worktree, so a Drone cannot rewrite what its Judge is told.
    pub fn standing_rules(&self) -> Option<&str> {
        self.standing_rules.as_deref()
    }
}
