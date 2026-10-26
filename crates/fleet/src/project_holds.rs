//! A Job holding the written tasks of a project it was started with. `docs/practices/protocol/projects.md`.
//!
//! **A task is held from the Job's start**, and only a written task: a linked issue row keeps no
//! state and is only named, so the Job's brief can carry it. Every move here publishes the
//! repository's projects whole, as a person's own write does.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ManifestId, ProjectRowRef};

use crate::daemon::Fleet;

/// A start names a project of another repository. A 422.
const OTHER_REPOSITORY: &str = "fleet.project_row_other_repository";

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
    /// The written tasks among `named`, as `(project, row)`, once every row is known to be one of
    /// `manifest`'s. **All or nothing**: one row that is missing, or another repository's, refuses
    /// the start. An issue row passes and is not returned.
    pub(crate) async fn project_rows_checked(
        &self,
        manifest: &str,
        named: &[ProjectRowRef],
    ) -> Result<Vec<(String, String)>, Refusal> {
        let store = self.store().lock().await;
        let kept = store.projects(manifest).map_err(|why| self.ledger_fault(why))?;
        let mut tasks = Vec::new();
        for one in named {
            if let Err(missing) = self.project_in(&kept, &one.project) {
                let elsewhere = store.project_manifest(&one.project).map_err(|why| self.ledger_fault(why))?;
                return Err(match elsewhere {
                    Some(_) => Refusal::Unacceptable(self.project_refusal(
                        OTHER_REPOSITORY,
                        format!("project {} is another repository's", one.project),
                    )),
                    None => missing,
                });
            }
            let row = self.row_in(&kept, &one.project, &one.row)?;
            if matches!(row.item, store::KeptRowItem::Task { .. }) {
                tasks.push((one.project.clone(), one.row.clone()));
            }
        }
        Ok(tasks)
    }

    /// `session` holds `tasks` from now, and the repository's projects are published.
    pub(crate) async fn project_tasks_held(&self, session: &str, manifest: &str, tasks: &[(String, String)]) {
        let held = self.store().lock().await.hold_project_tasks(session, tasks);
        if let Err(why) = held {
            self.session_warned(session, "a job's project tasks were not held", format!("{why:?}"));
            return;
        }
        self.projects_published(manifest).await;
    }

    /// The repository's projects as they stand, published as `projects.changed`.
    pub(crate) async fn projects_published(&self, manifest: &str) {
        let kept = self.store().lock().await.projects(manifest);
        let Ok(kept) = kept else {
            return;
        };
        let projects = self.projects_on_the_wire(ManifestId::carried(manifest), kept).await;
        self.publish(ipc::Event::ProjectsChanged(projects));
    }
}
