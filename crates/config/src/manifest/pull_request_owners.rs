//! `policy.pull_request_owners`, the switch that puts this repository's claims
//! and pull request owners in the central store.
//! `git show ecfffb2c1ebd:docs/capabilities/merge-line.md`,
//! *Fixing a red across machines*.
//!
//! **Off unless the file says `true`**, and read from the root only: the claim
//! is on the repository, so a second value in a workspace's file is one nothing
//! reads. **A `policy:` table of one key**, not a top-level flag, because the
//! other switches that change what other machines see belong beside it.

use super::Manifest;
use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// The keys read inside `policy`.
const POLICY_KEYS: &[&str] = &["pull_request_owners"];

/// Whether the file turns the switch on. A refused value reads as off from
/// here, since a file with any refusal in it does not load.
pub(super) fn read(top: &mut Table<'_>, in_a_workspace: bool, out: &mut Vec<Refusal>) -> bool {
    let Some(mut table) = top.optional("policy").and_then(|value| Table::open("policy", value, out)) else {
        return false;
    };
    let on = match table.optional("pull_request_owners") {
        None => false,
        Some(_) if in_a_workspace => {
            out.push(Refusal::new(table.at("pull_request_owners"), Fault::RootOnly));
            false
        }
        Some(value) => yaml::flag(&table.at("pull_request_owners"), value, out).unwrap_or(false),
    };
    table.close(POLICY_KEYS, out);
    on
}

impl Manifest {
    /// Whether this repository's claims and pull request owners go to the
    /// central store. **Read through the live cell**, so turning it off is
    /// answered at the next take rather than at a restart.
    pub fn pull_request_owners(&self) -> bool {
        self.live.read().pull_request_owners
    }
}
