//! Which Jobs a Job may `SendMessage`, asked by the mod before the tool runs. `message_reach`.
//!
//! **Any live Job.** A Job writes to any Job Fleet keeps that has not ended, in any repository and
//! whichever Job started either. It is refused an ended Job, and a Job address no Job has. A name
//! that is no Job's address, such as a teammate's, is not Fleet's to refuse.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Caller, Refusal};
use ipc::MessageReach;

use super::address_of;
use crate::daemon::Fleet;

/// Why a `SendMessage` to a Job address is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Unreachable {
    Ended,
    Unknown,
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
    /// Whether the Job holding `caller`'s connection may write to `to`.
    pub(super) async fn message_reach_of(&self, caller: Caller, to: &str) -> Result<MessageReach, Refusal> {
        if self.session_holding(&caller).is_none() {
            return Ok(MessageReach::Allowed {});
        }
        Ok(match self.unreachable(to).await? {
            None => MessageReach::Allowed {},
            Some(why) => MessageReach::Refused { message: refused_to(to, why) },
        })
    }

    /// Why `to` cannot be written to, where it is a Job address: the Job it names has ended, or none
    /// has it. `None` for a live Job and for a name that is no Job's address.
    pub(super) async fn unreachable(&self, to: &str) -> Result<Option<Unreachable>, Refusal> {
        let Some(prefix) = to.strip_prefix("s-").filter(|prefix| !prefix.is_empty()) else {
            return Ok(None);
        };
        let store = self.store().lock().await;
        let named = store.sessions_beginning(prefix).map_err(|why| self.ledger_fault(why))?;
        let mut named = named.into_iter().filter(|session| address_of(&session.id) == to).peekable();
        if named.peek().is_none() {
            return Ok(Some(Unreachable::Unknown));
        }
        if named.any(|session| session.state != store::SessionState::Ended) {
            return Ok(None);
        }
        Ok(Some(Unreachable::Ended))
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
