//! A project's dashboard board: where each block stands, kept by Fleet with the project's look values, and the
//! issues and pull requests ranked for it. `docs/practices/protocol/projects.md`, *The board*.
//!
//! **Fleet holds the words and Bridge decides what each names.** A block's `kind` and `view` are text, so a kind
//! Bridge adds later crosses without a wire break, and one this build does not know is kept and passed back as it was.

use serde::{Deserialize, Serialize};

use crate::{Instant, ManifestId, TrackedIssue};

/// One block of the board, in the grid's columns and rows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardBlock {
    /// What the block shows, as Bridge spells it: `needs`, `run`, `reviews`, `issues`, `features`, `checks` or `system`, and any
    /// kind a later Bridge adds.
    pub kind: String,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
    /// How it draws its rows, as Bridge spells it: `list`, `table`, `cards`, `graph` or `picture`.
    pub view: String,
}

/// A project's board.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardLayout {
    pub blocks: Vec<BoardBlock>,
}

/// `get_project_board` and `save_project_board`' answer: the project and the layout it keeps. **`layout` is absent
/// for a project that saved none**, and Bridge draws its preset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectBoard {
    pub project: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<BoardLayout>,
}

/// `save_project_board`: the layout to keep, whole. **Absent forgets it**, so the project draws the preset again.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveProjectBoard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<BoardLayout>,
}

/// How a feature came to be matched to an issue or a pull request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchedBy {
    /// For an issue: the files of a pull request that names it, by the catalog's file ownership.
    PullRequestFiles,
    /// For a pull request: its own files, by the same rules.
    Files,
    /// A label equal to the feature's slug or its area's.
    Label,
    /// The feature's slug or title named in its title.
    Title,
}

/// A feature a ranked row matched, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchedFeature {
    pub slug: String,
    pub title: String,
    /// Each way it matched, once each.
    pub by: Vec<MatchedBy>,
    /// The owner's Jobs that filed a file under this feature in the last thirty days.
    pub jobs: u32,
}

/// One open, unassigned issue and the features it matched.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankedIssue {
    /// The repository it was listed from, absent for an issue of the project's own tracker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    pub issue: TrackedIssue,
    /// Empty for an issue that matched none, which is ranked last.
    pub features: Vec<MatchedFeature>,
    /// The owner's Jobs across `features`, each counted once: what the row is ranked on.
    pub jobs: u32,
    /// The librarian's short reason for the row's place, where it ranked the list. Absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// One open pull request that is not his, and the features its files fall in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankedPull {
    pub manifest_id: ManifestId,
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub draft: bool,
    pub features: Vec<MatchedFeature>,
    pub jobs: u32,
    /// As on `RankedIssue`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// `list_ranked_issues`' answer. **Read from what Fleet holds**: the issue lists as the tracker was last read and the
/// pull requests as the lists left them, so it asks the forge for nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankedIssues {
    pub project: String,
    /// The window the Jobs were counted in, in days.
    pub days: u32,
    /// Where the issues were read, the oldest of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<Instant>,
    /// Open issues nobody is assigned to and no Job holds: the ones he might pick up.
    pub might_pick_up: Vec<RankedIssue>,
    /// Open pull requests of other people: the ones he might be interested in.
    pub might_be_interested: Vec<RankedPull>,
    /// When the librarian ranked both lists, which are then in its order with a `why` on each row. Absent where they are
    /// ranked by features, as they are until a press of Rank with the librarian and after Reset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by_librarian: Option<Instant>,
}
