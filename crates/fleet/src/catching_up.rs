//! The checkout each served repository stands in, caught up whenever main moves.
//!
//! **Off the reading of main, not off a Job's merge.** Sessions and the merge
//! queue land most pull requests, and a catch-up that waited for a Job's left
//! the owner's checkout 880 commits behind for two days (15 Oct 2026). The
//! same refusals hold: uncommitted work or a diverged history is never moved,
//! and what stood in the way is kept for a person to see. **A checkout on
//! another branch is not a refusal**: its local base branch is fast-forwarded,
//! and nothing checked out moves. The owner is called only for uncommitted
//! work on the base and for conflicts (16 Oct 2026).

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, RepositoryStanding, Vcs, WorkProduct};

use crate::daemon::Fleet;
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
    /// Bring `served`'s checkout up to `head`, main's tip on the forge, or keep
    /// why it could not be. Nothing is asked of git past one in-process read
    /// while the checkout already has `head`.
    pub(crate) async fn checkout_caught_up(&self, served: &Served, base: &str, head: &str) {
        let root = served.root().to_string();
        let has_it = self
            .forge_asked(&root, {
                let head = head.to_string();
                let base = base.to_string();
                move |vcs: &V, root: &str| vcs.checkout_has(root, &base, &head)
            })
            .await;
        if has_it {
            self.sweeping().lock().await.held_back.remove(&root);
            return;
        }
        let standing = {
            // For `caught_the_repository_up`'s reason: this writes into the
            // one `.git` every worktree shares.
            let _at_the_merge_end = self.merge_end().lock().await;
            let (vcs, root, base) = (Arc::clone(self.vcs()), root.clone(), base.to_string());
            tokio::task::spawn_blocking(move || vcs.caught_the_repository_up(&root, &base))
                .await
                .expect("git panicked bringing the checkout up to date")
        };
        let mut held = match &standing {
            RepositoryStanding::LeftAlone { .. } => {
                self.forge_asked(&root, {
                    let base = base.to_string();
                    move |vcs: &V, root: &str| vcs.held_back(root, &base)
                })
                .await
            }
            RepositoryStanding::MovedOn { .. } | RepositoryStanding::AlreadyHadIt { .. } => None,
        };
        // **On another branch the base is Fleet's to move, and nobody is told**:
        // the local base branch fast-forwards and what is checked out stays.
        if held.as_ref().is_some_and(|held| held.on.is_some()) {
            let _at_the_merge_end = self.merge_end().lock().await;
            let (vcs, root, base) = (Arc::clone(self.vcs()), root.clone(), base.to_string());
            let moved = tokio::task::spawn_blocking(move || vcs.base_fast_forwarded(&root, &base))
                .await
                .expect("git panicked bringing the base up to date");
            if moved {
                held = None;
            }
        }
        self.told_servers_the_checkout_moved(&root, &standing);
        let mut sweep = self.sweeping().lock().await;
        match held {
            Some(held) => sweep.held_back.insert(root, held),
            None => sweep.held_back.remove(&root),
        };
    }
}
