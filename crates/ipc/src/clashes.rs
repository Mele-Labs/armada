//! Where two agents' changes in one repository would conflict, or where main's would conflict with
//! one's. `docs/concepts/fleet.md`, *Write clashes*. Since protocol 23.80.
//!
//! **A reading, never a hold.** A clash is Fleet's own check of two whole trees against their merge
//! base, so it names files that really conflict and none that were only edited by both.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// What a clash is between.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClashSide {
    Job,
    Session,
    /// The repository's base branch, as it stands now.
    Main,
}

/// One side of a clash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClashHolder {
    pub kind: ClashSide,
    /// The Job's or Session's id; the base branch's name for `main`.
    pub id: String,
    /// How a person would name it: a Job's handle, a Session's title, the base branch.
    pub label: String,
}

/// Two holders whose changes conflict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteClash {
    pub one: ClashHolder,
    /// A Job, a Session, or `main`.
    pub other: ClashHolder,
    /// Repository-relative, sorted.
    pub paths: Vec<String>,
    /// When these two first conflicted. A change in which files keeps it.
    pub since: Instant,
}

/// `list_write_clashes`: every clash standing in one repository, or those one holder is in.
/// `GET /clashes?manifest_id=&holder=`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteClashes {
    pub clashes: Vec<WriteClash>,
}
