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
pub(crate) mod noting;
pub(crate) mod recalling;
pub(crate) mod serving;
pub(crate) mod tidying;

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{LogKind, LogSection, LogState};

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
    /// Whether a tidy is running, and when each repository's last call was asked, so one that
    /// failed waits a day.
    tidying: AtomicBool,
    tried: Mutex<BTreeMap<String, u64>>,
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
    /// served is marked, which keeps it from fading. The start note calls it.
    pub(crate) async fn logbook_to_serve(&self, root: &str, budget: usize) -> Vec<Logged> {
        let served = serving::served(self.logbook_read(root).await, budget);
        self.logbook_marked(&served).await;
        served
    }

    /// Mark the learnt standing entries among `served` as served now.
    async fn logbook_marked(&self, served: &[Logged]) {
        let ids: Vec<i64> = served
            .iter()
            .filter(|one| one.from == Source::Learnt && one.entry.state == LogState::Standing)
            .filter_map(|one| one.entry.id)
            .collect();
        let now = self.now().epoch_millis().unwrap_or(0).max(0) as u64;
        let _ = self.store().lock().await.logbook_served(&ids, now);
    }

    /// The Logbook note a Session's process is started with: habits, then the repository's
    /// standing entries, then this Mac, about [`noting::NOTE_TOKENS`] in all. `root` is `None` for
    /// a Session with no repository, which is handed the habits and this Mac's.
    pub(crate) async fn logbook_note_of(&self, root: Option<&str>) -> Option<String> {
        let served = self.logbook_to_serve(root.unwrap_or_default(), noting::NOTE_TOKENS - noting::HEADER_TOKENS).await;
        noting::note(&served)
    }

    /// What a subagent starting under `session` is handed for the paths its `prompt` names, as a
    /// note. Empty where the Logbook holds nothing for it. A Session Fleet does not know, or one
    /// with no repository, is handed the habits alone.
    pub(crate) async fn logbook_for_subagent(&self, session: &str, prompt: &str) -> String {
        let manifest = {
            let store = self.store().lock().await;
            store.session(session).ok().flatten().and_then(|row| row.manifest_id)
        };
        let root = manifest.and_then(|id| self.served_named(Some(&ipc::ManifestId::carried(&id))).ok()).map(|one| one.root().to_string());
        let named = recalling::named_paths(prompt, root.as_deref().unwrap_or_default());
        let all = self.logbook_read(root.as_deref().unwrap_or_default()).await;
        let handed = recalling::for_subagent(all, &named, recalling::SUBAGENT_TOKENS - noting::SUBAGENT_HEADER_TOKENS);
        self.logbook_marked(&handed).await;
        noting::subagent_note(&handed).unwrap_or_default()
    }

    /// The entries of the repository `manifest` names, the habits and this Mac's, that hold the
    /// words of `query`, best first. Marks nothing served.
    pub(crate) async fn logbook_recalled(
        &self,
        manifest: Option<&ipc::ManifestId>,
        query: &str,
        section: Option<&str>,
    ) -> Result<ipc::Recalled, api::Refusal> {
        let unasked = |what: String| {
            api::Refusal::Unacceptable(ipc::WireError::raised("fleet.session_unnamed", what, self.run_id()))
        };
        if query.trim().is_empty() {
            return Err(unasked("recall needs words to look for".to_string()));
        }
        let section = match section {
            None => None,
            Some(word) => Some(LogSection::of_text(word.trim()).ok_or_else(|| unasked(format!("`{word}` is not a section: `repo`, `habits` or `machine`")))?),
        };
        let root = self.served_named(manifest)?.root().to_string();
        let found = recalling::searched(self.logbook_read(&root).await, query, section);
        Ok(ipc::Recalled {
            entries: found
                .into_iter()
                .map(|one| ipc::RecalledEntry {
                    section: one.entry.section.as_text().to_string(),
                    kind: one.entry.kind.as_text().to_string(),
                    state: one.entry.state.as_text().to_string(),
                    confirmed_by: one.borne_out() as u32,
                    text: one.entry.text,
                })
                .collect(),
        })
    }
}
