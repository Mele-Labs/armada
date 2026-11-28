//! The Operator as a running Fleet holds it: what is waiting to be told to its session and the standing
//! orders. In memory.

use std::sync::Mutex;

use super::held;

#[derive(Default)]
pub(crate) struct Seat {
    /// Reports, issues and notes waiting to be told.
    pub(crate) sources: super::Sources,
    /// The standing orders, as the store keeps them: read where no lock on the store may be taken.
    orders: Mutex<super::Orders>,
    /// What reads Your turn against the Jobs' lists, one at a time.
    pub(super) raising: super::raising::Raising,
    /// One writer of the journal at a time.
    pub(super) journal: tokio::sync::Mutex<()>,
    /// One start or end of the Operator's own session at a time.
    pub(super) session: tokio::sync::Mutex<()>,
    /// Rung when the switch moves or a send waits for a session, so the session starts or ends now and not on the next tick.
    pub(super) wake: tokio::sync::Notify,
    /// When the soonest Later runs out: `None` until the store is first read, then `Some(None)` where
    /// no item is Later.
    pub(super) next_later: Mutex<Option<Option<String>>>,
}

impl Seat {
    pub(crate) fn orders(&self) -> super::Orders {
        held(&self.orders).clone()
    }

    pub(crate) fn orders_are(&self, orders: super::Orders) {
        *held(&self.orders) = orders;
    }
}
