//! A forge call a test holds shut, to see what a Fleet does while it is waiting on one.
//!
//! The fake's calls are synchronous, so the hold blocks the thread that made the call: a Fleet
//! that makes it on a runtime worker blocks the worker, and one that makes it on a blocking thread
//! does not. A test that holds a call and then expects the reply is proving the latter.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// How long a held call waits for [`Hold::open`] before giving up, so a test that forgets does not
/// hang the suite.
const NEVER_OPENED: Duration = Duration::from_secs(10);

#[derive(Debug, Default)]
struct HoldState {
    open: bool,
    waiting: usize,
}

/// One held call. [`Hold::open`] lets the calls waiting on it, and every later one, through.
#[derive(Debug, Default)]
pub struct Hold {
    state: Mutex<HoldState>,
    changed: Condvar,
}

impl Hold {
    pub(super) fn closed() -> Arc<Hold> {
        Arc::new(Hold::default())
    }

    /// Let every call through, the waiting ones now.
    pub fn open(&self) {
        self.state.lock().expect("not poisoned").open = true;
        self.changed.notify_all();
    }

    /// How many calls are waiting on it now.
    pub fn waiting(&self) -> usize {
        self.state.lock().expect("not poisoned").waiting
    }

    /// Wait here until the hold is opened.
    pub(super) fn pass(&self) {
        let mut state = self.state.lock().expect("not poisoned");
        state.waiting += 1;
        let (mut state, _) = self
            .changed
            .wait_timeout_while(state, NEVER_OPENED, |state| !state.open)
            .expect("not poisoned");
        state.waiting -= 1;
    }
}

impl super::FakeVcs {
    /// Hold every push shut until the returned [`Hold`] is opened.
    pub fn hold_pushes(&self) -> Arc<Hold> {
        let hold = Hold::closed();
        *self.pushes_held.lock().expect("not poisoned") = Some(Arc::clone(&hold));
        hold
    }

    /// Hold shut every read of a pull request made after a write the forge took, until the
    /// returned [`Hold`] is opened. The read before the write is not held.
    pub fn hold_reads_after_a_write(&self) -> Arc<Hold> {
        let hold = Hold::closed();
        self.pull_requests.lock().expect("not poisoned").hold_after_write = Some(Arc::clone(&hold));
        hold
    }

    /// Hold every send of the served checkout's files shut until the returned [`Hold`] is opened.
    pub fn hold_checkout_sends(&self) -> Arc<Hold> {
        let hold = Hold::closed();
        self.checkout_sends.lock().expect("not poisoned").held = Some(Arc::clone(&hold));
        hold
    }
}
