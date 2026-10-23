//! Issues, as the fake answers them: none, read just now. What Fleet reads and
//! keeps is `fleet::tracking`'s, against a scripted tracker.

use ipc::{
    Instant, IssueComment, IssueList, IssueRead, IssueState, LinkedPullRequest, LinkedPullRequestState,
    ManifestId, WireError,
};

use super::FakeDaemon;
use crate::{Issues, Refusal};

impl Issues for FakeDaemon {
    async fn list_issues(
        &self,
        manifest_id: ManifestId,
        _refresh: bool,
    ) -> Result<IssueList, Refusal> {
        Ok(IssueList {
            manifest_id,
            fetched_at: Instant::carried("2026-10-11T12:00:00Z"),
            issues: Vec::new(),
            stale: None,
        })
    }

    /// Issue 214 is open and 215 is closed with a comment and a merged pull request, written `214` or `#214`; any other reference is none.
    async fn get_issue(&self, _manifest_id: ManifestId, reference: String) -> Result<IssueRead, Refusal> {
        let number = reference.trim_start_matches('#');
        let state = match number {
            "214" => IssueState::Open,
            "215" => IssueState::Closed,
            _ => {
                return Err(Refusal::NotFound(WireError::raised(
                    "fleet.no_such_issue",
                    format!("the tracker has no issue {reference}"),
                    crate::tests::shapes::run_id(),
                )))
            }
        };
        Ok(IssueRead {
            reference: format!("#{number}"),
            title: String::from("A title"),
            url: format!("https://forge.invalid/o/r/issues/{number}"),
            state,
            body: String::from("The whole body"),
            author: String::from("nick"),
            opened_at: Instant::carried("2026-10-01T09:00:00Z"),
            labels: vec![String::from("bug")],
            assignees: Vec::new(),
            comments: match state {
                IssueState::Closed => vec![IssueComment {
                    author: String::from("sam"),
                    at: Instant::carried("2026-10-02T10:00:00Z"),
                    body: String::from("Seen here too"),
                }],
                IssueState::Open => Vec::new(),
            },
            linked_pull_requests: match state {
                IssueState::Closed => vec![LinkedPullRequest {
                    number: 230,
                    title: String::from("Fix it"),
                    url: String::from("https://forge.invalid/o/r/pull/230"),
                    state: LinkedPullRequestState::Merged,
                }],
                IssueState::Open => Vec::new(),
            },
            fields: Vec::new(),
            holders: Vec::new(),
        })
    }
}
