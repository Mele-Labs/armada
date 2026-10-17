//! A Session whose slot has fallen far behind its base, told once to merge the base in.
//! `docs/concepts/session.md`, *What Fleet tells a Session*.
//!
//! Read off the list `GET /worktrees` serves, so the keeper that keeps it warm is what notices a
//! slot, and `behind` is the count the pool already made. **Never written down**: a Fleet that
//! restarts may say it once more to a Session still behind, which costs one turn.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{SlotHolding, WorktreeSlot, WorktreesHeld};

use crate::daemon::Fleet;

/// Commits behind the base from which a holder is told. Bridge's `BEHIND_FROM` in
/// `slot-trouble.ts` is the same number: below it a merge is routine and not worth a turn.
pub(crate) const BEHIND_FROM: u32 = 20;

/// Whom each slot's notice went to, by Manifest and slot. A slot is absent once it is no longer
/// behind or no Session holds it, so falling behind again is a new notice.
#[derive(Default)]
pub(crate) struct BehindTold(Mutex<BTreeMap<(String, u32), String>>);

impl BehindTold {
    fn told(&self, key: &(String, u32), session: &str) -> bool {
        let told = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        told.get(key).is_some_and(|to| to == session)
    }

    fn keep(&self, key: (String, u32), session: &str) {
        let mut told = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        told.insert(key, session.to_string());
    }

    fn forget(&self, key: &(String, u32)) {
        let mut told = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        told.remove(key);
    }
}

/// The Session a slot is held for, where one is.
fn holder_of(slot: &WorktreeSlot) -> Option<&str> {
    match &slot.held {
        SlotHolding::HostedSession { session_id, .. } => Some(session_id),
        SlotHolding::Session {
            session_id: Some(session_id),
            ..
        } => Some(session_id),
        _ => None,
    }
}

/// What the Session is told: the slot, its branch, how far behind and what to do.
fn said_of(slot: &WorktreeSlot, behind: u32) -> String {
    let on = slot
        .branch
        .as_ref()
        .map(|branch| format!(" (branch {branch})"))
        .unwrap_or_default();
    format!(
        "slot-{n}{on} is {behind} commits behind {base}. Merge {base} into your branch before \
         your next push.",
        n = slot.slot,
        base = slot.base,
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
    /// Every Session in `held` whose slot is `BEHIND_FROM` or more commits behind and has not been
    /// told since it last caught up. Marked told once the message was queued or taken, so a
    /// Session that was not listening is asked again at the next reading.
    pub(crate) async fn behind_bays_told(self: &Arc<Self>, held: &WorktreesHeld) {
        for slot in &held.slots {
            let key = (slot.manifest_id.as_str().to_string(), slot.slot);
            let behind = slot.behind.filter(|behind| *behind >= BEHIND_FROM);
            let (Some(session), Some(behind)) = (holder_of(slot), behind) else {
                self.behind_told().forget(&key);
                continue;
            };
            if self.behind_told().told(&key, session) {
                continue;
            }
            if matches!(
                self.told_by_fleet(session, &said_of(slot, behind)).await,
                Ok(true)
            ) {
                self.behind_told().keep(key, session);
            }
        }
    }
}
