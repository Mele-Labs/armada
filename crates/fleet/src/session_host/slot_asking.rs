//! A hosted Session's `armada worktree lease` finding every slot held, and the call Fleet raises
//! for it. `ipc::slot_asking`, `docs/concepts/session.md`, *A full pool asks the owner*.
//!
//! **One call per Session, never two.** The call is the `slot_choice` item a queued write raises.
//! A command's asking adds a record beside the line, and the item is derived from whichever stands:
//! the line's where the Session is in it, the command's otherwise.
//!
//! **The command's record expires**: it is heard from about once a second and forgotten after
//! [`EXPIRES`], so a command that died takes its call and any reservation with it.
//!
//! **A slot added for a command is held for it** for [`RESERVED_FOR`] or until it says it is
//! done: the grant to the line leaves that repository alone meanwhile, and a write that would take
//! the slot straight off the pool is told to queue.

use std::sync::Arc;
use std::time::{Duration, Instant as Tick};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{AskForSlot, AskForSlotAct, Instant, SlotAskStanding, SlotAsked};

use super::queueing::{Choice, SlotQueue};
use crate::daemon::Fleet;

/// How long a command may go unheard before its record is forgotten.
pub(crate) const EXPIRES: Duration = Duration::from_secs(10);

/// How long a slot added for a command is held for it.
pub(crate) const RESERVED_FOR: Duration = Duration::from_secs(30);

/// One Session's command, asking.
pub(super) struct Want {
    manifest: String,
    pub(super) choice: Choice,
    pub(super) since: Instant,
    beat: Tick,
    reserved_until: Option<Tick>,
}

impl Want {
    pub(super) fn fresh(&self, now: Tick) -> bool {
        now.saturating_duration_since(self.beat) < EXPIRES
    }

    fn holds(&self, now: Tick) -> bool {
        self.fresh(now) && self.reserved_until.is_some_and(|until| now < until)
    }
}

impl SlotQueue {
    /// The command of `session` is asking in `manifest`'s pool. Raises its record, or hears from it
    /// again. Where the answer stands, and whether the call is newly on the person's list.
    pub(crate) fn want(&self, manifest: &str, session: &str, since: &Instant, now: Tick) -> (SlotAskStanding, bool) {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        let lined = waiting.entries.get(session).map(|entry| entry.choice);
        let stale = waiting.wants.get(session).is_none_or(|want| !want.fresh(now));
        if stale {
            let choice = match lined {
                Some(Choice::Pending) | None => Choice::Pending,
                Some(_) => Choice::Waiting,
            };
            waiting.wants.insert(
                session.to_string(),
                Want { manifest: manifest.to_string(), choice, since: since.clone(), beat: now, reserved_until: None },
            );
        }
        let want = waiting.wants.get_mut(session).expect("just put there");
        want.beat = now;
        let standing = match (lined, want.choice) {
            (Some(Choice::Pending), _) | (None, Choice::Pending) => SlotAskStanding::Asking,
            (None, Choice::Added) if want.holds(now) => SlotAskStanding::Added,
            _ => SlotAskStanding::Waiting,
        };
        (standing, stale && lined.is_none() && want.choice == Choice::Pending)
    }

    /// The command is done. Whether its call was on the person's list, and whether it held a slot.
    pub(super) fn done(&self, session: &str, now: Tick) -> (bool, bool) {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        let lined = waiting.entries.contains_key(session);
        match waiting.wants.remove(session) {
            Some(want) => (want.choice == Choice::Pending && !lined, want.holds(now)),
            None => (false, false),
        }
    }

    /// The repository a heard-from command of `session` asks in.
    pub(super) fn wanted_in(&self, session: &str, now: Tick) -> Option<String> {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting.wants.get(session).filter(|want| want.fresh(now)).map(|want| want.manifest.clone())
    }

    /// What the person chose for a command's record. Whether there was one to choose for.
    pub(crate) fn choose_want(&self, session: &str, choice: Choice) -> bool {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        match waiting.wants.get_mut(session) {
            Some(want) => {
                want.choice = choice;
                true
            }
            None => false,
        }
    }

    /// A slot was added for the command: hold it.
    pub(crate) fn reserve(&self, session: &str, now: Tick) {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        if let Some(want) = waiting.wants.get_mut(session) {
            want.choice = Choice::Added;
            want.reserved_until = Some(now + RESERVED_FOR);
        }
    }

    /// Whether a slot of `manifest` is held for a command, other than `session`'s own.
    pub(crate) fn reserved(&self, manifest: &str, except: Option<&str>, now: Tick) -> bool {
        let waiting = self.waiting.lock().expect("the line is not held across a panic");
        waiting
            .wants
            .iter()
            .any(|(one, want)| want.manifest == manifest && Some(one.as_str()) != except && want.holds(now))
    }

    /// Forget the commands not heard from. The Sessions whose list or reservation that changed.
    pub(crate) fn expire(&self, now: Tick) -> Vec<String> {
        let mut waiting = self.waiting.lock().expect("the line is not held across a panic");
        let gone: Vec<String> =
            waiting.wants.iter().filter(|(_, want)| !want.fresh(now)).map(|(one, _)| one.clone()).collect();
        gone.into_iter()
            .filter(|one| {
                let want = waiting.wants.remove(one).expect("listed above");
                let visible = want.choice == Choice::Pending && !waiting.entries.contains_key(one);
                visible || want.reserved_until.is_some()
            })
            .collect()
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
    /// `ask_for_slot`. A Session Fleet does not host, or one that has ended, is `unasked`: the
    /// command goes on as it did, and nothing is refused.
    pub(crate) async fn slot_asked(&self, call: AskForSlot) -> SlotAsked {
        let id = call.session_id.as_str();
        let queue = Arc::clone(self.hosts().slot_queue());
        let now = Tick::now();
        if call.act == AskForSlotAct::Done {
            let (visible, held) = queue.done(id, now);
            if visible {
                let _ = self.published_hosted(id).await;
            }
            if held {
                queue.freed();
            }
            return SlotAsked { standing: SlotAskStanding::Unasked };
        }
        let unasked = SlotAsked { standing: SlotAskStanding::Unasked };
        let Ok((session, hosting)) = self.session_and_hosting(id).await else {
            return unasked;
        };
        if session.state == store::SessionState::Ended {
            return unasked;
        }
        let (standing, raised) = queue.want(&hosting.manifest_id, id, &self.instant(), now);
        if raised {
            let _ = self.published_hosted(id).await;
        }
        SlotAsked { standing }
    }
}
