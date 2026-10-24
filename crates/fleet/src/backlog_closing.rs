//! What Fleet writes to a Job's beads issue besides its plan: the close when its pull request
//! merges, and the word to a Job whose claim lost. `docs/concepts/tracker.md`, *Beads*.
//!
//! **Both ride the backlog's look**, so a write lands in the copy under its lock and the look that
//! made it syncs it. A merge is noticed elsewhere (`pull_notices`), which only marks the Job's issue
//! row with the pull request's address; the close happens at the next look and is tried again
//! [`SYNC_EVERY`](crate::backlog::SYNC_EVERY) later where it fails, so a merge notice never waits on
//! `bd`.

use std::collections::BTreeMap;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{IssueReference, Notice};
use store::{AttachmentState, Holder, HolderKind, Store};

use crate::backlog::{ClaimHeld, ClaimLoss};
use crate::backlog_syncing::Kept;
use crate::daemon::Fleet;
use crate::plan_mirroring::{writing, Door};
use crate::repositories::Served;
use crate::session_issues::{CLAIMED_AS, CLOSED_IN, ISSUE, MERGED_IN};

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
    /// `url` merged for Job `session`: every issue it holds in `served` is to be closed with its
    /// children. Only marks the rows, and only in a repository whose backlog Fleet writes to.
    pub(crate) async fn issues_merged(&self, served: &Served, session: &str, url: &str) {
        if self.backlog_of(served).is_none() {
            return;
        }
        let manifest = served.manifest().id().as_str().to_string();
        let now = self.clock().now().as_str().to_string();
        let mut store = self.store().lock().await;
        let Ok(rows) = store.attachments_of(&Holder::session(session)) else {
            return;
        };
        for row in rows.iter().filter(|row| {
            row.kind == ISSUE
                && row.manifest_id == manifest
                && row.state == AttachmentState::Standing
                && row.detail.get(MERGED_IN).map(String::as_str) != Some(url)
        }) {
            let mut detail = row.detail.clone();
            detail.insert(MERGED_IN.to_string(), url.to_string());
            let _ = store.follow(row, AttachmentState::Standing, &detail, &now);
        }
    }

    /// Close each issue a merge marked, with every child still open, giving the pull request's
    /// address as the reason. What went wrong, one line each.
    pub(crate) async fn merged_issues_closed(&self, served: &Served, backlog: &Door, kept: &mut Kept, now: Duration) -> Vec<String> {
        let manifest = served.manifest().id().as_str().to_string();
        let due: Vec<(HeldIssue, String)> = {
            let store = self.store().lock().await;
            beads_issues_held(&store, &manifest)
                .into_iter()
                .filter_map(|held| {
                    let url = held.detail.get(MERGED_IN)?.clone();
                    (held.detail.get(CLOSED_IN) != Some(&url)).then_some((held, url))
                })
                .collect()
        };
        let mut failed = Vec::new();
        for (held, url) in due {
            let key = format!("close:{}:{}", held.session, held.issue);
            if !kept.due(&key, now) {
                continue;
            }
            let (root, issue, reason) = (served.root().to_string(), held.issue.clone(), format!("merged in {url}"));
            match writing(backlog, move |door| door.close_tree(&root, &issue, &reason)).await {
                Ok(()) => {
                    kept.failed_at.remove(&key);
                    self.issue_row_noted(&manifest, &held, CLOSED_IN, &url).await;
                }
                Err(why) => {
                    kept.failed_at.insert(key, now);
                    failed.push(format!(
                        "closing {} for the merge of {url} did not happen: {}: {}",
                        held.issue, why.doing, why.said
                    ));
                }
            }
        }
        failed
    }

    /// Put `key` as `value` on the Job's standing row for the issue.
    async fn issue_row_noted(&self, manifest: &str, held: &HeldIssue, key: &str, value: &str) {
        let now = self.clock().now().as_str().to_string();
        let mut store = self.store().lock().await;
        let Ok(rows) = store.attachments_at(ISSUE, &held.issue, Some(manifest)) else {
            return;
        };
        for row in rows.iter().filter(|row| row.holder == Holder::session(&held.session) && row.state == AttachmentState::Standing) {
            let mut detail = row.detail.clone();
            detail.insert(key.to_string(), value.to_string());
            let _ = store.follow(row, AttachmentState::Standing, &detail, &now);
        }
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
            store.session(&claim.session).ok().flatten().and_then(|one| self.ledger_row(&store, &one).ok())
        };
        if let Some(record) = record {
            self.publish(ipc::Event::SessionChanged(record));
        }
    }
}
