//! The pull requests waiting on a person, and the review a person leaves on one: the lists the
//! forge answers, a conversation on a diff, and the comments that go out together as one review.
//!
//! **Beside [`PullRequestPageFacts`](crate::PullRequestPageFacts), not in it.** The lists are read
//! when the Reviews tab opens across every served repository; the threads ride the page read.
//! Plain text, for the reason that page gives: nothing here is handed to a Drone.

use alloc::string::String;
use alloc::vec::Vec;

use crate::ReviewStanding;

/// Which of the four lists a read is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewList {
    /// Review asked of the person by name.
    Requested,
    /// Review asked of a team the person is on, and not of them by name.
    Team,
    /// Assigned to the person.
    Assigned,
    /// Opened by the person.
    Mine,
}

/// One list read: whose, and across which repositories. **One forge call for all of them**, so the
/// cost does not grow with the number served.
#[derive(Clone, Copy, Debug)]
pub struct ReviewListAsk<'a> {
    pub list: ReviewList,
    /// The person's login on the forge, named outright.
    pub login: &'a str,
    /// Each repository as `owner/name`.
    pub repositories: &'a [String],
    /// The most rows to ask for.
    pub most: usize,
}

/// What a pull request's forge checks have come to. Not Armada's Checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListedChecks {
    Passed,
    Failed,
    /// Some have not finished and none has failed, or the repository runs none.
    Running,
}

/// One pull request on a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedPull {
    /// `owner/name`, as the forge names the repository.
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub branch: String,
    pub address: String,
    pub draft: bool,
    /// The commit the branch is at, whole.
    pub head: String,
    pub author: String,
    /// As the forge prints it (RFC 3339).
    pub updated_at: String,
    pub checks: ListedChecks,
    pub review: ReviewStanding,
    /// The team the review was asked of, on the team list: the first requested team the person is
    /// on, else the first requested. Absent on the other lists.
    pub team: Option<String>,
}

/// Which side of a diff a line is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffSide {
    /// The base, as the base numbers its lines.
    Left,
    /// The head, as the head numbers its lines.
    Right,
}

/// One thing said on a thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadSaid {
    pub id: String,
    pub author: String,
    /// As the forge prints it (RFC 3339).
    pub at: String,
    pub body: String,
}

/// A conversation on a line of the diff, or on a file, as the forge holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewThreadFacts {
    pub id: String,
    pub path: String,
    /// Absent where the thread is on the file as a whole, or the line has since changed under it.
    pub line: Option<u64>,
    pub side: DiffSide,
    pub resolved: bool,
    /// The first opens it. Never empty: a thread with nothing said is not read.
    pub comments: Vec<ThreadSaid>,
}

/// One comment of a review, **posted with the review and not before it**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewComment {
    Line {
        path: String,
        line: u64,
        side: DiffSide,
        body: String,
    },
    File {
        path: String,
        body: String,
    },
    /// On a thread the forge holds, by [`ReviewThreadFacts::id`].
    Reply {
        thread: String,
        body: String,
    },
}
