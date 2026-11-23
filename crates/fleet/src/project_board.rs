//! A project's dashboard board, kept in its file with the rest of its look. `docs/practices/protocol/projects.md`,
//! *The board*.
//!
//! **A save changes the layout and nothing else of the project's values**, and is written as `edit_project` writes them,
//! so the file is checked and published as any edit of the values is, and a copy of another Mac's project sends it there.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Projects, Refusal};
use ipc::{EditProject, LookSet, ProjectAct, ProjectBoard, SaveProjectBoard};

use crate::daemon::Fleet;
use crate::values::board_wire;

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
    /// `get_project_board`: the layout the project's file keeps, **none where it keeps none**.
    pub(crate) async fn project_board_read(&self, project: String) -> Result<ProjectBoard, Refusal> {
        let held = self.project_held(&project).await?;
        Ok(ProjectBoard { layout: held.values().look.board.as_ref().map(board_wire), project })
    }

    /// `save_project_board`.
    pub(crate) async fn project_board_saved(&self, project: String, save: SaveProjectBoard) -> Result<ProjectBoard, Refusal> {
        let board = self.board_read(save.layout.as_ref())?;
        let layout = board.as_ref().map(board_wire);
        if let Some(mac) = self.copy_of_project(Some(&project)).await? {
            let shown = self.project_copies().await?.into_iter().find(|one| one.mac.as_deref() == Some(mac.as_str()) && one.id == project);
            let mut values = shown.and_then(|one| one.values).unwrap_or_default();
            let mut look = values.look.take().unwrap_or_default();
            look.board = layout.clone();
            values.look = (look != LookSet::default()).then_some(look);
            let edit = EditProject { values: Some(values), ..EditProject::default() };
            self.act_on_copy(&mac, ProjectAct::EditProject { project: project.clone(), edit }).await?;
            return Ok(ProjectBoard { project, layout });
        }
        {
            let kept = self.store().lock().await.projects().map_err(|why| self.ledger_fault(why))?;
            self.project_in(&kept, &project)?;
        }
        let mut values = self.project_read(&project)?.values().clone();
        values.look.board = board;
        let values = crate::values::values_set(&values).unwrap_or_default();
        self.edit_project(project.clone(), EditProject { values: Some(values), ..EditProject::default() }).await?;
        Ok(ProjectBoard { project, layout })
    }
}
