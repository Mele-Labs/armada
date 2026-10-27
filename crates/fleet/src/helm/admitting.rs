//! The agent door's question about a caller, answered over a real Fleet.
//! `#941`.
//!
//! **Placed by the connection, never by what the caller says.** A call is Helm's
//! where `crate::peer::held_within` finds its socket inside the process of a hosted Session that
//! is a repository's Helm (`Fleet::session_placed`). The reach is [`may`] under
//! [`Fleet::helm_authority`], so the note and the door read one answer. Any other hosted Session
//! keeps a Session's door.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Admitting, Caller, HelmReach};

use super::{may, Authority};
use crate::daemon::Fleet;

impl<H, V, W> Admitting for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn helm_at(&self, caller: Caller) -> Option<HelmReach> {
        let (_, helm) = self.session_placed(&caller)?;
        if !helm {
            return Some(HelmReach::of_a_hosted_session());
        }
        let authority = self.helm_authority();
        Some(HelmReach::deciding(
            |row| may(authority, row),
            otherwise(authority),
        ))
    }

    fn always_loads(&self) -> Option<&'static str> {
        self.harness().always_loads_tool()
    }
}

/// Why a call outside Helm's reach is refused, said after its name.
fn otherwise(authority: Authority) -> &'static str {
    match authority {
        Authority::Acting => "that act is left to the person",
        Authority::ReadOnly => "this machine is set so that Helm only reads",
    }
}
