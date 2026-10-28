//! `skip_session_step`: the person skips a step of a hosted Job's workflow, or undoes the skip.
//! `docs/concepts/workflow.md`, *Skipping a step*.
//!
//! **Only the list is kept here.** The rail reads it (`rail::Facts::skipped`) and moves the step as it
//! moves any other, so the move, its timeline entry and the next step's instructions are the rail's.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{SessionRecord, SessionRow, SkipSessionStep, TimelineKind, WithdrawDraft, WithdrawWindow};

use super::drafting::DRAFT_PREFIX;
use super::proposing::{WORKFLOWS_OFF, WORKFLOW_NOT_HELD};
use super::serving::SESSION_COMMAND_REFUSED;
use crate::daemon::Fleet;
use crate::rail;

/// The delivering step, or an index past the last step. A 422.
pub(crate) const STEP_NOT_SKIPPABLE: &str = "fleet.session_step_not_skippable";

/// What the agent opened on a step and still holds open: walk windows by address, drafts by row id.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct OpenOnStep {
    pub windows: Vec<String>,
    pub drafts: Vec<String>,
}

/// The windows and drafts still open that the thread shows since `since`, when the step began.
pub(crate) fn open_since(rows: &[SessionRow], since: &str) -> OpenOnStep {
    let mut open = OpenOnStep::default();
    for row in rows {
        match row {
            SessionRow::Window { url, at, ended_at: None, .. } if at.as_str() >= since => open.windows.push(url.clone()),
            SessionRow::Draft { id, at, settled: None, .. } if at.as_str() >= since => open.drafts.push(id.clone()),
            _ => {}
        }
    }
    open
}

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
    /// `skip_session_step`: keep the skip, enter it on the timeline, take back what the agent opened on
    /// the step it is on, and tell a Job that has run.
    pub(crate) async fn step_skipped(self: &Arc<Self>, skip: SkipSessionStep) -> Result<SessionRecord, Refusal> {
        let id = skip.session_id.as_str().to_string();
        if self.terminal_session(&id).await?.is_some() {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                &format!("job {id} is a terminal job, and only a hosted one has a workflow"),
            ));
        }
        let (session, hosting) = self.session_and_hosting(&id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(&id));
        }
        let Some(workflow) = hosting.workflow.as_deref() else {
            return Err(self.hosted_refusal(
                SESSION_COMMAND_REFUSED,
                &format!("job {id} follows no workflow, so it has no step to skip"),
            ));
        };
        if !self.workflows_on() {
            return Err(self.hosted_refusal(
                WORKFLOWS_OFF,
                "workflows are switched off in Settings, so no Job has a step to skip",
            ));
        }
        let Some(steps) = self.rail_steps(&hosting.manifest_id, workflow) else {
            return Err(self.hosted_refusal(
                WORKFLOW_NOT_HELD,
                &format!("this repository no longer holds the workflow `{workflow}`"),
            ));
        };
        let at = skip.step as usize;
        let Some(step) = steps.get(at).filter(|_| at < rail::SKIPPABLE) else {
            return Err(self.hosted_refusal(
                STEP_NOT_SKIPPABLE,
                &format!("the workflow has {} steps, and none is numbered {at}", steps.len()),
            ));
        };
        if step.delivers {
            return Err(self.hosted_refusal(
                STEP_NOT_SKIPPABLE,
                &format!("the \"{}\" step delivers the work, and it cannot be skipped", step.label),
            ));
        }
        // When the step it is on began, read before the skip moves it.
        let since = {
            let mut store = self.store().lock().await;
            let mut kept = store.session_rail(&id).map_err(|why| self.ledger_fault(why))?;
            if kept.skipped.contains(&skip.step) == skip.skipped {
                drop(store);
                return self.published_hosted(&id).await;
            }
            let on = self.ledger_row(&store, &session)?.workflow_step;
            if skip.skipped {
                kept.skipped.push(skip.step);
                kept.skipped.sort_unstable();
            } else {
                kept.skipped.retain(|one| *one != skip.step);
            }
            store.keep_session_rail(&id, &kept).map_err(|why| self.ledger_fault(why))?;
            let since = kept.visits.iter().rev().find(|(visited, _)| *visited == skip.step).map(|(_, since)| since.clone());
            since.filter(|_| skip.skipped && on == Some(skip.step))
        };
        let says = if skip.skipped { "skipped" } else { "skip undone" };
        self.session_noted(&session, TimelineKind::SessionStep, format!("{says}: {}", step.label));
        if let Some(since) = since {
            self.step_withdrawn(&id, &since).await;
        }
        // A Job that has not run is told with its first turn (`first_step_kept`).
        if hosting.ran {
            let words = rail::skip_told(&self.workflow_name(&hosting.manifest_id, workflow), step, skip.skipped);
            if let Err(why) = self.told_by_fleet(&id, &words, ipc::Notice::WorkflowChanged).await {
                eprintln!("job {id}: the skipped step was not sent to the agent: {why:?}");
            }
        }
        self.published_hosted(&id).await
    }

    /// Withdraw every walk window and draft the agent opened since `since` and has not closed, as
    /// though it withdrew each itself. **Best effort**: one that will not go stays for the person.
    async fn step_withdrawn(&self, id: &str, since: &str) {
        let Ok(rows) = self.rows_of(id).await else { return };
        let open = open_since(&rows, since);
        let session_id = Some(ipc::SessionId::carried(id));
        for url in open.windows {
            let withdraw = WithdrawWindow { url, session_id: session_id.clone() };
            if let Err(why) = self.window_withdrawn(None, withdraw).await {
                eprintln!("job {id}: a window on the skipped step was not withdrawn: {why:?}");
            }
        }
        for row in open.drafts {
            let withdraw = WithdrawDraft { draft: format!("{DRAFT_PREFIX}{row}"), session_id: session_id.clone() };
            if let Err(why) = self.draft_withdrawn(None, withdraw).await {
                eprintln!("job {id}: a draft on the skipped step was not withdrawn: {why:?}");
            }
        }
    }
}
