//! The Fleet half of [`super`]: reading what is remembered, finding links on a checkout, keeping
//! them in run memory, and telling Bridge the run that holds one.

use std::path::PathBuf;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Event, ServerState};
use store::KeptRun;

use super::{as_fact, expects, found, reconciled, remembered, Candidate, Changes, Link, LOG};
use crate::app_runs::RunView;
use crate::daemon::Fleet;

/// The processes of `view` as the scanner reads them.
pub(crate) fn candidates_of(view: &RunView) -> Vec<Candidate> {
    view.processes
        .iter()
        .map(|one| Candidate { name: one.name.clone(), dir: one.dir.clone(), command: one.serve.clone(), pinned: one.pinned })
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
    /// Every fact a repository remembers.
    pub(crate) async fn links_facts(&self, root: &str) -> Vec<KeptRun> {
        self.store().lock().await.run_facts(root).unwrap_or_default()
    }

    /// The links `root` remembers.
    pub(crate) async fn links_remembered(&self, root: &str) -> Vec<Link> {
        remembered(&self.links_facts(root).await)
    }

    /// Write `changes` into run memory, and on to the place that keeps it. **An ended link is
    /// written again as not succeeded, newer than the fact it ends**, since a place that keeps a
    /// file merges the old one back over a row that was only deleted.
    pub(crate) async fn links_kept(&self, root: &str, changes: Changes) {
        if changes.add.is_empty() && changes.end.is_empty() {
            return;
        }
        if self.run_memory_started(root).await.is_err() {
            return;
        }
        let now = self.now().epoch_millis().unwrap_or_default().max(0) as u64;
        let held = self.links_facts(root).await;
        let last = |fact: &KeptRun| held.iter().find(|one| (&one.workspace, &one.kind, &one.command) == (&fact.workspace, &fact.kind, &fact.command)).map_or(0, |one| one.last_ran);
        let mut facts: Vec<KeptRun> = changes
            .add
            .iter()
            .map(|link| as_fact(link, true, now))
            .chain(changes.end.iter().map(|link| as_fact(link, false, now)))
            .collect();
        for fact in &mut facts {
            fact.last_ran = now.max(last(fact) + 1);
        }
        let _ = self.store().lock().await.merge_run_facts(root, &facts);
        self.run_memory_written(root, None).await;
    }

    /// Read the folders of `view`'s run on its checkout, keep what is new, end what the main
    /// checkout's files no longer hold, and answer the links this checkout has: **what its files
    /// say, and what a process's output showed**.
    pub(crate) async fn links_refreshed(&self, view: &RunView) -> Vec<Link> {
        let Ok(served) = self.served_named(Some(&view.manifest_id)) else {
            return Vec::new();
        };
        let root = served.root().to_string();
        let facts = self.links_facts(&root).await;
        let (candidates, tree) = (candidates_of(view), PathBuf::from(&view.checkout));
        let read = facts.clone();
        let names: Vec<String> = candidates.iter().map(|one| one.name.clone()).collect();
        let in_files = tokio::task::spawn_blocking(move || found(&tree, &candidates, &read)).await.unwrap_or_default();
        let kept = remembered(&facts);
        let changes = reconciled(&kept, &in_files, &names, view.checkout == root);
        let mut now: Vec<Link> = in_files;
        now.extend(
            kept.into_iter()
                .filter(|link| link.found_in == LOG && names.contains(&link.from) && names.contains(&link.to))
                .filter(|link| !changes.end.contains(link)),
        );
        self.links_kept(&root, changes).await;
        self.servers().links_seen(&view.checkout, now.clone());
        now
    }

    /// The links to show on `view` without reading its folders: **what this checkout was last read
    /// to hold**, else what the repository remembers of these processes.
    pub(crate) async fn links_shown(&self, view: &RunView) -> Vec<Link> {
        if let Some(seen) = self.servers().links_at(&view.checkout) {
            return seen;
        }
        let Ok(served) = self.served_named(Some(&view.manifest_id)) else {
            return Vec::new();
        };
        let has = |name: &String| view.processes.iter().any(|one| &one.name == name);
        self.links_remembered(served.root()).await.into_iter().filter(|link| has(&link.from) && has(&link.to)).collect()
    }

    /// A process refused a connection to `port`. **If a sibling of its run is known on that port
    /// and is on another now, that is a link**: kept, and told on the run's own event.
    pub(crate) async fn links_noticed(&self, state: &ServerState, port: u16) {
        let Ok(served) = self.served_named(state.manifest_id.as_ref()) else {
            return;
        };
        let root = served.root().to_string();
        for view in self.runs_moved_by(state) {
            let facts = self.links_facts(&root).await;
            let (candidates, tree) = (candidates_of(&view), PathBuf::from(&view.checkout));
            let known = tokio::task::spawn_blocking(move || expects(&tree, &candidates, &facts)).await.unwrap_or_default();
            let at = |name: &str| view.processes.iter().position(|one| one.name == name);
            let Some(from) = at(&state.name) else { continue };
            if known.get(from).is_some_and(|ports| ports.contains(&port)) {
                continue;
            }
            let Some(to) = (0..view.processes.len()).find(|to| {
                *to != from && known.get(*to).is_some_and(|ports| ports.contains(&port)) && view.processes[*to].port() != Some(port)
            }) else {
                continue;
            };
            let current = self.links_shown(&view).await;
            let link = Link {
                from: state.name.clone(),
                to: view.processes[to].name.clone(),
                port,
                found_in: LOG.to_string(),
                workspace: view.processes[from].dir.clone().unwrap_or_else(|| String::from(".")),
            };
            // One link per pair, from a file or from output.
            if current.iter().any(|one| one.from == link.from && one.to == link.to) {
                continue;
            }
            let mut now = current;
            now.push(link.clone());
            self.servers().links_seen(&view.checkout, now);
            self.links_kept(&root, Changes { add: vec![link], end: Vec::new() }).await;
            if let Some(run) = self.app_run_wired(view, None).await {
                self.publish(Event::AppRunChanged(run));
            }
        }
    }
}
