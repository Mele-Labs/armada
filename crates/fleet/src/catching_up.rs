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

use adapter_traits::{AgentHarness, Delivery, HeldBack, RepositoryStanding, Vcs, WorkProduct};
use api::Refusal;
use ipc::WireError;

use crate::daemon::Fleet;
use crate::refusing::NOT_SERVED;
use crate::repositories::Served;

/// A catch-up pressed for a checkout nothing holds back, or that could not be
/// read. A 409: Bridge's card was drawn before the owner or Fleet moved it.
const CHECKOUT_NOT_HELD: &str = "fleet.checkout_not_held";
/// A catch-up pressed for a hold the one safe pull does not clear: another
/// branch, commits main lacks, a file main also changed, or nothing uncommitted
/// to set aside. A 409, and the message names which.
const CHECKOUT_NOT_CATCHABLE: &str = "fleet.checkout_not_catchable";
/// A diff asked for a path the hold does not list as uncommitted. A 409: nothing
/// outside the list is read, whatever the path.
const CHECKOUT_PATH_NOT_HELD: &str = "fleet.checkout_path_not_held";
/// git would not print a listed file's change. A 500: the read may go on the next press.
const CHECKOUT_DIFF_UNREAD: &str = "fleet.checkout_diff_unread";

/// Why the pull `catch_up_checkout` offers cannot be run for this hold, as a
/// code and a sentence. **The same four conditions Bridge's card names**, read
/// again here because the card may be old: on the base, ahead of nothing, no
/// uncommitted file main also changed, and a file to set aside.
fn why_not_pull(held: Option<&HeldBack>) -> Option<(&'static str, String)> {
    let Some(held) = held else {
        return Some((CHECKOUT_NOT_HELD, String::from("nothing holds the checkout back, or git could not be read there")));
    };
    if held.behind == 0 {
        return Some((CHECKOUT_NOT_HELD, format!("the checkout already has `{}`", held.base)));
    }
    if let Some(on) = &held.on {
        return Some((CHECKOUT_NOT_CATCHABLE, format!("the checkout is on `{on}` and not `{}`", held.base)));
    }
    if held.ahead > 0 {
        return Some((CHECKOUT_NOT_CATCHABLE, format!("the checkout has {} commit(s) that `{}` lacks", held.ahead, held.base)));
    }
    if !held.clashing.is_empty() {
        return Some((CHECKOUT_NOT_CATCHABLE, format!("{} also changed on `{}`: {}", if held.clashing.len() == 1 { "an uncommitted file" } else { "uncommitted files" }, held.base, held.clashing.join(", "))));
    }
    if held.uncommitted.is_empty() {
        return Some((CHECKOUT_NOT_CATCHABLE, String::from("nothing is uncommitted, so Fleet catches the checkout up itself")));
    }
    None
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

    /// `catch_up_checkout`: the one pull a held checkout is offered, run for the
    /// owner.
    ///
    /// **Fleet reads the hold itself** and runs only when it is the one this
    /// pull cannot lose anything in, under the merge end for
    /// `checkout_caught_up`'s reason. **No command crosses the wire.** Afterwards
    /// the checkout is read again so the hub's `checkout` clears, and the hub
    /// loop publishes it. A stash git left stays in git's stash.
    pub(crate) async fn caught_up_checkout(
        &self,
        asked: ipc::CatchUpCheckout,
    ) -> Result<ipc::CheckoutCaughtUp, Refusal> {
        let raised = |code: &'static str, said: String| WireError::raised(code, said, self.run_id());
        let Some(served) = self.repositories().serving(asked.manifest_id.as_str()) else {
            return Err(Refusal::Unacceptable(raised(
                NOT_SERVED,
                format!("this Fleet does not serve {}", asked.manifest_id.as_str()),
            )));
        };
        let root = served.root().to_string();
        let Some(base) = served.manifest().base().map(str::to_string) else {
            return Err(Refusal::IllegalMove(raised(
                CHECKOUT_NOT_HELD,
                String::from("the repository names no base to catch up to"),
            )));
        };
        let pulled = {
            let _at_the_merge_end = self.merge_end().lock().await;
            let held = self
                .forge_asked(&root, {
                    let base = base.clone();
                    move |vcs: &V, root: &str| vcs.held_back(root, &base)
                })
                .await;
            if let Some((code, said)) = why_not_pull(held.as_ref()) {
                return Err(Refusal::IllegalMove(raised(code, said)));
            }
            self.forge_asked(&root, |vcs: &V, root: &str| vcs.pulled_the_checkout(root))
                .await
        };
        // Read again whether or not the pull took: a stash that would not go back
        // on leaves the checkout on main's tip, and the hold is then not behind.
        let head = self
            .forge_asked(&root, {
                let base = base.clone();
                move |vcs: &V, root: &str| vcs.base_head_on_the_forge(root, &base)
            })
            .await;
        if let Some(head) = head {
            self.checkout_caught_up(&served, &base, &head).await;
        }
        Ok(ipc::CheckoutCaughtUp {
            code: pulled.code,
            output: pulled.output,
        })
    }

    /// `checkout_diff`: one uncommitted file of the held checkout, against `HEAD`.
    ///
    /// **Only a path the hold lists.** The list is the one Fleet kept at its last
    /// reading of main, which is the one Bridge drew the card from, so no path
    /// outside it is read and the press costs no fetch. Reads only, so it takes
    /// no part in the merge end.
    pub(crate) async fn checkout_diff_of(&self, ask: ipc::CheckoutDiffAsk) -> Result<ipc::CheckoutDiff, Refusal> {
        let raised = |code: &'static str, said: String| WireError::raised(code, said, self.run_id());
        let Some(served) = self.repositories().serving(ask.manifest_id.as_str()) else {
            return Err(Refusal::Unacceptable(raised(
                NOT_SERVED,
                format!("this Fleet does not serve {}", ask.manifest_id.as_str()),
            )));
        };
        let root = served.root().to_string();
        let held = self.sweeping().lock().await.held_back.get(&root).cloned();
        let Some(held) = held else {
            return Err(Refusal::IllegalMove(raised(
                CHECKOUT_NOT_HELD,
                String::from("nothing holds the checkout back"),
            )));
        };
        if !held.uncommitted.contains(&ask.path) {
            return Err(Refusal::IllegalMove(raised(
                CHECKOUT_PATH_NOT_HELD,
                format!("`{}` is not among the checkout's uncommitted files", ask.path),
            )));
        }
        let path = ask.path;
        self.forge_asked(&root, move |vcs: &V, root: &str| vcs.uncommitted_change_of(root, &path))
            .await
            .map(|diff| ipc::CheckoutDiff { diff })
            .map_err(|said| Refusal::Fault(raised(CHECKOUT_DIFF_UNREAD, said)))
    }
}
