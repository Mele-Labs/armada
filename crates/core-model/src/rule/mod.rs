//! A lifecycle rule: **When** something happens, **If** the Job and what it holds are so, **Do**
//! these acts, at an autonomy the owner set. Data and a pure engine: nothing here reads a tracker,
//! a forge or a watch, writes anything, or touches a Job, and what a rule plans is a value for
//! Fleet to do or to ask about.
//!
//! **Not the other rules in this codebase.** `ipc::FleetRule` is a notice rule from `fleet.toml`
//! (what Fleet says when it tells a Job something), and Teach proposes those. A lifecycle rule is
//! what the design "Workspaces, lifecycles and rules" calls a Rule, and this module is the only
//! place that name means this.
//!
//! **A condition is one of a closed set, and the set is what Fleet can read.** A rule asking for
//! something outside it does not load, as Teach refuses an `if` it cannot test, so no saved rule
//! waits on a fact nobody can see. The same goes for a rule that names a state its lifecycle lacks
//! ([`Rule::unknown_in`]).
//!
//! **A rule does nothing until it is saved**, and a Job started before rules existed holds no
//! instances, so nothing fires for it. The engine plans from the rules it is handed and holds none
//! of its own.

mod act;
mod autonomy;
mod declared;
mod engine;
mod failure;
mod ids;
mod occurrence;
mod select;
mod when;

#[cfg(test)]
mod tests;

pub use act::{Act, ActKind, ForgeWrite, TrackerWrite};
pub use autonomy::{Autonomy, AutonomyDefaults, Course};
pub use declared::{MalformedRule, Rule, RuleParts, UnknownName};
pub use engine::{Engine, PlannedAct};
pub use failure::{OnFailure, OverTheCap, Retries, Then};
pub use ids::{BlankWords, RuleId, Words};
pub use occurrence::{Happening, Occurrence, OccurrenceKey, RanOnce};
pub use select::{Among, Condition, JobCondition, JobFacts, Quantity, Target};
pub use when::{JobEvent, When};
