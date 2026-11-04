//! `when_base_moves:`, the Commands Fleet runs in the repository's own checkout
//! each time the base branch reaches a new commit there.
//!
//! **Commands, resolved as `setup.requires` resolves them**, so what may run is
//! what the registry declares, and a destructive one or a server is refused for
//! that key's reasons. **Read from the root only**: the checkout these run in is
//! the root's, so a workspace's list would be one nothing reads.

use std::collections::{BTreeMap, BTreeSet};

use super::declared::{Command, Preparation};
use super::referring::named_commands;
use super::{texts, Manifest};
use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// The Commands the file names, in its order. Empty where it names none.
pub(super) fn read(
    top: &mut Table<'_>,
    declares: &BTreeSet<String>,
    commands: &BTreeMap<String, Command>,
    serves: &BTreeSet<String>,
    in_a_workspace: bool,
    out: &mut Vec<Refusal>,
) -> Vec<Preparation> {
    let Some(value) = top.optional("when_base_moves") else {
        return Vec::new();
    };
    if in_a_workspace {
        out.push(Refusal::new("when_base_moves", Fault::RootOnly));
        return Vec::new();
    }
    let Some(items) = yaml::list("when_base_moves", value, out) else {
        return Vec::new();
    };
    named_commands(texts(items, out), declares, commands, serves, out)
        .into_iter()
        .map(|(name, run)| Preparation { name, run })
        .collect()
}

impl Manifest {
    /// What Fleet runs in this repository's checkout once it stands at a new
    /// base commit. Read from the Manifest Fleet started with, not live.
    pub fn when_base_moves(&self) -> &[Preparation] {
        &self.when_base_moves
    }
}
