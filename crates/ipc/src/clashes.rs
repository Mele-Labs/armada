//! Where two agents' changes in one repository would conflict, or where main's would conflict with
//! one's. `docs/concepts/fleet.md`, *Write clashes*. Since protocol 23.81.
//!
//! **A reading, never a hold.** A clash is Fleet's own check of two whole trees against their merge
//! base, so it names files that really conflict and none that were only edited by both.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// What a clash is between.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClashSide {
    Session,
    /// The repository's base branch, as it stands now.
    Main,
}

/// One side of a clash.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClashHolder {
    pub kind: ClashSide,
    /// The Session's id; the base branch's name for `main`.
    pub id: String,
    /// How a person would name it: a Session's title, the base branch.
    pub label: String,
}

/// Two holders whose changes conflict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteClash {
    pub one: ClashHolder,
    /// A Session, or `main`.
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

/// `read_clash_file`: one file of a standing clash, as each side changed it.
/// `GET /clashes/file?session_id=&other_kind=&other_id=&path=`.
///
/// Both are what `git diff` prints for that one path, from the commit where the two sides split,
/// with its `diff --git` header and `@@` hunks. A side that left the file as it was has an empty
/// string.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClashFile {
    /// The asking Session's change to the file.
    pub ours: String,
    /// The other holder's. For `main`, what main changed since the Session's branch met it.
    pub theirs: String,
}
