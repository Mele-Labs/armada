//! What a Fleet rule's `when` asks about a Job at the moment a notice is sent. `ipc::Condition`.
//!
//! **Every answer comes from what Fleet already keeps**: the ledger's record of the Job's pull
//! requests, the forge reading the pull request watch last made, the Job's own turn and the sleep
//! switch. The forge is never called here, and **a fact Fleet does not have is a condition that does
//! not hold**, never one that is guessed.

use super::*;

use crate::pull_requesting::PR;

/// A pull request a Job holds that is open, as the ledger last followed it.
struct Open {
    number: u64,
    /// The ledger's reading of its required checks: `failed` is the one that blocks.
    checks_failed: bool,
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
    /// What `rule` does with the notice now: the first of its `when` that holds replaces its own
    /// `does`. Conditions are asked in order and no further than the first that holds.
    pub(super) async fn does_now(&self, rule: &ipc::FleetRule, session: &str, manifest: &str) -> ipc::Does {
        for instead in &rule.when {
            if self.condition_holds(instead.condition, session, manifest).await {
                return instead.does;
            }
        }
        rule.does
    }

    async fn condition_holds(&self, condition: ipc::Condition, session: &str, manifest: &str) -> bool {
        match condition {
            ipc::Condition::Working => matches!(self.hosts().of(session).state().turn, ipc::SessionTurn::Working { .. }),
            ipc::Condition::OwnerAway => self.sleeping().await,
            ipc::Condition::OpenPullRequest => !self.held_open_pulls(session).await.is_empty(),
            ipc::Condition::BlocksMerge => self.a_pull_of_blocks(session, manifest).await,
        }
    }

    /// The pull requests `session` holds that are open or drafts, off the ledger.
    async fn held_open_pulls(&self, session: &str) -> Vec<Open> {
        let store = self.store().lock().await;
        let rows = store.attachments_of(&Holder::session(session)).unwrap_or_default();
        rows.into_iter()
            .filter(|row| row.kind == PR && row.state == store::AttachmentState::Standing)
            .filter(|row| matches!(row.detail.get("state").map(String::as_str), Some("open" | "draft")))
            .filter_map(|row| {
                Some(Open { number: row.target.parse().ok()?, checks_failed: row.detail.get("checks").map(String::as_str) == Some("failed") })
            })
            .collect()
    }

    /// Whether one of the Job's open pull requests cannot merge now: its required checks failed, the
    /// forge's last listing says it conflicts, or the merge queue marked it unmergeable. **Behind its
    /// base is not read**, since nothing Fleet keeps says a pull request is behind.
    async fn a_pull_of_blocks(&self, session: &str, manifest: &str) -> bool {
        let open = self.held_open_pulls(session).await;
        if open.iter().any(|one| one.checks_failed) {
            return true;
        }
        let Some(root) = self.repositories().serving(manifest).map(|served| served.root().to_string()) else { return false };
        let sweep = self.sweeping().lock().await;
        let conflicting = |number: u64| {
            sweep.pulls.get(&root).is_some_and(|listed| {
                listed.iter().any(|one| one.pull.number == number && one.pull.facts.mergeable == Some(adapter_traits::PullMergeable::Conflicting))
            })
        };
        let unmergeable = |number: u64| {
            sweep.queues.get(&root).is_some_and(|queue| {
                queue.iter().any(|entry| entry.number == number && entry.state == adapter_traits::QueueState::Unmergeable)
            })
        };
        open.iter().any(|one| conflicting(one.number) || unmergeable(one.number))
    }
}
