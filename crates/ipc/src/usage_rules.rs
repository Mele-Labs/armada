//! The token rules Fleet watches each Job against, and when each told one. `docs/practices/protocol.md`,
//! *The usage rules*. **Names and counts only**: a trip names its Job by id, and a client joins the
//! title it holds.

use serde::{Deserialize, Serialize};

/// A rule's limit as settings.json holds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageRuleLimit {
    pub value: i64,
    /// What `value` counts: `thousand tokens`, `percent` or `turns`.
    pub unit: String,
}

/// A Job told that it crossed a rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageRuleTrip {
    pub session_id: String,
    /// RFC3339: the newest turn when the Job was told.
    pub at: String,
    /// What crossed: tokens for `context`, a percent for `subagents` and `cache_hit`, turns for
    /// `opus_reading`.
    pub figure: f64,
    /// Cost units over the 60 minutes up to `at`.
    pub burn_per_hour_before: f64,
    /// Cost units per hour of the Job's active time over the 60 minutes after `at`. Left out until
    /// those minutes are covered by the Job's turns, and after, where it made fewer than 5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burn_per_hour_after: Option<f64>,
}

/// One rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageRule {
    /// `context`, `subagents`, `cache_hit` or `opus_reading`.
    pub rule: String,
    pub on: bool,
    pub limit: UsageRuleLimit,
    /// In the window, newest first. Empty for a rule that never tripped.
    pub trips: Vec<UsageRuleTrip>,
}

/// The four rules, in that order, over `[since, until)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageRules {
    pub since: String,
    pub until: String,
    pub rules: Vec<UsageRule>,
}
