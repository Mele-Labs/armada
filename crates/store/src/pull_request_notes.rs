//! The owner's notes on a pull request's lines, kept so a Job the owner sent one to can resolve it.
//!
//! **Plain text here**: `side` and `status` are the wire's words and validation is Fleet's.

/// One row of `pull_request_notes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPullRequestNote {
    pub id: String,
    /// As the forge names it: `owner/name`.
    pub repository: String,
    pub number: u32,
    pub path: String,
    pub line: u32,
    /// `left` or `right`.
    pub side: String,
    pub body: String,
    /// `open` or `done`.
    pub status: String,
    pub sent_to: Option<String>,
    pub sent_title: Option<String>,
    pub sent_at: Option<String>,
    pub created_at: String,
}
