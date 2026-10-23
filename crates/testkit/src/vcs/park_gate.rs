//! A park held until a test opens it, so a case can look at the Fleet while a slot is mid-park.

use std::sync::{Arc, Condvar, Mutex};

/// What a case holds while every park of a hosted slot waits
/// ([`FakeVcs::hold_parks`](super::FakeVcs::hold_parks)). **Dropping it opens the gate**, so a case
/// that fails with a park held does not hang the runtime on the blocked thread.
#[derive(Debug)]
pub struct ParkHold(pub(super) Arc<ParkGate>);

impl ParkHold {
    /// How many parks have reached the gate.
    pub fn arrived(&self) -> usize {
        self.0.arrived()
    }

    /// Let every park through, now and later.
    pub fn open(&self) {
        self.0.open();
    }
}

impl Drop for ParkHold {
    fn drop(&mut self) {
        self.0.open();
    }
}

/// Every park waits here once installed. A park runs on a blocking thread, so the wait is a plain
/// condvar.
#[derive(Debug, Default)]
pub(super) struct ParkGate {
    state: Mutex<State>,
    changed: Condvar,
}

#[derive(Debug, Default)]
struct State {
    arrived: usize,
    open: bool,
}

impl ParkGate {
    /// How many parks have reached the gate.
    fn arrived(&self) -> usize {
        self.state.lock().expect("not poisoned").arrived
    }

    /// Let every park through, now and later.
    fn open(&self) {
        self.state.lock().expect("not poisoned").open = true;
        self.changed.notify_all();
    }

    pub(super) fn pass(&self) {
        let mut state = self.state.lock().expect("not poisoned");
        state.arrived += 1;
        while !state.open {
            state = self.changed.wait(state).expect("not poisoned");
        }
    }
}
