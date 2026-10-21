//! What a person reads on a pull request's page beyond its standing: who wrote
//! it, what was said, what ran and for how long, which files and commits, and
//! the three things a person can write there.
//!
//! **Beside [`PullRequestFacts`](crate::PullRequestFacts), not in it.** That one
//! is read each interval for every pull request a Session holds; this is read
//! when somebody opens one, and carries a conversation.
//!
//! **Plain text, not [`FromOutside`](crate::FromOutside).** Nothing here is
//! handed to a Drone; it is drawn for the person who asked.

use alloc::string::String;
use alloc::vec::Vec;

use crate::pull_review::{ReviewComment, ReviewThreadFacts};

/// Where the forge says a merge would stand against the base branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mergeability {
    Mergeable,
    Conflicting,
    /// The forge has not worked it out yet, or would not say.
    Unknown,
}

/// What the reviews have come to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewStanding {
    Approved,
    ChangesRequested,
    /// A review the base branch asks for and none has given.
    Required,
    /// Nobody has been asked, or nobody has looked.
    None,
}

/// What a review said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewSaid {
    Approved,
    ChangesRequested,
    /// A note with no verdict, including a dismissed one.
    Commented,
}

/// Whether a reviewer is somebody or a team the forge asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewerKind {
    Person,
    Team,
}

/// Somebody asked to review, or who reviewed and is no longer asked: the forge
/// drops the request when the review lands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reviewer {
    /// A person's login, or a team's slug.
    pub login: String,
    pub kind: ReviewerKind,
    /// Their latest review's verdict. Absent where they have not reviewed, and always for a team.
    pub verdict: Option<ReviewSaid>,
    /// A person's full name, where their profile has one. Never set for a team.
    pub name: Option<String>,
    /// A person's profile address. Never set for a team.
    pub url: Option<String>,
    /// The slug of the first requested team they belong to. Never set for a team, nor for a person the forge would not place.
    pub team: Option<String>,
}

/// One thing said on the pull request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    Comment {
        author: String,
        at: String,
        body: String,
    },
    Review {
        author: String,
        at: String,
        verdict: ReviewSaid,
        body: String,
    },
}

impl Said {
    /// When it was said, as the forge printed it.
    pub fn at(&self) -> &str {
        match self {
            Said::Comment { at, .. } | Said::Review { at, .. } => at,
        }
    }
}

/// How one of the forge's checks stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckStanding {
    Passed,
    Failed,
    Running,
    Skipped,
}

/// One of the forge's checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckRow {
    pub name: String,
    pub standing: CheckStanding,
    /// Seconds it ran. Absent while it runs and where the forge gave no
    /// start and end.
    pub seconds: Option<u64>,
    /// Where the forge keeps its log.
    pub address: Option<String>,
}

/// How a file changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileChange {
    Added,
    Modified,
    Removed,
    Renamed,
}

/// One file of a pull request. Not [`ChangedFile`](crate::ChangedFile), which
/// is a Job's worktree against its base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageFile {
    pub path: String,
    pub additions: u64,
    pub deletions: u64,
    pub change: FileChange,
}

/// What the forge holds for one file of a pull request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilePatch {
    /// The file's hunks as the forge prints them, with no `diff --git` header.
    Patch(String),
    /// The pull request has the file and the forge gave no patch for it: a
    /// binary file, or a change the forge will not show.
    NoPatch,
    /// The forge listed the pull request's files and this was not one.
    NoSuchFile,
}

/// One file of a pull request as the forge lists it, with its patch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchedFile {
    pub path: String,
    /// `None` where the forge gave no patch, as [`FilePatch::NoPatch`].
    pub patch: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitLine {
    /// The first seven characters.
    pub sha: String,
    pub headline: String,
    pub author: Option<String>,
    pub at: Option<String>,
}

/// How many entries a list held beyond what was kept. **A list the reader
/// capped says so**, so a cut is a number somebody can show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Omitted {
    pub conversation: usize,
    pub files: usize,
    pub commits: usize,
}

/// One forge read of a pull request's page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullRequestPageFacts {
    /// The branch it merges into.
    pub base: String,
    pub author: Option<String>,
    /// As the forge prints it (RFC 3339).
    pub created_at: Option<String>,
    pub additions: u64,
    pub deletions: u64,
    pub mergeable: Mergeability,
    pub review: ReviewStanding,
    /// Reviewers whose latest review approved.
    pub approvals: u64,
    /// In the order it was said, oldest first.
    pub conversation: Vec<Said>,
    pub checks: Vec<CheckRow>,
    pub files: Vec<PageFile>,
    pub commits: Vec<CommitLine>,
    pub omitted: Omitted,
    /// The commit the branch is at, whole. Absent where the forge did not say.
    pub head: Option<String>,
    /// The description, as the author wrote it. Absent where there is none.
    pub body: Option<String>,
    /// The conversations on the diff, resolved ones too, in the forge's order.
    pub threads: Vec<ReviewThreadFacts>,
    /// Who is asked to review, then who reviewed unasked, each once.
    pub reviewers: Vec<Reviewer>,
}

/// What a person writes on a pull request.
///
/// **The body of a comment or of a request for changes is not optional**, so a
/// write with nothing to say cannot be built: the caller that has no body has
/// no variant to reach for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PullRequestWrite {
    Comment { body: String },
    Approve { body: Option<String> },
    RequestChanges { body: String },
    /// One review that carries line, file and reply comments, with `verdict` as its verdict.
    /// **Fleet builds it only with comments**, so the body may be empty here. `commit` is the
    /// commit the person read, whole; absent is the branch as it stands.
    Review {
        verdict: ReviewSaid,
        body: Option<String>,
        comments: Vec<ReviewComment>,
        commit: Option<String>,
    },
}
