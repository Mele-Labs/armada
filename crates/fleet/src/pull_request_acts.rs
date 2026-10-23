//! The three writes a Session makes on a pull request, ready, merge and auto-merge, and the
//! reply they give.
//!
//! **The reply is the pre-read plus what the write changed.** Every forge call is made off the
//! runtime's threads, and the read that follows a write, with the ledger that follows it, runs
//! after the reply in a task of its own. A refusal is decided by the pre-read or the write and
//! comes back as the reply; a failure of the later read is logged, because the press succeeded.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ForgeChecks, ManifestId, PullRequestStanding, PullRequestState};

use crate::daemon::Fleet;
use crate::pull_requesting::{AUTO_MERGE_REFUSED, NOT_A_DRAFT, READY_REFUSED};

/// What one write came to: the reply, and whether the forge was written to, which is whether a
/// read after it is owed.
pub(crate) struct Acted {
    manifest_id: ManifestId,
    reply: PullRequestState,
    written: bool,
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
    /// Take a draft out of draft: the pre-read, the refusals it decides, and the write.
    pub(crate) async fn ready_acted(&self, manifest_id: &ManifestId, number: u64) -> Result<Acted, Refusal> {
        let served = self.served_named(Some(manifest_id))?;
        let root = served.root();
        let before = self.pull_request_read_off(manifest_id, root, number).await?;
        match before.state {
            PullRequestStanding::Draft => {}
            PullRequestStanding::Open => {
                return Err(Refusal::IllegalMove(
                    self.pr_refusal(NOT_A_DRAFT, format!("pull request {number} is open already")),
                ))
            }
            _ => return Err(self.not_open_refusal(&before).expect("not open")),
        }
        self.forge_asked(root, move |vcs: &V, root: &str| vcs.mark_ready(root, &number.to_string()))
            .await
            .map_err(|said| Refusal::IllegalMove(self.pr_refusal(READY_REFUSED, said)))?;
        let reply = PullRequestState { state: PullRequestStanding::Open, ..before };
        Ok(Acted { manifest_id: manifest_id.clone(), reply, written: true })
    }

    /// Merge it, only once the checks have passed.
    pub(crate) async fn merge_acted(&self, manifest_id: &ManifestId, number: u64) -> Result<Acted, Refusal> {
        let served = self.served_named(Some(manifest_id))?;
        let root = served.root();
        let before = self.pull_request_read_off(manifest_id, root, number).await?;
        match before.state {
            // **Not a refusal**: the work is where the press was trying to put it.
            PullRequestStanding::Merged => {
                self.ledger_follows(&before).await;
                return Ok(Acted { manifest_id: manifest_id.clone(), reply: before, written: false });
            }
            PullRequestStanding::Open => {}
            _ => return Err(self.not_open_refusal(&before).expect("not open")),
        }
        if before.checks != ForgeChecks::Passed {
            return Err(self.checks_not_passed(&before));
        }
        self.forge_asked(root, move |vcs: &V, root: &str| vcs.merge(root, &number.to_string()))
            .await
            .map_err(|why| self.not_merged_refusal(number, why))?;
        let reply = PullRequestState { state: PullRequestStanding::Merged, ..before };
        Ok(Acted { manifest_id: manifest_id.clone(), reply, written: true })
    }

    /// Ask the forge to merge it when its checks pass.
    pub(crate) async fn auto_merge_acted(&self, manifest_id: &ManifestId, number: u64) -> Result<Acted, Refusal> {
        let served = self.served_named(Some(manifest_id))?;
        let root = served.root();
        let before = self.pull_request_read_off(manifest_id, root, number).await?;
        if let Some(refusal) = self.not_open_refusal(&before) {
            return Err(refusal);
        }
        // Asked twice, the second answers as it stands. A pull request in the
        // queue has been asked already, whatever the flag says.
        if before.auto_merge || before.queued {
            self.ledger_follows(&before).await;
            return Ok(Acted { manifest_id: manifest_id.clone(), reply: before, written: false });
        }
        // Passed already is not a refusal: the forge queues it, as `gh pr merge --auto` does.
        if let ForgeChecks::Failed { .. } = &before.checks {
            return Err(self.checks_not_passed(&before));
        }
        self.forge_asked(root, move |vcs: &V, root: &str| vcs.enable_auto_merge(root, &number.to_string()))
            .await
            .map_err(|said| Refusal::IllegalMove(self.pr_refusal(AUTO_MERGE_REFUSED, said)))?;
        let reply = PullRequestState { auto_merge: true, ..before };
        Ok(Acted { manifest_id: manifest_id.clone(), reply, written: true })
    }

    /// The read after a write, and the ledger following it. A forge that took the write and would
    /// not answer after it is not a failure of the write: the ledger follows the reply instead.
    async fn written_followed(&self, acted: &Acted) -> PullRequestState {
        let Acted { manifest_id, reply, .. } = acted;
        let root = match self.served_named(Some(manifest_id)) {
            Ok(served) => served.root().to_string(),
            Err(_) => return reply.clone(),
        };
        let state = match self.pull_request_read_off(manifest_id, &root, reply.number).await {
            Ok(state) => state,
            Err(why) => {
                eprintln!("pull request {} was written and not read again: {}", reply.number, why.error().message);
                reply.clone()
            }
        };
        self.ledger_follows(&state).await;
        state
    }

    /// The reply to a press. Where the forge was written to, the read after it and the ledger run
    /// in a task of their own.
    pub(crate) fn replied_to(self: Arc<Self>, acted: Acted) -> PullRequestState {
        if !acted.written {
            return acted.reply;
        }
        let reply = acted.reply.clone();
        tokio::spawn(async move {
            self.written_followed(&acted).await;
        });
        reply
    }

    /// Ask the forge to merge it when its checks pass, for a caller that holds the Fleet by
    /// reference and goes on when the ledger has followed: Auto land, which asks pull request by
    /// pull request. **The same act as the trait's**, answering after the read instead of before.
    pub(crate) async fn enable_auto_merge(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let acted = self.auto_merge_acted(&manifest_id, number).await?;
        if !acted.written {
            return Ok(acted.reply);
        }
        Ok(self.written_followed(&acted).await)
    }
}
