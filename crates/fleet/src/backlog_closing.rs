//! What Fleet tells a Job whose claim on a beads issue lost, and the issues Jobs hold.
//! `docs/concepts/tracker.md`, *Beads*. **A merge closes no issue**: which issues a pull request
//! fixed is the person's to say, or the agent's own close.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{IssueReference, Notice};
use store::{AttachmentState, Holder, HolderKind, Store};

use crate::backlog::{ClaimHeld, ClaimLoss};
use crate::daemon::Fleet;
use crate::session_issues::{CLAIMED_AS, ISSUE};

/// An issue a live Job holds in a beads repository, and what its row says.
pub(crate) struct HeldIssue {
    pub session: String,
    pub issue: String,
    pub detail: BTreeMap<String, String>,
}

/// The issues Jobs hold in `manifest`, earliest held first. **Standing rows only**: a Job that gave
/// one back, or ended, holds nothing. A bare number is the forge's, never a beads issue.
pub(crate) fn beads_issues_held(store: &Store, manifest: &str) -> Vec<HeldIssue> {
    let mut rows = store.standing_of_kind(ISSUE, manifest).unwrap_or_default();
    rows.sort_by(|a, b| (&a.since, &a.target).cmp(&(&b.since, &b.target)));
    rows.into_iter()
        .filter(|row| row.holder.kind == HolderKind::Session)
        .filter(|row| IssueReference::read(&row.target).is_some_and(|issue| !issue.is_number()))
        .map(|row| HeldIssue { session: row.holder.id, issue: row.target, detail: row.detail })
        .collect()
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
    /// The issues live Jobs in `manifest` claimed through Fleet, and who as.
    pub(crate) async fn claims_held(&self, manifest: &str) -> Vec<ClaimHeld> {
        let store = self.store().lock().await;
        beads_issues_held(&store, manifest)
            .into_iter()
            .filter_map(|held| {
                let actor = held.detail.get(CLAIMED_AS)?.clone();
                Some(ClaimHeld { session: held.session, issue: held.issue, actor })
            })
            .collect()
    }

    /// Tell a Job its claim lost, naming who holds the issue, and give the issue back from it. **Told
    /// before it is given back**, so a Job that cannot be told yet is told at a later sync.
    pub(crate) async fn claim_lost_told(self: &std::sync::Arc<Self>, manifest: &str, lost: &ClaimLoss) {
        let ClaimLoss { claim, holder } = lost;
        let text = format!(
            "{issue} is held by {holder}: the backlog names them after a sync, so the claim you made on it lost. \
             Fleet gave the issue back from this job. Pick another issue.",
            issue = claim.issue
        );
        if !matches!(self.told_by_fleet(&claim.session, &text, Notice::IssueLost).await, Ok(true)) {
            return;
        }
        let now = self.clock().now().as_str().to_string();
        let record = {
            let mut store = self.store().lock().await;
            let gave = store
                .settle(&Holder::session(&claim.session), ISSUE, manifest, &claim.issue, AttachmentState::GivenBack, &now)
                .unwrap_or(false);
            if !gave {
                return;
            }
            store.session(&claim.session).ok().flatten().and_then(|one| self.ledger_row(&store, &one).ok())
        };
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
    }
}
