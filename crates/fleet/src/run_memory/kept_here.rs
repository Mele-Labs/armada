//! A repository marked Keep on this Mac: its runs, and every piece of the person's own for it,
//! stay in Kit's folder on this Mac whatever place the rest of Kit uses. `docs/concepts/kit.md`,
//! *Kept on this Mac*.
//!
//! **The mark holds the place, not a check beside it.** Marking moves the run memory place to
//! `this_mac`, a mark refuses any other place, and the repository's tiles read the mark before the
//! place, so no write for the repository reaches a synced folder or a bucket while it stands.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{KeptOnThisMac, RunPlace, SetKeptOnThisMac, WireError};

use super::place_word;
use crate::daemon::Fleet;

/// Another place asked for a repository kept on this Mac. A 422.
pub(crate) const KEPT_HERE: &str = "fleet.run_place_kept_here";

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
    /// Whether the repository at `root` is kept on this Mac.
    pub(crate) async fn kept_here(&self, root: &str) -> Result<bool, Refusal> {
        self.store().lock().await.kept_on_this_mac(root).map_err(|why| self.run_fault(why))
    }

    /// The refusal for any place but `this_mac` while the repository is kept on this Mac.
    pub(crate) async fn refused_while_kept_here(&self, root: &str, place: RunPlace) -> Result<(), Refusal> {
        if place == RunPlace::ThisMac || !self.kept_here(root).await? {
            return Ok(());
        }
        Err(Refusal::Unacceptable(WireError::raised(
            KEPT_HERE,
            format!("{root} is kept on this Mac, so its runs stay on this Mac. Clear Keep on this Mac to choose another place"),
            self.run_id(),
        )))
    }

    /// `get_kept_on_this_mac`.
    pub(crate) async fn kept_on_this_mac_read(&self, repository: String) -> Result<KeptOnThisMac, Refusal> {
        let root = self.run_repository(&repository)?;
        let kept = self.kept_here(&root).await?;
        Ok(KeptOnThisMac { repository: root, kept })
    }

    /// `set_kept_on_this_mac`. Marking moves the place to `this_mac` and writes what the repository
    /// remembers there; clearing the mark moves nothing.
    pub(crate) async fn kept_on_this_mac_set(&self, asked: SetKeptOnThisMac) -> Result<KeptOnThisMac, Refusal> {
        let root = self.run_repository(&asked.repository)?;
        if asked.kept {
            self.run_memory_started(&root).await?;
            let mut store = self.store().lock().await;
            store.keep_on_this_mac(&root, true).map_err(|why| self.run_fault(why))?;
            let moved = store.run_memory_place(&root).map_err(|why| self.run_fault(why))?;
            if moved.is_some_and(|held| held.place != place_word(RunPlace::ThisMac)) {
                store
                    .set_run_memory_place(&root, place_word(RunPlace::ThisMac), None, None)
                    .map_err(|why| self.run_fault(why))?;
            }
            drop(store);
            self.run_memory_written(&root, None).await;
        } else {
            self.store().lock().await.keep_on_this_mac(&root, false).map_err(|why| self.run_fault(why))?;
        }
        self.kept_on_this_mac_read(root).await
    }
}
