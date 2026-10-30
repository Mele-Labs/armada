//! His projects, across repositories, and the tasks in none: tasks a person wrote and issues he linked, in order,
//! kept on this Mac. Its own surface, for [`Issues`](super::Issues)' reason: a handler taking only this cannot reach
//! for a Job. `docs/practices/protocol/projects.md`.
//!
//! **Every write answers his projects and the loose tasks whole**, as `list_projects` does, and publishes
//! them as `projects.changed`. A row id is unique, so a row is named by it alone.

use std::future::Future;

use ipc::{
    LinkProjectIssue, LinkTaskJob, LinkTaskPull, MoveProjectRow, MoveProjectTaskRepository, NameProject, NewProjectIssue,
    NewProjectTask, ProjectTask, TickProjectTask,
};

use crate::daemon::Refusal;

pub trait Projects: Send + Sync + 'static {
    /// `list_projects` — each project in order with its rows, and the loose rows, an issue row's state and title read
    /// from its repository's tracker. A tracker that cannot be read leaves them absent and refuses nothing.
    fn list_projects(&self) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `add_project` — a new project at the end of the list.
    fn add_project(&self, name: NameProject) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `rename_project`.
    fn rename_project(&self, project: String, name: NameProject) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `delete_project` — the project and its rows.
    fn delete_project(&self, project: String) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `add_project_task` — a task he wrote, open, in a repository, at the end of a project or of the loose tasks.
    fn add_project_task(&self, task: NewProjectTask) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `add_project_subtask` — a task under a top-level task, open, at the end of its subtasks, in its repository and
    /// project. [`Refusal::Unacceptable`] for a parent that is an issue row or itself a subtask.
    fn add_project_subtask(&self, row: String, task: ProjectTask) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `link_task_issue` — an issue, by the tracker's reference, on a task. [`Refusal::Unacceptable`]
    /// for an issue row.
    fn link_task_issue(&self, row: String, link: LinkProjectIssue) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `unlink_task_issue` — takes an issue off a task; one it did not have changes nothing.
    fn unlink_task_issue(&self, row: String, link: LinkProjectIssue) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `link_task_job` — a running Job of the task's repository holds the task, and its subtasks, as if it had been started
    /// with it; a Job holding it already is replaced. [`Refusal::Unacceptable`] for an issue row, a Job of another repository
    /// or one that ended; [`Refusal::NotFound`] for a Job nobody knows.
    fn link_task_job(&self, row: String, link: LinkTaskJob) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `unlink_task_job` — lets a held task go to open, and the subtasks the same Job held; any other state changes nothing.
    fn unlink_task_job(&self, row: String) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `link_task_pull` — a pull request, by number, on a task. [`Refusal::Unacceptable`] for an issue row.
    fn link_task_pull(&self, row: String, link: LinkTaskPull) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `unlink_task_pull` — takes a pull request off a task; one it did not have changes nothing.
    fn unlink_task_pull(&self, row: String, link: LinkTaskPull) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `edit_project_task` — a task's title and notes, whole. [`Refusal::Unacceptable`] for an issue row.
    fn edit_project_task(&self, row: String, task: ProjectTask) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `link_project_issue` — an issue of a repository by the tracker's reference, at the end of a project or of the loose rows.
    fn link_project_issue(&self, link: NewProjectIssue) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `remove_project_row` — a task or a linked issue, either.
    fn remove_project_row(&self, row: String) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `move_project_row` — to a position among the rows of one parent of one project, either of which it may change.
    fn move_project_row(&self, row: String, to: MoveProjectRow) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `move_project_task_repository` — a top-level task and its subtasks to another repository Fleet serves; its links keep
    /// the repository each was linked in. [`Refusal::Unacceptable`] for a subtask, an issue row or a repository not served;
    /// [`Refusal::NotFound`] for a row nobody knows.
    fn move_project_task_repository(
        &self,
        row: String,
        to: MoveProjectTaskRepository,
    ) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `tick_project_task` — done, or open again. [`Refusal::Unacceptable`] for an issue row, whose
    /// state is the tracker's.
    fn tick_project_task(&self, row: String, tick: TickProjectTask) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;
}
