//! What Fleet remembers of a pull request for the person between visits, and what it works out
//! from it: the head he last left it at, the files he marked viewed, where each row stands in his
//! inbox, the change since he looked, and his named Reviews layouts.
//! `docs/concepts/pull-review.md`, *Since you looked*.
//!
//! **Kept on his Mac, in Fleet's store, never on the forge**, so any Bridge reads the same.

use serde::{Deserialize, Serialize};

use crate::pull_requests::PullRequestFilePatch;

/// The head a person last left a pull request at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullSeen {
    /// The commit, whole.
    pub head: String,
    /// RFC 3339, when he left it.
    pub at: String,
}

/// What his own latest submitted review said. A pending review is not one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MyReviewSaid {
    Approved,
    ChangesRequested,
    Commented,
}

/// His own latest submitted review of a pull request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullMyReview {
    pub said: MyReviewSaid,
    /// The commit it was written on, whole. Absent where the forge did not say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// When it was submitted, RFC 3339. Absent where the forge did not say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<String>,
}

/// What came on a pull request since his own latest review: the page's *Your review* bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullSinceMyReview {
    /// Commits pushed after the one his review was written on. Absent where that commit is not
    /// among the pull request's, as after a force-push, or the forge did not say which it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commits: Option<u64>,
    /// Comments someone else wrote after his review, on the review threads he wrote on. Absent
    /// where the forge did not say when his review was submitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replies: Option<u64>,
}

/// Where a pull request stands in his inbox. **The first that holds wins**, in this order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullInbox {
    /// He looked before, and the head has moved since.
    ChangedSinceYouLooked,
    /// His latest review approved it.
    Done,
    /// His latest review commented or asked for changes, and the head has not moved since he looked.
    WaitingOnAuthor,
    /// Anything else.
    #[default]
    NeedsFirstLook,
}

/// `mark_pull_request_seen`'s body. `POST /pull_request_seen/:repository`. Bridge sends it when he
/// leaves the pull request or presses Mark as seen, and not when he opens it, so the range since
/// he looked holds for the whole visit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkPullSeen {
    pub number: u64,
    /// The head he was looking at, whole.
    pub head: String,
}

/// A file he marked viewed, as the page carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullViewedFile {
    pub path: String,
    /// The commit he viewed it at, whole.
    pub commit: String,
    /// RFC 3339.
    pub viewed_at: String,
    /// Whether it is still viewed: **the file has not changed since**. Fleet compares the forge's
    /// blob for the file at the head with the one he viewed, and where none was kept, the head
    /// with the commit he viewed it at.
    pub still: bool,
}

/// `mark_file_viewed`'s body. `POST /pull_request_viewed/:repository`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkFileViewed {
    pub number: u64,
    pub path: String,
    /// The commit he viewed it at, whole.
    pub commit: String,
    /// [`PullRequestFilePatch::blob`] as he read it. Absent, the file stops being viewed when the
    /// head moves at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// `unmark_file_viewed`'s body, and its answer. `POST /pull_request_viewed/:repository/unmark`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmarkFileViewed {
    pub number: u64,
    pub path: String,
}

/// `get_pull_request_compare`'s query: the commit the change starts from, and where it ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestCompareOf {
    /// His seen head, whole.
    pub since: String,
    /// The head to compare to, whole: the page's.
    pub head: String,
}

/// `get_pull_request_compare`: what the author changed from `since` to `head`.
///
/// **The forge compares from the merge base of the two commits**, so after a rebase the files can
/// hold changes that came from the target branch and not from the author.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PullRequestCompare {
    /// Every file changed in the range, each as `get_pull_request_file` answers one.
    Patches {
        since: String,
        head: String,
        files: Vec<PullRequestFilePatch>,
    },
    /// The forge no longer has `since`, as after a force-push. Bridge offers the whole pull request.
    BaseGone { since: String, head: String },
}

/// A named Reviews layout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewLayout {
    pub name: String,
    /// The pane-tree arrangement as Bridge writes it, `{ root, focus, next, threadWas? }`, as a
    /// JSON string Fleet does not read into.
    pub arrangement: String,
    /// Fleet's own, which cannot be saved over or deleted.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shipped: bool,
}

/// `list_review_layouts`, and the answer of every write on them: every layout, Fleet's first, and
/// the one every pull request opens on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewLayouts {
    pub layouts: Vec<ReviewLayout>,
    /// A name in `layouts`. Fleet's Default where he never picked, or deleted the one he picked.
    pub picked: String,
}

/// `save_review_layout`'s body. `POST /review_layouts`. A name already kept is replaced.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveReviewLayout {
    pub name: String,
    pub arrangement: String,
}

/// `delete_review_layout`'s and `pick_review_layout`'s body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedReviewLayout {
    pub name: String,
}

/// The name of Fleet's own layout.
pub const DEFAULT_REVIEW_LAYOUT: &str = "Default";
