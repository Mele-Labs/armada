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
use ipc::{SessionList, SetSessionProject, WireError};

use super::serving::{NO_SUCH_SESSION, SESSION_COMMAND_REFUSED};
use crate::daemon::Fleet;

/// A Job moved into a copy of another Mac's project whose members do not list its repository. A 422.
const COPY_NOT_MEMBER: &str = "fleet.project_copy_not_member";

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
    /// `fleet.no_such_project`, 422 `fleet.project_unreadable`. **A copy of another Mac's project takes the Jobs
    /// where it lists their repositories**, else 422 `fleet.project_copy_not_member`. Each Job is published as `session.changed`,
    /// and the projects as `projects.changed` where the project took a repository in. **A Job that was in a
    /// shared project and is not any more** also has `projects.changed` published once, whatever the move: the
    /// other Mac reads its Jobs again on that and drops the row. `session.changed` for it is cut by the share
    /// filter, and nothing is sent naming the Job, so a private Job's id never crosses.
    pub(crate) async fn sessions_project_set(&self, asked: SetSessionProject) -> Result<SessionList, Refusal> {
        if asked.session_ids.is_empty() {
            return Err(self.hosted_refusal(SESSION_COMMAND_REFUSED, "no job was named to move into a project"));
        }
        let mut ids: Vec<String> = Vec::new();
        let mut repositories = BTreeSet::new();
        for id in &asked.session_ids {
            let id = id.as_str().to_string();
            let hosting = self.store().lock().await.hosting(&id).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| {
                Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("no job Fleet hosts is named {id}"), self.run_id()))
            })?;
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
        match asked.project.as_deref() {
            // A copy of another Mac's project takes a Job as a merge does; its members are that Mac's to change.
            Some(project) if !self.project_own(project) && self.copy_held(project, None).await => {
                for repository in &repositories {
                    if !self.copy_held(project, Some(repository.as_str())).await {
                        let said = format!("This project is kept on your other Mac and does not list {repository}. Add it to the project there first.");
                        return Err(Refusal::Unacceptable(WireError::raised(COPY_NOT_MEMBER, said, self.run_id())));
                    }
                }
            }
            Some(project) => {
                self.project_repositories(project).await?;
                self.members_followed_published(project, repositories.into_iter().collect()).await?;
            }
            None => {}
        }
        let mut was_shared = Vec::new();
        for id in &ids {
            if self.job_shared(id).await {
                was_shared.push(id.clone());
            }
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
        let mut left_sharing = false;
        for id in &was_shared {
            left_sharing |= !self.job_shared(id).await;
        }
        if left_sharing {
            self.projects_published().await;
        }
        Ok(SessionList { sessions })
    }

    /// Every Job of this Mac whose project is `gone` moved into `kept`, as a merge of the two asks. **Nothing is
    /// checked and nothing else of a Job changes**: its frozen values and the tasks it holds stay, as in
    /// [`Fleet::sessions_project_set`]. Each is published as `session.changed`, and the projects once where one
    /// of them was in a shared project and is not now.
    pub(crate) async fn jobs_repointed(&self, gone: &str, kept: &str) -> Result<(), Refusal> {
        let ids: Vec<String> = {
            let store = self.store().lock().await;
            let hostings = store.hostings().map_err(|why| self.ledger_fault(why))?;
            hostings.into_iter().filter(|one| one.project.as_deref() == Some(gone)).map(|one| one.session_id).collect()
        };
        if ids.is_empty() {
            return Ok(());
        }
        let mut was_shared = Vec::new();
        for id in &ids {
            if self.job_shared(id).await {
                was_shared.push(id.clone());
            }
        }
        {
            let mut store = self.store().lock().await;
            for id in &ids {
                if let Some(mut hosting) = store.hosting(id).map_err(|why| self.ledger_fault(why))? {
                    hosting.project = Some(kept.to_string());
                    store.keep_hosting(&hosting).map_err(|why| self.ledger_fault(why))?;
                }
            }
        }
        for id in &ids {
            self.published_hosted(id).await?;
        }
        let mut left_sharing = false;
        for id in &was_shared {
            left_sharing |= !self.job_shared(id).await;
        }
        if left_sharing {
            self.projects_published().await;
        }
        Ok(())
    }
}
