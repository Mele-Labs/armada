//! Who a call at the agent's door was placed as, by its connection. `#2164`.
//!
//! **One value, so a call cannot be Helm's and a Commander's at once.** The
//! daemon is asked twice, once per kind of host, and a connection is held by at
//! most one process; if it were ever both, the narrower reach answers.

use axum::extract::Extension;

use crate::daemon::{Admitting, CommanderReach, HelmReach, Steerer};
use crate::door::HelmCalled;
use crate::mcp::Caller;

/// Set on a call the door makes for a Commander it placed, and by nothing
/// else. **An extension, never a header**: bytes on the wire cannot carry one.
/// It holds the Job the Commander was started for, which is what
/// [`Resolved`](crate::reference::Resolved) narrows every act to.
#[derive(Clone, Debug)]
pub(crate) struct CommanderCalled(pub(crate) ipc::JobId);

/// The caller behind one door call.
#[derive(Debug)]
pub(crate) enum Placed {
    /// Any agent of the person's, and a Drone's own door: every row marked `Yes`.
    Anyone,
    Helm(HelmReach),
    Commander(CommanderReach),
}

impl Placed {
    pub(crate) fn of(daemon: &impl Admitting, caller: Caller) -> Placed {
        if let Some(reach) = daemon.commander_at(caller) {
            return Placed::Commander(reach);
        }
        match daemon.helm_at(caller) {
            Some(reach) => Placed::Helm(reach),
            None => Placed::Anyone,
        }
    }

    /// Whether the caller is narrowed at all, and so reads against everything
    /// the door could offer and is refused by name below it.
    pub(crate) fn is_narrowed(&self) -> bool {
        !matches!(self, Placed::Anyone)
    }

    /// Whether the caller is a session Fleet hosts for Bridge, rather than Helm.
    pub(crate) fn is_a_hosted_session(&self) -> bool {
        matches!(self, Placed::Helm(reach) if reach.is_a_hosted_session())
    }

    pub(crate) fn may(&self, operation: &str) -> bool {
        match self {
            Placed::Anyone => true,
            Placed::Helm(reach) => reach.may(operation),
            Placed::Commander(reach) => reach.may(operation),
        }
    }

    pub(crate) fn refusing(&self, operation: &str) -> String {
        match self {
            Placed::Anyone => format!("`{operation}` is not this caller's to call"),
            Placed::Helm(reach) => reach.refusing(operation),
            Placed::Commander(reach) => reach.refusing(operation),
        }
    }

    /// Marks the request the door makes on this caller's behalf, for the
    /// handler that records who acted. **A hosted session's acts are the
    /// person's own**, so it is marked as no one.
    pub(crate) fn mark(&self, request: &mut axum::http::Request<axum::body::Body>) {
        match self {
            Placed::Helm(reach) if !reach.is_a_hosted_session() => {
                request.extensions_mut().insert(HelmCalled);
            }
            Placed::Commander(reach) => {
                request
                    .extensions_mut()
                    .insert(CommanderCalled(reach.job().clone()));
            }
            Placed::Helm(_) | Placed::Anyone => {}
        }
    }
}

/// Who is steering, from the marks the door left on the request. **A Commander
/// outranks Helm**: [`Placed`] never sets both.
pub(crate) fn steered_by(
    helm: Option<Extension<HelmCalled>>,
    commander: Option<Extension<CommanderCalled>>,
) -> Steerer {
    match (commander, helm) {
        (Some(_), _) => Steerer::Commander,
        (None, Some(_)) => Steerer::Helm,
        (None, None) => Steerer::Person,
    }
}
