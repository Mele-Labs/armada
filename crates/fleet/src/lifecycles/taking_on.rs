//! What a Job takes on, read off the rows its ledger keeps: the tickets it works, the pull requests
//! it holds and which tickets each is for, and the Job that started it.
//!
//! **The rows are Fleet's own bookkeeping and never the agent's word**: an issue is on the ledger
//! because it was named or claimed through a tool Fleet heard, and a pull request because the forge
//! showed its branch. A row given back is not taken on.

use ipc::{Attachment, AttachmentState};

use crate::session_issues::{pull_requests_named, ISSUE};

use crate::pull_requesting::PR;

/// A Session started by another carries this row, whose target is the parent.
const STARTED_BY: &str = "started_by";

/// A pull request the Job holds, and the tickets it is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullRequestHeld {
    pub number: String,
    pub tickets: Vec<String>,
}

/// What a Job's ledger says it holds, in the order the rows are kept.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Holds {
    pub tickets: Vec<String>,
    pub pull_requests: Vec<PullRequestHeld>,
}

/// The tickets and pull requests a Job's ledger rows name.
pub fn holds(attachments: &[Attachment]) -> Holds {
    let live = |row: &&Attachment| row.state != AttachmentState::GivenBack;
    let issues: Vec<&Attachment> = attachments.iter().filter(|row| row.kind == ISSUE).filter(live).collect();
    let pull_requests = attachments
        .iter()
        .filter(|row| row.kind == PR)
        .filter(live)
        .map(|row| PullRequestHeld {
            number: row.target.clone(),
            tickets: issues
                .iter()
                .filter(|issue| {
                    row.target
                        .parse::<u64>()
                        .is_ok_and(|number| pull_requests_named(issue.detail.get("pull_requests")).contains(&number))
                })
                .map(|issue| issue.target.clone())
                .collect(),
        })
        .collect();
    Holds { tickets: issues.iter().map(|issue| issue.target.clone()).collect(), pull_requests }
}

/// The Job that started this one.
pub fn started_by(attachments: &[Attachment]) -> Option<&str> {
    attachments.iter().find(|row| row.kind == STARTED_BY).map(|row| row.target.as_str())
}

/// Whether the Job's pull request has merged.
pub fn landed(attachments: &[Attachment]) -> bool {
    attachments
        .iter()
        .any(|row| row.kind == PR && row.detail.get("state").map(String::as_str) == Some("merged"))
}

/// Whether the Job holds a pull request at all.
pub fn opened_pull_request(attachments: &[Attachment]) -> bool {
    attachments.iter().any(|row| row.kind == PR && row.state != AttachmentState::GivenBack)
}
