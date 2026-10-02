//! A test broken on main, claimed by the Job drafted to fix it. #999.
//!
//! **One claim per repository, Check and test**, so a second Drone reporting
//! the same failure drafts nothing. The claim belongs to the fix: it ends when
//! the fix's pull request settles, or when the fix ends without one, and
//! forgetting the fix removes it.

use alloc::string::String;
use alloc::vec::Vec;

use crate::job::ids::{JobId, ManifestId, RepoPath};

/// What broke: the Check a failing test ran under, the test's name as the
/// Drone copied it from the output, and what running it on main came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breakage {
    pub check: String,
    pub test: String,
    pub failure: String,
}

/// A breakage and the Job fixing it.
///
/// **`reported_by` is an id and not a link.** The reporting Job may be
/// forgotten while the fix is still being worked, and the claim outlives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakageClaim {
    pub fix: JobId,
    pub repository: ManifestId,
    pub breakage: Breakage,
    pub reported_by: JobId,
    /// The test's files, as the reporting Drone named them and Fleet found
    /// them in main's checkout. **Empty on a claim from before #1673**, and on
    /// one Fleet spotted itself: those hold off only what the fix declares.
    pub files: Vec<RepoPath>,
}

/// A Job whose Check failed on a claimed test, pointed at the Job fixing it.
/// #1001.
///
/// **`fix` is an id and not a link**, for `reported_by`'s reason: the pointer
/// belongs to the waiting Job, and it is given back when the fix settles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixWaiter {
    pub waiting: JobId,
    pub fix: JobId,
    pub repository: ManifestId,
    pub check: String,
    pub test: String,
}

/// Files a fix that has landed still holds off a Job, until the Job's own copy
/// has taken the fix. #1673.
///
/// **Written when the fix lands and given back at the Job's next catch-up**,
/// because a Job pointed at a fix is not caught up when the fix merges: Fleet
/// merges the base into a Job's branch only as a Drone is put on it. Until
/// then the Job's copy holds the test as it was broken, so these stay out of
/// its write scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LandedHold {
    pub held: JobId,
    pub fix: JobId,
    pub test: String,
    pub paths: Vec<RepoPath>,
}
