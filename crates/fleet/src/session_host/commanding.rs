//! A Job reads and commands the Jobs it started. `docs/concepts/session/a-job-that-started-jobs.md`.
//!
//! **The caller is placed by its connection and its children are the Jobs its own `started_to` links
//! name**, so a call cannot name a parent. A read walks down through the children's own links to the
//! whole tree; a command stays for a direct child. What a command may do is decided on values in
//! [`crate::started_jobs`]; what is done is the same act the person's End, Pause, Resume and
//! `change_waiting` run, so there is no second path to end a Job or move its wait.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal};
use ipc::{
    ChangeWaiting, CloseSession, Notice, PauseSession, ResumeSession, SessionId, StartedCommand, StartedJob, StartedSessions, WaitBy,
};
use store::SessionState;

use crate::daemon::Fleet;
use crate::session_issues::CLAIMED_AS;
use crate::started_jobs::{self, Child, Do, Refused};

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
    /// The hosted Job whose process holds this connection, or the 422 that says the call places none.
    fn started_placed(&self, caller: Option<&Caller>) -> Result<String, Refusal> {
        caller.and_then(|caller| self.session_holding(caller)).ok_or_else(|| {
            self.hosted_refusal(
                ipc::STARTED_UNPLACED,
                "only a Job's own agent reads or commands the Jobs it started, and this call comes from none",
            )
        })
    }

    /// The ids of the Jobs `parent` started, oldest first.
    async fn started_ids(&self, parent: &str) -> Result<Vec<String>, Refusal> {
        let store = self.store().lock().await;
        let session = store
            .session(parent)
            .map_err(|why| self.ledger_fault(why))?
            .ok_or_else(|| self.hosted_refusal(ipc::STARTED_UNPLACED, "this Job is not one Fleet knows"))?;
        Ok(started_jobs::children_in(&self.ledger_row(&store, &session)?.attachments))
    }

    /// `parent`'s children and the two facts a ruling reads of each, read now.
    async fn started_children(&self, parent: &str) -> Result<Vec<Child>, Refusal> {
        let ids = self.started_ids(parent).await?;
        let store = self.store().lock().await;
        let mut children = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(session) = store.session(&id).map_err(|why| self.ledger_fault(why))? else { continue };
            let live = session.state == SessionState::Live;
            let held = store.hosting(&id).map_err(|why| self.ledger_fault(why))?.is_some_and(|one| one.start_held);
            let waits = store.wait_of(&id).map_err(|why| self.ledger_fault(why))?.is_some_and(|one| one.started_at.is_none());
            children.push(Child { id, live, begun: live && !held && !waits });
        }
        Ok(children)
    }

    /// Every Job below `parent` as `(id, started_by)`: each child followed by its own descendants,
    /// oldest first at each level. A Job met twice is listed once, so a cycle ends.
    async fn started_tree(&self, parent: &str) -> Result<Vec<(String, String)>, Refusal> {
        let mut seen = vec![parent.to_string()];
        let mut found = Vec::new();
        // Depth first: the stack holds the next Jobs to visit, the nearest sibling on top.
        let mut stack: Vec<(String, String)> =
            self.started_ids(parent).await?.into_iter().rev().map(|id| (id, parent.to_string())).collect();
        while let Some((id, by)) = stack.pop() {
            if seen.contains(&id) {
                continue;
            }
            seen.push(id.clone());
            // A child no record names has nothing below it to read.
            let below = if self.store().lock().await.session(&id).map_err(|why| self.ledger_fault(why))?.is_some() {
                self.started_ids(&id).await?
            } else {
                Vec::new()
            };
            stack.extend(below.into_iter().rev().map(|child| (child, id.clone())));
            found.push((id, by));
        }
        Ok(found)
    }

    /// One child as the read gives it, with the step it is on named and, once it ended, what it said.
    async fn started_job(&self, id: &str, started_by: &str) -> Result<StartedJob, Refusal> {
        let (record, said) = {
            let store = self.store().lock().await;
            let session = store
                .session(id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(started_jobs::NOT_YOURS, &format!("no job {id} is known")))?;
            let said = store.done_summary(id).map_err(|why| self.ledger_fault(why))?;
            (self.ledger_row(&store, &session)?, said)
        };
        let step = match (record.manifest_id.as_ref(), record.workflow.as_deref(), record.workflow_step) {
            (Some(manifest), Some(workflow), Some(at)) => self
                .job_rail_steps(id, manifest.as_str(), workflow)
                .await
                .and_then(|steps| steps.get(at as usize).map(|step| step.label.clone())),
            _ => None,
        };
        Ok(StartedJob::of(&record, SessionId::carried(started_by), step, said))
    }

    /// `list_started_sessions`: the whole tree below the caller. Commands below stay for direct children.
    pub(crate) async fn started_listed(&self, caller: Option<Caller>) -> Result<StartedSessions, Refusal> {
        let parent = self.started_placed(caller.as_ref())?;
        let mut jobs = Vec::new();
        for (id, by) in self.started_tree(&parent).await? {
            jobs.push(self.started_job(&id, &by).await?);
        }
        Ok(StartedSessions { jobs })
    }

    /// `command_started_sessions`: each command done as a call of its own would be, in order. **A
    /// refused command leaves the others done**; a caller that places no Job is refused whole.
    pub(crate) async fn started_commanded(
        self: &Arc<Self>,
        caller: Option<Caller>,
        commands: Vec<StartedCommand>,
    ) -> Result<Vec<Result<StartedJob, Refusal>>, Refusal> {
        let parent = self.started_placed(caller.as_ref())?;
        let mut answers = Vec::with_capacity(commands.len());
        for command in commands {
            answers.push(self.started_commanded_once(&parent, command).await);
        }
        Ok(answers)
    }

    async fn started_commanded_once(self: &Arc<Self>, parent: &str, command: StartedCommand) -> Result<StartedJob, Refusal> {
        // Read again for each command: the one before it may have ended or moved a child.
        let children = self.started_children(parent).await?;
        let ruling = started_jobs::ruled(&children, &command).map_err(|refused| self.started_refused(refused))?;
        let id = command.session_id.as_str().to_string();
        let session_id = || SessionId::carried(&id);
        match ruling {
            Do::Withdraw => {
                self.closed_and_parked(CloseSession { session_id: session_id() }).await?;
            }
            Do::End { why } => {
                let begun = children.iter().any(|one| one.id == id && one.begun);
                self.started_ended(parent, &id, &why, begun).await?;
            }
            Do::Pause => {
                self.session_paused(PauseSession { session_id: session_id() }).await?;
            }
            Do::Resume => {
                self.session_resumed(ResumeSession { session_id: session_id() }).await?;
            }
            Do::Change { condition, base } => {
                let was = self.open_wait_of(&id).await?;
                let change = ChangeWaiting {
                    waiting_session: session_id(),
                    condition,
                    base,
                    held: None,
                    workflow: None,
                    model: None,
                    effort: None,
                };
                self.wait_changed_by(id.clone(), was, WaitBy::Agent, Some(parent), change).await?;
            }
            Do::Drop => self.wait_dropped(parent, &id).await?,
        }
        self.started_job(&id, parent).await
    }

    fn started_refused(&self, refused: Refused) -> Refusal {
        match refused.code {
            started_jobs::BEGUN | started_jobs::ENDED => {
                Refusal::IllegalMove(ipc::WireError::raised(refused.code, refused.said, self.run_id()))
            }
            code => self.hosted_refusal(code, &refused.said),
        }
    }

    /// `end`: the child is told why, then closed as the person's End closes it, so its slot goes back
    /// and its work stays on its branch. **A child still on its card or in its wait has no agent to
    /// tell.** An issue it claimed through Fleet is handed back, naming the Job that ended it.
    async fn started_ended(self: &Arc<Self>, parent: &str, id: &str, why: &str, begun: bool) -> Result<(), Refusal> {
        let (manifest, claims) = {
            let store = self.store().lock().await;
            let manifest = store.session(id).ok().flatten().and_then(|one| one.manifest_id);
            let claims: Vec<(String, String)> = manifest
                .iter()
                .flat_map(|manifest| crate::backlog_closing::beads_issues_held(&store, manifest))
                .filter(|held| held.session == id)
                .filter_map(|held| Some((held.issue, held.detail.get(CLAIMED_AS)?.clone())))
                .collect();
            (manifest, claims)
        };
        let parent_title = self.started_title(parent).await;
        if begun {
            let text = format!(
                "Job \"{parent_title}\" ({parent}), which started this job, ended it: {why}. What you committed stays on your branch."
            );
            let _ = self.told_by_fleet(id, &text, Notice::StartedJobEnded).await;
        }
        self.closed_and_parked(CloseSession { session_id: SessionId::carried(id) }).await?;
        if let Some(manifest) = manifest.filter(|_| !claims.is_empty()) {
            self.claims_handed_back(parent, &parent_title, id, &manifest, claims).await;
        }
        Ok(())
    }

    /// Hand each issue `id` claimed back in its repository's backlog, with a note naming `parent`. A
    /// backlog that would not take one is said in the parent's thread, and the end stands.
    async fn claims_handed_back(&self, parent: &str, parent_title: &str, id: &str, manifest: &str, claims: Vec<(String, String)>) {
        let Ok(served) = self.served_named(Some(&ipc::ManifestId::carried(manifest))) else { return };
        let Some(backlog) = self.backlog_of(&served) else { return };
        let note = format!("Handed back: job \"{parent_title}\" ({parent}) ended job {id}");
        for (issue, actor) in claims {
            if let Err(why) = self.released_in_backlog(&served, Arc::clone(&backlog), &issue, &actor, &note).await {
                self.fleet_note(parent, format!("{issue} was not handed back from job {id}: {}: {}", why.doing, why.said)).await;
            }
        }
    }

    async fn started_title(&self, id: &str) -> String {
        let found = self.store().lock().await.session(id).ok().flatten();
        found.and_then(|one| one.title).filter(|one| !one.is_empty()).unwrap_or_else(|| id.to_string())
    }
}
