//! A repository's open issues, as Fleet read them from its tracker. #91,
//! `docs/concepts/tracker.md`.

use serde::{Deserialize, Serialize};

use crate::{Instant, JobId, ManifestId};

/// One open issue. Every string is the tracker's own, unedited.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedIssue {
    pub number: u64,
    pub title: String,
    /// The tracker's address for it: what a dispatch from it sends as the request.
    pub url: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub updated_at: Instant,
    /// The start of its body, whitespace folded.
    pub excerpt: String,
    /// The tracker says a pull request already closes it.
    pub linked_pull_request: bool,
    /// A Job dispatched from it that has not ended, where there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<JobId>,
}

/// `list_issues`' answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueList {
    pub manifest_id: ManifestId,
    /// When the tracker was read for this list.
    pub fetched_at: Instant,
    pub issues: Vec<TrackedIssue>,
    /// Why this list is older than asked for: the tracker would not answer the
    /// read just tried, in its own words. Absent on a fresh list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale: Option<String>,
}
