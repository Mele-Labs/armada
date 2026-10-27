//! Projects: a person's ordered lists of tasks he wrote and issues he linked, across repositories.
//! `docs/practices/protocol/projects.md`.
//!
//! **A project is his, on this Mac, and has no repository.** Every row names its own, so a project
//! may hold rows of several; a task in no project is loose.
//!
//! **An issue row keeps no state.** It is the tracker's reference only; whether it is open, and its
//! title, are read from the tracker each time the rows are, and are absent where it could not say.

use serde::{Deserialize, Serialize};

use crate::{IssueState, ManifestId, SessionId};

/// `list_projects`' answer, what every write answers, and `projects.changed` whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Projects {
    /// In the order he keeps them.
    pub projects: Vec<Project>,
    /// The tasks in no project, of every repository, in their own order.
    pub loose: Vec<ProjectRow>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    /// In order, across repositories: a row's position is its place here.
    pub rows: Vec<ProjectRow>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectRow {
    /// A task he wrote, with its own state. `notes` is empty for none. `held_by` is present while the
    /// state is `held`, or `asked` by a Job; an `asked` task a linked pull request's merge asked about has none.
    /// **A subtask is a task with a `parent`**, the id of a
    /// top-level task of the same project; rows are flat, and a parent is followed by its subtasks. `issues`
    /// are the issues linked to it, in the order linked, each with what the tracker says of it now. `pulls` are the
    /// pull requests linked to it, in the order linked, each with what the forge says of it now.
    Task {
        id: String,
        /// The repository the task is in. Moving it between projects keeps it.
        manifest_id: ManifestId,
        title: String,
        notes: String,
        state: ProjectTaskState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        held_by: Option<ProjectHolder>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        issues: Vec<TaskIssue>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pulls: Vec<TaskPull>,
    },
    /// A linked issue: the tracker's reference (`7`, `ABC-12`, `armada-3fz`), with what the tracker
    /// says of it now.
    Issue {
        id: String,
        /// The repository the issue is in.
        manifest_id: ManifestId,
        issue: String,
        /// The tracker's title. Absent where it could not be read.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Open or closed, the tracker's. Absent where it could not be read.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<IssueState>,
    },
}

/// An issue linked to a task: the tracker's reference, with what the tracker says of it now. **It keeps
/// no state**: `title` and `state` are read when the rows are, and are absent where it could not say.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskIssue {
    pub issue: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<IssueState>,
}

/// A pull request linked to a task, by its number. **It keeps no state**: `title` and `state` are the forge's,
/// read when the rows are, and are absent where it could not say.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPull {
    pub number: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<TaskPullState>,
}

/// Where a linked pull request stands. A draft is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPullState {
    Open,
    Merged,
    Closed,
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
    /// Whether the task is done is yet to be said: its Job finished, or a pull request linked to it merged.
    /// It outlives the Job.
    Asked,
}

/// One row a Job starts with: the task, or the linked issue its brief should carry. Row ids are
/// unique across projects and the loose rows, so the row is the whole address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRowRef {
    pub row: String,
}

/// `add_project` and `rename_project`: the project's name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameProject {
    pub name: String,
}

/// `edit_project_task` and `add_project_subtask`: a task's title and notes, whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTask {
    pub title: String,
    #[serde(default)]
    pub notes: String,
}

/// `add_project_task`: a task in the repository `manifest_id`, in the project `project` or, absent, in none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewProjectTask {
    pub manifest_id: ManifestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub title: String,
    #[serde(default)]
    pub notes: String,
}

/// `link_project_issue`: an issue of the repository `manifest_id`, by the tracker's reference, `#7` or
/// `7`, `ABC-12`, `armada-3fz`, as a row of the project `project` or, absent, of none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewProjectIssue {
    pub manifest_id: ManifestId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub issue: String,
}

/// `link_task_issue` and `unlink_task_issue`: an issue by the tracker's reference, `#7` or `7`, `ABC-12`, `armada-3fz`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkProjectIssue {
    pub issue: String,
}

/// `move_project_row`: where the row goes, counted from 0 among the rows of the parent it ends up
/// under. Past the end is the end. `project` is the project it goes to, `""` for none, and absent to
/// leave it in the one it is in. `parent` is the task to make it a subtask of, `""` to take it to
/// the top level, and absent to leave it under the parent it has, or at the top level where the
/// project changes. The row keeps its repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveProjectRow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub position: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// `tick_project_task`: done, or open again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickProjectTask {
    pub done: bool,
}

/// `link_task_job`: a running Job to hold the task, as if it had been started with it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkTaskJob {
    pub session_id: SessionId,
}

/// `link_task_pull` and `unlink_task_pull`: a pull request of the repository, by number.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkTaskPull {
    pub number: u64,
}
