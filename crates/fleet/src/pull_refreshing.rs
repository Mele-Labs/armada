//! A person's refresh of one pull request: Fleet reads it from the forge now, rather than at the
//! next pull watch, and does for it what a watch reading does. `crate::pull_notices`,
//! `crate::main_hub`.
//!
//! **Told as the watch tells, and never twice**: the notices go through the watch's own body, its
//! record of the merge queue and its record of what was told, so a refresh and the next reading
//! say each thing once between them.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{PullRequestStanding, PullRequestState};

use crate::daemon::Fleet;
use crate::main_hub::OpenPulled;
use crate::repositories::Served;

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
    /// Everything the pull watch does for one pull request, after `state` was read and the ledger
    /// followed it: an open one's owners told and its merge line entry read again, a merged one's
    /// merge told on the watch's rule, and one that is over taken off the merge line.
    pub(crate) async fn pull_request_refreshed(self: &Arc<Self>, served: &Served, state: &PullRequestState) {
        let root = served.root().to_string();
        let base = served.manifest().base().unwrap_or("the base").to_string();
        let _reading = self.pulls_reading(&root).await;
        match state.state {
            PullRequestStanding::Open | PullRequestStanding::Draft => {
                self.open_pull_told(served, &base, state.number).await;
                self.open_pull_listed(served, state.number).await;
            }
            PullRequestStanding::Merged => {
                // From the forge's list of merges, as the watch tells it: a merge hours old is not news.
                self.notice_merged(served).await;
                let mine = self.mine_in(&root).await;
                self.merges_told(served, &base, &mine).await;
                self.off_the_merge_line(&root, state.number).await;
            }
            PullRequestStanding::Closed => self.off_the_merge_line(&root, state.number).await,
        }
    }

    /// The watch's notices for one open pull request, and **only this number moved in the queue
    /// last seen**: the rest of it is the watch's.
    async fn open_pull_told(self: &Arc<Self>, served: &Served, base: &str, number: u64) {
        let gate = served.manifest().ci_gate().to_string();
        let Some(pull) = self
            .forge_asked(served.root(), move |vcs: &V, root: &str| {
                vcs.watched_pull(root, &gate, number)
            })
            .await
        else {
            return;
        };
        let root = served.root();
        let mine = self.mine_in(root).await;
        // **Only the person's own**, as the watch.
        if mine.owns(pull.author.as_ref().map(|it| it.as_written())) {
            let before = self.store().lock().await.pulls_seen_queued(root).unwrap_or_default();
            let queued = self.watched_told(served, base, &before, &pull).await;
            let mut kept: Vec<u64> = before.into_iter().filter(|one| *one != number).collect();
            if queued {
                kept.push(number);
            }
            let _ = self.store().lock().await.keep_pulls_queued(root, &kept);
        }
        self.pulls_attached(served, std::slice::from_ref(&pull)).await;
    }

    /// The merge line's entry for one open pull request, read again with its owner. **A forge that
    /// will not answer keeps the last reading**, and a repository not listed yet waits for its visit.
    async fn open_pull_listed(&self, served: &Served, number: u64) {
        let gate = served.manifest().ci_gate().to_string();
        let Some(pull) = self
            .forge_asked(served.root(), move |vcs: &V, root: &str| {
                vcs.open_pull_request(root, &gate, number)
            })
            .await
        else {
            return;
        };
        let root = served.root();
        let read = OpenPulled {
            owner: self.owners_read(root, vec![number]).await.remove(&number),
            mine: self.mine_in(root).await.owns(pull.author.as_ref().map(|it| it.as_written())),
            pull,
        };
        let mut sweep = self.sweeping().lock().await;
        let Some(pulls) = sweep.pulls.get_mut(root) else { return };
        match pulls.iter_mut().find(|one| one.pull.number == number) {
            Some(one) => *one = read,
            None => pulls.push(read),
        }
    }

    /// Take a pull request that is over off the merge line, and delete its owner row: **the
    /// watch deletes it when it leaves the list**, and once off, the watch never sees it leave.
    async fn off_the_merge_line(&self, root: &str, number: u64) {
        let gone = {
            let mut sweep = self.sweeping().lock().await;
            let Some(pulls) = sweep.pulls.get_mut(root) else { return };
            let listed = pulls.len();
            pulls.retain(|one| one.pull.number != number);
            pulls.len() != listed
        };
        if gone {
            self.owners_deleted(root, vec![number]).await;
        }
    }
}
