//! A slot seen stranded: its last holder told once, and a rescue Scout started when none could be.
//! `docs/concepts/fleet.md`, *Rescuing a stranded slot*.
//!
//! **Once a stranding, kept in `slot_strandings`**, so a restarted Fleet repeats neither. Both are
//! read off the list `GET /worktrees` serves, so the keeper that keeps it warm is what notices a
//! slot.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, SlotReading, Vcs, WorkProduct};
use ipc::{
    ManifestId, RescueAct, RescueSlot, SlotFindingState, SlotHolderKind, SlotHolding,
    SlotLastHolder, SlotStranded, WorktreeSlot, WorktreesHeld,
};
use store::KeptStranding;

use crate::daemon::Fleet;
use crate::clashing::listed;

/// Which stranding a reading is of: its branch, and when its lease began as the wire spells it.
/// Empty where the slot named neither.
pub(crate) fn key_of(reading: &SlotReading) -> (String, String) {
    (
        reading.branch.clone().unwrap_or_default(),
        crate::wire::since_of(reading.since)
            .map(|since| since.as_str().to_string())
            .unwrap_or_default(),
    )
}

/// What a stranded slot's Session is asked.
fn asked_of(slot: &WorktreeSlot, work: &SlotStranded) -> String {
    let on = slot
        .branch
        .as_ref()
        .map(|branch| format!(" on branch {branch}"))
        .unwrap_or_default();
    let mut left = Vec::new();
    if !work.uncommitted.is_empty() {
        left.push(format!("uncommitted: {}", listed(&work.uncommitted)));
    }
    if work.unpushed > 0 {
        left.push(format!("{} commits on no remote or base", work.unpushed));
    }
    format!(
        "slot-{n}{on} holds work and nothing holds the slot now ({left}). Commit it, push the \
         branch, then release the slot with `armada worktree release {path}`.",
        n = slot.slot,
        left = left.join("; "),
        path = slot.path,
    )
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
    /// A Session as a slot's last holder: open while the ledger holds it live. **One the ledger does
    /// not know is not open**, which is how a Session Fleet forgot reads as gone.
    pub(crate) async fn session_holder(&self, id: &str) -> SlotLastHolder {
        let kept = self.store().lock().await.session(id).ok().flatten();
        SlotLastHolder {
            kind: SlotHolderKind::Session,
            id: id.to_string(),
            title: kept.as_ref().and_then(|session| session.title.clone()),
            open: kept
                .as_ref()
                .is_some_and(|session| session.state == store::SessionState::Live),
        }
    }

    /// Every stranded slot in `held` that Fleet has not yet acted on: an open Session is told once, and
    /// where nobody could be told a Scout reads the slot, as a press of Rescue would.
    ///
    /// **One Scout reads at a time.** A Fleet that comes up on several stranded slots would otherwise
    /// start a model call for each at once.
    pub(crate) async fn strandings_attended(self: &Arc<Self>, held: &WorktreesHeld) {
        let kept = self.store().lock().await.strandings().unwrap_or_default();
        let mut reading = held.slots.iter().any(|slot| {
            slot.rescue
                .as_ref()
                .is_some_and(|found| found.state == SlotFindingState::Reading)
        });
        for slot in &held.slots {
            let manifest = slot.manifest_id.as_str();
            let row = kept
                .iter()
                .find(|row| row.manifest_id == manifest && row.slot == slot.slot);
            let work = slot.stranded.as_ref().filter(|_| {
                matches!(
                    slot.held,
                    SlotHolding::Stranded { .. }
                        | SlotHolding::HostedSession { .. }
                )
            });
            let Some(work) = work else {
                if row.is_some() {
                    let _ = self.store().lock().await.forget_stranding(manifest, slot.slot);
                }
                continue;
            };
            let branch = slot.branch.clone().unwrap_or_default();
            let since = slot
                .since
                .as_ref()
                .map(|since| since.as_str().to_string())
                .unwrap_or_default();
            let mut done = row
                .filter(|row| row.is_of(&branch, &since))
                .cloned()
                .unwrap_or(KeptStranding {
                    manifest_id: manifest.to_string(),
                    slot: slot.slot,
                    branch,
                    since,
                    told_at: None,
                    scouted: false,
                });
            let mut changed = false;
            let gone = match work.last_holder.as_ref() {
                Some(holder) if holder.open && holder.kind == SlotHolderKind::Session => {
                    if done.told_at.is_some() || done.scouted {
                        false
                    } else if let Ok(told) =
                        self.noticed_by_fleet(&holder.id, &asked_of(slot, work), ipc::Notice::SlotStranded).await
                    {
                        if told.settled() {
                            done.told_at = Some(self.now().as_str().to_string());
                            // Only a Session that was told can have refused it.
                            if told.sent() {
                                self.tell_kept(&holder.id, crate::session_host::tell_refusal::STRANDED_SLOT).await;
                            }
                            changed = true;
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                }
                // A Job still going is neither told nor read over.
                Some(holder) if holder.open => false,
                _ => true,
            };
            if gone && !done.scouted && slot.rescue.is_none() && !reading {
                let _ = Arc::clone(self)
                    .rescue_slot(
                        RescueSlot {
                            act: RescueAct::Start,
                            slot: slot.slot,
                        },
                        Some(&ManifestId::carried(manifest)),
                    )
                    .await;
                done.scouted = true;
                changed = true;
                reading = true;
            }
            if changed {
                let _ = self.store().lock().await.keep_stranding(&done);
                self.worktrees_memory().changed();
            }
        }
    }
}
