//! The calls Helm and the hosted Sessions hold open for a person. `#939`.
//!
//! **A repository's Helm is a hosted Session**, so its thread is a Session's and
//! is kept with its rows. What Fleet holds here is the one list of asks that
//! wait on a person, across every Session.

use super::asking::{Asks, HelmAskHold};

/// Every call held open for a person, across every conversation. **One list**,
/// because the dock is: what is waiting on a person is waiting on them
/// wherever it came from.
pub struct Conversations {
    asks: Asks,
}

impl Conversations {
    pub fn holding_for(hold: HelmAskHold) -> Conversations {
        Conversations {
            asks: Asks::holding_for(hold),
        }
    }

    pub(crate) fn asks(&self) -> &Asks {
        &self.asks
    }

    /// The hold this Fleet was assembled with, so a Fleet rebuilt around a
    /// different host keeps the bound the composition root resolved.
    pub fn hold(&self) -> HelmAskHold {
        self.asks.held_for()
    }
}
