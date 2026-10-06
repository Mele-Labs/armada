//! Every Check run one repository's Jobs asked for or ran, in one read. Since 23.38.
//!
//! **A read across Jobs and no new record.** The gate's rows and a Drone's asked
//! runs are kept per Job, and a surface that draws Checks for a repository has
//! to see them together. Merge-line Checks are not here (`get_merge_lines` has
//! them) and neither are checkout runs (`list_checkout_runs`).

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, JobId, StepId};
use crate::Requester;

/// `list_manifest_checks`: the newest rows, and whether there were more.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestChecks {
    /// Newest first, at most [`ManifestChecks::MOST`].
    pub rows: Vec<ManifestCheckRow>,
    /// How many rows there were before the cut.
    pub total: u32,
    /// True where `total` is more than `rows` holds. Absent otherwise.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

impl ManifestChecks {
    /// The most rows one answer carries.
    pub const MOST: usize = 200;
}

/// `source` of a row a gate wrote.
pub const SOURCE_GATE: &str = "gate";
/// `source` of a run a Drone asked for.
pub const SOURCE_ASKED_RUN: &str = "asked_run";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestCheckRow {
    /// `gate` for one Check's result at a gate, `asked_run` for one whole run a
    /// Drone asked for. An opaque string, as `Requester::kind` is.
    pub source: String,
    pub requester: Requester,
    pub job_id: JobId,
    /// What a person calls the Job.
    pub job_handle: String,
    pub job_title: String,
    pub step: StepId,
    /// Which run of the step.
    pub attempt: u32,
    /// The group whose gate it ran at, `G1` and on. Absent at a step's own gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// A gate row's Check, or an asked run's Checks joined by `, `.
    pub name: String,
    /// A gate row's outcome (`passed`, `failed`, `signalled`, `timed_out`,
    /// `never_ran`, `skipped`) or an asked run's state (`running`, `passed`,
    /// `failed`, `stopped`, `lost`). Opaque strings.
    pub state: String,
    /// When it started. **Absent on a gate row**, which records when its ruling
    /// was written and not when the Check began.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Instant>,
    /// When it ended: a gate row's write, an asked run's close. Absent while an
    /// asked run goes, and on one nobody saw end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// Start to end, on an asked run that has both. **A gate row has none**: the
    /// gate keeps no duration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took_ms: Option<u64>,
    /// The logs it kept, each openable with `get_check_output` on this row's
    /// `job_id` and the log's `kept`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<ManifestCheckLog>,
    /// The asked run's own id, on an `asked_run` row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asked_run_id: Option<i64>,
}

/// One kept log: whose Check it is and the name `get_check_output` takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestCheckLog {
    pub check: String,
    /// The log file's own name, `get_check_output`'s `:kept`.
    pub kept: String,
}
