//! The Logbook: what Fleet learns from every Session of a repository, with no person's press, so
//! the next agent does not learn it again. `docs/concepts/logbook.md`.
//!
//! **Distil reads, Curate decides.** A Session's record is distilled into claims as it ends and
//! before it hands on ([`distilling`]); plain rules merge them ([`curating`]). Run memory and the
//! machine note keep their own detectors and are read in beside, so one read shows everything.

pub(crate) mod answer;
mod acting;
mod backlog;
pub(crate) mod curating;
mod distilling;
mod kept;
pub(crate) mod serving;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{LogKind, LogState};

use crate::daemon::Fleet;
pub(crate) use distilling::QUESTION;
pub(crate) use serving::{Logged, Source};

/// The Sessions waiting to be distilled, and whether one is being. Never written down: a Fleet
/// that stops loses the queue, and a Session's next distil reads from where its last one ended.
#[derive(Default)]
pub(crate) struct Distils {
    waiting: Mutex<VecDeque<String>>,
    running: AtomicBool,
    /// Whether this process already started the pass over the open lessons.
    backlog: AtomicBool,
}

impl Distils {
    /// Put a Session in the queue, once.
    pub(crate) fn queue(&self, session: &str) {
        let mut waiting = self.waiting.lock().expect("the distil queue is not held across a panic");
        if !waiting.iter().any(|one| one == session) {
            waiting.push_back(session.to_string());
        }
    }

    /// The next Session to distil, where none is being.
    fn next(&self) -> Option<String> {
        if self.running.swap(true, Ordering::AcqRel) {
            return None;
        }
        let next = self.waiting.lock().expect("the distil queue is not held across a panic").pop_front();
        if next.is_none() {
            self.running.store(false, Ordering::Release);
        }
        next
    }
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
    /// Start the next distil on a task of its own, where none is running. Called on the notice
    /// loop's tick, so a Session's end never waits on it.
    pub(crate) fn logbook_distils_run(self: &Arc<Self>) {
        let Some(session) = self.logbook_distils().next() else { return };
        let fleet = Arc::clone(self);
        tokio::spawn(async move {
            // Quiet, as run memory is: nothing was kept, so the Session's next distil reads it again.
            let _ = fleet.logbook_distilled(&session).await;
            fleet.logbook_distils().running.store(false, Ordering::Release);
        });
    }

    /// Everything the Logbook holds for a repository: its entries, the habits and this Mac's, and
    /// what run memory and the machine note know, as entries. Forgotten entries are not in it.
    pub(crate) async fn logbook_read(&self, root: &str) -> Vec<Logged> {
        self.logbook_read_in(Some(root)).await
    }

    /// [`Self::logbook_read`], or with no repository the habits and this Mac's alone.
    pub(crate) async fn logbook_read_in(&self, root: Option<&str>) -> Vec<Logged> {
        let key = root.map(crate::run_memory::places::key_of).unwrap_or_default();
        let learnt = self.store().lock().await.logbook_of(&key).unwrap_or_default();
        let mut all: Vec<Logged> = learnt.into_iter().map(|entry| Logged { entry, from: Source::Learnt }).collect();
        let missing = self.missing_now().await;
        let runs = match root {
            Some(root) => self.store().lock().await.run_facts(root).unwrap_or_default(),
            None => Vec::new(),
        };
        all.extend(
            runs.iter()
                .filter(|fact| fact.succeeded && !crate::machine_note::starts_with_missing(&fact.command, &missing))
                .map(|fact| Logged::folded(Source::Runs, LogKind::Command, crate::run_memory::run_line(fact), fact.last_ran)),
        );
        all.extend(
            missing
                .iter()
                .map(|name| Logged::folded(Source::Machine, LogKind::Environment, crate::machine_note::missing_line(name), 0)),
        );
        let browser = crate::machine_note::browser_line(adapters::browser_shell::found().as_deref());
        all.push(Logged::folded(Source::Machine, LogKind::Environment, browser, 0));
        all
    }

    /// The entries to serve a repository's next agent: standing ones, tripped first, then by how
    /// many Sessions bore each out, then the most recent, cut to `budget` tokens. Each learnt one
    /// served is marked, which keeps it from fading. **For the start note to call.**
    #[allow(dead_code, reason = "the start note calls it, and another change wires that")]
    pub(crate) async fn logbook_to_serve(&self, root: &str, budget: usize) -> Vec<Logged> {
        let served = serving::served(self.logbook_read(root).await, budget);
        let ids: Vec<i64> = served
            .iter()
            .filter(|one| one.from == Source::Learnt && one.entry.state == LogState::Standing)
            .filter_map(|one| one.entry.id)
            .collect();
        let now = self.now().epoch_millis().unwrap_or(0).max(0) as u64;
        let _ = self.store().lock().await.logbook_served(&ids, now);
        served
    }
}
