//! A slot whose branch merged: its holder told it is free to reuse or give back, and Fleet gives
//! it back when the holder's turn ends with nothing new in it. `docs/concepts/fleet.md`, *Worktree
//! slots*.
//!
//! **The holder is told first**, because it may want the slot for follow-up work. **The release is
//! Fleet's only once the holder has been told**, a session whose turn ends, a clean slot still on
//! the merged branch and every commit on a remote or the base. A dirty tree or unpushed commits
//! keep the slot and are told once a kind. Another branch checked out is new work, and left.
//! Nothing here deletes a branch. Both are kept in `pull_notices`, so a restart repeats neither.
//!
//! A hosted session's slot is the one released: a terminal session's turn is seen by its mod, which
//! cannot say that it has read what Fleet held for it.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SlotAfterMerge, Vcs, WorkProduct};
use ipc::{ChangeSlotPool, ManifestId, SlotAct};

use crate::daemon::Fleet;
use crate::leasing::pool_of;
use crate::repositories::Served;
use crate::slot_holders::{session_in_slot, slots_of};

/// The branches of a review bay, private to the review that holds it: never a session's slot to offer
/// back or give back, whatever merged under such a name.
const REVIEW_BAYS: &str = "review/";

/// The cause a holder told its slot is free is kept under, with `slot-<n>` as the commit.
const FREE: &str = "slot_free";

/// The cause a holder told what keeps its slot is kept under, with `slot-<n>:<what>` as the commit.
const KEPT: &str = "slot_kept";

/// What a holder is told when the branch its slot is leased on merged.
fn free_to_reuse(slot: u32, branch: &str) -> String {
    format!(
        "slot-{slot}'s branch {branch} merged. Lease it for new work or give it back with \
         `armada worktree release`. Fleet gives it back when your turn ends with nothing new in it."
    )
}

/// What keeps a slot, as the holder is told and as it is kept: the kind, and the sentence.
fn kept_by(slot: u32, held: &SlotAfterMerge) -> Option<(&'static str, String)> {
    match held {
        SlotAfterMerge::Dirty(files) => Some((
            "dirty",
            format!(
                "slot-{slot} is kept: {} uncommitted, first {}.",
                files.len(),
                files.first().map(String::as_str).unwrap_or_default()
            ),
        )),
        SlotAfterMerge::Unpushed(commits) => Some((
            "unpushed",
            format!(
                "slot-{slot} is kept: {commits} {} on no remote or base.",
                if *commits == 1 { "commit" } else { "commits" }
            ),
        )),
        SlotAfterMerge::Idle | SlotAfterMerge::Moved => None,
    }
}

/// Every slot that is on disk under `served`'s pool.
fn slots_on_disk(served: &Served) -> Vec<u32> {
    let first = adapter_traits::slot_path(served.root().trim_end_matches('/'), 1);
    let mut found: Vec<u32> = Path::new(&first)
        .parent()
        .and_then(|slots| std::fs::read_dir(slots).ok())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok()?.file_name().to_str()?.strip_prefix("slot-")?.parse().ok())
        .collect();
    found.sort_unstable();
    found
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
    /// Tell the Session holding each slot leased on `branch` that pull request `number` merged, once
    /// a slot. **The lease names the branch**, and the pool the holder: a Session that moved off the
    /// branch in its slot is told all the same, since the lease is what it holds.
    pub(crate) async fn slots_told_of_merge(self: &Arc<Self>, served: &Served, number: u64, branch: &str) {
        let manifest = served.manifest().id().as_str().to_string();
        let root = served.root().to_string();
        for slot in slots_on_disk(served) {
            let at = adapter_traits::slot_path(root.trim_end_matches('/'), slot);
            if branch.starts_with(REVIEW_BAYS) || adapters::leasing::branch_of(Path::new(&at)).as_deref() != Some(branch) {
                continue;
            }
            let held = session_in_slot(&*self.store().lock().await, &manifest, &root, slot);
            let Some(session) = held else { continue };
            let (commit, recipient) = (format!("slot-{slot}"), format!("session:{session}"));
            let told = self
                .store()
                .lock()
                .await
                .pull_notice_kept(&root, number, &commit, FREE, &recipient)
                .unwrap_or(true);
            if told || !matches!(self.told_news_by_fleet(&session, &free_to_reuse(slot, branch), ipc::Notice::SlotFree).await, Ok(true)) {
                continue;
            }
            let now = self.now();
            let _ = self
                .store()
                .lock()
                .await
                .keep_pull_notice(&root, number, &commit, FREE, &recipient, &now);
        }
    }

    /// A hosted session's turn ended and it has nothing queued: give back each slot of its that was
    /// told to it as free, if nothing new is in it, or tell it what keeps the slot.
    pub(crate) async fn slots_reclaimed(self: &Arc<Self>, session: &str) {
        let Some(hosting) = self.store().lock().await.hosting(session).ok().flatten() else {
            return;
        };
        let manifest = ManifestId::carried(&hosting.manifest_id);
        let Ok(served) = self.served_named(Some(&manifest)) else { return };
        let root = served.root().to_string();
        let merged = self
            .sweeping()
            .lock()
            .await
            .merged
            .get(&root)
            .cloned()
            .unwrap_or_default();
        for slot in slots_of(&served, session) {
            let at = adapter_traits::slot_path(root.trim_end_matches('/'), slot);
            let Some(branch) = adapters::leasing::branch_of(Path::new(&at)).filter(|it| !it.starts_with(REVIEW_BAYS))
            else {
                continue;
            };
            let Some(number) = merged
                .iter()
                .find(|one| one.pull.branch.as_written() == branch)
                .map(|one| one.pull.number)
            else {
                continue;
            };
            let (commit, recipient) = (format!("slot-{slot}"), format!("session:{session}"));
            let told = self
                .store()
                .lock()
                .await
                .pull_notice_kept(&root, number, &commit, FREE, &recipient)
                .unwrap_or(false);
            if !told {
                continue;
            }
            let held = {
                let (vcs, pool, branch) = (Arc::clone(self.vcs()), pool_of(&served), branch.clone());
                tokio::task::spawn_blocking(move || vcs.slot_after_merge(&pool, slot, &branch))
                    .await
                    .expect("git panicked reading a slot")
            };
            match (&held, kept_by(slot, &held)) {
                (SlotAfterMerge::Idle, _) => {
                    let _ = self
                        .slot_pool_changed(
                            ChangeSlotPool {
                                act: SlotAct::Release,
                                slot: Some(slot),
                                holder: Some(session.to_string()),
                            },
                            Some(&manifest),
                        )
                        .await;
                }
                (_, Some((what, said))) => {
                    let commit = format!("slot-{slot}:{what}");
                    let once = self
                        .store()
                        .lock()
                        .await
                        .pull_notice_kept(&root, number, &commit, KEPT, &recipient)
                        .unwrap_or(true);
                    if !once && matches!(self.told_news_by_fleet(session, &said, ipc::Notice::SlotKept).await, Ok(true)) {
                        let now = self.now();
                        let _ = self
                            .store()
                            .lock()
                            .await
                            .keep_pull_notice(&root, number, &commit, KEPT, &recipient, &now);
                    }
                }
                _ => {}
            }
        }
    }
}
