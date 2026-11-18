//! A feature's health on the wire: a mark and the reasons behind it. `docs/concepts/feature-catalog.md`, *Health*.
//!
//! **A mark always carries its reasons**, and a source Fleet does not keep is not guessed: `good` is the mark of a
//! feature no kept evidence speaks against, and its reasons are empty.

use serde::{Deserialize, Serialize};

/// How a feature stands, from what Fleet keeps about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthMark {
    /// Nothing kept speaks against it.
    Good,
    /// Something kept is worth a look.
    Watch,
    /// A check failed in its files, or flaky tests or open bugs piled up.
    Poor,
}

/// What a reason was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthReasonKind {
    /// A flaky test that reaches its files is being fixed. `target` is the test.
    FlakyTest,
    /// A written section is stale. `target` is the pull request's number.
    StaleSection,
    /// The test map holds edges and none reaches its files.
    NoTestReaches,
    /// An open bug is linked to it. `target` is what the bug link names.
    OpenBug,
    /// A Check failed on its last run in a Job that works in its files. `target` is the Job's id.
    CheckFailed,
    /// A lint or type Check failed the same way. `target` is the Job's id.
    AnalyzerFailed,
}

/// One fact behind a mark.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthReason {
    pub kind: HealthReasonKind,
    /// One short sentence, such as `test section stale since #3862`.
    pub fact: String,
    /// What the fact names, where there is something to open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// A feature's mark and why. `good` carries no reasons.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureHealth {
    pub mark: HealthMark,
    pub reasons: Vec<HealthReason>,
}
