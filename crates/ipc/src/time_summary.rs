//! Where Jobs' time went, summed for the Usage page and read by a Job about itself.
//! `docs/practices/protocol.md`, *Where a Job's time went*.
//!
//! **A split is [`RecordSplit`], milliseconds, wherever it appears**; the four medians of an hour or
//! a build are seconds. A figure nobody produced is absent, never `0`. **Names and counts only.**

use serde::{Deserialize, Serialize};

use crate::building::BuildSource;
use crate::retro::RecordSplit;
use crate::usage_summary::NamedUnits;

/// The usual Job of a workflow: the median milliseconds in each bucket over every kept Job of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsualTime {
    pub workflow: String,
    /// How many kept Jobs of the workflow the medians are of.
    pub jobs: u32,
    pub split: RecordSplit,
}

/// A Job's whole time, kept when it ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSplit {
    /// Empty where the Job ran no named workflow.
    pub workflow: String,
    pub began_at: String,
    pub ended_at: String,
    /// The buckets sum to `ended_at` less `began_at`.
    pub spent: RecordSplit,
    /// Absent where the Job has no workflow name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usual: Option<UsualTime>,
}

/// `get_job_time`: one Job's split against the usual of its workflow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobTime {
    pub session_id: String,
    /// Absent where none is kept: the Job is still running, or ended before splits were kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<JobSplit>,
}

/// One of the longest Jobs of a window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlowJob {
    pub session_id: String,
    /// Absent where Fleet holds no title for the Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub workflow: String,
    pub began_at: String,
    pub ended_at: String,
    pub split: RecordSplit,
}

/// `get_usage_time`: the Jobs that ended in `[since, until)`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageTime {
    /// The window as resolved, RFC3339 UTC.
    pub since: String,
    pub until: String,
    /// How many Jobs ended in it.
    pub jobs: u32,
    /// Every Job's split, summed.
    pub total: RecordSplit,
    /// The longest first.
    pub slowest: Vec<SlowJob>,
    /// By workflow, alphabetically, from every Job kept of it and not only this window's.
    pub usual: Vec<UsualTime>,
}

/// One hour that had something in it. Seconds in the four medians; each is absent where the hour
/// produced none, so a line breaks there and does not fall to zero.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HourFigures {
    /// Hours from the window's `since`, from 0.
    pub hour: u32,
    /// The hour's start, RFC3339 UTC.
    pub at: String,
    /// Cost units by model, most first; absent where no turn was taken in the hour.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub burn: Vec<NamedUnits>,
    /// The wait on the model per turn of the main thread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<f64>,
    /// The time the tools of a turn ran, per turn that called any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<f64>,
    /// A Check run's time, queued or running.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks: Option<f64>,
    /// A pull request's wait in CI and the merge queue with nothing else running.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<f64>,
}

/// One build Fleet ran and what ran on it. The four medians are as on [`HourFigures`], over the
/// build's whole run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildFigures {
    /// The build's own commit, its first seven characters. A commit run twice has two builds, told
    /// apart by `started`.
    pub commit: String,
    pub started: String,
    /// When the next build began, which is where this one's run ends; absent while it runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    /// Absent where git had not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<BuildSource>,
    /// The main commit under it, seven characters; absent where git had not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_commit: Option<String>,
    /// What a preview holds; empty for a build on main.
    pub branches: Vec<String>,
    /// Every turn kept while it ran, subagents' included.
    pub turns: u64,
    /// **Cost units per turn, not millions of them**: the build's summed units, which are millions, times
    /// a million over `turns`. Absent where the build took no turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burn: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<f64>,
    /// The Jobs that took a turn on it.
    pub jobs: Vec<String>,
    /// The time of every Job in the stretch it ran, cut at its start.
    pub split: RecordSplit,
}

/// `get_usage_hours`: the hours from `since`, and the builds that ran in them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsageHours {
    /// The first hour's start and the end of the last, RFC3339 UTC.
    pub since: String,
    pub until: String,
    /// Oldest first. **An hour in which nothing ran is left out**: place each by `hour`.
    pub hours: Vec<HourFigures>,
    /// The builds that ran at any time since `since`, the one running at `since` among them, oldest first.
    pub builds: Vec<BuildFigures>,
}
