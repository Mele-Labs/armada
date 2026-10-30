//! A lifecycle is a state machine for one kind of thing a Job holds: a ticket, a pull request, a
//! deploy, a Job it started, or one a person defines. An instance is one such thing, standing in
//! one of its states. Data only: nothing here reads a tracker, a forge, a watch or an event, and
//! nothing runs a rule.
//!
//! **A state is read, never claimed.** Fleet decides which state an instance is in from what it
//! observes, so a state names a fact a reader can see ([`StateRead`]) and an instance's
//! [`Standing`] can only be set from one ([`Lifecycle::standing_of`]). A value no state claims is
//! [`Standing::Unmapped`], its own fact, and never the nearest state.
//!
//! **Applies to Jobs started after it exists.** A Job freezes its lifecycles as it freezes its
//! workflow, and one started before has none and runs as it did.

mod built_in;
mod ids;
mod instance;
mod machine;
mod read;

#[cfg(test)]
mod tests;

pub use ids::{BlankScope, LifecycleId, NotAnId, ScopeId, StateId};
pub use instance::{AlreadyHeld, BlankKey, HeldInstances, InstanceRef, LifecycleInstance, Moved};
pub use machine::{
    Lifecycle, LifecycleScope, LifecycleState, LifecycleTransition, MalformedLifecycle, Standing,
};
pub use read::{FleetFact, ForgeFact, Observed, StateRead, StateSource, TrackerWords};
