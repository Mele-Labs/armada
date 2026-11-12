//! Where a Job's lanes meet the store and the record: what [`job_lanes`](crate::job_lanes) is handed, and the
//! field it fills. **Read under the lock the caller holds** (`lanes_read`) and **drawn once it is let go**
//! (`lanes_drawn`), as the rail's reads are.

use std::collections::HashMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Ulid, WorkflowId};
use ipc::{JobLifecycles, SessionRecord, SessionTurn};
use store::{Holder, KeptAttachment, KeptSession, Store};

use crate::daemon::Fleet;
use crate::job_lanes::{lanes_of_job, LaneInput, LaneWorkflow};

/// What a Job's lanes are drawn from, beyond the record.
pub(crate) struct LaneReads {
    /// What the Job holds.
    held: JobLifecycles,
    /// What each Job it started holds, by the Job's id.
    started: HashMap<String, JobLifecycles>,
    /// The titles of the issues the Job holds and the Jobs it started do, by key.
    titles: HashMap<String, String>,
}

/// The issues among `attachments`, titled, by their number or key.
fn issue_titles(attachments: &[KeptAttachment], into: &mut HashMap<String, String>) {
    for one in attachments.iter().filter(|one| one.kind == "issue") {
        if let Some(title) = one.detail.get("title").filter(|title| !title.is_empty()) {
            into.insert(one.target.clone(), title.clone());
        }
    }
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
    /// What the lanes of `session` need from the store. **Nothing for a Job that has ended, and for one that
    /// holds nothing and follows no workflow**, whose lanes are absent. `attachments` are the Job's own.
    pub(crate) fn lanes_read(&self, store: &Store, session: &KeptSession, attachments: &[KeptAttachment], follows: bool) -> Option<LaneReads> {
        if session.state != store::SessionState::Live {
            return None;
        }
        if !follows && store.lifecycle_instances_of(&session.id).unwrap_or_default().is_empty() {
            return None;
        }
        let held = self.job_lifecycles_read(store, &session.id);
        let mut titles = HashMap::new();
        issue_titles(attachments, &mut titles);
        let mut started = HashMap::new();
        // The lifecycle of a Job another Job started is `job`; its instance's key is that Job's id.
        for one in held.instances.iter().filter(|one| one.lifecycle == "job") {
            issue_titles(&store.attachments_of(&Holder::session(&one.key)).unwrap_or_default(), &mut titles);
            started.insert(one.key.clone(), self.job_lifecycles_read(store, &one.key));
        }
        Some(LaneReads { held, started, titles })
    }

    /// The workflow `record` follows and where it stands, as the lanes draw it. **A workflow Fleet does not hold has a name and
    /// no steps**, and a step the record names that the workflow lacks is no step the Job is on.
    fn lane_workflow(&self, session: &KeptSession, record: &SessionRecord) -> Option<LaneWorkflow> {
        let id = record.workflow.as_deref()?;
        let found = match record.workflow_copy.as_ref().filter(|copy| copy.id.as_str() == id) {
            Some(copy) => Some((copy.name.clone(), copy.steps.iter().map(|step| (step.step_id.as_str().to_string(), step.label.clone())).collect::<Vec<_>>())),
            None => session.manifest_id.as_deref().and_then(|manifest| {
                let held = self.repositories().serving(manifest)?.workflows();
                let found = held.get(&WorkflowId::carried(Ulid::carried(id)))?;
                Some((found.name().to_string(), crate::rail::steps_of(found).into_iter().map(|step| (step.id, step.label)).collect()))
            }),
        };
        let (name, steps) = found.unwrap_or_else(|| (id.to_string(), Vec::new()));
        let at = record.workflow_step.filter(|at| (*at as usize) < steps.len());
        Some(LaneWorkflow {
            name,
            steps,
            at,
            began: !record.workflow_visits.is_empty(),
            skipped: record.skipped_steps.clone(),
            visits: record.workflow_visits.clone(),
        })
    }

    /// Set `record.lanes` off what [`lanes_read`](Self::lanes_read) took. **Called once the rail has drawn the step**, which
    /// the lanes stand on. Workflows off drops the workflow and keeps what the Job holds.
    pub(crate) fn lanes_drawn(&self, session: &KeptSession, reads: Option<LaneReads>, record: &mut SessionRecord) {
        let Some(reads) = reads else { return };
        let workflow = if self.tuned(|dials| dials.workflows) { self.lane_workflow(session, record) } else { None };
        // The rules the Job's copy holds win; a Job whose copy holds none reads those of its repository.
        let rules = match (&session.manifest_id, reads.held.rules.is_empty()) {
            (Some(manifest), true) => self.rules_that_apply(manifest),
            _ => Vec::new(),
        };
        let working = matches!(record.hosted.as_ref().map(|hosted| &hosted.turn), Some(SessionTurn::Working { .. }));
        record.lanes = lanes_of_job(&LaneInput {
            held: &reads.held,
            workflow: workflow.as_ref(),
            rules: &rules,
            started: &reads.started,
            titles: &reads.titles,
            working,
            waiting: &record.workflow_waiting_on,
        });
    }
}
