//! What the fake's version control refuses with.

use std::error::Error;
use std::fmt;

/// Why the fake refused.
///
/// One variant per split the real error draws: a name already taken, the
/// machine not cooperating, and a commit git would not make. A caller that
/// handles them handles the real implementation's whole surface as far as its
/// own logic is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeVcsError {
    /// A branch of that name is already there and was refused, never reused.
    BranchExists { branch: String },
    /// A scripted failure standing in for a disk, a permission or a repository
    /// that would not answer.
    Refused { standing_in_for: &'static str },
    /// A scripted failure of the commit, which is its own case: it happens
    /// after a Job's Checks have passed, and the caller must not lose the work
    /// over it.
    NotCommitted { standing_in_for: &'static str },
    /// No ref of that name, which is what the real one raises when `base:`
    /// names a branch the repository does not have.
    NoSuchRef { r#ref: String },
}

impl fmt::Display for FakeVcsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FakeVcsError::BranchExists { branch } => {
                write!(f, "the branch `{branch}` is already there")
            }
            FakeVcsError::Refused { standing_in_for } => {
                write!(f, "refused, standing in for {standing_in_for}")
            }
            FakeVcsError::NotCommitted { standing_in_for } => {
                write!(f, "not committed, standing in for {standing_in_for}")
            }
            FakeVcsError::NoSuchRef { r#ref } => {
                write!(f, "there is no ref `{name}`", name = r#ref)
            }
        }
    }
}

impl Error for FakeVcsError {}
