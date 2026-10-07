//! The `needs` commit status on every open pull request of a served repository.
//! `docs/capabilities/needs.md`, *What a person sees*.
//!
//! **Pending while a need ahead of the holder's has not merged, success
//! otherwise**, including a pull request with no needs at all and one a terminal
//! session Fleet does not know about. The holder is found by branch, as
//! `armada need` finds it (`ledger::holder_of_branch`), and the order is the
//! ledger's, read here and never kept.
//!
//! **Published when the order changes** (a need declared, given back, a Job
//! ended) and **on the pull request listing's own sweep**, so a change nobody
//! announced heals. **Only a state or description that differs from what was
//! last put on that commit is sent.** A forge or token that refuses is a line in
//! the owning Job's log, once, and never a failure of anything Fleet was doing.

use adapter_traits::{AgentHarness, CommitStatus, Delivery, StatusState, Vcs, WorkProduct};
use core_model::Level;

use super::ledger::{behind, holder_of_branch, standing, waiting_text};
use crate::daemon::Fleet;
use crate::repositories::Served;

/// The status' context, as a ruleset would name it.
pub(crate) const CONTEXT: &str = "needs";

/// The most a forge keeps of a status description.
const DESCRIPTION_MAX: usize = 140;

/// What the sweep last put on each commit, as `repository@commit`.
pub(crate) type Published = std::collections::BTreeMap<String, (StatusState, String)>;

fn clipped(said: String) -> String {
    if said.chars().count() <= DESCRIPTION_MAX {
        return said;
    }
    let kept: String = said.chars().take(DESCRIPTION_MAX - 1).collect();
    format!("{kept}…")
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
    /// Put `needs` on each open pull request as last listed. **Asks the forge
    /// nothing but the writes**: the listing is the sweep's.
    pub(crate) async fn needs_status_published(&self, served: &Served) {
        let root = served.root().to_string();
        let manifest = served.manifest().id().as_str().to_string();
        let pulls = self
            .sweeping()
            .lock()
            .await
            .pulls
            .get(&root)
            .cloned()
            .unwrap_or_default();
        if pulls.is_empty() {
            return;
        }
        self.gone_branches_given_back(served).await;
        let wanted: Vec<_> = {
            let store = self.store().lock().await;
            let Ok(all) = standing(&store, &manifest, None) else {
                return;
            };
            pulls
                .iter()
                .filter_map(|open| {
                    let commit = open.pull.head.clone()?;
                    let branch = open.pull.branch.as_written();
                    let holder = holder_of_branch(&store, &manifest, branch).ok()?;
                    let waits = behind(&all, &holder);
                    let (state, said) = match waits.is_empty() {
                        true => (
                            StatusState::Success,
                            "no need ahead of this pull request".to_string(),
                        ),
                        false => (StatusState::Pending, clipped(waiting_text(&waits))),
                    };
                    Some((commit, state, said, open.job.clone()))
                })
                .collect()
        };
        for (commit, state, said, job) in wanted {
            let key = format!("{root}@{commit}");
            let unchanged = self
                .sweeping()
                .lock()
                .await
                .needs_published
                .get(&key)
                .is_some_and(|last| *last == (state, said.clone()));
            if unchanged {
                continue;
            }
            let status = CommitStatus {
                commit: commit.clone(),
                context: CONTEXT.to_string(),
                state,
                description: said.clone(),
            };
            let done = self
                .forge_asked(&root, move |vcs: &V, root: &str| {
                    vcs.publish_status(root, &status)
                })
                .await;
            let mut sweep = self.sweeping().lock().await;
            match done {
                Ok(()) => {
                    sweep.needs_published.insert(key.clone(), (state, said));
                    sweep.needs_refused.remove(&key);
                }
                Err(why) => {
                    let first = sweep.needs_refused.insert(key);
                    drop(sweep);
                    if let (true, Some(job)) = (first, job) {
                        self.said_about_the_ledger(
                            &job,
                            Level::Warn,
                            "the needs status could not be put on the pull request",
                            Some(&why),
                        );
                    }
                }
            }
        }
    }

    /// The same, for the repository a Job's work is in. A Job whose repository
    /// is not served has nothing to publish.
    pub(crate) async fn needs_status_republished_for(&self, job: &core_model::JobId) {
        if let Ok(served) = self.served_by_id(job) {
            self.needs_status_published(&served).await;
        }
    }
}
