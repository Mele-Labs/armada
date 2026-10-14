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
    Failed {
        failing: Vec<String>,
        /// How much the failure decides. A merge is refused on any of them; this is for display.
        #[serde(default)]
        weight: FailureWeight,
    },
}

/// How much a failed check decides about the pull request. With a gate named, only the gate does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureWeight {
    /// The gate failed, or there is no gate.
    #[default]
    Required,
    /// A check outside the gate failed and the gate has not finished.
    WhileRunning,
    /// A check outside the gate failed after the gate passed.
    AfterRequired,
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
    /// It is in the base branch's merge queue now.
    #[serde(default)]
    pub queued: bool,
    pub checks: ForgeChecks,
    pub title: String,
    /// The branch it is opened from.
    pub branch: String,
    /// Where it is on the forge.
    pub address: String,
    /// Its description, as Markdown. Absent where the author wrote none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
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

/// Where the forge says a merge would stand against the base branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestMergeable {
    Mergeable,
    Conflicting,
    Unknown,
}

/// What the reviews have come to. `Required` is a review the base branch asks
/// for and none has given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestReviewState {
    Approved,
    ChangesRequested,
    Required,
    None,
}

/// What a review said.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestVerdict {
    Approved,
    ChangesRequested,
    Commented,
}

/// One thing said on the pull request, in the order it was said.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PullRequestEntry {
    Comment {
        author: String,
        at: String,
        body: String,
    },
    Review {
        author: String,
        at: String,
        verdict: PullRequestVerdict,
        body: String,
    },
}

/// How one of the forge's checks stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestCheckState {
    Passed,
    Failed,
    Running,
    Skipped,
}

/// One of the forge's checks, with how long it ran where the forge said.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestCheckRow {
    pub name: String,
    pub state: PullRequestCheckState,
    /// Seconds it ran. Absent while it runs, and where the forge did not say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u64>,
    /// Where the forge keeps its log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

/// How a file changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestFileStatus {
    Added,
    Modified,
    Removed,
    Renamed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestFile {
    pub path: String,
    pub additions: u64,
    pub deletions: u64,
    pub status: PullRequestFileStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestCommit {
    /// The first seven characters.
    pub sha: String,
    pub headline: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// `get_pull_request_detail`: everything the panel's tabs and facts draw that
/// [`PullRequestState`] does not hold. Read when the panel opens, not carried
/// on `session.changed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestPage {
    pub manifest_id: ManifestId,
    pub number: u64,
    /// The branch it merges into.
    pub base: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// RFC 3339.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    pub additions: u64,
    pub deletions: u64,
    pub mergeable: PullRequestMergeable,
    pub review: PullRequestReviewState,
    /// Reviews that approved, counted once per reviewer's latest.
    pub approvals: u64,
    pub conversation: Vec<PullRequestEntry>,
    pub checks: Vec<PullRequestCheckRow>,
    pub files: Vec<PullRequestFile>,
    pub commits: Vec<PullRequestCommit>,
}

/// What the panel's comment box writes. `POST /pull_request_writes/:repository`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestWriteKind {
    Comment,
    Approve,
    RequestChanges,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WritePullRequest {
    /// The pull request's number.
    pub pull_request: u64,
    pub kind: PullRequestWriteKind,
    /// Markdown. Required for `comment` and `request_changes`; optional for
    /// `approve`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// The Session the press came from. **Absent records no Session.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}
