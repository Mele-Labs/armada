//! `armada need` on the wire: what a checkout says it needs on a path, and what
//! Fleet answers. `docs/capabilities/needs.md`. Since protocol 23.46.
//!
//! **A need is a row on the session ledger**, so what these carry is that row
//! as a person reads it: the path, what was said, what was taken, and who holds
//! it. Who holds it is Fleet's to resolve. The CLI names a branch and Fleet
//! finds the Job or the session standing on it, as `docs/concepts/fleet.md`,
//! *Declared needs*, says.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, ManifestId};
use crate::sessions::Holder;

/// Which form of `armada need` was asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedAct {
    /// Declare it, and hear who is ahead. Declaring again changes nothing.
    Declare,
    /// Say what the holder took there.
    Took,
    /// Give it back.
    Release,
}

/// What `armada need` sends. `POST /needs`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedCall {
    pub act: NeedAct,
    /// The repository the checkout is of. Absent is the first Fleet serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    /// The branch the caller stands on: a need belongs to what holds the branch.
    pub branch: String,
    /// Repository-relative: `protocol-version.toml`.
    pub path: String,
    /// What is needed there, in the caller's words. `declare` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what: Option<String>,
    /// What it took (`23.41`). `took` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// One need, as a person reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedLine {
    pub holder: Holder,
    /// Who holds it, named the way a person would: the branch, or the session's
    /// title where there is no branch.
    pub held_by: String,
    pub path: String,
    pub what: String,
    /// Absent until the holder says what it took.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub took: Option<String>,
    pub since: Instant,
}

/// What an act came to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedAnswer {
    /// The holder's need after a `declare` or a `took`; absent after a `release`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mine: Option<NeedLine>,
    /// A `declare` of a need the holder already had: nothing was recorded.
    pub already: bool,
    /// The standing needs on the same path that were declared first, in order.
    pub ahead: Vec<NeedLine>,
    /// A `release` that gave one back; false where the holder had none there.
    pub gave_back: bool,
}

/// `list_needs`: every standing need of one repository, grouped by path and in
/// the order they are served within one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedList {
    pub needs: Vec<NeedLine>,
}
