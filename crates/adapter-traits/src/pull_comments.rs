//! Every comment on a pull request, read on Fleet's pull reading for the Job that holds it: the
//! conversation's comments, each review that has a body, and every comment on the diff.
//!
//! **Beside [`PullRequestPageFacts`](crate::PullRequestPageFacts), not from it.** The page is
//! read when a person opens it and may come through a reader that cannot tell an edit or a bot;
//! this is the read a notice is decided on, so both are always known here.

use alloc::string::String;

/// Where a comment on the diff sits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnFile {
    pub path: String,
    /// Absent where it is on the file as a whole, or the line has since changed under it.
    pub line: Option<u64>,
    /// Whether its thread is resolved.
    pub resolved: bool,
}

/// One comment. `id` is the forge's own, so the next reading finds the same one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullComment {
    pub id: String,
    pub author: String,
    /// Whether an app wrote it.
    pub bot: bool,
    /// As the forge prints it (RFC 3339).
    pub at: String,
    /// When it was last edited, as the forge prints it. Absent when it never was.
    pub edited_at: Option<String>,
    /// Where the forge shows it.
    pub url: String,
    pub body: String,
    /// Absent for the conversation's comments and a review's body.
    pub file: Option<OnFile>,
}
