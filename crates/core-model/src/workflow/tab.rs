//! The right-column tabs a workflow step opens, as its `tabs` list names them.
//!
//! The words are the native Bridge's zone ids (`Zone::id` in `crates/bridge-native/src/job_view/side.rs`),
//! and two the column draws that the zone list lacks: `now` and `walks`. core-model cannot see Bridge,
//! so the list is spelled here and a bridge-native test holds every zone id inside it. A child Job's
//! tab is `jobs`, the zone that draws the Jobs this one started.

/// Every id a step's `tabs` may name, in the order the column lists them.
pub const STEP_TABS: &[&str] = &[
    "open",
    "done",
    "deferred",
    "plan",
    "story",
    "changes",
    "pulls",
    "issues",
    "jobs",
    "ledger",
    "files",
    "subagents",
    "checks",
    "workflow",
    "system",
    "bays",
    "spine",
    "judge",
    "artifacts",
    "for_you",
    "qa",
    // Added for the column by step: the Job's current act and the walks it showed.
    "now",
    "walks",
];

/// Whether `id` is a tab a step may open.
pub fn is_step_tab(id: &str) -> bool {
    STEP_TABS.contains(&id)
}
