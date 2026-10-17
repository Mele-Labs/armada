//! The pull requests waiting on a person, as four lists read from the forge, and the acts a
//! review makes on one: a line comment, a reply, a resolve. `docs/concepts/pull-review.md`.
//!
//! **Read when the person opens the Reviews tab or presses Refresh, and never polled.** Armada
//! does not watch other people's pull requests. A pull request is named by its repository and
//! its number, as everywhere in `pull_requests.rs`, and **any open pull request of a repository
//! Fleet serves can be listed and read here**, whoever opened it and whether or not a Session
//! holds it.

use serde::{Deserialize, Serialize};

use crate::ids::ManifestId;
use crate::pull_requests::PullRequestReviewState;

/// The four lists. A pull request can be on several.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullReviewList {
    /// Review requested of the person by name.
    Requested,
    /// Review requested of a team the person is on, and not of them by name.
    Team,
    /// Assigned to the person on the forge.
    Assigned,
    /// Opened by the person: the account Armada is signed in as, else the forge command line's.
    Mine,
}

/// What a row's forge checks have come to. Not Armada's Checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullReviewChecks {
    Passed,
    Failed,
    /// Some have not finished and none has failed, or the repository runs none.
    Running,
}

/// One pull request on one list: what a row of the list draws.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullReviewRow {
    pub manifest_id: ManifestId,
    /// `owner/name`, as the forge names the repository.
    pub repository: String,
    pub number: u64,
    pub title: String,
    /// The branch it is opened from.
    pub branch: String,
    /// Where it is on the forge.
    pub address: String,
    pub draft: bool,
    pub checks: PullReviewChecks,
    pub review: PullRequestReviewState,
    /// The commit the branch is at, whole.
    pub head: String,
    pub author: String,
    /// RFC 3339.
    pub updated_at: String,
    /// The team asked, as the forge names it, on the `team` list only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    /// The Session of the person's that holds the pull request, from the ledger Fleet already
    /// keeps of who owns one. Absent where none does, and then a note on it stays in Armada.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<crate::review_runs::PullReviewHolder>,
}

/// `get_review_lists`: the four lists, one forge call each, at most [`MOST_PER_LIST`] a list
/// and newest-updated first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullReviewLists {
    /// Who the person is on the forge: the "mine" login, the account Armada is signed in as for
    /// the repository, else the forge command line's. Their own words on a thread are signed so.
    pub me: String,
    /// RFC 3339, when Fleet read the lists.
    pub fetched_at: String,
    pub requested: Vec<PullReviewRow>,
    pub team: Vec<PullReviewRow>,
    pub assigned: Vec<PullReviewRow>,
    pub mine: Vec<PullReviewRow>,
    /// The lists the forge would not answer. **Their rows above are empty and say nothing**, so
    /// a reader keeps what it last held for them. All four silent is a 500
    /// `fleet.review_lists_unreadable` and no body.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub silent: Vec<PullReviewList>,
}

/// The most rows one list keeps. The forge is asked for exactly this many, so a list at the cap
/// has more behind it.
pub const MOST_PER_LIST: usize = 50;

/// Which side of the diff a line is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffSide {
    /// The base: a line that was removed, or one the change left alone, as the base numbers it.
    Left,
    /// The head: a line that was added, or left alone, as the head numbers it.
    Right,
}

/// One thing said on a review thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullReviewThreadComment {
    pub id: String,
    pub author: String,
    /// RFC 3339.
    pub at: String,
    /// Markdown.
    pub body: String,
}

/// A conversation on a line of the diff, or on a file, as the forge holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullReviewThread {
    /// The forge's own id for it, which a reply and a resolve name it by.
    pub id: String,
    pub path: String,
    /// The line in the file on `side`. **Absent where the thread is on the file as a whole, or
    /// where the line has since changed under it** (the forge's outdated threads).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    pub side: DiffSide,
    pub resolved: bool,
    /// The first comment opens it. The forge's cap is 50 comments a thread.
    pub comments: Vec<PullReviewThreadComment>,
}

/// One comment of the review a person submits. **They leave together, with the review.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PullReviewComment {
    /// A new thread on a line of the diff.
    Line {
        path: String,
        line: u64,
        side: DiffSide,
        body: String,
    },
    /// A new thread on a file as a whole.
    File { path: String, body: String },
    /// A reply on a thread the forge holds, named by [`PullReviewThread::id`].
    Reply { thread: String, body: String },
}

/// `resolve_review_thread`'s body. `POST /review_threads/:repository`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvePullThread {
    /// [`PullReviewThread::id`].
    pub thread: String,
    /// `false` opens it again.
    pub resolved: bool,
}

/// What `resolve_review_thread` answers: the thread as the forge now holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullThreadResolved {
    pub thread: String,
    pub resolved: bool,
}
