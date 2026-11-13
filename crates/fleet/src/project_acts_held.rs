//! A task act another Mac made on its copy of one of this Mac's shared projects, run here. `docs/concepts/project.md`,
//! *Sharing with his other Macs*.
//!
//! **Run by the code that runs the same act for the person here**, so every refusal, every stamp and every
//! `projects.changed` is the one he would have had. What this adds is the gate (the project is shared, or the
//! act is refused as one on a row or project that does not exist) and `at`: an act made earlier than a field was
//! last edited here leaves that field as it is. **A tick is only ever `tick_project_task`**, the person's own.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Projects, Refusal};
use ipc::{ProjectAct, ProjectActFromMac};
use store::Edited;

use crate::daemon::Fleet;

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `sent`, run here where its project is shared, and this Mac's shared projects as they are after.
    pub(crate) async fn project_act_run(&self, sent: ProjectActFromMac) -> Result<ipc::Projects, Refusal> {
        self.act_on_project_from_other_mac(&sent.act).await?;
        let edited = match sent.at {
            Some(at) => Edited::Replayed(at),
            None => self.edited_now(),
        };
        let answered = match sent.act {
            ProjectAct::AddProjectTask { task, id } => self.add_project_task_as(id, task).await,
            ProjectAct::AddProjectSubtask { row, task, id } => self.add_project_subtask_as(id, row, task).await,
            ProjectAct::EditProjectTask { row, task, fields } => self.edit_project_task_edited(row, task, edited, fields).await,
            ProjectAct::TickProjectTask { row, tick } => self.tick_project_task_edited(row, tick, edited).await,
            ProjectAct::LinkTaskIssue { row, link } => Projects::link_task_issue(self, row, link).await,
            ProjectAct::UnlinkTaskIssue { row, link } => Projects::unlink_task_issue(self, row, link).await,
            ProjectAct::LinkTaskPull { row, link } => Projects::link_task_pull(self, row, link).await,
            ProjectAct::UnlinkTaskPull { row, link } => Projects::unlink_task_pull(self, row, link).await,
            ProjectAct::LinkProjectIssue { link } => Projects::link_project_issue(self, link).await,
            ProjectAct::MoveProjectRow { row, to } => self.move_project_row_edited(row, to, edited).await,
            ProjectAct::RemoveProjectRow { row } => Projects::remove_project_row(self, row).await,
        }?;
        Ok(self.projects_for_other_mac(answered).sent())
    }
}
