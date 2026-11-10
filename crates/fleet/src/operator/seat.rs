//! The Operator as a running Fleet holds it: the one seated at start, the call it makes, what is
//! waiting to be handed to it and the standing orders. In memory.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, OnceLock};

use super::{held, KeptStarted, Operator};

#[derive(Default)]
pub(crate) struct Seat {
    operator: OnceLock<Arc<Operator>>,
    /// The call the seated Operator makes, where it is the kept one: started and stopped with the switch.
    kept: OnceLock<Arc<KeptStarted>>,
    /// Reports, issues and notes waiting to be handed, and those a call is out on.
    pub(super) sources: super::Sources,
    /// The standing orders, as the store keeps them: read where no lock on the store may be taken.
    orders: Mutex<super::Orders>,
    /// What waits on the person and is being reviewed to be raised to him.
    pub(super) raising: super::raising::Raising,
    /// One writer of the journal at a time.
    pub(super) journal: tokio::sync::Mutex<()>,
    /// A briefing is out.
    pub(super) briefing: AtomicBool,
}

impl Seat {
    /// Seat `operator`, once: `false` where one already sits.
    pub(crate) fn seat(&self, operator: Arc<Operator>) -> bool {
        self.operator.set(operator).is_ok()
    }

    pub(super) fn operator(&self) -> Option<Arc<Operator>> {
        self.operator.get().cloned()
    }

    pub(crate) fn orders(&self) -> super::Orders {
        held(&self.orders).clone()
    }

    pub(crate) fn orders_are(&self, orders: super::Orders) {
        *held(&self.orders) = orders;
    }

    pub(crate) fn keep(&self, call: Arc<KeptStarted>) {
        let _ = self.kept.set(call);
    }

    pub(crate) fn kept(&self) -> Option<Arc<KeptStarted>> {
        self.kept.get().cloned()
    }
}
