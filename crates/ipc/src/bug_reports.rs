//! `report_bug`: an agent says Armada itself got something wrong. Fleet keeps the report on this
//! Mac and posts it nowhere; Bridge lists, keeps and drops the reports.

use serde::{Deserialize, Serialize};

use crate::{Instant, SessionId};

/// `report_bug`: what went wrong in Armada, and how to see it again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportBug {
    /// One line naming the fault.
    pub title: String,
    /// What went wrong: what the agent did or expected, and what Armada did instead.
    pub what: String,
    /// The steps that led to it, one line each.
    #[serde(default)]
    pub steps: Vec<String>,
    /// The exact error lines, as they were printed.
    #[serde(default)]
    pub errors: Vec<String>,
    /// Paths of log files that hold more.
    #[serde(default)]
    pub logs: Vec<String>,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `report_bug` kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BugReported {
    pub report_id: String,
}

/// One kept report, as `list_bug_reports` and `bug_reports.changed` carry it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BugReport {
    pub id: String,
    /// The Job that reported it.
    pub session_id: String,
    /// The Job's title when it reported; empty where it had none.
    pub job_title: String,
    pub title: String,
    pub what: String,
    pub steps: Vec<String>,
    pub errors: Vec<String>,
    pub logs: Vec<String>,
    pub at: Instant,
    /// When the owner kept it. Absent is not kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept: Option<Instant>,
    /// The pull request that fixed it. Absent while the report stands open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<BugFixed>,
}

/// The merge that fixed a report: a pull request one of its Jobs held. Set once, by the first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BugFixed {
    /// When the pull request merged.
    pub at: Instant,
    pub pull_request: u64,
    pub title: String,
    pub url: String,
}

/// `list_bug_reports`, and the `bug_reports.changed` event: every report on this Mac, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BugReports {
    pub reports: Vec<BugReport>,
}

/// `drop_bug_report`'s answer: the report is gone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BugReportDropped {
    pub id: String,
}
