//! The clashes of every served repository in one read, and the push when that set moves.
//! `list_all_write_clashes` and `write_clashes.changed`: Electron polls `list_write_clashes` once
//! for each Manifest, and the native Bridge and Pocket read this instead.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{AllWriteClashes, Event, ManifestId, RepositoryClashes, WriteClash};

use crate::daemon::Fleet;

/// The repositories that have a clash, each as it was read, in the order given. One with none is left
/// out, so an empty set is the same whether one repository is served or ten.
pub(crate) fn across(repositories: Vec<(ManifestId, Vec<WriteClash>)>) -> AllWriteClashes {
    AllWriteClashes {
        repositories: repositories
            .into_iter()
            .filter(|(_, clashes)| !clashes.is_empty())
            .map(|(manifest_id, clashes)| RepositoryClashes { manifest_id, clashes })
            .collect(),
    }
}

/// `now` where it differs from the set last sent, which `last` then becomes. **The first read sets the
/// baseline and sends nothing**, so a Bridge that connects is told by the read and not by the push.
pub(crate) fn moved(last: &mut Option<AllWriteClashes>, now: AllWriteClashes) -> Option<AllWriteClashes> {
    let before = last.replace(now.clone())?;
    (before != now).then_some(now)
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
    /// The clashes standing in every served repository.
    pub(crate) async fn all_write_clashes(&self) -> AllWriteClashes {
        let mut read = Vec::new();
        for served in self.repositories().served() {
            let id = ManifestId::carried(served.manifest().id().as_str());
            read.push((id, self.clashes_in(&served).await));
        }
        across(read)
    }

    /// Publish `write_clashes.changed` when the set differs from the one last sent. Run on the notice
    /// loop's tick, after the checks that make a clash and the holders that end take one away.
    pub(crate) async fn clashes_published(self: &Arc<Self>) {
        let now = self.all_write_clashes().await;
        if let Some(moved) = self.writes().sent(now) {
            self.publish(Event::WriteClashesChanged(moved));
        }
    }
}
