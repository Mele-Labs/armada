//! The feed on the Operator's page: what happened since he looked that needs no decision. A Job that ended and a pull
//! request that merged are read from the ledger; the Operator's dispatches, what a Job asked or told it and the cards it
//! settled are kept as they happen. `crates/ipc/operations/operator_page.toml`.


use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::operator_page::{OperatorFeedEntry, OperatorFeedKind};
use ipc::{Instant, SessionId};
use store::KeptFeed;

use crate::daemon::Fleet;

/// The most pull requests one read of the merged shows.
const MERGED_SHOWN: usize = 30;

/// A pull request a hosted Job held that merged.
pub(super) struct MergedPull {
    pub(super) session_id: String,
    pub(super) job: String,
    pub(super) number: String,
    pub(super) title: String,
    /// When its attachment last changed, which for a merged one is when it merged. An RFC 3339 instant.
    pub(super) at: String,
}

/// The pull requests hosted Jobs held that merged at or after `since`, newest first.
pub(super) fn merged_since(store: &store::Store, since: &str) -> Vec<MergedPull> {
    let mut merged: Vec<MergedPull> = Vec::new();
    for hosting in store.hostings().unwrap_or_default() {
        let job = store.session(&hosting.session_id).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| hosting.session_id.clone());
        for pull in store.attachments_of(&store::Holder::session(&hosting.session_id)).unwrap_or_default() {
            let spent = pull.state == store::AttachmentState::Spent;
            if pull.kind == "pr" && crate::pull_requesting::pull_merged(&pull.detail, spent) && pull.changed_at.as_str() >= since {
                let title = pull.detail.get("title").cloned().unwrap_or_default();
                merged.push(MergedPull { session_id: hosting.session_id.clone(), job: job.clone(), number: pull.target.clone(), title, at: pull.changed_at.clone() });
            }
        }
    }
    merged.sort_by(|a, b| b.at.cmp(&a.at));
    merged.truncate(MERGED_SHOWN);
    merged
}

/// The line a kind is kept under.
fn word(kind: OperatorFeedKind) -> &'static str {
    match kind {
        OperatorFeedKind::Dispatched => "dispatched",
        OperatorFeedKind::Merged => "merged",
        OperatorFeedKind::Ended => "ended",
        OperatorFeedKind::Asked => "asked",
        OperatorFeedKind::Told => "told",
        OperatorFeedKind::Settled => "settled",
    }
}

fn kind_of(word: &str) -> Option<OperatorFeedKind> {
    [OperatorFeedKind::Dispatched, OperatorFeedKind::Asked, OperatorFeedKind::Told, OperatorFeedKind::Settled].into_iter().find(|kind| self::word(*kind) == word)
}

/// The feed since `since`, newest first: what was kept, the Jobs that ended and the pull requests that merged.
pub(super) fn feed_read(store: &store::Store, since: &str) -> Vec<OperatorFeedEntry> {
    let mut lines: Vec<OperatorFeedEntry> = store
        .operator_feed_since(since)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|line| {
            Some(OperatorFeedEntry {
                at: Instant::carried(&line.at),
                kind: kind_of(&line.kind)?,
                title: line.title,
                session_id: line.session_id.as_deref().map(SessionId::carried),
                number: None,
                said: line.said,
            })
        })
        .collect();
    lines.extend(merged_since(store, since).into_iter().map(|one| OperatorFeedEntry {
        at: Instant::carried(&one.at),
        kind: OperatorFeedKind::Merged,
        title: one.title,
        session_id: Some(SessionId::carried(&one.session_id)),
        number: Some(one.number),
        said: None,
    }));
    for session in store.sessions_ended_since(since, 60).unwrap_or_default() {
        let Some(at) = session.ended_at.as_deref() else { continue };
        let said = store.done_summary(&session.id).ok().flatten();
        lines.push(OperatorFeedEntry { at: Instant::carried(at), kind: OperatorFeedKind::Ended, title: session.title.unwrap_or_else(|| session.id.clone()), session_id: Some(SessionId::carried(&session.id)), number: None, said });
    }
    lines.sort_by(|a, b| b.at.as_str().cmp(a.at.as_str()));
    lines
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
    /// Keep one line of `project`'s Operator's feed and tell the page. A line that cannot be kept is only a line the feed lacks.
    pub(crate) async fn feed_kept(&self, project: Option<&str>, kind: OperatorFeedKind, title: &str, session: Option<&str>, said: Option<&str>) {
        let line = KeptFeed { at: self.now().as_str().to_string(), kind: word(kind).to_string(), title: title.to_string(), session_id: session.map(str::to_string), said: said.map(str::to_string), project: project.map(str::to_string) };
        let _ = self.store().lock().await.put_operator_feed(&line);
        self.operator_page_changed();
    }
}
