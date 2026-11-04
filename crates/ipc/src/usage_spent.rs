//! What the burn was spent on, and who spent it, for the Usage page's Spent on view.
//! `docs/practices/protocol.md`, *What the burn was spent on*.
//!
//! **Units throughout**, weighed as `get_usage_summary` weighs them, so a window's whole is the
//! summary's for the same span. Both splits of a window or a Job add up to its whole. **Names and
//! counts only**: a Job is its id, and a client joins it to the title it holds.

use serde::{Deserialize, Serialize};

/// Units by what they went on. `docs/concepts/session/where-the-tokens-went.md` names each part.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpentOn {
    pub start_up: f64,
    pub reads: f64,
    pub own_words: f64,
    pub output: f64,
    pub cold_cache: f64,
    pub announced: f64,
    pub other_results: f64,
    pub messages: f64,
    pub background: f64,
}

/// Units by who spent them: `subagents` is the summary's `subagent_units`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpentBy {
    pub main: f64,
    pub subagents: f64,
    pub background: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpentSplit {
    pub on: SpentOn,
    pub by: SpentBy,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpentJob {
    pub session_id: String,
    pub split: SpentSplit,
}

/// One window up to `until`: `day`, `week` or `fortnight`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpentWindow {
    pub id: String,
    pub since: String,
    pub until: String,
    pub total: SpentSplit,
    /// The thirty largest Jobs by units, largest first.
    pub jobs: Vec<SpentJob>,
}

/// `get_usage_spent` (`GET /usage/spent?until=`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageSpent {
    pub until: String,
    /// Shortest first.
    pub windows: Vec<SpentWindow>,
}
