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
//!
//! **A file that is main's own content is not a conflict.** Once the change an
//! owner left uncommitted has reached main by another road, the pull over it
//! takes nothing from anyone, so Fleet runs it itself and nobody is called
//! (`settled_on_main`).

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, HeldBack, RepositoryStanding, Vcs, WorkProduct};
use api::Refusal;
use ipc::WireError;

use crate::daemon::Fleet;
use crate::refusing::NOT_SERVED;
use crate::repositories::Served;

/// A catch-up pressed for a checkout nothing holds back, or that could not be
/// read. A 409: Bridge's card was drawn before the owner or Fleet moved it.
pub(crate) const CHECKOUT_NOT_HELD: &str = "fleet.checkout_not_held";
/// A catch-up pressed for a hold the one safe pull does not clear: another
/// branch, commits main lacks, a file main also changed, or nothing uncommitted
/// to set aside. A 409, and the message names which.
pub(crate) const CHECKOUT_NOT_CATCHABLE: &str = "fleet.checkout_not_catchable";
/// A diff asked for a path the hold does not list as uncommitted. A 409: nothing
/// outside the list is read, whatever the path.
const CHECKOUT_PATH_NOT_HELD: &str = "fleet.checkout_path_not_held";
/// git would not print a listed file's change. A 500: the read may go on the next press.
const CHECKOUT_DIFF_UNREAD: &str = "fleet.checkout_diff_unread";

/// The most lines of one file's change `checkout_diff` and `get_session_file_change` answer. The
/// card draws no more than this (`DRAWN_LINES` in `packages/screens/src/drawn-patch.ts`), so a
/// longer change is never sent whole.
const CHECKOUT_DIFF_LINES: usize = 2000;
/// The most bytes of it. A generated or minified file can be a few lines of megabytes each.
const CHECKOUT_DIFF_BYTES: usize = 256 * 1024;

/// A change cut to the lines and bytes the card can draw, and whether it was cut.
/// **Whole lines only**, except where a single line is itself past the byte bound.
pub(crate) fn cut_for_the_card(diff: String) -> (String, bool) {
    let mut end = diff.len();
    if let Some((at, _)) = diff.match_indices('\n').nth(CHECKOUT_DIFF_LINES - 1) {
        end = at + 1;
    }
    if end > CHECKOUT_DIFF_BYTES {
        let mut at = CHECKOUT_DIFF_BYTES;
        while !diff.is_char_boundary(at) {
            at -= 1;
        }
        end = diff[..at].rfind('\n').map_or(at, |newline| newline + 1);
    }
    if end >= diff.len() {
        return (diff, false);
    }
    let mut kept = diff;
    kept.truncate(end);
    (kept, true)
}

/// Why the pull `catch_up_checkout` offers cannot be run for this hold, as a
/// code and a sentence. **The same four conditions Bridge's card names**, read
/// again here because the card may be old: on the base, ahead of nothing, no
/// uncommitted file main also changed, and a file to set aside.
pub(crate) fn why_not_pull(held: Option<&HeldBack>) -> Option<(&'static str, String)> {
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

/// Whether Fleet runs the pull over this hold itself, with no call raised: on
/// the base, ahead of nothing, no file main changed to something else, and **at
/// least one file that is main's own content now**. Without that one, the files
/// are the owner's unfinished work and the card offers the pull and the pull
/// request for them to choose; with it, the change they were keeping has landed
/// and what remains is what main never touched.
fn settled_on_main(held: &HeldBack) -> bool {
    held.on.is_none() && held.ahead == 0 && held.behind > 0 && held.clashing.is_empty() && !held.already_on_base.is_empty()
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
            let mut sweep = self.sweeping().lock().await;
            sweep.held_back.remove(&root);
            sweep.checkout_sent.remove(&root);
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
        if let Some(settled) = held.take_if(|held| settled_on_main(held)) {
            held = self.pulled_over_what_main_holds(&root, base, head, settled).await;
        }
        self.told_servers_the_checkout_moved(&root, &standing);
        let mut sweep = self.sweeping().lock().await;
        match held {
            Some(held) => {
                // A hold that no longer has files on the base has nothing a pull request carries.
                if held.on.is_some() || held.uncommitted.is_empty() {
                    sweep.checkout_sent.remove(&root);
                }
                sweep.held_back.insert(root, held)
            }
            None => {
                sweep.checkout_sent.remove(&root);
                sweep.held_back.remove(&root)
            }
        };
    }

    /// The pull over a hold whose every uncommitted file is untouched by main or
    /// main's own, run by Fleet. **The hold is read again under the merge end**,
    /// for `caught_up_checkout`'s reason and because the owner may have edited a
    /// file since the reading: a file that now differs from main's is a clash,
    /// and then nothing is pulled. Answers the hold as it stands afterwards,
    /// `None` where the checkout has main's tip. **Never forced**: a pull git
    /// refuses leaves the hold standing.
    async fn pulled_over_what_main_holds(&self, root: &str, base: &str, head: &str, read: HeldBack) -> Option<HeldBack> {
        let _at_the_merge_end = self.merge_end().lock().await;
        let again = self
            .forge_asked(root, {
                let base = base.to_string();
                move |vcs: &V, root: &str| vcs.held_back(root, &base)
            })
            .await;
        let held = again.unwrap_or(read);
        if !settled_on_main(&held) {
            return Some(held);
        }
        let pulled = self.forge_asked(root, |vcs: &V, root: &str| vcs.pulled_the_checkout(root)).await;
        let reached = self
            .forge_asked(root, {
                let (base, head) = (base.to_string(), head.to_string());
                move |vcs: &V, root: &str| vcs.checkout_has(root, &base, &head)
            })
            .await;
        if pulled.code != 0 || !reached {
            return Some(held);
        }
        self.told_servers_the_checkout_moved(
            root,
            &RepositoryStanding::MovedOn { base: base.to_string(), commits: held.behind, head: head.to_string() },
        );
        None
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
            .map(|diff| {
                let (diff, cut) = cut_for_the_card(diff);
                ipc::CheckoutDiff { diff, cut }
            })
            .map_err(|said| Refusal::Fault(raised(CHECKOUT_DIFF_UNREAD, said)))
    }
}
