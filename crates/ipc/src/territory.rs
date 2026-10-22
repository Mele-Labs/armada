//! What a repository holds and where each agent is changing it, for the flight deck's map.
//! `docs/concepts/fleet.md`, *The territory*.
//!
//! **Readings of git, never holds.** A directory's file count is the base head's tree and a holder's
//! paths are its slot's tree against its merge base, so nothing here is something an agent declared.

use serde::{Deserialize, Serialize};

use crate::holding::SlotHolderKind;
use crate::ids::ManifestId;

/// `list_repository_areas`: the directories of each served repository's base, to two names deep.
/// `GET /repository_areas`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryAreas {
    /// One for each served repository whose base git could read; one it could not is absent.
    pub repositories: Vec<AreasOfRepository>,
}

/// One repository's directories.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreasOfRepository {
    pub manifest_id: ManifestId,
    /// The repository's root, as `list_repositories` names it.
    pub root: String,
    /// The commit of the base branch the counts were read at.
    pub head: String,
    /// The largest directories, by files, then by name. At most 40 counting each top directory and
    /// each directory inside one; a top directory is always listed before anything inside it.
    pub areas: Vec<RepositoryArea>,
    /// How many directories past the 40 were left out.
    #[serde(default)]
    pub omitted: u32,
}

/// A top directory, its files and the directories inside it that made the list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryArea {
    /// One name: `crates`.
    pub dir: String,
    /// Every file at or under it.
    pub files: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kids: Vec<AreaKid>,
    /// Its first files, as git lists them, named relative to `dir`: `src/lib.rs`. Only where it has no
    /// kids, since each kid names its own; the rest of its files are `files` less these. At most 100.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
}

/// A directory inside a top directory, by its own name: `fleet` for `crates/fleet`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaKid {
    pub name: String,
    pub files: u32,
    /// Its first files, as git lists them, named relative to the kid: `src/lib.rs`. At most 100, so
    /// the files past them are `files` less these; the count stays exact.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
}

/// `list_slot_touches`: what each Job or Session holding a slot has changed in it.
/// `GET /slot_touches`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotTouches {
    pub repositories: Vec<TouchesInRepository>,
}

/// The holders of one repository's slots that have changed something.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TouchesInRepository {
    pub manifest_id: ManifestId,
    /// A holder with no slot, or no change in it, is absent.
    pub holders: Vec<HolderTouches>,
}

/// What one holder has changed in its slot since the slot met the base branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolderTouches {
    pub kind: SlotHolderKind,
    /// The Job's or the Session's id.
    pub id: String,
    pub slot: u32,
    /// Repository-relative and sorted: committed on its branch, edited, deleted, or not yet tracked.
    /// At most 200.
    pub paths: Vec<String>,
    /// How many paths past the 200 were left out.
    #[serde(default)]
    pub omitted: u32,
}

/// `get_session_file_change`: one file a Job has edited, as its worktree changed it since its branch
/// met the base. `GET /sessions/file_change?session_id=&path=&seen=`.
///
/// `diff` is what `git diff` prints for that one path, `diff --git` header and `@@` hunks, with the
/// edits not yet committed in it. Empty where the file is as the base has it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFileChange {
    /// What the reading stood on, opaque. Sent back as `seen`, an equal stamp is answered
    /// `unchanged` without the diff.
    pub stamp: String,
    /// Empty where `unchanged` is set.
    pub diff: String,
    /// Set where Fleet cut `diff` short of what git printed, at a line or byte bound. The text is
    /// still whole lines of the start of the change.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
    /// Set where `seen` was this stamp: the diff the caller holds is the current one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unchanged: bool,
}
