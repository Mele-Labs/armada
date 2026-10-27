//! Projects, as the fake answers them: one project named for the call that reached it, so a route
//! test reads what arrived. What Fleet keeps and reads is `fleet::projects`', against a real store.

use ipc::{
    IssueState, LinkProjectIssue, LinkTaskJob, LinkTaskPull, ManifestId, MoveProjectRow, NameProject, NewProjectIssue, NewProjectTask,
    Project, ProjectRow, ProjectTask, ProjectTaskState, TickProjectTask, WireError,
};

use super::FakeDaemon;
use crate::{Projects, Refusal};

/// The answer to every call: one project, `project`, named `said`, with a task and an issue row, and a loose task.
fn echoed(project: &str, said: String) -> ipc::Projects {
    let task = |id: &str, title: &str| ProjectRow::Task {
        id: id.to_string(),
        manifest_id: ManifestId::carried("armada"),
        title: title.to_string(),
        notes: String::new(),
        state: ProjectTaskState::Open,
        held_by: None,
        parent: None,
        issues: Vec::new(),
        pulls: Vec::new(),
    };
    ipc::Projects {
        projects: vec![Project {
            id: project.to_string(),
            name: said,
            rows: vec![
                task("r1", "Write the page"),
                ProjectRow::Issue {
                    id: "r2".to_string(),
                    manifest_id: ManifestId::carried("site"),
                    issue: "214".to_string(),
                    title: Some("Open issue".to_string()),
                    state: Some(IssueState::Open),
                },
            ],
        }],
        loose: vec![task("r9", "Not in a project")],
    }
}

fn raised(code: &str, said: &str) -> WireError {
    WireError::raised(code, said, crate::tests::shapes::run_id())
}

/// Only `nope` is missing, so the route table's walk, which calls with `:project` as written, finds it routed.
fn held(project: &str) -> Result<(), Refusal> {
    match project {
        "nope" => Err(Refusal::NotFound(raised("fleet.no_such_project", "there is no project nope"))),
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
    async fn list_projects(&self) -> Result<ipc::Projects, Refusal> {
        Ok(echoed("p1", "list_projects".to_string()))
    }

    async fn add_project(&self, name: NameProject) -> Result<ipc::Projects, Refusal> {
        named(&name.name)?;
        Ok(echoed("p1", format!("add_project {}", name.name)))
    }

    async fn rename_project(&self, project: String, name: NameProject) -> Result<ipc::Projects, Refusal> {
        held(&project)?;
        named(&name.name)?;
        Ok(echoed(&project, format!("rename_project {}", name.name)))
    }

    async fn delete_project(&self, project: String) -> Result<ipc::Projects, Refusal> {
        held(&project)?;
        Ok(echoed(&project, "delete_project".to_string()))
    }

    async fn add_project_task(&self, task: NewProjectTask) -> Result<ipc::Projects, Refusal> {
        let project = task.project.as_deref().unwrap_or("-");
        held(project)?;
        Ok(echoed("p1", format!("add_project_task {} {project} {} / {}", task.manifest_id.as_str(), task.title, task.notes)))
    }

    async fn add_project_subtask(&self, row: String, task: ProjectTask) -> Result<ipc::Projects, Refusal> {
        if row == "r3" {
            return Err(Refusal::Unacceptable(raised("fleet.project_subtask_too_deep", "a subtask has no subtasks")));
        }
        Ok(echoed("p1", format!("add_project_subtask {row} {} / {}", task.title, task.notes)))
    }

    async fn link_task_issue(&self, row: String, link: LinkProjectIssue) -> Result<ipc::Projects, Refusal> {
        Ok(echoed("p1", format!("link_task_issue {row} {}", link.issue)))
    }

    async fn unlink_task_issue(&self, row: String, link: LinkProjectIssue) -> Result<ipc::Projects, Refusal> {
        Ok(echoed("p1", format!("unlink_task_issue {row} {}", link.issue)))
    }

    async fn link_task_job(&self, row: String, link: LinkTaskJob) -> Result<ipc::Projects, Refusal> {
        Ok(echoed("p1", format!("link_task_job {row} {}", link.session_id.as_str())))
    }

    async fn unlink_task_job(&self, row: String) -> Result<ipc::Projects, Refusal> {
        Ok(echoed("p1", format!("unlink_task_job {row}")))
    }

    async fn link_task_pull(&self, row: String, link: LinkTaskPull) -> Result<ipc::Projects, Refusal> {
        Ok(echoed("p1", format!("link_task_pull {row} {}", link.number)))
    }

    async fn unlink_task_pull(&self, row: String, link: LinkTaskPull) -> Result<ipc::Projects, Refusal> {
        Ok(echoed("p1", format!("unlink_task_pull {row} {}", link.number)))
    }

    async fn edit_project_task(&self, row: String, task: ProjectTask) -> Result<ipc::Projects, Refusal> {
        Ok(echoed("p1", format!("edit_project_task {row} {} / {}", task.title, task.notes)))
    }

    async fn link_project_issue(&self, link: NewProjectIssue) -> Result<ipc::Projects, Refusal> {
        let project = link.project.as_deref().unwrap_or("-");
        held(project)?;
        Ok(echoed("p1", format!("link_project_issue {} {project} {}", link.manifest_id.as_str(), link.issue)))
    }

    async fn remove_project_row(&self, row: String) -> Result<ipc::Projects, Refusal> {
        Ok(echoed("p1", format!("remove_project_row {row}")))
    }

    async fn move_project_row(&self, row: String, to: MoveProjectRow) -> Result<ipc::Projects, Refusal> {
        held(to.project.as_deref().unwrap_or("-"))?;
        Ok(echoed(
            "p1",
            format!(
                "move_project_row {row} {} {} {}",
                to.project.as_deref().unwrap_or("-"),
                to.position,
                to.parent.as_deref().unwrap_or("-")
            ),
        ))
    }

    /// `r2` is the fake's issue row, and ticking it is refused as Fleet refuses it.
    async fn tick_project_task(&self, row: String, tick: TickProjectTask) -> Result<ipc::Projects, Refusal> {
        if row == "r2" {
            return Err(Refusal::Unacceptable(raised(
                "fleet.project_row_not_a_task",
                "an issue's state is its tracker's",
            )));
        }
        Ok(echoed("p1", format!("tick_project_task {row} {}", tick.done)))
    }
}
