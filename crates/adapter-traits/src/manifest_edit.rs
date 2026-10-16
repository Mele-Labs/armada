//! A form's edit to `armada.yml`, sent as a pull request instead of being
//! written into the checkout Fleet serves.
//!
//! **Owned strings throughout**, because the call is made off the runtime's own
//! thread and the value has to move into it.

use alloc::string::String;

/// `armada.yml` as the base branch holds it on the remote, with the commit that
/// holds it, so the edit is made on exactly what was read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestOnTheBase {
    pub commit: String,
    pub text: String,
}

/// Where the commit goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PullRequestFor {
    /// A new branch and a new pull request into `base`.
    Open { base: String },
    /// On top of the branch of a pull request already open, which updates it.
    /// `branch` is the one it was opened from.
    Update { branch: String, pull_request: String },
}

/// One edit, ready to send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestEditToSend {
    /// Where the Manifest is on this machine. The path in the commit is this
    /// one made relative to the repository's root.
    pub file: String,
    /// The Manifest whole, as it reads after the edit.
    pub text: String,
    /// The commit the new one is made on top of.
    pub parent: String,
    pub message: String,
    /// The branch it is pushed to. Named by the caller, so a second edit can
    /// say which branch the first one made.
    pub branch: String,
    pub to: PullRequestFor,
}

/// What sending came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestEditSent {
    /// The commit that was pushed.
    pub commit: String,
    /// The pull request's address, new or the one that was updated.
    pub pull_request: String,
    /// The forge's reply to turning auto-merge on, in its own words on a
    /// refusal. **Not a failure of the send**: the branch is out and the pull
    /// request is open either way.
    pub auto_merge: Result<(), String>,
}
