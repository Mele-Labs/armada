//! The issue tracker a repository keeps: what is open there, and the three
//! things Fleet writes back on an issue a Job came from. #91.
//!
//! **Its own trait, not more of [`Delivery`](crate::Delivery) or
//! [`LinkLookup`](crate::LinkLookup).** Delivery publishes a Job's branch and is
//! held as Fleet's version-control type; a tracker need not be the forge the
//! branch goes to. LinkLookup renders calls and runs none, which a listing and
//! a write cannot keep to. `docs/concepts/tracker.md`.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{FromOutside, IssueAddress};

/// One open issue, as the tracker listed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenIssue {
    pub number: u64,
    pub title: FromOutside,
    /// The tracker's own address for it: what a dispatch names, and what
    /// [`IssueAddress::url`] holds once a Job came from it.
    pub url: FromOutside,
    pub labels: Vec<FromOutside>,
    /// Who it is assigned to, by login.
    pub assignees: Vec<FromOutside>,
    /// When it was last edited, RFC 3339, as the tracker wrote it.
    pub updated_at: String,
    /// The start of its body, whitespace folded. Never the whole body: the
    /// list is for choosing, and the proposer reads the issue itself.
    pub excerpt: FromOutside,
    /// The tracker says a pull request already closes it.
    pub linked_pull_request: bool,
}

/// Why the tracker did not do what was asked, in its own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotTracked {
    /// What was being attempted, as a noun: `listing the open issues`.
    pub doing: &'static str,
    pub said: String,
}

impl NotTracked {
    pub fn of(doing: &'static str, said: impl Into<String>) -> NotTracked {
        NotTracked {
            doing,
            said: said.into(),
        }
    }
}

/// Reading a repository's open issues, and writing back on one.
///
/// **No method closes an issue or edits its text.** Fleet comments and moves
/// its own labels; an issue closes when the pull request that names it merges
/// (`closing_line`), which is the forge's act and a person's merge.
///
/// Every method is the operator's credentials, as with `Delivery`: Fleet runs
/// these, and no Drone holds a type that implements this.
pub trait Tracker {
    /// Every open issue in the repository checked out at `in_repo`, newest
    /// edit first. Pull requests are not issues here.
    fn open_issues(&self, in_repo: &str) -> Result<Vec<OpenIssue>, NotTracked>;

    /// Post `text` on the issue as a comment.
    fn comment(&self, issue: &IssueAddress, text: &str) -> Result<(), NotTracked>;

    /// Add `add` and take off `remove`, in one write. **Only these labels are
    /// touched**: a label somebody else put on the issue stays. A label the
    /// repository does not have yet is made.
    fn relabel(
        &self,
        issue: &IssueAddress,
        add: &[&str],
        remove: &[&str],
    ) -> Result<(), NotTracked>;

    /// The line a pull request body carries so that merging it closes the
    /// issue, in this tracker's own syntax. `None` where it cannot say one.
    fn closing_line(&self, issue: &IssueAddress) -> Option<String>;
}
