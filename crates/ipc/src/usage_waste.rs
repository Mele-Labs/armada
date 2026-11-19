//! The token waste Fleet can name, with its cost and the Jobs it hit, for the Usage page and for an agent.
//! `docs/practices/protocol.md`, *The waste in the burn*.
//!
//! **Units**, weighed as `get_usage_summary` weighs them. No unit is in two kinds, so the kinds add up to
//! `waste_units`, which is no more than `total_units`. `title` and `fix` are Fleet's sentences, so a page
//! and an agent read the same words. **Names and counts only**: a Job is its id.

use serde::{Deserialize, Serialize};

/// The window the waste was read over: `day`, `week` or `fortnight`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WasteWindow {
    pub id: String,
    pub since: String,
    pub until: String,
}

/// One Job's share of a kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WasteJob {
    pub session_id: String,
    pub units: f64,
    pub occurrences: u32,
    /// The most context one of the counted requests held, for the two context kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peak_context: Option<u64>,
}

/// One kind of waste: `subagent_context`, `main_context`, `cold_cache` or `fresh_starts`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WasteKind {
    pub kind: String,
    pub title: String,
    pub fix: String,
    pub units: f64,
    pub occurrences: u32,
    /// The thirty largest Jobs, largest first.
    pub jobs: Vec<WasteJob>,
}

/// `get_usage_waste` (`GET /usage/waste?window=`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageWaste {
    pub until: String,
    pub window: WasteWindow,
    pub total_units: f64,
    pub waste_units: f64,
    /// Largest first; a kind with nothing is left out.
    pub kinds: Vec<WasteKind>,
}
