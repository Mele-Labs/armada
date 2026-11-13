//! `set_session_project`: Jobs, live or ended, moved into a project made later or out to none.
//! `docs/practices/protocol/projects.md`.
//!
//! **Every Job is checked before anything is written**, and so is the project. A repository of theirs that the
//! project does not hold becomes a whole member of it, written to its file before the Jobs are set, as a task
//! moved into a project brings its repository. The Jobs' frozen values and the tasks they hold are not touched:
//! what reads a Job's project live follows the new value.

use std::collections::BTreeSet;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{SessionList, SetSessionProject};

use super::serving::SESSION_COMMAND_REFUSED;
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
    /// `set_session_project`. Refused with nothing written: 422 for no Job and for a Job with no repository,
    /// 404 `fleet.no_such_session` for a Job Fleet does not host, and the refusals of a project: 404
    /// `fleet.no_such_project`, 422 `fleet.project_unreadable`. Each Job is published as `session.changed`,
    /// and the projects as `projects.changed` where the project took a repository in.
    pub(crate) async fn sessions_project_set(&self, asked: SetSessionProject) -> Result<SessionList, Refusal> {
        if asked.session_ids.is_empty() {
            return Err(self.hosted_refusal(SESSION_COMMAND_REFUSED, "no job was named to move into a project"));
        }
        let mut ids: Vec<String> = Vec::new();
        let mut repositories = BTreeSet::new();
        for id in &asked.session_ids {
            let id = id.as_str().to_string();
            let (_, hosting) = self.session_and_hosting(&id).await?;
            if hosting.manifest_id.is_empty() {
                return Err(self.hosted_refusal(
                    SESSION_COMMAND_REFUSED,
                    &format!("job {id} has no repository, so it belongs to no project"),
                ));
            }
            repositories.insert(hosting.manifest_id);
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        if let Some(project) = asked.project.as_deref() {
            self.project_repositories(project).await?;
            self.members_followed_published(project, repositories.into_iter().collect()).await?;
        }
        {
            let mut store = self.store().lock().await;
            for id in &ids {
                if let Some(mut hosting) = store.hosting(id).map_err(|why| self.ledger_fault(why))? {
                    hosting.project = asked.project.clone();
                    store.keep_hosting(&hosting).map_err(|why| self.ledger_fault(why))?;
                }
            }
        }
        let mut sessions = Vec::with_capacity(ids.len());
        for id in &ids {
            sessions.push(self.published_hosted(id).await?);
        }
        Ok(SessionList { sessions })
    }
}
