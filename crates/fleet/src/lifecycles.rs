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

pub mod holding;
mod keeping;
mod polling;
pub mod reading;
pub(crate) mod taking_on;

use core_model::Lifecycle;

pub use keeping::keep_the_lifecycles;

/// The lifecycles that apply to a Job in the repository `manifest_id`.
///
/// **The one place that says.** Today every Job gets the four Armada carries. The work that keeps
/// lifecycles as pieces, scoped to a Workspace or a repository, replaces this body, and nothing
/// else in this module changes.
pub fn lifecycles_that_apply(manifest_id: &str) -> Vec<Lifecycle> {
    let _ = manifest_id;
    Lifecycle::built_in()
}
