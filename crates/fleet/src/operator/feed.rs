//! The feed on the Operator's page: what happened since he looked that needs no decision. A Job that ended and a pull
//! request that merged are read from the ledger; the Operator's dispatches, what a Job asked or told it and the cards it
//! settled are kept as they happen. `crates/ipc/operations/operator_page.toml`.


use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::operator_page::{OperatorFeedEntry, OperatorFeedKind};
use ipc::{Instant, SessionId};
use store::KeptFeed;

use super::talking::moved_since;
use crate::daemon::Fleet;

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
    let (_, merged) = moved_since(store, since);
    lines.extend(merged.into_iter().map(|one| OperatorFeedEntry {
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
    /// Keep one line of the feed and tell the page. A line that cannot be kept is only a line the feed lacks.
    pub(crate) async fn feed_kept(&self, kind: OperatorFeedKind, title: &str, session: Option<&str>, said: Option<&str>) {
        let line = KeptFeed { at: self.now().as_str().to_string(), kind: word(kind).to_string(), title: title.to_string(), session_id: session.map(str::to_string), said: said.map(str::to_string) };
        let _ = self.store().lock().await.put_operator_feed(&line);
        self.operator_page_changed();
    }
}
