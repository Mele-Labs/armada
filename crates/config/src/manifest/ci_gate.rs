//! `ci_gate:`, the name of the check on a pull request that decides its `ci`.
//!
//! **Absent is `ci`**, which is what every repository read before the key
//! existed. Where a pull request carries a check of that name only that one
//! decides; where it carries none, every check does.

use super::Manifest;
use crate::error::Refusal;
use crate::yaml::{self, Table};

/// The check name a repository that writes no `ci_gate` is read by.
pub const DEFAULT_CI_GATE: &str = "ci";

/// A refused value reads as the default from here: the refusal is already in `out`.
pub(super) fn read(top: &mut Table<'_>, out: &mut Vec<Refusal>) -> String {
    top.optional("ci_gate")
        .and_then(|value| yaml::text("ci_gate", value, out))
        .map(|name| name.trim().to_string())
        .unwrap_or_else(|| DEFAULT_CI_GATE.to_string())
}

impl Manifest {
    /// The check whose result is a pull request's `ci` in this repository.
    pub fn ci_gate(&self) -> &str {
        &self.ci_gate
    }
}
