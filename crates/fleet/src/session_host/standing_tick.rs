//! The re-derive that makes a stuck Session reach the person. A Session goes stuck by time passing, which
//! no event says, so each tick of the notice loop looks at every running turn and publishes the Sessions
//! whose standing moved: one that went quiet past the bound, and one heard from again.
//! `docs/concepts/session.md`, *What a Session is doing*.

use std::collections::HashSet;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use crate::daemon::Fleet;

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
    /// Publish each Session whose stuck standing moved since the last look.
    pub async fn standing_checked(self: &Arc<Self>) {
        let after = self.hosts().stuck_after();
        let stuck: HashSet<String> = self
            .hosts()
            .turns_quiet()
            .into_iter()
            .filter(|(_, quiet)| *quiet > after)
            .map(|(id, _)| id)
            .collect();
        // A reply the language model has read since: its call may have moved.
        let read = self.reply_reads().settled().into_iter().chain(self.refusal_reads().settled());
        for id in self.hosts().stuck_moved(stuck).into_iter().chain(read) {
            let _ = self.published_hosted(&id).await;
        }
    }
}
