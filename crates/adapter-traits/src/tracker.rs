//! The issue tracker a repository keeps: what is open there, and the three
//! things Fleet writes back on an issue a Job came from. #91.
//!
//! **Its own trait, not more of [`Delivery`](crate::Delivery) or
//! [`LinkLookup`](crate::LinkLookup).** Delivery publishes a Job's branch and is
//! held as Fleet's version-control type; a tracker need not be the forge the
//! branch goes to. LinkLookup renders calls and runs none, which a listing and
//! a write cannot keep to. `docs/concepts/tracker.md`.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::{FromOutside, IssueAddress, LinkLookup};

/// One open issue, as the tracker listed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenIssue {
    /// How the tracker names it, as a person writes it: the forge's `#123`, Jira's `ABC-123`.
    /// **The identity of the issue in this crate**: nothing looks one up by a bare number.
    pub reference: String,
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

/// One issue read by its reference, open or closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadIssue {
    /// As [`OpenIssue::reference`].
    pub reference: String,
    pub title: FromOutside,
    pub url: FromOutside,
    /// The tracker no longer holds it open.
    pub closed: bool,
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

    /// The issue `reference` names, as [`OpenIssue::reference`] writes it, closed ones included.
    /// **A forge also takes the bare number**, since that is what a person types after `#`.
    /// `None` where there is none or the text names none, **and where it is a pull request's
    /// number**: a pull request is not an issue here, as it is not in
    /// [`open_issues`](Tracker::open_issues).
    fn read_issue(&self, in_repo: &str, reference: &str) -> Result<Option<ReadIssue>, NotTracked>;

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

    /// What the pull request's **title** carries so that this tracker links the
    /// two, for a tracker that reads the title and not the body. `None` where
    /// the body's [`closing_line`](Tracker::closing_line) is the whole of it,
    /// which is every tracker but the ones that say otherwise.
    fn title_mark(&self, _issue: &IssueAddress) -> Option<String> {
        None
    }
}

/// Which tracker a repository's Manifest chose: the provider where it named
/// one, and every other key of the block as written.
///
/// **The provider is the only key this crate knows.** What the rest mean is the
/// implementation's to read and refuse, so a new tracker adds no field here.
/// `docs/contracts/adapters.md`, *Where adapter config lives*.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TrackerChoice {
    provider: Option<String>,
    settings: BTreeMap<String, String>,
}

impl TrackerChoice {
    /// A Manifest that names no tracker: the forge the repository is on.
    pub fn of_the_forge() -> TrackerChoice {
        TrackerChoice::default()
    }

    pub fn named(provider: Option<&str>, settings: &BTreeMap<String, String>) -> TrackerChoice {
        TrackerChoice {
            provider: provider.map(String::from),
            settings: settings.clone(),
        }
    }

    pub fn provider(&self) -> Option<&str> {
        self.provider.as_deref()
    }

    pub fn settings(&self) -> &BTreeMap<String, String> {
        &self.settings
    }
}

/// The tracker and the link reader for a repository's choice. **Fleet holds one
/// of these and asks it per repository**, so no code above the composition root
/// learns which tracker is behind a Manifest.
///
/// **Total.** A choice that cannot be honoured, an unknown provider or a block
/// missing a key it needs, still answers: with a tracker that refuses every
/// call and says why. A bad block in one repository's file must not stop Fleet
/// starting or hide the others.
pub trait Trackers {
    fn tracker(&self, choice: &TrackerChoice) -> Arc<dyn Tracker + Send + Sync>;

    fn links(&self, choice: &TrackerChoice) -> Arc<dyn LinkLookup + Send + Sync>;
}
