//! Projects: per repository, a person's ordered lists of tasks he wrote and issues he linked.
//! `docs/practices/protocol/projects.md`.
//!
//! **An issue row keeps no state.** It is the tracker's reference only; whether it is open, and its
//! title, are read from the tracker each time the rows are, and are absent where it could not say.

use serde::{Deserialize, Serialize};

use crate::{IssueState, ManifestId};

/// `list_projects`' answer, what every write answers, and `projects.changed` whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Projects {
    pub manifest_id: ManifestId,
    /// In the order he keeps them.
    pub projects: Vec<Project>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    /// In order: a row's position is its place here.
    pub rows: Vec<ProjectRow>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectRow {
    /// A task he wrote, with its own state. `notes` is empty for none. `held_by` is present while the
    /// state is `held` or `asked`, and only then.
    Task {
        id: String,
        title: String,
        notes: String,
        state: ProjectTaskState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        held_by: Option<ProjectHolder>,
    },
    /// A linked issue: the tracker's reference (`7`, `ABC-12`, `armada-3fz`), with what the tracker
    /// says of it now.
    Issue {
        id: String,
        issue: String,
        /// The tracker's title. Absent where it could not be read.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Open or closed, the tracker's. Absent where it could not be read.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<IssueState>,
    },
}

/// The Job holding a task.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectHolder {
    pub session_id: String,
    /// The Job's title. Absent where it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// A written task's own state. **More may come**, so a client treats one it does not know as open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTaskState {
    Open,
    Done,
    /// A Job started with it and has not finished.
    Held,
    /// The Job finished and whether the task is done is yet to be said. It outlives the Job.
    Asked,
}

/// One row of a project a Job starts with: the task, or the linked issue its brief should carry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRowRef {
    pub project: String,
    pub row: String,
}

/// `add_project` and `rename_project`: the project's name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameProject {
    pub name: String,
}

/// `add_project_task` and `edit_project_task`: a task's title and notes, whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTask {
    pub title: String,
    #[serde(default)]
    pub notes: String,
}

/// `link_project_issue`: an issue by the tracker's reference, `#7` or `7`, `ABC-12`, `armada-3fz`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkProjectIssue {
    pub issue: String,
}

/// `move_project_row`: where the row goes, counted from 0 among its project's rows. Past the end is
/// the end.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveProjectRow {
    pub position: u32,
}

/// `tick_project_task`: done, or open again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickProjectTask {
    pub done: bool,
}
