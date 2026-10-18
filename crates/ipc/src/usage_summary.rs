//! What a window of turns cost, summed for the Usage page. `docs/practices/protocol.md`, *The usage
//! summary*.
//!
//! **Burn is in cost units**: input tokens weigh 1, cache-creation 1.25, cache-read 0.1 and output 5,
//! and the sum is divided by a million. The weights live in `store::usage_summary`. Every split below
//! is a partition of `units` except `by_bash_use`, which counts only the turns whose first call was a
//! shell command. **Names and counts only.**

use serde::{Deserialize, Serialize};

/// Units under a name: a model, a tool or a `bash_use` label.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedUnits {
    pub name: String,
    pub units: f64,
}

/// The four token kinds, in units.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUnits {
    pub input: f64,
    pub cache_creation: f64,
    pub cache_read: f64,
    pub output: f64,
}

/// One local day.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DayUnits {
    /// `YYYY-MM-DD` in the caller's local time.
    pub day: String,
    pub units: f64,
    /// Present in a day of the window, empty in the prior period's two days.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_model: Vec<NamedUnits>,
}

/// One slice of the turns: the whole window, one Session, or the Sessions past the list.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UsageBreakdown {
    pub units: f64,
    /// `units` less `subagent_units` is the main sessions' own.
    pub subagent_units: f64,
    /// Units from turns whose context (input, cache-creation and cache-read tokens) is above 200k.
    pub above_200k_units: f64,
    /// And above 300k, which is inside the first.
    pub above_300k_units: f64,
    pub by_token: TokenUnits,
    pub by_model: Vec<NamedUnits>,
    /// By the first tool a turn called; `none` where it called nothing.
    pub by_first_tool: Vec<NamedUnits>,
    /// Turns whose first tool was a shell command, by what it was for.
    pub by_bash_use: Vec<NamedUnits>,
    /// Oldest first; a day with no turn is left out.
    pub by_day: Vec<DayUnits>,
    /// Seven rows, Monday first, each of eight three-hour blocks from local midnight.
    pub hour_grid: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionUsageSummary {
    pub session_id: String,
    /// The most input, cache-creation and cache-read tokens any one of its turns held.
    pub peak_context: u64,
    /// Share of the Session's units that subagents spent, 0 to 1.
    pub subagent_share: f64,
    /// The models it used, most units first.
    pub models: Vec<String>,
    pub usage: UsageBreakdown,
}

/// The Sessions past the listed ones, rolled into one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RestUsage {
    pub sessions: u32,
    pub usage: UsageBreakdown,
}

/// The period of equal length just before `since`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PriorUsage {
    pub since: String,
    pub until: String,
    pub units: f64,
    pub subagent_share: f64,
    pub above_200k_share: f64,
    /// The last whole local day before the window's `until`, and the one before it, whatever the
    /// window's length.
    pub last_day: DayUnits,
    pub day_before: DayUnits,
}

/// `get_usage_summary`: the turns in `[since, until)` summed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageSummary {
    /// The window as resolved, RFC3339 UTC.
    pub since: String,
    pub until: String,
    pub tz_offset_minutes: i32,
    pub total: UsageBreakdown,
    /// The largest Sessions by units, at most thirty, largest first.
    pub sessions: Vec<SessionUsageSummary>,
    /// The Sessions past the thirty; absent where there are none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rest: Option<RestUsage>,
    pub prior: PriorUsage,
}
