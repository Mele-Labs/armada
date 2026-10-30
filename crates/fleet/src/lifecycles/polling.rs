//! The readers that ask a source: the forge for a pull request, the tracker for a ticket, and a
//! watch for what it follows. **None of them has a loop of its own.** Each is called where Fleet
//! already has the reading in hand, at the cadence it already keeps, and none asks a source twice
//! for what the ledger just asked it.

use adapter_traits::{AgentHarness, Delivery, PullRequestStanding, ReadIssue, Vcs, WorkProduct};
use core_model::{Observed, ReadingState};
use ipc::{ManifestId, PullRequestState};

use super::keeping::{PULL_REQUEST, TICKET};
use super::reading::{forge_fact, tracker_word};
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
    /// The live Jobs in `manifest_id` that hold `lifecycle` `key`.
    async fn lifecycle_holders_of(&self, lifecycle: &str, manifest_id: &str, key: &str) -> Vec<String> {
        let store = self.store().lock().await;
        let Ok(holders) = store.live_lifecycle_holders(lifecycle) else { return Vec::new() };
        holders
            .into_iter()
            .filter(|job| {
                let here = store.session(job).ok().flatten().and_then(|one| one.manifest_id);
                here.as_deref() == Some(manifest_id)
                    && store
                        .lifecycle_instances_of(job)
                        .is_ok_and(|held| held.iter().any(|one| one.lifecycle == lifecycle && one.key == key))
            })
            .collect()
    }

    /// A pull request as the pull ledger just read it. **The forge is asked once more, for its
    /// reviewers and reviews, and only for an open pull request some Job holds**: draft, merged and
    /// closed are in what was read, and a forge that stays silent leaves the instance as it was.
    pub(crate) async fn lifecycle_pull_seen(&self, manifest_id: &ManifestId, root: &str, state: &PullRequestState) {
        let number = state.number.to_string();
        let holders = self.lifecycle_holders_of(PULL_REQUEST, manifest_id.as_str(), &number).await;
        if holders.is_empty() {
            return;
        }
        let standing = match state.state {
            ipc::PullRequestStanding::Draft => PullRequestStanding::Draft,
            ipc::PullRequestStanding::Open => PullRequestStanding::Open,
            ipc::PullRequestStanding::Merged => PullRequestStanding::Merged,
            ipc::PullRequestStanding::Closed => PullRequestStanding::Closed,
        };
        let page = match standing {
            PullRequestStanding::Open => {
                let asked = number.clone();
                self.forge_asked(root, move |vcs: &V, root: &str| vcs.pull_request_page(root, &asked)).await
            }
            _ => None,
        };
        let Some(fact) = forge_fact(standing, page.as_ref()) else { return };
        for job in holders {
            self.lifecycle_seen(&job, PULL_REQUEST, &number, Observed::Forge(fact)).await;
        }
    }

    /// An issue as the issue ledger just read it, handed to every live Job that holds it.
    pub(crate) async fn lifecycle_ticket_seen(&self, manifest_id: &ManifestId, issue: &str, read: &ReadIssue) {
        let word = tracker_word(read);
        for job in self.lifecycle_holders_of(TICKET, manifest_id.as_str(), issue).await {
            self.lifecycle_seen(&job, TICKET, issue, Observed::Tracker(&word)).await;
        }
    }

    /// A reading a watch just kept, handed to what the watch follows.
    pub(crate) async fn lifecycle_watch_seen(&self, watch: &str, state: ReadingState) {
        let Ok(followed) = self.store().lock().await.lifecycle_instances_followed_by(watch) else { return };
        for one in followed {
            self.lifecycle_seen(&one.session_id, &one.lifecycle, &one.key, Observed::Watch(state)).await;
        }
    }
}
