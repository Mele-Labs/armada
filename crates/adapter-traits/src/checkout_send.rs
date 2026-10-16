//! The uncommitted files of a served checkout, sent up as a pull request
//! instead of being stashed or left for the owner to commit.
//!
//! **Owned strings throughout**, for `manifest_edit`'s reason.

use alloc::string::String;
use alloc::vec::Vec;

/// One send, ready to go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckoutToSend {
    /// The branch the pull request goes into. The commit's parent is the tip
    /// `origin` holds of it.
    pub base: String,
    /// The uncommitted paths, from the repository's root. A path that is gone
    /// from the working tree is a deletion.
    pub paths: Vec<String>,
    /// The commit's message and the pull request's title.
    pub message: String,
    /// The branch it is pushed to, named by the caller.
    pub branch: String,
}

/// What sending came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckoutSent {
    /// The commit that was pushed.
    pub commit: String,
    /// The pull request's address.
    pub pull_request: String,
    /// The forge's reply to turning auto-merge on, in its own words on a
    /// refusal. **Not a failure of the send**: the branch is out and the pull
    /// request is open either way.
    pub auto_merge: Result<(), String>,
}
