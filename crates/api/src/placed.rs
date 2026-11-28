//! Who a call at the agent's door was placed as, by its connection. `#2164`.
//!
//! **One value**: a connection is held by at most one process, so a call is
//! a placed Job's or anyone's.


use crate::daemon::{Admitting, WordsReach};
use crate::door::WordsCalled;
use crate::mcp::Caller;

/// The caller behind one door call.
#[derive(Debug)]
pub(crate) enum Placed {
    /// Any agent of the person's, and a Drone's own door: every row marked `Yes`.
    Anyone,
    Words(WordsReach),
}

impl Placed {
    pub(crate) async fn of(daemon: &impl Admitting, caller: Caller) -> Placed {
        match daemon.words_reach_at(caller).await {
            Some(reach) => Placed::Words(reach),
            None => Placed::Anyone,
        }
    }

    /// Whether the caller is narrowed at all, and so reads against everything
    /// the door could offer and is refused by name below it.
    pub(crate) fn is_narrowed(&self) -> bool {
        !matches!(self, Placed::Anyone)
    }

    /// Whether the caller is a session Fleet hosts for Bridge, rather than a Job on your words.
    pub(crate) fn is_a_hosted_session(&self) -> bool {
        matches!(self, Placed::Words(reach) if reach.is_a_hosted_session())
    }

    /// Whether the caller is the Operator's own session.
    pub(crate) fn is_the_operator(&self) -> bool {
        matches!(self, Placed::Words(reach) if reach.is_the_operator())
    }

    /// Whether a setting of the caller's left `operation` out of what it is offered.
    pub(crate) fn is_withheld(&self, operation: &str) -> bool {
        matches!(self, Placed::Words(reach) if reach.is_withheld(operation))
    }

    /// Whether the tool is in the list the caller is shown.
    pub(crate) fn listed(&self, operation: &str) -> bool {
        match self {
            Placed::Anyone => true,
            Placed::Words(reach) => reach.listed(operation),
        }
    }

    /// Why the caller's own settings refuse `operation`: a tool switched off, or one open only after he has said so.
    pub(crate) fn held_back(&self, operation: &str) -> Option<String> {
        match self {
            Placed::Anyone => None,
            Placed::Words(reach) => reach.held_back(operation),
        }
    }

    pub(crate) fn may(&self, operation: &str) -> bool {
        match self {
            Placed::Anyone => true,
            Placed::Words(reach) => reach.may(operation),
        }
    }

    pub(crate) fn refusing(&self, operation: &str) -> String {
        match self {
            Placed::Anyone => format!("`{operation}` is not this caller's to call"),
            Placed::Words(reach) => reach.refusing(operation),
        }
    }

    /// Marks the request the door makes on this caller's behalf, for the
    /// handler that records who acted. **A hosted session's acts are the
    /// person's own**, so it is marked as no one.
    pub(crate) fn mark(&self, request: &mut axum::http::Request<axum::body::Body>) {
        match self {
            Placed::Words(reach) if !reach.is_a_hosted_session() => {
                request.extensions_mut().insert(WordsCalled);
            }
            Placed::Words(_) | Placed::Anyone => {}
        }
    }
}

