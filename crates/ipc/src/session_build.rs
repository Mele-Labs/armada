//! What a Session has built, as Fleet reads it off what it observed. `docs/concepts/session.md`,
//! *The build record*.
//!
//! **Observed, and the agent's own words only add detail.** The phase, the stops and the quiet are
//! read from edits, commits, pushes, walks and the forge. The one thing the agent states is its task
//! list, through the task tools it already calls, and an item of it counts as built only where edits
//! were seen. Nothing here is asked of the model.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// Where the Session stands on the way from nothing to main.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildPhase {
    /// No edit seen yet, and no plan.
    Scope,
    /// A task list written or plan mode left, and no edit since.
    Plan,
    /// Edits seen.
    Implement,
    /// A stop of the road to main reached after the first edit.
    Road,
    /// The Session's pull request merged.
    Landed,
}

/// How much of one task is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildPieceKind {
    /// Completed, and an edit was seen while it was worked.
    Solid,
    /// Completed, and no edit was seen while it was worked: claimed and not observed.
    Hollow,
    /// The first task not completed, while edits have landed since the last completion.
    Scaffold,
    /// Every other task not completed.
    Ghost,
}

/// One task of the agent's list, in list order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildPiece {
    /// The id the agent's task tool gave it, or its place in a `TodoWrite` list.
    pub id: String,
    pub text: String,
    pub piece: BuildPieceKind,
}

/// A place on the road from the Session's edits to main. Landing is not one: it closes the road and
/// is [`SessionBuild::landed_at`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildStopId {
    /// A walk shown to the person, after the first edit.
    Walk,
    /// A `git push` that succeeded.
    Pushed,
    /// The Session's pull request opened.
    Pr,
    /// The pull request's checks came to a state.
    Checks,
    /// Auto-merge set, or the pull request queued.
    AutoMerge,
}

/// One stop of the road. `at` is absent where it has not been reached: it is a forecast, drawn faint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildStop {
    pub id: BuildStopId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Instant>,
}

/// A Session's build record. Absent from the row of a Session that has done nothing Fleet observed.
///
/// Derived on every read from what Fleet kept, so no figure here is held beside the facts it is
/// read from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionBuild {
    pub phase: BuildPhase,
    /// One per task, in list order. Empty where the agent wrote no list: progress is never invented.
    #[serde(default)]
    pub pieces: Vec<BuildPiece>,
    /// The road, ready to draw: the stops reached in the order they were, then the ones its
    /// repository's recent landed Sessions usually reached and this one has not, each with no `at`.
    #[serde(default)]
    pub stops: Vec<BuildStop>,
    /// The stops that repository's recent landed Sessions usually reached, in order, reached or not.
    /// Empty where it has no history.
    #[serde(default)]
    pub forecast: Vec<BuildStopId>,
    /// When the pull request merged. The road closes here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landed_at: Option<Instant>,
    /// The last tool activity Fleet observed, as of the last time the row went out. Bridge dims
    /// the unfinished part by how long ago this was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_since: Option<Instant>,
    /// The Session's standing is `stuck`.
    #[serde(default)]
    pub stalled: bool,
    /// The subagents still running, each as `Type: what it was asked`.
    #[serde(default)]
    pub workers: Vec<String>,
}

/// Where one task of the agent's list stands, as the agent's tool says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportedTaskStatus {
    Pending,
    InProgress,
    Completed,
}

/// One task of the list a mod heard the agent write. The mod sends the whole list on every change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportedTask {
    /// The task tool's id, or the item's place in a `TodoWrite` list.
    pub id: String,
    pub text: String,
    pub status: ReportedTaskStatus,
}
