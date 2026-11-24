//! A project's system map: what it is made of and how the parts reach each other, read at three levels, and what Fleet
//! knows of each part. `docs/practices/protocol/projects.md`, *The system map*.
//!
//! **Fleet finds the parts from the project's own files, with no model**: build manifests, workspaces, compose files,
//! Kit runs and CI. The librarian adds what those miss only on his press, and its parts stay apart until he keeps them.

use serde::{Deserialize, Serialize};

use crate::{FeatureJob, Instant, ManifestId};

/// What a part is, which sets where Bridge draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemPartKind {
    /// Someone who uses the project. Drawn left of its boundary.
    Person,
    /// A part people use directly: an app, a page, a command, an extension.
    Entry,
    /// A part behind the entry points: a server, a daemon, a worker, a library they share.
    Service,
    /// Where state is kept: a database, a file store, a queue.
    Store,
    /// Something the project reaches that it does not hold: an API, a hosted service. Drawn right of its boundary.
    Outside,
    /// One piece of a part, seen when the part is zoomed into.
    Piece,
}

/// Where a part came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "by")]
pub enum PartOrigin {
    /// Read from the project's files. `from` names what was read, such as `cargo`, `npm`, `compose`, `kit run` or `ci`.
    Detected { from: String },
    /// The librarian added it and he has not kept it yet. `why` is the librarian's one line.
    Librarian { addition: String, why: String },
}

/// One part of the map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPart {
    /// Stable within the project: `bridge`, and `bridge/jobs` for a piece of it.
    pub id: String,
    pub name: String,
    pub kind: SystemPartKind,
    /// One line: what it is.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub summary: String,
    /// The part this piece belongs to; absent on level one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within: Option<String>,
    /// Its repository, where the project holds more than one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    /// Where its code is, repository-relative; a folder ends in `/`. Empty for a person or an outside part.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    /// It has pieces to zoom into.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub zooms: bool,
    pub origin: PartOrigin,
}

/// One part reaching another.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemLink {
    pub from: String,
    pub to: String,
    /// What it does there, where the code names it: an operation, a route, a table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Its place in a feature's path, from 1, on level three.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<u32>,
}

/// A feature of the project, and the parts on this level it owns files in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemFeature {
    pub slug: String,
    pub title: String,
    pub parts: Vec<String>,
}

/// One stop of a feature's path, on level three: a part and the feature's files in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathStop {
    pub step: u32,
    pub part: String,
    pub files: Vec<String>,
}

/// `get_system_map`: one level of the map. **Level one has neither `part` nor `feature`**; level two names `part`,
/// the part zoomed into; level three names `feature`, and `stops` is its path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemMap {
    pub project: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    pub parts: Vec<SystemPart>,
    pub links: Vec<SystemLink>,
    pub features: Vec<SystemFeature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stops: Vec<PathStop>,
    /// When the librarian last added to this project's map. Absent where it never has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub librarian_at: Option<Instant>,
}

/// Someone who commits to a part's files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartOwner {
    pub name: String,
    /// Their share of the part's commits in the window, in percent.
    pub share: u8,
}

/// Something to watch a part by: a server, a health route, a log, a Check.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartWatch {
    /// What it is, as the project names it: `GET :40000/health`, `~/Library/Logs/Armada/`.
    pub label: String,
    /// `server`, `health`, `log`, `check` or `run`.
    pub kind: String,
}

/// An open issue that names a part.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartIssue {
    pub id: String,
    pub title: String,
}

/// An open pull request whose files fall in a part.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartPull {
    pub number: u64,
    pub title: String,
    pub author: String,
}

/// `get_system_part`: what Fleet knows of one part, of one feature, or of the whole project, in the nine sections Bridge shows. **Every section is present**;
/// an empty one is empty, and Bridge says why from it and the flags beside it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPartAnswers {
    pub project: String,
    /// The part asked about. Empty for the whole project and for a feature.
    pub part: String,
    /// The feature asked about, where one was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    pub features: Vec<SystemFeature>,
    /// The folders and files the part's code starts at.
    pub code: Vec<String>,
    /// Live Jobs that wrote a file of it.
    pub working: Vec<FeatureJob>,
    /// Most commits first, over `owner_days`.
    pub owners: Vec<PartOwner>,
    pub owner_days: u32,
    /// A shot of it, as a path Bridge can read. Absent where none was kept or it draws nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub looks: Option<String>,
    /// It draws a screen, so a missing shot is "No shot kept yet", not "Not a screen".
    pub has_screen: bool,
    /// Repository-relative documents that name it.
    pub docs: Vec<String>,
    pub monitor: Vec<PartWatch>,
    pub issues: Vec<PartIssue>,
    /// The project has a tracker Fleet reads.
    pub tracker: bool,
    pub pulls: Vec<PartPull>,
}

/// `list_system_folder`: one folder of a part's code, for the tree Bridge opens a level at a time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemFolder {
    pub project: String,
    pub path: String,
    /// Names in the folder at the checkout's `HEAD`, folders first, each folder ending in `/`.
    pub entries: Vec<String>,
}

/// What he chose for a part the librarian added.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "answer")]
pub enum AdditionAnswer {
    /// It joins the map as if it were found.
    Keep,
    /// The librarian redoes it from his note.
    Rework { note: String },
    /// It leaves the map and is not added again.
    Remove,
}

/// `answer_system_addition`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerSystemAddition {
    pub addition: String,
    #[serde(flatten)]
    pub answer: AdditionAnswer,
}
