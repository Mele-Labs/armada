//! A Session whose slot has fallen far behind its base, told once how far behind it is.
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

/// The branch checked out in the slot, where `behind` was counted, before the one its lease was cut on.
fn branch_of(slot: &WorktreeSlot) -> Option<String> {
    adapters::leasing::checked_out(Path::new(&slot.path)).or_else(|| slot.branch.clone())
}

/// What the Session is told: each slot it holds that is behind, and nothing more. One message
/// covers every slot, so a pass over several slots reaches the Session once.
fn said_of(slots: &[(&WorktreeSlot, u32)]) -> String {
    let lines: Vec<String> = slots
        .iter()
        .map(|(slot, behind)| {
            let on = branch_of(slot).map(|branch| format!(" (branch {branch})")).unwrap_or_default();
            format!("slot-{n}{on} is {behind} commits behind {base}.", n = slot.slot, base = slot.base)
        })
        .collect();
    lines.join(" ")
}

impl Fleet
{
    /// Every Session in `held` whose slot is `BEHIND_FROM` or more commits behind, with commits of
    /// its own, and has not been told since it last caught up. Marked told once the message was queued or taken, so a
    /// Session that was not listening is asked again at the next reading. **A mute is not a told
    /// mark**: `told_by_fleet` answers a muted notice as untold, so lifting the mute resumes it.
    pub(crate) async fn behind_bays_told(self: &Arc<Self>, held: &WorktreesHeld) {
        let leaving = self.branches_leaving().await;
        let mut by_session: BTreeMap<&str, Vec<(&WorktreeSlot, u32)>> = BTreeMap::new();
        for slot in &held.slots {
            let key = (slot.manifest_id.as_str().to_string(), slot.slot);
            let leaving_now = branch_of(slot).is_some_and(|branch| leaving.contains(&branch));
            let (Some(session), Some(behind)) = (holder_of(slot), behind_to_tell(slot).filter(|_| !leaving_now))
            else {
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

    /// The branches of pull requests already merged, in the merge queue or with auto-merge on, as
    /// last read. Merging the base into one tells nothing a person wants.
    async fn branches_leaving(&self) -> std::collections::BTreeSet<String> {
        let sweep = self.sweeping().lock().await;
        let mut leaving = std::collections::BTreeSet::new();
        for merged in sweep.merged.values().flatten() {
            leaving.insert(merged.pull.branch.as_written().to_string());
        }
        for (root, pulls) in &sweep.pulls {
            let queue = sweep.queues.get(root).map_or(&[][..], Vec::as_slice);
            for open in pulls {
                if open.pull.auto_merge || queue.iter().any(|entry| entry.number == open.pull.number) {
                    leaving.insert(open.pull.branch.as_written().to_string());
                }
            }
        }
        leaving
    }
}
