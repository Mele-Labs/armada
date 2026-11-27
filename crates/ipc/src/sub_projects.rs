//! Sub projects: a group of Jobs the owner picked in the Jobs list and named, kept with the features they worked in when he
//! started it. `docs/concepts/feature-catalog.md`, *Sub projects*.

use serde::{Deserialize, Serialize};

use crate::{ids::Instant, SessionId};

/// A group of Jobs the owner named. Fleet keeps only what he picked; the Jobs list adds the live Jobs that write to one of
/// `features` itself, from `list_features`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubProject {
    /// A ulid.
    pub id: String,
    /// A project's id, as `list_features` takes it.
    pub project: String,
    pub name: String,
    /// The Jobs he picked, in his order.
    pub jobs: Vec<SessionId>,
    /// The slugs of the features whose `jobs_now` held one of `jobs` when he started it.
    pub features: Vec<String>,
    pub started_at: Instant,
}

/// `list_sub_projects`: the sub projects Fleet keeps, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubProjects {
    pub sub_projects: Vec<SubProject>,
}

/// `start_sub_project`: group the Jobs he picked under a name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartSubProject {
    /// A project's id, as `list_features` takes it.
    pub project: String,
    pub name: String,
    /// In his order.
    pub jobs: Vec<SessionId>,
}

/// `ungroup_sub_project`: drop a sub project. Its Jobs are untouched.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UngroupSubProject {
    pub id: String,
}
