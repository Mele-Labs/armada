//! The slots a full pool is held by, on the call that asks the person what to do, and the press that
//! frees one. `docs/concepts/session.md`, *A full pool asks*.
//!
//! **Read now and then, never on a publish.** Item derivation is hot and the dirty read is git, so
//! the holders live on the line's entries: read when the keeper looks at the lines (a Job joining
//! wakes it, and so does every slot given back) while anyone has a call open, and a Job is published
//! only when its rows changed. Only a slot a Job Fleet hosts holds is a row; a terminal's or a
//! person's has none.
//!
//! **Fleet never frees a slot by itself.** [`Fleet::slot_freed_for`] is `free_slot`, the person's
//! press, and nothing else calls it.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SlotHeld, Vcs, WorkProduct};
use api::Refusal;
use ipc::{FreeSlot, Instant, ManifestId, SessionRecord, SessionTurn, SlotHolder, SlotHolderJob, SlotKeeps, WireError};

use super::queueing::{Choice, SlotQueue, SLOT_CHOICE_ITEM};
use crate::daemon::Fleet;
use crate::leasing::{pool_of, SLOT_HOLDER_CHANGED};

impl SlotQueue {
    /// The repositories where a session in a line has its call open.
    pub(super) fn open_call_manifests(&self) -> Vec<String> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        let mut open: Vec<String> = waiting
            .entries
            .values()
            .filter(|one| matches!(one.choice, Choice::Pending | Choice::Adding))
            .map(|one| one.manifest.clone())
            .collect();
        open.sort();
        open.dedup();
        open
    }

    /// Take `holders` as what every session in `manifest`'s line is offered. The sessions with a call
    /// open whose rows changed, which are to be published.
    pub(super) fn set_holders(&self, manifest: &str, holders: &[SlotHolder]) -> Vec<String> {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        let mut changed = Vec::new();
        for (id, entry) in waiting.entries.iter_mut().filter(|(_, one)| one.manifest == manifest) {
            if entry.holders == holders {
                continue;
            }
            entry.holders = holders.to_vec();
            if matches!(entry.choice, Choice::Pending | Choice::Adding) {
                changed.push(id.clone());
            }
        }
        changed
    }

    /// The row a session's open call offers for `slot`, and the repository it waits in.
    /// `Err(false)` where the session has no call open, `Err(true)` where the call has no such row.
    pub(super) fn holder_row(&self, session: &str, slot: u32) -> Result<(String, SlotHolder), bool> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        let entry = waiting
            .entries
            .get(session)
            .filter(|one| matches!(one.choice, Choice::Pending | Choice::Adding))
            .ok_or(false)?;
        let row = entry.holders.iter().find(|one| one.slot == slot).ok_or(true)?;
        Ok((entry.manifest.clone(), row.clone()))
    }
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
    /// Read the holders of every repository where a call is open, and publish the Jobs whose rows
    /// changed.
    pub(crate) async fn refresh_slot_holders(&self) {
        let queue = Arc::clone(self.hosts().slot_queue());
        for manifest in queue.open_call_manifests() {
            self.refresh_holders_of(&queue, &manifest).await;
        }
    }

    async fn refresh_holders_of(&self, queue: &SlotQueue, manifest: &str) {
        let read = self.read_slot_holders(manifest).await;
        for id in queue.set_holders(manifest, &read) {
            let _ = self.published_hosted(&id).await;
        }
    }

    /// The slots in `manifest`'s pool that a Job Fleet hosts holds, longest quiet first. **A holder
    /// keeps its slot** (`keeps`) while its turn runs, and while its checkout holds files no commit
    /// has: Fleet decides, Bridge only draws it.
    async fn read_slot_holders(&self, manifest: &str) -> Vec<SlotHolder> {
        let Ok(served) = self.served_named(Some(&ManifestId::carried(manifest))) else {
            return Vec::new();
        };
        let pool = pool_of(&served);
        let readings = {
            let (vcs, pool) = (Arc::clone(self.vcs()), pool.clone());
            tokio::task::spawn_blocking(move || vcs.slot_pool_kept(&pool, false)).await.unwrap_or_default()
        };
        let mut held = Vec::new();
        for reading in readings {
            let SlotHeld::Hosted(job) = &reading.held else {
                continue;
            };
            let Some(kept) = self.store().lock().await.session(job).ok().flatten() else {
                continue;
            };
            let running = matches!(self.hosts().of(job).state().turn, SessionTurn::Working { .. });
            // Asked of git for the slots that are not running only, side by side.
            let dirty = (!running).then(|| {
                let (vcs, pool, slot) = (Arc::clone(self.vcs()), pool.clone(), reading.slot);
                tokio::task::spawn_blocking(move || vcs.slot_dirty(&pool, slot))
            });
            let quiet_since = kept
                .last_turn_at
                .clone()
                .map(Instant::carried)
                .or_else(|| crate::wire::since_of(reading.since))
                .unwrap_or_else(|| Instant::carried(kept.started_at.clone()));
            held.push((reading.slot, job.clone(), kept.title.clone(), quiet_since, running, dirty));
        }
        let mut rows = Vec::new();
        for (slot, id, title, quiet_since, running, dirty) in held {
            let uncommitted = match dirty {
                Some(read) => read.await.ok().and_then(Result::ok).unwrap_or(false),
                None => false,
            };
            let keeps = match (running, uncommitted) {
                (true, _) => Some(SlotKeeps::Running),
                (false, true) => Some(SlotKeeps::Uncommitted),
                _ => None,
            };
            let title = title.unwrap_or_else(|| String::from("A job"));
            rows.push(SlotHolder { slot, job: SlotHolderJob { id, title }, quiet_since, keeps });
        }
        // Longest quiet first: instants are RFC 3339 in one zone, so they order as text.
        rows.sort_by(|a, b| a.quiet_since.as_str().cmp(b.quiet_since.as_str()).then(a.slot.cmp(&b.slot)));
        rows
    }

    /// `free_slot`: give back the slot a waiting Job's call lists, to that Job. **Looked at again
    /// here, at the press**: the call, then whether the slot is still the holder's, whether its turn
    /// runs, and whether its checkout holds uncommitted work, each refused with nothing changed. The
    /// release is `change_slot_pool`'s for a Job's bay, and the lease the one the line's grant takes,
    /// both under `granting`, which nothing here holds across a checkout being made.
    pub(crate) async fn slot_freed_for(self: &Arc<Self>, free: FreeSlot) -> Result<SessionRecord, Refusal> {
        let id = free.session_id.as_str();
        if free.item_id != SLOT_CHOICE_ITEM {
            return Err(self.settled_already());
        }
        let queue = Arc::clone(self.hosts().slot_queue());
        let changed = |why: &str| {
            Refusal::IllegalMove(WireError::raised(SLOT_HOLDER_CHANGED, why, self.run_id()))
        };
        let (manifest, row) = match queue.holder_row(id, free.slot) {
            Ok(found) => found,
            Err(false) => return Err(self.settled_already()),
            Err(true) => return Err(changed("that slot is not held by a job in the list any more")),
        };
        {
            let _alone = queue.granting.lock().await;
            // Asked again under the lock: another Free or the line's grant may have served the Job.
            if queue.holder_row(id, free.slot).is_err() {
                return Err(self.settled_already());
            }
            let served = self.served_named(Some(&ManifestId::carried(&manifest)))?;
            let hosting = self.store().lock().await.hosting(&row.job.id).ok().flatten();
            let Some(hosting) = hosting else {
                return Err(changed("that job does not hold the slot any more"));
            };
            self.hosted_released(&served, free.slot, hosting, true).await?;
            match self.take_lease_resuming(id, &served).await {
                super::gate::Taken::Full => {}
                taken => self.granted(id, taken).await,
            }
        }
        queue.freed();
        self.refresh_holders_of(&queue, &manifest).await;
        self.published_hosted(id).await
    }
}
