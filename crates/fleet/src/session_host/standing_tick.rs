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
        let extracted = self.extract_reads().settled();
        // A reply the language model has read since, or items it has embedded: its list may have moved.
        let read = self
            .reply_reads()
            .settled()
            .into_iter()
            .chain(self.refusal_reads().settled())
            .chain(self.item_vectors().settled())
            .chain(extracted);
        let hung = stuck.clone();
        for id in self.hosts().stuck_moved(stuck).into_iter().chain(read) {
            let _ = self.published_hosted(&id).await;
        }
        self.hung_repaired(&hung).await;
    }

    /// A turn quiet past the bound gets a hidden repair that stops the command holding it: **once a turn**,
    /// one repair at a time across Fleet, and never for a Job whose row lists anything waiting on him.
    /// Only a hosted agent has a process to name; a terminal's turn is left as it reads.
    async fn hung_repaired(self: &Arc<Self>, stuck: &HashSet<String>) {
        if self.repairs().running_any() {
            return;
        }
        let mut ids: Vec<&String> = stuck.iter().collect();
        ids.sort();
        for id in ids {
            let Some(pid) = self.hosts().of(id).state().process.as_ref().and_then(|process| process.pid()) else { continue };
            if self.store().lock().await.session_stop(id).is_ok_and(|kept| kept.repair_job.is_some()) {
                continue;
            }
            let Some(record) = self.stop_read_again(id).await else { continue };
            if record.standing != Some(ipc::SessionStanding::Stuck) || !record.waiting_for.is_empty() {
                continue;
            }
            let saw = vec![(String::from("Its agent's process id"), pid.to_string())];
            self.repair_tried(id, crate::repair::RepairBreak::Hung, saw).await;
            return;
        }
    }
}
