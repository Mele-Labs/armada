//! A pull request on the wire: what it is, and the acts a person takes on one
//! from a Session. Since protocol 23.48. `docs/concepts/session.md`, *Acts on a
//! pull request*.
//!
//! **A pull request is named by its repository and its number**, the pair the
//! session ledger's `pr` row already holds, and nothing here is a Job's. The
//! three writes are a person's ask in Helm's terms (`pushes to shared`), and each
//! answers with the pull request as the forge shows it afterwards.

use serde::{Deserialize, Serialize};

use crate::ids::ManifestId;
use crate::sessions::SessionId;
use crate::JobId;

/// Where a pull request stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestStanding {
    /// Open and not yet ready for review. The forge will not merge one.
    Draft,
    Open,
    Merged,
    /// Closed and never merged.
    Closed,
}

/// What the forge's own checks have come to.
///
/// **Not Armada's Checks**, which are Fleet's and run in a worktree it made.
/// A repository that runs nothing reads `pending`: nothing passed, so nothing
/// may be said to have.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ForgeChecks {
    /// Some have not finished, and none has failed.
    Pending,
    Passed,
    /// At least one did not pass, named as the forge names it.
    Failed { failing: Vec<String> },
}

/// One pull request as the forge shows it now. `get_pull_request`, and what
/// each act answers with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestState {
    pub manifest_id: ManifestId,
    pub number: u64,
    pub state: PullRequestStanding,
    /// The forge will merge it when its required checks pass.
    pub auto_merge: bool,
    pub checks: ForgeChecks,
    pub title: String,
    /// The branch it is opened from.
    pub branch: String,
    /// Where it is on the forge.
    pub address: String,
}

/// What `review_pull_request` is asked. `POST /pull_requests/:repository/review`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewPullRequest {
    /// The pull request's number, or its address.
    pub pull_request: String,
    /// The Session the press came from. **Absent is a press from anywhere
    /// else**, and the Job then records no Session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `review_pull_request` made: a Code Review Job at the approval gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewDispatched {
    pub job_id: JobId,
    /// The pull request it reviews, as an address.
    pub address: String,
    /// The Session that asked, where one did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}
