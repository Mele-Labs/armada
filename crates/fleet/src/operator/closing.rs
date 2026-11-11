//! The Operator's closes and groups, which write to the tracker only on a go-ahead: an open issue a merged
//! pull request finished without naming it, and a new issue that repeats an open one or belongs under it. **Fleet finds what
//! is near and the call decides**, and what is ruled is kept as a suggestion until he says yes or a rule
//! he kept does it. Only beads takes the write: a close with its reason, or a parent link, nothing else.

use adapter_traits::{AgentHarness, BeadState, Delivery, Vcs, WorkProduct};
use ipc::ManifestId;

use super::candidates::{done_near, Finished};
use super::desk::held_of;
use super::sources::{Filed, MergeNear};
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
    /// The open issues the work of pull request `pull` of Job `id` may have finished, each handed to the
    /// Operator to ask. **No candidate, no call**, and none where the repository's tracker is not
    /// the beads Fleet writes to, or the Operator is off.
    pub(super) async fn finished_asked(&self, id: &str, job: &str, pull: &str, title: &str) {
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

    /// `reference` closed in the repository's beads with `reason`, which is how a close and a group are
    /// written. **Nothing else is written to it**: an issue is not retitled, commented on or moved.
    pub(super) async fn operator_closed_issue(&self, manifest: &str, reference: &str, reason: &str) -> Result<(), String> {
        let served = self.served_named(Some(&ManifestId::carried(manifest))).map_err(|_| "the repository is not served".to_string())?;
        let backlog = self.backlog_of(&served).ok_or("the repository's tracker takes no write from Fleet")?;
        let (root, reference, reason) = (served.root().to_string(), reference.to_string(), reason.to_string());
        let closed = tokio::task::spawn_blocking(move || backlog.put_state(&root, &reference, BeadState::Closed, &reason)).await;
        match closed {
            Ok(Ok(())) => {
                self.tracking().forget(manifest);
                Ok(())
            }
            Ok(Err(why)) => Err(format!("the issue was not closed: {}", why.said)),
            Err(_) => Err("the issue was not closed".to_string()),
        }
    }

    /// `reference` put under `parent` in the repository's beads. **Only the link is written**: the issue
    /// is closed by nothing and keeps its status.
    pub(super) async fn operator_put_under(&self, manifest: &str, reference: &str, parent: &str) -> Result<(), String> {
        let served = self.served_named(Some(&ManifestId::carried(manifest))).map_err(|_| "the repository is not served".to_string())?;
        let backlog = self.backlog_of(&served).ok_or("the repository's tracker takes no write from Fleet")?;
        let (root, reference, parent) = (served.root().to_string(), reference.to_string(), parent.to_string());
        match tokio::task::spawn_blocking(move || backlog.set_parent(&root, &reference, &parent)).await {
            Ok(Ok(())) => {
                self.tracking().forget(manifest);
                Ok(())
            }
            Ok(Err(why)) => Err(format!("the issue was not put under the other: {}", why.said)),
            Err(_) => Err("the issue was not put under the other".to_string()),
        }
    }
}
