//! The repositories one Fleet serves, and adding one by folder or by URL, or
//! making a new one — `list_repositories`, `add_repository`, `clone_repository`
//! and `create_project` in `crates/ipc/operations/`.
//!
//! **A repository is listed whether or not it has a Manifest yet.** Setup
//! starts from a folder nobody wrote an `armada.yml` for, so `manifest` is
//! absent until one loads there, and Scan is named by `root` rather than by a
//! Manifest id it does not have.

use serde::{Deserialize, Serialize};

use crate::{ManifestId, ManifestSummary};

/// A folder to serve, as a person chose it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddRepository {
    /// Absolute. Fleet resolves it before comparing, so a symlink to a served
    /// repository is refused as that repository.
    pub path: String,
}

/// A repository to clone, and the folder a person picked to clone it under.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloneRepository {
    /// What `git clone` takes, read with the credentials git already has.
    pub url: String,
    /// Absolute, and already there. The clone lands in a new folder under it,
    /// named from the URL.
    pub parent: String,
}

/// A new project to make: a folder named `name` under `parent`, started as a
/// git repository and served.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateProject {
    /// Absolute, and already there. The project is a new folder under it.
    pub parent: String,
    /// The new folder's name: one segment, never a path.
    pub name: String,
}

/// One repository Fleet serves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositorySummary {
    /// The repository's root, resolved. What `?repository=` names on Scan and
    /// its proposals.
    pub root: String,
    /// Where its Job records live, for the reason `ManifestSummary` carries it.
    pub records_root: String,
    /// Absent until a Manifest at `root` loads: its `armada.yml`, Kit's copy, or both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<ManifestSummary>,
    /// The `manifest_id` a Job started here carries while there is no `manifest`:
    /// the id Fleet gives a repository with no Manifest, no `armada.yml` and no Kit copy.
    /// Absent where there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit_id: Option<ManifestId>,
}

/// Every repository Fleet serves, the one it was started in first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryList {
    pub repositories: Vec<RepositorySummary>,
}

/// Which served repository a request's words are about —
/// `read_request_repository` in `crates/ipc/operations/`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestRepository {
    /// Set only where exactly one repository is clearly meant. **Absent is
    /// "could not tell", and Bridge asks the person before anything is built.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    /// Every served repository, best first: the ones the words pointed at, then
    /// the rest by how many Jobs each has. Each is named by the id a Job there carries.
    pub likely: Vec<ManifestId>,
}
