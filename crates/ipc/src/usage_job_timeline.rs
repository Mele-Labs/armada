//! One Job's burn over its life, for the page that draws it and for an agent.
//! `docs/practices/protocol.md`, *A Job's burn over time*.
//!
//! **Units**, weighed as `get_usage_summary` weighs them. Every stamp is RFC3339. `label` is Fleet's words for what a
//! pause waited on, so a page and an agent read the same ones. **Names and counts only.**

use serde::{Deserialize, Serialize};

/// The burn of one bucket of time, split by who spent it. A compaction's own request is the main thread's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelineBucket {
    /// Where the bucket begins.
    pub at: String,
    pub main_units: f64,
    pub subagent_units: f64,
}

/// The main thread's context at one request: its input, cache writes and cache read, in tokens.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelineContext {
    pub at: String,
    pub tokens: u64,
}

/// A gap of more than five minutes between two main-thread requests.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelinePause {
    pub from: String,
    pub to: String,
    /// `you`, `command`, `subagent` or `other`.
    pub waited_on: String,
    /// Fleet's words for `waited_on`: `Waited on you`, `Command ran`, `Subagent ran` or `Paused`.
    pub label: String,
    /// The request after the pause found the cache cold.
    pub cold: bool,
}

/// `get_job_timeline` (`GET /usage/job_timeline?session_id=`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobTimeline {
    pub session_id: String,
    pub started: String,
    /// Absent while the Job runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<String>,
    /// 60, 300, 600, 900, 1800 or 3600.
    pub bucket_seconds: u32,
    pub total_units: f64,
    /// Every Job's burn over the seven days ending at `ended`, or now.
    pub week_units: f64,
    /// Every bucket from `started` to the end of the span, oldest first, one with no burn included.
    pub buckets: Vec<TimelineBucket>,
    /// The main thread, a point per request, thinned to at most 400.
    pub context: Vec<TimelineContext>,
    pub compactions: Vec<String>,
    pub pauses: Vec<TimelinePause>,
    pub peak_context: u64,
    pub cold_wakes: u32,
    /// The part of `total_units` subagents burned, 0 to 1.
    pub subagent_share: f64,
}
