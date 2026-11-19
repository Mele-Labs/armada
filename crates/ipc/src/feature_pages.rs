//! The two pages of the Features screen that belong to a project, not to one feature: its overview and what changed since
//! he last looked. Read from the library and the repository with no model. `docs/concepts/feature-catalog.md`,
//! *Project overview* and *Since you looked*. The third, *Who's on what*, reads the open pull requests beside the Jobs.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// The first sentence of a repository's README, and the file it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmeSentence {
    pub sentence: String,
    /// The README's name as the repository spells it.
    pub file: String,
}

/// One room of the project's code map, and the area that owns most of its files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapRoom {
    pub module: String,
    pub path: String,
    pub kind: String,
    /// The repository, where the project holds more than one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<String>,
    /// The slug of the area that owns the most of its files. None where no feature owns a file in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<String>,
}

/// A feature with merges in the busiest window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusyFeature {
    pub slug: String,
    pub title: String,
    /// The pull requests merged in the window, newest first.
    pub pulls: Vec<u64>,
}

/// A word of the project and what it means.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryTerm {
    pub term: String,
    pub meaning: String,
}

/// `get_project_overview`: the project's map first, then its busiest features and its glossary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectOverview {
    pub project: String,
    /// The first repository's README, absent where it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what_it_is: Option<ReadmeSentence>,
    pub rooms: Vec<MapRoom>,
    pub busiest: Vec<BusyFeature>,
    pub glossary: Vec<GlossaryTerm>,
}

/// Why a feature is on the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LookedTag {
    /// Added since he looked.
    New,
    /// Merged on since he looked.
    Changed,
    /// Pending, waiting for his Confirm.
    NeedsYou,
}

/// A feature that moved since he looked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookedFeature {
    pub tag: LookedTag,
    pub slug: String,
    pub title: String,
    /// One line, from the titles of the pull requests merged since.
    pub what: String,
    /// The pull requests merged since, newest first.
    pub pulls: Vec<u64>,
}

/// `get_since_you_looked`: what moved since `since`, by feature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SinceYouLooked {
    pub project: String,
    /// A model's paragraph over the list. Empty until one is asked for.
    pub paragraph: String,
    /// Needs you first, then Changed by newest merge, then New.
    pub features: Vec<LookedFeature>,
}

/// A person on the page: a teammate with a pull request open on a feature, or the owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhoPerson {
    /// As the forge writes it.
    pub login: String,
    /// The owner, whose own Jobs fill his cells.
    pub me: bool,
}

/// An open pull request in a cell.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhoPull {
    pub number: u64,
    /// `owner/name`, since two repositories of a project can each have a pull request of one number.
    pub repository: String,
    pub title: String,
    pub draft: bool,
}

/// What one person has on one feature: the pull requests open on it and, for the owner, the Jobs running on it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhoCell {
    pub login: String,
    /// Newest number first.
    pub pulls: Vec<WhoPull>,
    /// The ids of the owner's Jobs on the feature now. Always empty in a teammate's cell.
    pub jobs: Vec<SessionId>,
}

/// A feature somebody is on, with a cell for each person on it. Two or more cells is an overlap.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhoFeature {
    pub slug: String,
    pub title: String,
    /// The owner's cell first, then the others by login.
    pub cells: Vec<WhoCell>,
}

/// `get_whos_on_what`: who has an open pull request or a running Job on which feature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhosOnWhat {
    pub project: String,
    /// The owner first, where his login is known, then the people with a cell by login.
    pub people: Vec<WhoPerson>,
    /// Most people first, then by title. Only a feature with a pull request or a Job.
    pub features: Vec<WhoFeature>,
}
