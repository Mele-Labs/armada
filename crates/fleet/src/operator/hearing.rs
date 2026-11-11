//! Fleet hearing, by itself, that a hosted Job ended or one of its pull requests merged. **Every
//! hosted Job, whoever started it**, on its own tap beside the one that tells a parent
//! (`linked_following`), which it leaves as it is.
//!
//! **Each is heard once, and makes no call of its own**, and goes in the Operator's journal of the week. The store keeps `operator_ended` and `operator_merged`,
//! under names of their own so neither shadows what a parent was told. Nothing is raised, proposed or
//! ruled: the Operator is only told what finished, and Discussion reads it. **A merge that left an open
//! issue near its work, which it did not name, is handed on to ask whether it finished that issue**:
//! none near, no call. **Nothing is replayed**: when the tap starts, every end and merge already there
//! is marked heard. A Job handed on for context is no end: its work goes on in its successor.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Event, ManifestId, SessionRecord};
use store::{AttachmentState, Holder};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::candidates::{done_near, Finished};
use super::desk::held_of;
use super::sources::{Filed, MergeNear};
use crate::daemon::Fleet;
use crate::session_host::replacing::REPLACED_BY;

const ENDED: &str = "operator_ended";
const MERGED: &str = "operator_merged";
/// Start hearing ends and merges. **The end and merge already there are marked seen first**, so
/// only those after this are heard. Production starts it beside the parents' tap, and a
/// test starts it itself.
pub async fn keep_the_ends_told<H, V, W>(fleet: Arc<Fleet<H, V, W>>) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fleet.ends_marked_seen().await;
    let (tell, mut heard) = mpsc::unbounded_channel::<SessionRecord>();
    fleet.events().tap(move |event| {
        if let Event::SessionChanged(record) = event {
            if record.state == ipc::SessionState::Ended || record.attachments.iter().any(|one| one.kind == "pr" && crate::after_following::merged(one)) {
                let _ = tell.send(record.clone());
            }
        }
    });
    tokio::spawn(async move {
        while let Some(record) = heard.recv().await {
            fleet.end_heard(record).await;
        }
    })
}

/// Whether `record` is a Job that ended with its work its own: not one handed on for context.
fn ended_for_good(record: &SessionRecord) -> bool {
    record.state == ipc::SessionState::Ended && !record.attachments.iter().any(|one| one.kind == REPLACED_BY)
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
    /// Every end and merge of a hosted Job that is there now, marked handed. Nothing is said.
    async fn ends_marked_seen(&self) {
        let now = self.now();
        let mut store = self.store().lock().await;
        for hosting in store.hostings().unwrap_or_default() {
            let id = hosting.session_id;
            let held = store.attachments_of(&Holder::session(&id)).unwrap_or_default();
            if store.session(&id).ok().flatten().is_some_and(|one| one.state == store::SessionState::Ended) && !held.iter().any(|one| one.kind == REPLACED_BY) {
                let _ = store.keep_linked_told(&id, ENDED, "", &now);
            }
            for pull in held.iter().filter(|one| one.kind == "pr" && crate::pull_requesting::pull_merged(&one.detail, one.state == AttachmentState::Spent)) {
                let _ = store.keep_linked_told(&id, MERGED, &pull.target, &now);
            }
        }
    }

    /// Whether this is the first time `event` under `key` is seen for `job`, and keeps it.
    async fn heard_once(&self, job: &str, event: &str, key: &str) -> bool {
        let now = self.now();
        self.store().lock().await.keep_linked_told(job, event, key, &now).unwrap_or(false)
    }

    /// One record of a Job that ended or holds a merged pull request: each end and merge in it that
    /// was not heard before is marked. **No call is made and nothing is raised or suggested**: an
    /// end and a merge only tell the Operator what finished, which Discussion reads from the store.
    async fn end_heard(self: &Arc<Self>, record: SessionRecord) {
        let id = record.id.as_str().to_string();
        if self.store().lock().await.hosting(&id).ok().flatten().is_none() {
            return;
        }
        let job = record.title.clone().unwrap_or_else(|| id.clone());
        if ended_for_good(&record) && self.heard_once(&id, ENDED, "").await {
            self.operator_journaled(&format!("Ended: {job}")).await;
        }
        for pull in record.attachments.iter().filter(|one| one.kind == "pr" && crate::after_following::merged(one)) {
            if self.heard_once(&id, MERGED, &pull.target).await {
                let title = pull.detail.get("title").cloned().unwrap_or_default();
                self.operator_journaled(&format!("Merged: #{} {title}, held by {job}", pull.target)).await;
                self.finished_asked(&id, &job, &pull.target, &title).await;
            }
        }
    }

    /// The open issues the work of pull request `pull` of Job `id` may have finished, each handed to the
    /// Operator to ask. **No candidate, no call**, and none where the repository's tracker is not
    /// the beads Fleet writes to, or the Operator is off.
    async fn finished_asked(&self, id: &str, job: &str, pull: &str, title: &str) {
        if !self.operator_on() {
            return;
        }
        let (manifest, paths, named, words) = {
            let store = self.store().lock().await;
            let Some(hosting) = store.hosting(id).ok().flatten() else { return };
            let (paths, named, words) = held_of(&store, id);
            (hosting.manifest_id, paths, named, words)
        };
        if !self.files_in(&manifest) {
            return;
        }
        let open = self.open_issues(&ManifestId::carried(&manifest)).await.unwrap_or_default();
        let open: Vec<(Filed, String)> = open
            .iter()
            .filter(|one| !one.closed)
            .map(|one| (Filed { reference: one.reference.clone(), title: one.title.as_written().to_string() }, one.excerpt.as_written().to_string()))
            .collect();
        let said = format!("{job} {words}");
        for issue in done_near(&Finished { title, words: &said, paths: &paths, named: &named }, &open) {
            let near = MergeNear { manifest_id: manifest.clone(), pull: pull.to_string(), title: title.to_string(), job: (id.to_string(), job.to_string()), words: words.clone(), paths: paths.clone(), named: named.clone(), issue };
            self.hand_once(near.arrival()).await;
        }
    }
}
