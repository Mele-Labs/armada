//! A pull request read as its change sets, the problems found in them, and how each set holds to
//! the repository's own practices. `docs/concepts/pull-review.md`, *Change sets*.
//!
//! **Each is a run of the review engine** (`review_runs.rs`), started with `start_review_run` and kept
//! per head like any other. The overview and a practice check read the change sets kept at the same
//! head, so the sets are run first.

use serde::{Deserialize, Serialize};

use crate::review_runs::ReviewPlace;

/// What kind of change a set is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSetKind {
    Migration,
    Data,
    Logic,
    Api,
    Ui,
    Tests,
    Config,
    Other,
}

/// One hunk of the diff by where it is, never by its text. The ranges are the hunk header's: the
/// base's first line and how many lines, then the head's. A count of 0 is a hunk that only adds, or
/// only removes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HunkRef {
    pub path: String,
    pub old_start: u64,
    pub old_lines: u64,
    pub new_start: u64,
    pub new_lines: u64,
}

/// A set of changes with one purpose, across every file it touches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet {
    /// `set-1`, `set-2`, and on, in the order the sets run. Fleet's, never the agent's, so it is
    /// the same for every run that reads these sets.
    pub id: String,
    pub title: String,
    pub kind: ChangeSetKind,
    /// One sentence on what the set makes happen.
    pub purpose: String,
    /// The directory every hunk of the set is under, `""` for the repository's root. Fleet's: the
    /// key a practice check is catalogued under, with `kind`.
    pub area: String,
    /// In the order they run.
    pub hunks: Vec<HunkRef>,
}

/// How much a problem the overview found matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverviewSeverity {
    /// It must not merge as it is.
    Blocker,
    /// Something could go wrong that the change does not handle.
    Risk,
    /// Code that does not do what it means to.
    Bug,
}

/// One problem the overview found. **Armada's own note: never posted.**
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewFinding {
    pub severity: OverviewSeverity,
    /// The id of the change set it is in.
    pub set: String,
    pub title: String,
    /// One plain sentence on what is wrong and what to do, at most
    /// [`COMMENT_AT_MOST`](crate::COMMENT_AT_MOST) characters, so it can be posted as it is.
    pub reason: String,
    pub at: ReviewPlace,
}

/// How a change set stands against one practice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PracticeVerdict {
    Met,
    Missed,
    NotApplicable,
}

/// A doc a practice was read from: its path, and its blob at the base when it was read. `None`
/// where the base did not have it, as for a doc the pull request adds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PracticeSource {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// One practice as the catalogue keeps it: a checklist item, and the doc that states it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PracticeItem {
    pub item: String,
    /// The path of one of the entry's sources.
    pub source: String,
}

/// One practice, checked against one change set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PracticeCheck {
    pub item: String,
    pub source: String,
    pub verdict: PracticeVerdict,
    /// A short line on why.
    pub note: String,
}
