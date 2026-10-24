//! What Fleet writes to a Job's beads issue besides its plan: the close when its pull request
//! merges, and the word to a Job whose claim lost. `docs/concepts/tracker.md`, *Beads*.
//!
//! **Both ride the backlog's look**, so a write lands in the copy under its lock and the look that
//! made it syncs it. A merge is noticed elsewhere (`pull_notices`), which only marks the issues of the
//! Jobs that held the pull request (`job_issues`, kept past the Job's end) with its address; the
//! close happens at the next look and is tried again
//! [`SYNC_EVERY`](crate::backlog::SYNC_EVERY) later where it fails, so a merge notice never waits on
//! `bd`.

use std::collections::BTreeMap;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{IssueReference, Notice};
use store::{AttachmentState, Holder, HolderKind, KeptJobIssue, Store};

use crate::backlog::{ClaimHeld, ClaimLoss};
use crate::backlog_syncing::Kept;
use crate::daemon::Fleet;
use crate::plan_mirroring::{writing, Door};
use crate::repositories::Served;
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
    /// Pull request `number`, at `url`, merged: every issue a Job that held it worked is to be
    /// closed with its children. **Held, not standing**: a Job that ended and had its rows given
    /// back still closes its issue. Only marks, and only in a repository whose backlog Fleet writes to.
    pub(crate) async fn issues_merged(&self, served: &Served, number: u64, url: &str) {
        if self.backlog_of(served).is_none() {
            return;
        }
        let manifest = served.manifest().id().as_str().to_string();
        let mut store = self.store().lock().await;
        let held: Vec<String> = store
            .attachments_at("pr", &number.to_string(), Some(&manifest))
            .unwrap_or_default()
            .into_iter()
            .filter(|row| row.holder.kind == HolderKind::Session)
            .map(|row| row.holder.id)
            .collect();
        for kept in store.job_issues(&manifest).unwrap_or_default() {
            if held.contains(&kept.session_id) {
                let _ = store.job_issue_merged(&kept.session_id, &manifest, &kept.issue, url);
            }
        }
    }

    /// Close each issue a merge marked, with every child still open, giving the pull request's
    /// address as the reason. What went wrong, one line each.
    pub(crate) async fn merged_issues_closed(&self, served: &Served, backlog: &Door, kept: &mut Kept, now: Duration) -> Vec<String> {
        let manifest = served.manifest().id().as_str().to_string();
        let due: Vec<KeptJobIssue> = {
            let mut store = self.store().lock().await;
            // Every issue a live Job holds is kept as worked, for when the Job is gone.
            for held in beads_issues_held(&store, &manifest) {
                let _ = store.keep_job_issue(&held.session, &manifest, &held.issue);
            }
            store
                .job_issues(&manifest)
                .unwrap_or_default()
                .into_iter()
                .filter(|one| !one.merged_in.is_empty() && one.closed_in != one.merged_in)
                .collect()
        };
        let mut failed = Vec::new();
        for one in due {
            let key = format!("close:{}:{}", one.session_id, one.issue);
            if !kept.due(&key, now) {
                continue;
            }
            let (root, issue, reason) = (served.root().to_string(), one.issue.clone(), format!("merged in {}", one.merged_in));
            match writing(backlog, move |door| door.close_tree(&root, &issue, &reason)).await {
                Ok(()) => {
                    kept.failed_at.remove(&key);
                    let _ = self.store().lock().await.job_issue_closed(&one.session_id, &manifest, &one.issue, &one.merged_in);
                }
                Err(why) => {
                    kept.failed_at.insert(key, now);
                    failed.push(format!(
                        "closing {} for the merge of {} did not happen: {}: {}",
                        one.issue, one.merged_in, why.doing, why.said
                    ));
                }
            }
        }
        failed
    }

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
            let _ = store.forget_job_issue(&claim.session, manifest, &claim.issue);
            store.session(&claim.session).ok().flatten().and_then(|one| self.ledger_row(&store, &one).ok())
        };
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
    }
}
