//! A Session whose slot has fallen far behind its base, told once to merge the base in.
//! `docs/concepts/session.md`, *What Fleet tells a Session*.
//!
//! Read off the list `GET /worktrees` serves, so the keeper that keeps it warm is what notices a
//! slot, and `behind` is the count the pool already made. **Never written down**: a Fleet that
//! restarts may say it once more to a Session still behind, which costs one turn.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Notice, SlotHolding, WorktreeSlot, WorktreesHeld};

use crate::daemon::Fleet;

/// Commits behind the base from which a holder is told. Bridge's `BEHIND_FROM` in
/// `slot-trouble.ts` is the same number: below it a merge is routine and not worth a turn.
pub(crate) const BEHIND_FROM: u32 = 20;

/// Whom each slot's notice went to, by Manifest and slot. A slot is absent once it is no longer
/// behind or no Session holds it, so falling behind again is a new notice. A reading that cannot
/// tell (`unread`) forgets nothing.
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

/// The hosted Session a slot is held for, where one is. A process's slot naming a Session is a subagent's bay, whose branch is not
/// the Session's own, so it is not told: the stranded read skips it too (`docs/concepts/fleet.md`).
fn holder_of(slot: &WorktreeSlot) -> Option<&str> {
    match &slot.held {
        SlotHolding::HostedSession { session_id, .. } => Some(session_id),
        _ => None,
    }
}

/// Whether this reading cannot say if the slot caught up: a lease or release is under way on it, or
/// its holder's counts could not be made. It leaves the told mark as it was.
fn unread(slot: &WorktreeSlot) -> bool {
    matches!(slot.held, SlotHolding::Busy)
        || (holder_of(slot).is_some() && (slot.behind.is_none() || slot.ahead.is_none()))
}

/// How far behind a slot is, where that is worth a turn: `BEHIND_FROM` or more, on a branch with
/// commits of its own. A branch level with an old base has nothing stale to push.
fn behind_to_tell(slot: &WorktreeSlot) -> Option<u32> {
    let behind = slot.behind.filter(|behind| *behind >= BEHIND_FROM)?;
    slot.ahead.is_some_and(|ahead| ahead > 0).then_some(behind)
}

/// What the Session is told: each slot it holds that is behind, then what to do. One message
/// covers every slot, so a pass over several slots reaches the Session once. A slot names the branch
/// checked out in it, where `behind` was counted, before the one its lease was cut on.
fn said_of(slots: &[(&WorktreeSlot, u32)]) -> String {
    let lines: Vec<String> = slots
        .iter()
        .map(|(slot, behind)| {
            let on = adapters::leasing::checked_out(Path::new(&slot.path))
                .or_else(|| slot.branch.clone())
                .map(|branch| format!(" (branch {branch})"))
                .unwrap_or_default();
            format!("slot-{n}{on} is {behind} commits behind {base}.", n = slot.slot, base = slot.base)
        })
        .collect();
    format!(
        "{} Merge {base} into your branch before your next push.",
        lines.join(" "),
        base = slots[0].0.base,
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
    /// Every Session in `held` whose slot is `BEHIND_FROM` or more commits behind, with commits of
    /// its own, and has not been told since it last caught up. Marked told once the message was queued or taken, so a
    /// Session that was not listening is asked again at the next reading. **A mute is not a told
    /// mark**: `told_by_fleet` answers a muted notice as untold, so lifting the mute resumes it.
    pub(crate) async fn behind_bays_told(self: &Arc<Self>, held: &WorktreesHeld) {
        let mut by_session: BTreeMap<&str, Vec<(&WorktreeSlot, u32)>> = BTreeMap::new();
        for slot in &held.slots {
            let key = (slot.manifest_id.as_str().to_string(), slot.slot);
            let (Some(session), Some(behind)) = (holder_of(slot), behind_to_tell(slot)) else {
                if !unread(slot) {
                    self.behind_told().forget(&key);
                }
                continue;
            };
            if self.behind_told().told(&key, session) {
                continue;
            }
            by_session.entry(session).or_default().push((slot, behind));
        }
        for (session, slots) in by_session {
            if matches!(
                self.told_by_fleet(session, &said_of(&slots), Notice::BehindBase).await,
                Ok(true)
            ) {
                for (slot, _) in slots {
                    let key = (slot.manifest_id.as_str().to_string(), slot.slot);
                    self.behind_told().keep(key, session);
                }
            }
        }
    }
}
