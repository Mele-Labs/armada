//! A ticket goes with its plan group. **When a Job starts another for a group whose ticket it holds,
//! the started Job holds that ticket from then on**, so the rules of the Job doing the work move it,
//! and the Job that planned it lets go and is not held open by it.
//! `docs/concepts/session/linked-jobs.md`, *The ticket goes with its group*.
//!
//! The ticket is the group's `issue`, which `record_plan` fills where the agent left it blank. A group
//! with no ticket, or one the Job does not hold, hands nothing and changes nothing.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{IssueReference, SessionRecord};

use super::keeping::{JOB, TICKET};
use crate::daemon::Fleet;
use crate::session_host::linking::GROUP;

/// The plan group a Job was started for, from its `started_by` row.
fn group_of(record: &SessionRecord) -> Option<&str> {
    let link = record.attachments.iter().find(|one| one.kind == "started_by")?;
    link.detail.get(GROUP).map(String::as_str)
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
    /// Hand the ticket of the group `child` was started for from `parent` to it, where `parent` holds
    /// it and has not handed it already, so a second look at the same Job changes nothing.
    ///
    /// **Three things, in one hold of the store**: the child takes the ticket on, the parent's instance
    /// of it ends as handed standing where it was, and the parent's instance of the child is linked to
    /// the ticket, which is how a rule or the Workflow view finds the ticket through the Job. The child
    /// also holds it on its ledger, as every Job holds what it works; that row is written after.
    pub(super) async fn ticket_handed(&self, parent: &str, child: &SessionRecord) {
        let (Some(group), Some(manifest)) = (group_of(child), child.manifest_id.as_ref()) else { return };
        let (child_id, at) = (child.id.as_str(), self.now().as_str().to_string());
        let key = {
            let mut store = self.store().lock().await;
            if !store.is_lifecycle_job(child_id).unwrap_or(false) {
                return;
            }
            let Some(ticket) = store
                .session_plan(parent)
                .ok()
                .flatten()
                .and_then(|plan| plan.groups.into_iter().find(|one| one.id == group))
                .and_then(|one| one.issue)
                .and_then(|issue| IssueReference::read(&issue))
            else {
                return;
            };
            let key = ticket.as_str().to_string();
            let live = store
                .lifecycle_instances_of(parent)
                .unwrap_or_default()
                .iter()
                .any(|one| one.lifecycle == TICKET && one.key == key && one.ended_at.is_empty());
            if !live || store.take_on_lifecycle_instance(child_id, TICKET, &key, &[], &at).is_err() {
                return;
            }
            let _ = store.hand_lifecycle_instance(parent, TICKET, &key, child_id, &at);
            let _ = store.take_on_lifecycle_instance(parent, JOB, child_id, &[(TICKET, &key)], &at);
            key
        };
        if let Some(issue) = IssueReference::read(&key) {
            self.issues_held(child_id, manifest.as_str(), BTreeMap::from([(issue, BTreeMap::new())])).await;
        }
        self.job_lifecycles_published(parent).await;
        self.job_lifecycles_published(child_id).await;
    }
}
