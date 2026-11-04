//! `connectors:`, whether a hosted Job here may reach the connectors on the
//! person's own account.
//!
//! **On unless the file says `false`**, so a repository that writes nothing is
//! read as it was before the key existed. **Read from the root only**: every
//! Job of the repository starts under it, so a second value in a workspace's
//! file is one nothing reads. Servers the repository names itself are never
//! touched by it.

use super::Manifest;
use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

/// Whether the file leaves the connectors on. A refused value reads as on from
/// here, since a file with any refusal in it does not load.
pub(super) fn read(top: &mut Table<'_>, in_a_workspace: bool, out: &mut Vec<Refusal>) -> bool {
    let Some(value) = top.optional("connectors") else {
        return true;
    };
    if in_a_workspace {
        out.push(Refusal::new("connectors", Fault::RootOnly));
        return true;
    }
    yaml::flag("connectors", value, out).unwrap_or(true)
}

impl Manifest {
    /// Whether a hosted Job in this repository may reach the person's account
    /// connectors. Read from the Manifest Fleet started with, not live.
    pub fn connectors(&self) -> bool {
        self.connectors
    }
}
