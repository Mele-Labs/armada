//! A settled subagent its parent sent more work. `docs/concepts/session.md`, *A terminal
//! session's thread*.
//!
//! **A resume is no new spawn**, so nothing the mod hears says it began again: its row read spent
//! while it worked (14 Oct 2026). Fleet reads that from the subagent's own transcript instead. Only
//! work written after the row settled counts, since a transcript can end on a line that reads as
//! still working (13 Oct 2026) and the stop came after that line.

use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{AttachmentState, KeptAttachment};

use crate::daemon::Fleet;

/// How often the notice loop looks: a resumed subagent reads as running within this.
pub(crate) const EVERY: Duration = Duration::from_secs(2);

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
    /// Every settled subagent of a live session whose transcript shows work after it settled is
    /// standing again, and its session published. An unchanged transcript costs a `stat`.
    pub(crate) async fn subagents_followed(&self) {
        let Ok(settled) = self.store().lock().await.settled_subagents() else {
            return;
        };
        if settled.is_empty() {
            return;
        }
        let home = self.host().home.clone();
        let Ok(resumed) = tokio::task::spawn_blocking(move || {
            settled
                .into_iter()
                .filter(|one| {
                    adapters::terminal_thread::subagent_working_at(&home, &one.holder.id, &one.target)
                        .is_some_and(|at| at > one.changed_at)
                })
                .collect::<Vec<KeptAttachment>>()
        })
        .await
        else {
            return;
        };
        let now = self.now().as_str().to_string();
        let mut changed = Vec::new();
        {
            let mut store = self.store().lock().await;
            for one in resumed {
                // A stop that landed since the read is the later word.
                let still = store.attachments_of(&one.holder).unwrap_or_default().into_iter().any(|kept| {
                    kept.kind == one.kind && kept.target == one.target && kept.changed_at == one.changed_at
                });
                if !still || !matches!(store.follow(&one, AttachmentState::Standing, &one.detail, &now), Ok(true)) {
                    continue;
                }
                let Some(session) = store.session(&one.holder.id).ok().flatten() else {
                    continue;
                };
                if let Ok(record) = self.ledger_row(&store, &session) {
                    changed.push(record);
                }
            }
        }
        for record in changed {
            self.publish(ipc::Event::SessionChanged(record));
        }
    }
}
