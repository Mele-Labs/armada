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

pub(crate) mod acting;
pub mod copying;
pub mod ending;
mod following;
pub mod ends;
pub mod holding;
mod keeping;
pub mod looking;
mod polling;
pub mod reading;
pub mod ruling;
pub mod saying;
mod serving;
pub(crate) mod taking_on;
pub(crate) mod waking;
pub mod wire;
pub mod writing;

use adapter_traits::{AgentHarness, Delivery, TicketState, Vcs, WorkProduct};
use core_model::{Lifecycle, ManifestId, Ulid};
use store::Store;

use crate::daemon::Fleet;
use crate::tracking::choice_of;

pub use following::keep_the_rules;
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

    /// The four Armada carries as the repository `manifest_id` reads them: **the ticket with the words
    /// its tracker declares**, and as carried where it declares none, the repository is not served
    /// or there is no repository. A lifecycle a Workspace or a repository writes replaces the ticket
    /// outright, so the tracker's words are the starting point and never laid over a person's.
    pub(crate) fn built_in_lifecycles(&self, manifest_id: Option<&str>) -> Vec<Lifecycle> {
        let Some(served) = manifest_id.and_then(|id| self.repositories().serving(id)) else {
            return Lifecycle::built_in();
        };
        let declared = self.trackers().ticket_words(&choice_of(served.manifest()));
        let words: Vec<(&str, &str)> = declared.iter().map(|(state, word)| (ticket_state_id(*state), *word)).collect();
        Lifecycle::built_in_reading(&words)
    }

    /// `built_in_lifecycles` for the repository `session` is in.
    pub(crate) fn built_in_lifecycles_of_job(&self, store: &Store, session: &str) -> Vec<Lifecycle> {
        let manifest = store.session(session).ok().flatten().and_then(|one| one.manifest_id);
        self.built_in_lifecycles(manifest.as_deref())
    }
}

/// The state of the ticket Armada carries that a tracker's `state` is. **The one place the two
/// vocabularies meet**: the match has no wildcard, so a state added to either fails to build here.
fn ticket_state_id(state: TicketState) -> &'static str {
    match state {
        TicketState::ToDo => "todo",
        TicketState::InProgress => "in_progress",
        TicketState::InReview => "in_review",
        TicketState::Closed => "closed",
    }
}

#[cfg(test)]
pub(crate) fn ticket_state_ids() -> [&'static str; 4] {
    TicketState::ALL.map(ticket_state_id)
}
