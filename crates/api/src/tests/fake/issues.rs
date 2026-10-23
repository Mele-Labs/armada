//! Issues, as the fake answers them: none, read just now. What Fleet reads and
//! keeps is `fleet::tracking`'s, against a scripted tracker.

use ipc::{
    Instant, IssueComment, IssueList, IssueQueries, IssueQueryGuide, IssueRead, IssueState, LinkedPullRequest,
    LinkedPullRequestState, ManifestId, SaveIssueQuery, SavedIssueQuery, SearchIssues, WireError, WriteIssueQuery,
    WrittenIssueQuery,
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

    /// A query that reads `refuse` is refused with the tracker's words; any other finds nothing.
    async fn search_issues(&self, manifest_id: ManifestId, search: SearchIssues) -> Result<IssueList, Refusal> {
        if search.query.contains("refuse") {
            return Err(refused_query());
        }
        Ok(IssueList {
            manifest_id,
            fetched_at: Instant::carried("2026-10-11T12:00:00Z"),
            issues: Vec::new(),
            stale: None,
        })
    }

    async fn list_issue_queries(&self, manifest_id: ManifestId) -> Result<IssueQueries, Refusal> {
        Ok(IssueQueries {
            manifest_id,
            guide: Some(IssueQueryGuide {
                language: String::from("Fake search"),
                open: String::from("state = open"),
                assigned_to_me: String::from("state = open assignee = me"),
                fields: vec![String::from("Labels")],
            }),
            refused: None,
            saved: vec![saved("q1", "Mine", "state = open assignee = me")],
        })
    }

    async fn write_issue_query(
        &self,
        _manifest_id: ManifestId,
        write: WriteIssueQuery,
    ) -> Result<WrittenIssueQuery, Refusal> {
        Ok(WrittenIssueQuery {
            query: format!("text ~ \"{}\"", write.words),
            row_fields: vec![String::from("Labels")],
            issues: Vec::new(),
            cost_micros: Some(1200),
        })
    }

    async fn save_issue_query(&self, _manifest_id: ManifestId, save: SaveIssueQuery) -> Result<SavedIssueQuery, Refusal> {
        if save.query.contains("refuse") {
            return Err(refused_query());
        }
        Ok(SavedIssueQuery {
            id: save.id.unwrap_or_else(|| String::from("q2")),
            name: save.name,
            words: save.words,
            query: save.query,
            row_fields: save.row_fields,
        })
    }

    async fn forget_issue_query(&self, manifest_id: ManifestId, id: String) -> Result<IssueQueries, Refusal> {
        match id.as_str() {
            "q1" => Ok(IssueQueries { manifest_id, guide: None, refused: Some(String::from("no tool")), saved: Vec::new() }),
            _ => Err(Refusal::NotFound(WireError::raised(
                "fleet.no_such_issue_query",
                format!("the repository has no saved query {id}"),
                crate::tests::shapes::run_id(),
            ))),
        }
    }
}

fn saved(id: &str, name: &str, query: &str) -> SavedIssueQuery {
    SavedIssueQuery {
        id: id.to_string(),
        name: name.to_string(),
        words: String::new(),
        query: query.to_string(),
        row_fields: Vec::new(),
    }
}

fn refused_query() -> Refusal {
    Refusal::Unacceptable(WireError::raised(
        "fleet.issue_query_refused",
        "searching the issues did not happen: unknown field",
        crate::tests::shapes::run_id(),
    ))
}
