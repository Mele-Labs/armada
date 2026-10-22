//! `test_map.shared`, whether this repository's test map goes through the
//! central store or stays with each machine. `docs/concepts/test-map.md`,
//! *What reads it*.
//!
//! **Not shared unless the file says `true`**, so a repository that writes
//! nothing keeps the map it has on each machine. **A `test_map:` table of one
//! key**, because the rest of what a repository says about its map belongs
//! beside this. **Read from the root only**: the map is the repository's, so a
//! second value in a workspace's file is one nothing reads.

use super::Manifest;
use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// The keys read inside `test_map`.
const TEST_MAP_KEYS: &[&str] = &["shared"];

/// Whether the file says the map is shared. A refused value reads as not
/// shared from here, since a file with any refusal in it does not load.
pub(super) fn read(top: &mut Table<'_>, in_a_workspace: bool, out: &mut Vec<Refusal>) -> bool {
    let Some(value) = top.optional("test_map") else {
        return false;
    };
    if in_a_workspace {
        out.push(Refusal::new("test_map", Fault::RootOnly));
        return false;
    }
    let Some(mut table) = Table::open("test_map", value, out) else {
        return false;
    };
    let shared = table
        .optional("shared")
        .and_then(|value| yaml::flag(&table.at("shared"), value, out))
        .unwrap_or(false);
    table.close(TEST_MAP_KEYS, out);
    shared
}

impl Manifest {
    /// Whether this repository's test map is shared through the central store.
    /// Read from the Manifest Fleet started with, not live.
    pub fn test_map_shared(&self) -> bool {
        self.test_map_shared
    }
}
