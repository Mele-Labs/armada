//! A repository's projects: tasks a person wrote and issues he linked, in order, kept on this Mac.
//! Its own surface, for [`Issues`](super::Issues)' reason: a handler taking only this cannot reach
//! for a Job. `docs/practices/protocol/projects.md`.
//!
//! **Every write answers the repository's projects whole**, as `list_projects` does, and publishes
//! them as `projects.changed`.

use std::future::Future;

use ipc::{LinkProjectIssue, ManifestId, MoveProjectRow, NameProject, ProjectTask, TickProjectTask};

use crate::daemon::Refusal;

pub trait Projects: Send + Sync + 'static {
    /// `list_projects` — each project in order with its rows, an issue row's state and title read
    /// from the tracker. A tracker that cannot be read leaves them absent and refuses nothing.
    fn list_projects(&self, manifest_id: ManifestId) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `add_project` — a new project at the end of the list.
    fn add_project(
        &self,
        manifest_id: ManifestId,
        name: NameProject,
    ) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `rename_project`.
    fn rename_project(
        &self,
        manifest_id: ManifestId,
        project: String,
        name: NameProject,
    ) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `delete_project` — the project and its rows.
    fn delete_project(
        &self,
        manifest_id: ManifestId,
        project: String,
    ) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `add_project_task` — a task he wrote, open, at the end of the project.
    fn add_project_task(
        &self,
        manifest_id: ManifestId,
        project: String,
        task: ProjectTask,
    ) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `edit_project_task` — a task's title and notes, whole. [`Refusal::Unacceptable`] for an issue row.
    fn edit_project_task(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        task: ProjectTask,
    ) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `link_project_issue` — an issue by the tracker's reference, at the end of the project.
    fn link_project_issue(
        &self,
        manifest_id: ManifestId,
        project: String,
        link: LinkProjectIssue,
    ) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `remove_project_row` — a task or a linked issue, either.
    fn remove_project_row(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
    ) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `move_project_row` — to a position among the project's rows.
    fn move_project_row(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        to: MoveProjectRow,
    ) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;

    /// `tick_project_task` — done, or open again. [`Refusal::Unacceptable`] for an issue row, whose
    /// state is the tracker's.
    fn tick_project_task(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        tick: TickProjectTask,
    ) -> impl Future<Output = Result<ipc::Projects, Refusal>> + Send;
}
