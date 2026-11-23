//! Which Jobs a Job may `SendMessage`, asked by the mod before the tool runs, and who carries what
//! it sends. `message_reach`.
//!
//! **Any live Job.** A Job writes to any Job Fleet keeps that has not ended, in any repository and
//! whichever Job started either. It is refused an ended Job, and a Job address no Job has. A name
//! that is no Job's address, such as a teammate's, is not Fleet's to refuse.
//!
//! **And a live Job his other Mac shares, where the writer is shared too.** A private writer is refused
//! as for an address no Job has, so it learns nothing of what the other Mac holds.
//! `docs/practices/protocol/his-other-macs-fleets-half.md`, *Messages and alerts*.
//!
//! **One carrier for each message, and the sender is told which.** The CLI's own `SendMessage`
//! reaches only a live process, and Fleet lets a Job's process go between turns. Two sides see the
//! call: the mod's ask before the tool runs, and Fleet's reading of the call off the sender's stream
//! (`delivered`). Whichever comes first settles the carrier and the other follows it: the CLI for a
//! Job whose process runs, Fleet for one between turns, paused, on its start card or on his other
//! Mac. A message Fleet carries is `Queued`, and the mod answers the call with that so the CLI's
//! "not reachable" never reaches the sender. `docs/spikes/026-does-a-message-from-another-session-wake-a-live-process.md`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal};
use ipc::{MessageReach, QuietJob, QuietJobs, QuietWhy, SessionRecord};

use super::address_of;
use super::to_other_mac::MOVED_TO_MAC;
use crate::daemon::Fleet;

/// Why a `SendMessage` to a Job address is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Unreachable {
    Ended,
    Unknown,
}

/// Who takes one `SendMessage` to a Job Fleet keeps to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Carrier {
    /// The CLI's own delivery, to the live process of a Job this Mac hosts.
    Cli,
    /// Fleet, as a turn of the Job's own, because no process of it runs for the CLI to reach.
    Fleet(Kept),
}

/// Why Fleet carries a message, which is what the sender is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kept {
    Quiet(QuietWhy),
    OtherMac,
}

/// The side that settled a message's carrier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// The mod's `message_reach`, before the tool runs.
    Ask,
    /// Fleet's reading of the call off the sender's stream.
    Relay,
}

/// One message whose carrier one side settled, for the other to follow.
#[derive(Clone, Debug)]
pub(crate) struct Settled {
    to: String,
    carrier: Carrier,
    by: Side,
    at: Instant,
}

/// What a side reads when it settles a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Agreed {
    pub carrier: Carrier,
    /// This side settled it; the other side follows.
    pub first: bool,
}

/// How long a settled carrier waits for the other side. One that never comes, where the mod's hooks
/// are skipped or a subagent's call is not on the stream, is forgotten after it.
const SETTLED_FOR: Duration = Duration::from_secs(60);

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
    /// Whether the Job holding `caller`'s connection may write to `to`, and **with the words, who carries
    /// them**: a Job with no process is carried by Fleet, at once, and is `Queued`.
    pub(super) async fn message_reach_of(self: &Arc<Self>, caller: Caller, to: &str, message: Option<String>) -> Result<MessageReach, Refusal> {
        let Some(from) = self.session_holding(&caller) else {
            return Ok(MessageReach::Allowed {});
        };
        let why = match self.unreachable(to).await? {
            Some(Unreachable::Unknown) => self.unreachable_across(&from, to).await.err(),
            other => other,
        };
        if let Some(why) = why {
            return Ok(MessageReach::Refused { message: refused_to(to, why) });
        }
        let Some(message) = message else {
            return Ok(MessageReach::Allowed {});
        };
        let Some(agreed) = self.carrier_settled(&from, to, Side::Ask).await else {
            return Ok(MessageReach::Allowed {});
        };
        let Carrier::Fleet(kept) = agreed.carrier else {
            return Ok(MessageReach::Allowed {});
        };
        let title = self.title_at(to).await;
        if agreed.first {
            let fleet = Arc::clone(self);
            let to = to.to_string();
            tokio::spawn(async move { fleet.carried(&from, &to, &message, true).await });
        }
        Ok(MessageReach::Queued { message: queued_to(to, title.as_deref(), kept) })
    }

    /// The carrier of `from`'s message to `to`, settled once for both sides: the other side's, where
    /// it settled first, or this side's reading of `to` now. None for a name that is no Job Fleet
    /// keeps, and for the sender itself.
    pub(super) async fn carrier_settled(&self, from: &str, to: &str, side: Side) -> Option<Agreed> {
        let runtime = self.hosts().of(from);
        let _settling = runtime.messaging.lock().await;
        {
            let mut state = runtime.state();
            state.messages_settled.retain(|one| one.at.elapsed() < SETTLED_FOR);
            let other = state.messages_settled.iter().position(|one| one.to == to && one.by != side);
            if let Some(at) = other {
                let one = state.messages_settled.remove(at);
                return Some(Agreed { carrier: one.carrier, first: false });
            }
        }
        let carrier = self.carrier_of(from, to).await?;
        let settled = Settled { to: to.to_string(), carrier, by: side, at: Instant::now() };
        runtime.state().messages_settled.push(settled);
        Some(Agreed { carrier, first: true })
    }

    /// Who would take a message to `to` now: the CLI where a process of it runs, Fleet where none does.
    async fn carrier_of(&self, from: &str, to: &str) -> Option<Carrier> {
        let hosting = {
            let store = self.store().lock().await;
            let hostings = store.hostings().ok()?;
            hostings.into_iter().find(|hosting| address_of(&hosting.session_id) == to)
        };
        let hosting = match hosting {
            Some(hosting) if !self.has_moved_to_mac(&hosting.session_id).await => Some(hosting),
            _ => None,
        };
        let Some(hosting) = hosting else {
            return self.unreachable_across(from, to).await.ok().map(|_| Carrier::Fleet(Kept::OtherMac));
        };
        if hosting.session_id == from {
            return None;
        }
        Some(match self.quiet(&hosting) {
            Some(why) => Carrier::Fleet(Kept::Quiet(why)),
            None => Carrier::Cli,
        })
    }

    /// Whether Job `id` moved to another Mac: its hosting here is parked, and the Mac holding it is the one to write to.
    pub(super) async fn has_moved_to_mac(&self, id: &str) -> bool {
        self.store().lock().await.session(id).ok().flatten().is_some_and(|one| one.end_reason.as_deref() == Some(MOVED_TO_MAC))
    }

    /// Why no process of a hosted Job runs for the CLI to reach, or None where one does.
    fn quiet(&self, hosting: &store::KeptHosting) -> Option<QuietWhy> {
        if hosting.start_held {
            Some(QuietWhy::Held)
        } else if hosting.paused_at.is_some() {
            Some(QuietWhy::Paused)
        } else if self.hosts().of(&hosting.session_id).state().process.is_none() {
            Some(QuietWhy::BetweenTurns)
        } else {
            None
        }
    }

    /// `quiet_jobs`: the live Jobs this Mac hosts with no process, which `ListAgents` leaves out, for the
    /// Job holding `caller`'s connection. Not the caller itself. **None for a caller Fleet cannot place**,
    /// such as a terminal Job: Fleet carries no message of one, so a Job listed to it could not be reached.
    pub(super) async fn quiet_jobs_of(&self, caller: Caller) -> Result<QuietJobs, Refusal> {
        let Some(from) = self.session_holding(&caller) else {
            return Ok(QuietJobs::default());
        };
        let kept = {
            let store = self.store().lock().await;
            let hostings = store.hostings().map_err(|why| self.ledger_fault(why))?;
            let mut kept = Vec::new();
            for hosting in hostings {
                if hosting.session_id == from {
                    continue;
                }
                let session = store.session(&hosting.session_id).map_err(|why| self.ledger_fault(why))?;
                let Some(session) = session.filter(|one| one.state != store::SessionState::Ended) else {
                    continue;
                };
                kept.push((hosting, session.title));
            }
            kept
        };
        let jobs = kept
            .into_iter()
            .filter_map(|(hosting, title)| {
                let why = self.quiet(&hosting)?;
                Some(QuietJob { address: address_of(&hosting.session_id), title, why })
            })
            .collect();
        Ok(QuietJobs { jobs })
    }

    /// The title of the Job at address `to`, held here or from his other Mac.
    async fn title_at(&self, to: &str) -> Option<String> {
        let prefix = to.strip_prefix("s-")?;
        let named = self.store().lock().await.sessions_beginning(prefix).ok()?;
        if let Some(one) = named.into_iter().find(|one| address_of(&one.id) == to) {
            return one.title;
        }
        let held = self.sessions_from_macs().await.ok()?;
        held.into_iter().find(|one| address_of(one.id.as_str()) == to)?.title
    }

    /// The live Job at address `to` that his other Mac holds and shares, for Job `from` of this Mac to write to:
    /// its row as held here. **Refused as `Unknown` where `from` is not shared**, as for an address no Job has.
    pub(super) async fn unreachable_across(&self, from: &str, to: &str) -> Result<SessionRecord, Unreachable> {
        let held = self.sessions_from_macs().await.unwrap_or_default();
        let mut named = held.into_iter().filter(|one| address_of(one.id.as_str()) == to);
        let (Some(record), None) = (named.next(), named.next()) else {
            return Err(Unreachable::Unknown);
        };
        if !self.job_shared(from).await {
            return Err(Unreachable::Unknown);
        }
        if record.state == ipc::SessionState::Ended {
            return Err(Unreachable::Ended);
        }
        Ok(record)
    }

    /// Why `to` cannot be written to, where it is a Job address: the Job it names has ended, or none
    /// has it. `None` for a live Job and for a name that is no Job's address.
    pub(super) async fn unreachable(&self, to: &str) -> Result<Option<Unreachable>, Refusal> {
        let Some(prefix) = to.strip_prefix("s-").filter(|prefix| !prefix.is_empty()) else {
            return Ok(None);
        };
        let store = self.store().lock().await;
        let named = store.sessions_beginning(prefix).map_err(|why| self.ledger_fault(why))?;
        let named: Vec<_> = named.into_iter().filter(|session| address_of(&session.id) == to).collect();
        if named.is_empty() {
            return Ok(Some(Unreachable::Unknown));
        }
        if named.iter().any(|session| session.state != store::SessionState::Ended) {
            return Ok(None);
        }
        // A row that only moved to another Mac is not an ended Job: the Mac holding it says whether it lives.
        if named.iter().all(|session| session.end_reason.as_deref() == Some(MOVED_TO_MAC)) {
            return Ok(Some(Unreachable::Unknown));
        }
        Ok(Some(Unreachable::Ended))
    }
}

/// What the sender of a message Fleet carries reads in place of the CLI's answer.
pub(crate) fn queued_to(to: &str, title: Option<&str>, kept: Kept) -> String {
    let job = match title {
        Some(title) => format!("job \"{title}\" ({to})"),
        None => format!("job {to}"),
    };
    match kept {
        Kept::Quiet(QuietWhy::BetweenTurns) => {
            format!("Queued for {job}. Its agent is between turns, and Fleet starts it with your message.")
        }
        Kept::Quiet(QuietWhy::Paused) => {
            format!("Queued for {job}. It is paused, and your message is its first turn when it is resumed.")
        }
        Kept::Quiet(QuietWhy::Held) => {
            format!("Queued for {job}. It has not been started, and your message goes to it when it starts.")
        }
        Kept::OtherMac => format!("Queued for {job} on his other Mac. You are told here if that Mac refuses it."),
    }
}

/// Why a `SendMessage` to `to` is refused, as the hook and Fleet's own delivery both say it.
pub(super) fn refused_to(to: &str, why: Unreachable) -> String {
    match why {
        Unreachable::Ended => format!("`SendMessage` to {to} is refused: that Job has ended"),
        Unreachable::Unknown => {
            format!("`SendMessage` to {to} is refused: no Job has that address. ListAgents names the live ones")
        }
    }
}
