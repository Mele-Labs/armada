//! Projects, as the fake answers them: one project named for the call that reached it, so a route
//! test reads what arrived. What Fleet keeps and reads is `fleet::projects`', against a real store.

use ipc::{
    IssueState, LinkProjectIssue, ManifestId, MoveProjectRow, NameProject, Project, ProjectRow, ProjectTask,
    ProjectTaskState, TickProjectTask, WireError,
};

use super::FakeDaemon;
use crate::{Projects, Refusal};

/// The answer to every call: one project, `project`, named `said`, with a task and an issue row.
fn echoed(manifest_id: ManifestId, project: &str, said: String) -> ipc::Projects {
    ipc::Projects {
        manifest_id,
        projects: vec![Project {
            id: project.to_string(),
            name: said,
            rows: vec![
                ProjectRow::Task {
                    id: "r1".to_string(),
                    title: "Write the page".to_string(),
                    notes: String::new(),
                    state: ProjectTaskState::Open,
                    held_by: None,
                },
                ProjectRow::Issue {
                    id: "r2".to_string(),
                    issue: "214".to_string(),
                    title: Some("Open issue".to_string()),
                    state: Some(IssueState::Open),
                },
            ],
        }],
    }
}

fn raised(code: &str, said: &str) -> WireError {
    WireError::raised(code, said, crate::tests::shapes::run_id())
}

/// Only `nope` is missing, so the route table's walk, which calls with `:project` as written, finds it routed.
fn held(project: &str) -> Result<(), Refusal> {
    match project {
        "nope" => Err(Refusal::NotFound(raised("fleet.no_such_project", "the repository has no project nope"))),
        _ => Ok(()),
    }
}

fn named(name: &str) -> Result<(), Refusal> {
    match name.trim().is_empty() {
        true => Err(Refusal::Unacceptable(raised("fleet.project_blank", "a project needs a name"))),
        false => Ok(()),
    }
}

impl Projects for FakeDaemon {
    async fn list_projects(&self, manifest_id: ManifestId) -> Result<ipc::Projects, Refusal> {
        Ok(echoed(manifest_id, "p1", "list_projects".to_string()))
    }

    async fn add_project(&self, manifest_id: ManifestId, name: NameProject) -> Result<ipc::Projects, Refusal> {
        named(&name.name)?;
        Ok(echoed(manifest_id, "p1", format!("add_project {}", name.name)))
    }

    async fn rename_project(
        &self,
        manifest_id: ManifestId,
        project: String,
        name: NameProject,
    ) -> Result<ipc::Projects, Refusal> {
        held(&project)?;
        named(&name.name)?;
        Ok(echoed(manifest_id, &project, format!("rename_project {}", name.name)))
    }

    async fn delete_project(&self, manifest_id: ManifestId, project: String) -> Result<ipc::Projects, Refusal> {
        held(&project)?;
        Ok(echoed(manifest_id, &project, "delete_project".to_string()))
    }

    async fn add_project_task(
        &self,
        manifest_id: ManifestId,
        project: String,
        task: ProjectTask,
    ) -> Result<ipc::Projects, Refusal> {
        held(&project)?;
        Ok(echoed(manifest_id, &project, format!("add_project_task {} / {}", task.title, task.notes)))
    }

    async fn edit_project_task(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        task: ProjectTask,
    ) -> Result<ipc::Projects, Refusal> {
        held(&project)?;
        Ok(echoed(manifest_id, &project, format!("edit_project_task {row} {} / {}", task.title, task.notes)))
    }

    async fn link_project_issue(
        &self,
        manifest_id: ManifestId,
        project: String,
        link: LinkProjectIssue,
    ) -> Result<ipc::Projects, Refusal> {
        held(&project)?;
        Ok(echoed(manifest_id, &project, format!("link_project_issue {}", link.issue)))
    }

    async fn remove_project_row(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
    ) -> Result<ipc::Projects, Refusal> {
        held(&project)?;
        Ok(echoed(manifest_id, &project, format!("remove_project_row {row}")))
    }

    async fn move_project_row(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        to: MoveProjectRow,
    ) -> Result<ipc::Projects, Refusal> {
        held(&project)?;
        Ok(echoed(manifest_id, &project, format!("move_project_row {row} {}", to.position)))
    }

    /// `r2` is the fake's issue row, and ticking it is refused as Fleet refuses it.
    async fn tick_project_task(
        &self,
        manifest_id: ManifestId,
        project: String,
        row: String,
        tick: TickProjectTask,
    ) -> Result<ipc::Projects, Refusal> {
        held(&project)?;
        if row == "r2" {
            return Err(Refusal::Unacceptable(raised(
                "fleet.project_row_not_a_task",
                "an issue's state is its tracker's",
            )));
        }
        Ok(echoed(manifest_id, &project, format!("tick_project_task {row} {}", tick.done)))
    }
}
