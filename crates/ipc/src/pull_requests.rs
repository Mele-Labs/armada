//! A pull request on the wire: what it is, and the acts a person takes on one
//! from a Session. Since protocol 23.48. `docs/concepts/session.md`, *Acts on a
//! pull request*.
//!
//! **A pull request is named by its repository and its number**, the pair the
//! session ledger's `pr` row already holds, and nothing here is a Job's. The
//! three writes are a person's ask (`pushes to shared`), and each
//! answers with the pull request as the forge shows it afterwards.

use serde::{Deserialize, Serialize};

use crate::ids::ManifestId;
use crate::sessions::SessionId;

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
    /// The branch it merges into. Absent where the forge named none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub base: String,
    /// Where it is on the forge.
    pub address: String,
    /// Its description, as Markdown. Absent where the author wrote none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// The issues the forge says it closes when it merges, by number, ascending. Absent where none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closes: Vec<u64>,
}

/// Where the forge says a merge would stand against the base branch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestMergeable {
    Mergeable,
    Conflicting,
    #[default]
    Unknown,
}

/// What the reviews have come to. `Required` is a review the base branch asks
/// for and none has given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestReviewState {
    Approved,
    ChangesRequested,
    Required,
    #[default]
    None,
}

/// A description a Job proposed with `propose_draft` for a pull request it holds, waiting on the
/// person. Fleet links it where the draft's `before` is the pull request's body, whitespace aside,
/// or where the Job named the pull request. **Gone once the draft is settled**, however it was.
/// Carried on `SessionRecord::pull_request_drafts` and `PullRequestPage::draft`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestDraft {
    pub manifest_id: ManifestId,
    pub number: u64,
    /// The draft's row id, as its `draft:<id>` item names it. Approve, Edit and Note answer that item.
    pub draft: String,
    /// The proposed description, in markdown.
    pub text: String,
    /// The description it replaces, as the Job gave it. Absent where it gave none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// 1 for the Job's first draft for this pull request, and one more for each it proposed since.
    pub revision: u32,
}

/// What a review said.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestVerdict {
    Approved,
    ChangesRequested,
    Commented,
}

/// Whether a reviewer is somebody or a team.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestReviewerKind {
    Person,
    Team,
}

/// Somebody asked to review, or who reviewed and is no longer asked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestReviewer {
    /// A person's login, or a team's slug.
    pub login: String,
    pub kind: PullRequestReviewerKind,
    /// Their latest review. Absent where they have not reviewed, and for a team.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<PullRequestVerdict>,
    /// A person's full name, where their profile has one. Absent for a team.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A person's profile address. Absent for a team.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The slug of the first requested team they belong to. Absent for a team, and where the forge would not place them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
}

/// One thing said on the pull request, in the order it was said. `id` is the forge's own, which
/// a later read names the same one by. `edited_at` is left out when it was never edited, and
/// always where Fleet read the page through `gh pr view`, which does not say. `bot` is left out
/// for a person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PullRequestEntry {
    Comment {
        id: String,
        author: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        bot: bool,
        at: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        edited_at: Option<String>,
        body: String,
    },
    Review {
        id: String,
        author: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        bot: bool,
        at: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        edited_at: Option<String>,
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    /// The commit the branch is at, whole. Absent where the forge did not say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    /// Its description, as Markdown. Absent where the author wrote none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// The conversations on its diff, resolved ones too: the forge's first 100, in its own order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threads: Vec<crate::pull_reviews::PullReviewThread>,
    /// Who is asked to review, and who reviewed unasked.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewers: Vec<PullRequestReviewer>,
    /// Who registered it in the central store, as the merge line's open pull requests carry
    /// it. Absent as `HubPullRequest::owner` is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<crate::merge_hub::PullRequestOwner>,
    /// A description a Job proposed for it and the person has not settled. Absent where none waits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<PullRequestDraft>,
    /// The head the person last left this pull request at. Absent where he never has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seen: Option<crate::pull_review_memory::PullSeen>,
    /// Each file he marked viewed, and whether it still is. Left out when he marked none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub viewed: Vec<crate::pull_review_memory::PullViewedFile>,
}

/// `get_pull_request_file`'s query string: the file a person opened, as the page's `files` names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestFileOf {
    pub path: String,
}

/// `get_pull_request_file`: one file of a pull request, read when a person opens it and carried
/// nowhere. One of `patch` and `said` is set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestFilePatch {
    pub manifest_id: ManifestId,
    pub number: u64,
    pub path: String,
    /// The file's hunks as the forge prints them, with no `diff --git` header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<String>,
    /// Why there is no patch, in Fleet's words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
    /// The forge's blob for the file's content at the head it was read at, which
    /// `mark_file_viewed` takes back. Absent where the forge gave none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// `get_pull_request_patches`: every file of a pull request, in the forge's order, each as
/// `get_pull_request_file` answers it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestFilePatches {
    pub files: Vec<PullRequestFilePatch>,
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
    /// The comments of the review, **posted with it as one review** and not as comments of their
    /// own. With any, `body` may be empty for `comment`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<crate::pull_reviews::PullReviewComment>,
    /// The commit the person read, whole: their line comments land on it, whatever has been pushed
    /// since. Absent reviews the branch as it stands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// The Session the press came from. **Absent records no Session.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}
