//! Sub projects: a group of Jobs the owner picked in the Jobs list and named, kept with the features they worked in when he
//! started it, the goal he gave it and the Job that orchestrates it. `docs/concepts/feature-catalog.md`, *Sub projects*.

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
    /// His words for what the sub project is for. A sub project kept before goals has its name.
    pub goal: String,
    /// The Jobs he picked, in his order. The orchestrator is not among them.
    pub jobs: Vec<SessionId>,
    /// The slugs of the features whose `jobs_now` held one of `jobs` when he started it.
    pub features: Vec<String>,
    /// The Job that coordinates `jobs` toward `goal`, an ordinary Job with a thread. Absent only where Fleet could not
    /// start one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orchestrator: Option<SessionId>,
    pub started_at: Instant,
    /// When his Agree on the Goal met? card folded it away. Absent on an open one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
}

/// `list_sub_projects`: the sub projects Fleet keeps, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubProjects {
    pub sub_projects: Vec<SubProject>,
}

/// `start_sub_project`: group the Jobs he picked under a name, toward a goal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartSubProject {
    /// A project's id, as `list_features` takes it.
    pub project: String,
    pub name: String,
    /// His words. The orchestrator is titled with them.
    pub goal: String,
    /// In his order.
    pub jobs: Vec<SessionId>,
}

/// `turn_into_orchestrator`: a running Job becomes the orchestrator of a new sub project, and its own work moves to a
/// new first child Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnIntoOrchestrator {
    /// A project's id, as `list_features` takes it.
    pub project: String,
    pub session_id: SessionId,
    /// His words.
    pub goal: String,
}

/// `ungroup_sub_project`: drop a sub project. Its Jobs are untouched.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UngroupSubProject {
    pub id: String,
}

/// On a `goal_met` waiting item only: what the card shows beside its question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalMet {
    /// The sub project's id.
    pub sub_project: String,
    pub goal: String,
    /// The pull requests of its Jobs that merged, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub landed: Vec<LandedPull>,
}

/// One pull request that merged.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandedPull {
    pub number: u64,
    pub title: String,
}
