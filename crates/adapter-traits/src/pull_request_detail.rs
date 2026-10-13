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
}
