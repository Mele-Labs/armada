//! `tracker:`, which tracker holds this repository's issues.
//! `docs/concepts/tracker.md`, *Choosing a tracker*.
//!
//! **One key is this crate's and the rest are the adapter's.** `provider` names
//! the implementation, as `docs/contracts/adapters.md` fixes for every adapter;
//! every other key is carried as written, for that implementation to read and
//! refuse. Naming `site` or `project` here would make a new tracker a change to
//! this crate. A provider this build has no word for is the composition root's
//! to answer, so it is not refused here either.
//!
//! **Absent is the tracker the forge keeps**, as before the key existed.

use std::collections::BTreeMap;

use super::Manifest;
use crate::error::Refusal;
use crate::yaml::{self, Table};

/// What `tracker:` said: the provider where it named one, and every other key.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrackerBlock {
    provider: Option<String>,
    keys: BTreeMap<String, String>,
}

impl TrackerBlock {
    /// The provider, where the block names one. `None` is the forge's own.
    pub fn provider(&self) -> Option<&str> {
        self.provider.as_deref()
    }

    /// Every other key, as written.
    pub fn keys(&self) -> &BTreeMap<String, String> {
        &self.keys
    }
}

/// A key that is not text is refused at its own path, and the rest still read.
pub(super) fn read(top: &mut Table<'_>, out: &mut Vec<Refusal>) -> Option<TrackerBlock> {
    let value = top.optional("tracker")?;
    let table = Table::open("tracker", value, out)?;
    let mut block = TrackerBlock::default();
    for (name, item) in table.into_entries() {
        let at = format!("tracker.{name}");
        let Some(text) = yaml::text(&at, item, out) else {
            continue;
        };
        match name.as_str() {
            "provider" => block.provider = Some(text.trim().to_string()),
            _ => {
                block.keys.insert(name, text.trim().to_string());
            }
        }
    }
    Some(block)
}

impl Manifest {
    /// Which tracker this repository's issues are in, where it says.
    pub fn tracker(&self) -> Option<&TrackerBlock> {
        self.tracker.as_ref()
    }
}
