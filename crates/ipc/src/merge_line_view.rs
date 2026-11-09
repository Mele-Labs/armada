//! The merge line as a screen draws it, worked out by Fleet: `get_merge_line_view` and
//! `merge_line_view.changed`.
//!
//! **Fleet reads, every client draws.** `get_merge_lines` serves what Fleet read of each forge; a
//! screen draws which pull requests to list and in what order, which of them call for nobody, and
//! whether main reads green, red or nothing worth a mark. That reading was Bridge's in TypeScript,
//! and it is here so the native Bridge and Pocket draw the same line.
//!
//! **States, not words.** Nothing here is a sentence; a commit is whole, and a screen cuts it.
//!
//! **No land runner.** `MergeLine`'s `line`, `off`, `landed` and `sent_back` are always empty since
//! `armada land` was retired, so nothing of them is here: what merged is the forge's `merged`.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;
use crate::merge_hub::{
    FixingElsewhere, HeldCheckout, HubMerged, HubPullRequest, HubSession, MainChecking, MainMerge,
};

/// `get_merge_line_view`, and what `merge_line_view.changed` carries whole.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeLineViews {
    /// One a served repository whose forge Fleet has read, in `get_merge_lines`' order.
    pub lines: Vec<MergeLineView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeLineView {
    pub root: String,
    /// What the repository is called among the ones Fleet serves: its Manifest's name, widened to
    /// `parent/folder` and then the whole root only where another repository reads alike.
    pub name: String,
    /// Main, where it is worth a mark. **Absent for a main not read yet, one running with nothing
    /// failed, one the forge ran nothing on, and a red that names no failed job.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main: Option<MainView>,
    /// The pull requests the line draws, in its order: the forge's merge queue by position, then
    /// those waiting for ci to join it, then the rest as the forge lists them. **Someone else's
    /// pull request is listed only while it is in the queue, and quiet** (`mine` false): its
    /// number, address, branch, author and place, with no ci, review, mergeability or hand, and its
    /// queue state `in_queue`, so nothing downstream raises a card for it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pulls: Vec<HubPullRequest>,
    /// The newest pull requests merged, newest first, with the run each started on main.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub landed: Vec<HubMerged>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkout: Option<HeldCheckout>,
    /// When the forge last answered a whole visit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_at: Option<Instant>,
    /// When the forge first failed to answer, since the last whole visit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_since: Option<Instant>,
}

/// Main as the line marks it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum MainView {
    Green {
        /// Newer commits whose ci still runs, newest first.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        checking: Vec<MainChecking>,
    },
    Red {
        /// The first job that failed, in the forge's order.
        failed: MainRedJob,
        /// The others, in order.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        also: Vec<MainRedJob>,
        /// When main first read red.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        since: Option<Instant>,
        /// The pull request that turned it red. **Its Job's `title` is always set**: the
        /// ledger's, else the branch, else the Job's id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        merge: Option<MainMerge>,
        /// The Job working on it, with its `title` always set as `merge`'s is.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        taken_by: Option<HubSession>,
        /// Another machine's claim on it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        elsewhere: Option<FixingElsewhere>,
        /// Newer commits whose ci still runs. **Non-empty is a held red**: a newer run may have
        /// fixed it already, so nothing is handed a Job.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        checking: Vec<MainChecking>,
    },
}

/// One ci job that failed on main.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainRedJob {
    /// The Manifest Check it maps to, else the job's own name.
    pub check: String,
    /// The job maps to no Manifest Check.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unmapped: bool,
    /// The first test its log names. Absent where none could be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_url: Option<String>,
}
