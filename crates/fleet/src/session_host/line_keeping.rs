//! The line for a slot, kept in the store as Jobs join and leave it, and read back once as Fleet
//! starts, so a restart serves the same Jobs in the same order. **What the store cannot keep is
//! let go**: a write that fails leaves the line in memory as it was, and only a restart loses it.
//!
//! **A Job moved in the line renumbers the whole of it**, 0 upward in its new order, so the
//! Operator's place and a person's drag are kept the same way a join is.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::Instant;
use store::KeptInLine;

use super::queueing::{Lines, SlotQueue};
use crate::daemon::Fleet;
use crate::operator::{InLine, Place};

impl SlotQueue {
    /// The repository whose line `session` is in.
    pub(crate) fn line_holding(&self, session: &str) -> Option<String> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.entries.get(session).map(|entry| entry.manifest.clone())
    }

    /// Every Job in a line, each repository's in its order, by session id and repository.
    pub(crate) fn everyone(&self) -> Vec<(String, String)> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        let mut manifests: Vec<&String> = waiting.by_manifest.keys().collect();
        manifests.sort();
        manifests
            .into_iter()
            .flat_map(|manifest| waiting.by_manifest[manifest].iter().map(move |one| (one.clone(), manifest.clone())))
            .collect()
    }

    /// Move `session` to `place` in `manifest`'s line, and answer everyone's new place there with
    /// when each joined. `None`, and nothing moved, where it is not in that line or `place` names a
    /// Job that is not, or itself.
    pub(crate) fn reordered(&self, manifest: &str, session: &str, place: &Place) -> Option<Vec<(String, i64, Instant)>> {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        let Lines { by_manifest, entries, .. } = &mut *waiting;
        let line = by_manifest.get_mut(manifest)?;
        let from = line.iter().position(|one| one == session)?;
        if place.against().is_some_and(|against| against == session || !line.iter().any(|one| one == against)) {
            return None;
        }
        let moving = line.remove(from)?;
        let at = |line: &std::collections::VecDeque<String>, id: &str| line.iter().position(|one| one == id).unwrap_or(line.len());
        let to = match place {
            Place::Ahead(id) => at(line, id),
            Place::Behind(id) => (at(line, id) + 1).min(line.len()),
            Place::Last => line.len(),
        };
        line.insert(to, moving);
        let mut places = Vec::new();
        for (number, one) in line.iter().enumerate() {
            if let Some(entry) = entries.get_mut(one) {
                entry.place = i64::try_from(number).unwrap_or(i64::MAX);
                places.push((one.clone(), entry.place, entry.since.clone()));
            }
        }
        Some(places)
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
    /// `id` joined `manifest`'s line at `place`.
    pub(crate) async fn line_kept(&self, manifest: &str, id: &str, place: i64, since: &Instant) {
        let one = KeptInLine { session_id: id.to_string(), manifest_id: manifest.to_string(), place, since: since.as_str().to_string() };
        if let Err(why) = self.store().lock().await.keep_in_slot_line(&one) {
            self.session_warned(id, "its place in the line for a slot was not kept", why);
        }
    }

    /// `id` is in no line now, and a place ruled for it is let go.
    pub(crate) async fn line_let_go(&self, id: &str) {
        self.operator_seat().take_place(id);
        let _ = self.store().lock().await.leave_slot_line(id);
    }

    /// Move `id` to `place` in `manifest`'s line, and keep every place there. `false` where it could
    /// not be moved: it is not in that line, or `place` names a Job that is not.
    pub(crate) async fn line_reordered(&self, manifest: &str, id: &str, place: &Place) -> bool {
        let Some(places) = self.hosts().slot_queue().reordered(manifest, id, place) else { return false };
        for (one, at, since) in places {
            self.line_kept(manifest, &one, at, &since).await;
        }
        // The head may have changed while a slot was free.
        self.hosts().slot_queue().freed();
        true
    }

    /// As `id` joins `manifest`'s line, the place the Operator ruled for it, where Dispatch is on.
    pub(crate) async fn ruled_place_taken(&self, manifest: &str, id: &str) {
        let Some(place) = self.operator_seat().take_place(id) else { return };
        if self.operator_position(ipc::operator_positions::OperatorRole::Dispatch, manifest) == ipc::operator_positions::OperatorPosition::On {
            self.line_reordered(manifest, id, &place).await;
        }
    }

    /// Every Job waiting to start, with its title, each repository's in its order.
    pub(crate) async fn waiting_to_start(&self) -> Vec<InLine> {
        let everyone = self.hosts().slot_queue().everyone();
        let store = self.store().lock().await;
        everyone
            .into_iter()
            .map(|(session_id, manifest_id)| {
                let title = store.session(&session_id).ok().flatten().and_then(|one| one.title).unwrap_or_else(|| session_id.clone());
                InLine { session_id, title, manifest_id }
            })
            .collect()
    }

    /// Every Job the store has in a line, back in it at its place, its wait begun again and its call
    /// raised again. **One that cannot wait any more is let go**: ended, leased a slot, or paused
    /// with no Resume pressed. Then the line is looked at, since a slot may have freed meanwhile.
    pub(crate) async fn slot_lines_restored(self: &Arc<Self>) {
        let kept = match self.store().lock().await.slot_lines() {
            Ok(kept) => kept,
            Err(why) => {
                self.session_warned("", "the lines for a slot were not read back", why);
                return;
            }
        };
        let queue = Arc::clone(self.hosts().slot_queue());
        for one in kept {
            let waits = match self.session_and_hosting(&one.session_id).await {
                Ok((session, hosting)) => {
                    session.state != store::SessionState::Ended
                        && hosting.lease_slot.is_none()
                        && (hosting.paused_at.is_none() || hosting.paused_resuming)
                }
                Err(_) => false,
            };
            if !waits {
                self.line_let_go(&one.session_id).await;
                continue;
            }
            queue.join_at(&one.manifest_id, &one.session_id, one.place, &Instant::carried(&one.since));
            self.slot_wait_began(&one.session_id, "line").await;
            let _ = self.published_hosted(&one.session_id).await;
        }
        queue.freed();
    }
}
