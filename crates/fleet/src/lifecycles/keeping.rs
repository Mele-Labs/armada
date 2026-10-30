//! Marking a Job as one that holds instances, taking them on as its ledger names them, and reading
//! the events Fleet itself hears.
//!
//! **The tap hears every Job's row change**, as the rail's and the linked Sessions' do, and does its
//! work only when something a Job holds moved. A Job with no mark in `lifecycle_jobs` is left alone,
//! which is every Job that was running before lifecycles existed.

use std::collections::HashMap;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Observed, StepId};
use ipc::{AttachmentState, Event, SessionRecord, SessionState};
use store::KeptRead;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::holding::{read_instance, rebuilt, subject_of, Transition};
use super::lifecycles_that_apply;
use super::reading::{fleet_fact, JobSeen};
use super::taking_on::{holds, landed, opened_pull_request, started_by};
use crate::daemon::Fleet;

pub(super) const TICKET: &str = "ticket";
pub(super) const PULL_REQUEST: &str = "pr";
pub(super) const JOB: &str = "job";

/// What of a row decides whether anything a Job holds moved.
type Row = (String, String, AttachmentState, String, String);

#[derive(PartialEq, Eq)]
struct Fingerprint {
    rows: Vec<Row>,
    ended: bool,
    step: Option<u32>,
}

fn fingerprint(record: &SessionRecord) -> Fingerprint {
    let detail = |row: &ipc::Attachment, key: &str| row.detail.get(key).cloned().unwrap_or_default();
    Fingerprint {
        rows: record
            .attachments
            .iter()
            .filter(|row| matches!(row.kind.as_str(), "issue" | "pr" | "started_by"))
            .map(|row| (row.kind.clone(), row.target.clone(), row.state, detail(row, "pull_requests"), detail(row, "state")))
            .collect(),
        ended: record.state == SessionState::Ended,
        step: record.workflow_step,
    }
}

/// Start taking instances on and reading Fleet's own events. Production starts it beside the
/// rail's, and a test starts it itself.
pub fn keep_the_lifecycles<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    let (tell, mut heard) = mpsc::unbounded_channel::<SessionRecord>();
    fleet.events().tap(move |event| {
        if let Event::SessionChanged(record) = event {
            if record.manifest_id.is_some() && !fingerprint(record).rows.is_empty() {
                let _ = tell.send(record.clone());
            }
        }
    });
    tokio::spawn(async move {
        let mut last: HashMap<String, Fingerprint> = HashMap::new();
        while let Some(record) = heard.recv().await {
            let (id, print) = (record.id.as_str().to_string(), fingerprint(&record));
            if last.get(&id) == Some(&print) {
                continue;
            }
            fleet.lifecycles_moved(record).await;
            last.insert(id, print);
        }
    })
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
    /// What a Job's row says it holds now: instances taken on, and a Job it started read.
    pub(crate) async fn lifecycles_moved(self: &Arc<Self>, record: SessionRecord) {
        let id = record.id.as_str().to_string();
        if self.store().lock().await.is_lifecycle_job(&id).unwrap_or(false) {
            self.lifecycles_taken_on(&id, &record).await;
        }
        if let Some(parent) = started_by(&record.attachments) {
            self.job_started_by(parent, &record).await;
        }
    }

    /// The tickets and pull requests the Job's ledger names, taken on once each. **A pull request
    /// is linked to the tickets it is for**, and takes a link a later row names.
    async fn lifecycles_taken_on(&self, id: &str, record: &SessionRecord) {
        let held = holds(&record.attachments);
        let at = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        for ticket in &held.tickets {
            let _ = store.take_on_lifecycle_instance(id, TICKET, ticket, &[], &at);
        }
        for pull in &held.pull_requests {
            let links: Vec<(&str, &str)> = pull.tickets.iter().map(|ticket| (TICKET, ticket.as_str())).collect();
            let _ = store.take_on_lifecycle_instance(id, PULL_REQUEST, &pull.number, &links, &at);
        }
    }

    /// A Job another Job started: the parent takes it on as an instance of the Job lifecycle, linked
    /// to itself, and it is handed what Fleet's events say of it. **Only a parent that holds
    /// instances does**, and no other Job is looked at.
    async fn job_started_by(&self, parent: &str, child: &SessionRecord) {
        let at = self.now().as_str().to_string();
        let child_id = child.id.as_str();
        {
            let mut store = self.store().lock().await;
            if !store.is_lifecycle_job(parent).unwrap_or(false) {
                return;
            }
            let _ = store.take_on_lifecycle_instance(parent, JOB, child_id, &[(JOB, parent)], &at);
        }
        let Some(manifest) = self.store().lock().await.session(parent).ok().flatten().and_then(|one| one.manifest_id) else {
            return;
        };
        let step = child
            .workflow
            .as_deref()
            .zip(child.workflow_step)
            .zip(child.manifest_id.as_ref())
            .and_then(|((workflow, step), manifest)| self.rail_steps(manifest.as_str(), workflow)?.get(step as usize).map(|one| one.id.clone()))
            .map(|id| StepId::new(&id));
        let seen = JobSeen {
            step,
            opened_pull_request: opened_pull_request(&child.attachments),
            landed: landed(&child.attachments),
            ended: child.state == SessionState::Ended,
        };
        let lifecycles = lifecycles_that_apply(&manifest);
        let Some(job) = lifecycles.iter().find(|one| one.id().as_str() == JOB) else { return };
        if let Some(fact) = fleet_fact(job, &seen) {
            self.lifecycle_seen(parent, JOB, child_id, Observed::Fleet(fact)).await;
        }
    }

    /// Hand a thing a Job holds what was seen, and keep the move it makes. **Every reader comes
    /// here**, so the Job's instances are rebuilt, read through `observe` and written in one hold of
    /// the store, and two readers cannot both record the same entering of a state. `None` where the
    /// Job does not hold it or it stood there already.
    pub(crate) async fn lifecycle_seen(
        &self,
        session: &str,
        lifecycle: &str,
        key: &str,
        seen: Observed<'_>,
    ) -> Option<Transition> {
        let at = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let manifest = store.session(session).ok().flatten()?.manifest_id?;
        let lifecycles = lifecycles_that_apply(&manifest);
        let mut held = rebuilt(&lifecycles, &store.lifecycle_instances_of(session).ok()?);
        let subject = subject_of(&held, lifecycle, key)?;
        let transition = read_instance(&mut held, &lifecycles, &subject, &seen, &at)?;
        let (from, to) = (transition.moved.from.as_wire(), transition.moved.to.as_wire());
        store
            .keep_lifecycle_read(&KeptRead {
                session_id: session,
                lifecycle,
                key,
                seen_source: transition.source.as_wire(),
                seen: &transition.seen,
                moved: Some((&from, &to)),
                at: &at,
            })
            .ok()?;
        Some(transition)
    }
}
