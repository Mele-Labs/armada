//! What Fleet remembers of how a repository runs — `get_run_memory`, `get_run_memory_place`,
//! `set_run_memory_place` and `drop_run_fact` in
//! `crates/ipc/operations/`.
//!
//! The facts are [`RunFact`]: the shape the import reads from past sessions and the live Jobs
//! produce alike, so the two merge under one key (`workspace`, `kind`, `command`).

use serde::{Deserialize, Serialize};

use crate::session_import::{RunFact, RunFactKind};

/// Where a repository's remembered runs are kept, chosen when the repository is added.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPlace {
    /// A file in the person's Kit folder, keyed by the repository's remote.
    #[default]
    ThisMac,
    /// The repository's own `armada.yml`, on the Job's branch, so it lands with the Job's pull request.
    Repository,
    /// The Kit layout under a folder the person named, which a sync tool carries.
    Synced,
    /// Storage in the cloud, in the form `cloud` on [`RunMemoryPlace`] names.
    Cloud,
}

/// Where in the cloud, for the `cloud` place. Written `{"store":{}}` or `{"bucket":"s3://b/prefix"}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunCloud {
    /// Armada's central store: the profile the repository is bound to, keyed per repository.
    Store {},
    /// A bucket the person owns: `s3://`, `r2://` or `gs://`, the bucket, and a prefix. It is read
    /// with that provider's own credentials, which Fleet never keeps.
    Bucket(String),
}

/// `get_run_memory_place`' answer, and `set_run_memory_place`'.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunMemoryPlace {
    /// A root `list_repositories` names.
    pub repository: String,
    pub place: RunPlace,
    /// The folder `synced` writes under; empty for every other place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synced_folder: Option<String>,
    /// Where `cloud` keeps them; required for `cloud`, empty for every other place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud: Option<RunCloud>,
}

/// `set_run_memory_place`' body: the same fields, `synced_folder` required for `synced` and
/// `cloud` for `cloud`.
pub type SetRunMemoryPlace = RunMemoryPlace;

/// Whether the repository's place holds a fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunFactState {
    /// Written to the place, which `where` names.
    Kept,
    /// Learnt, and the place has not taken it yet: `why` says what it waits on.
    Waiting,
}

/// One fact and where it stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RememberedRun {
    #[serde(flatten)]
    pub fact: RunFact,
    pub state: RunFactState,
    /// The place that holds it, where `state` is `kept`.
    #[serde(rename = "where", default, skip_serializing_if = "Option::is_none")]
    pub kept_in: Option<RunPlace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// One workspace's facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunWorkspace {
    /// Relative to the repository root; `.` is the root.
    pub workspace: String,
    pub runs: Vec<RememberedRun>,
}

/// `get_run_memory`' answer and `drop_run_fact`'.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunMemory {
    pub repository: String,
    pub place: RunPlace,
    pub workspaces: Vec<RunWorkspace>,
}

/// `drop_run_fact`' body. The line is never offered or written again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropRunFact {
    pub repository: String,
    pub workspace: String,
    pub kind: RunFactKind,
    pub command: String,
}
