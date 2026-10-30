//! What a Job holds, standing in the states of its lifecycles, and the four readers that say where.
//! The design is "Workspaces, lifecycles and rules", *Lifecycles*.
//!
//! **A state is read and never claimed.** [`reading`] turns what a tracker, the forge, a watch and
//! Fleet's own events say into the vocabulary `core_model` reads a state from, and [`holding`] hands
//! it to the instance through `observe`, the only door a standing has. Nothing here takes the word
//! of the agent.
//!
//! **Only a Job started after this existed holds any.** [`keeping`] marks a Job as it starts, and a
//! Job with no mark is never given an instance and never read.
//!
//! **No new loop.** The forge is read where the pull request ledger already reads it, the tracker
//! where the issue ledger already reads it, a watch where its reading is kept, and Fleet's events
//! from the tap that already hears every Job's row change.

pub mod ends;
pub mod holding;
mod keeping;
mod polling;
pub mod reading;
mod serving;
pub(crate) mod taking_on;
pub mod wire;
pub(crate) mod waking;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Lifecycle, ManifestId, Ulid};

use crate::daemon::Fleet;

pub use keeping::keep_the_lifecycles;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The lifecycles that apply to a Job in the repository `manifest_id`: the four Armada carries and
    /// the pieces scoped to it (`lifecycles_applying_to`).
    ///
    /// **The one place that says.** A repository Fleet does not serve has no pieces to read, and
    /// answers the four Armada carries, so a Job outside one holds what it always could.
    pub fn lifecycles_that_apply(&self, manifest_id: &str) -> Vec<Lifecycle> {
        match self.lifecycles_applying_to(&ManifestId::carried(Ulid::carried(manifest_id))) {
            Ok(applying) => applying.running().cloned().collect(),
            Err(_) => Lifecycle::built_in(),
        }
    }
}
