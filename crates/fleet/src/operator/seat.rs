//! The Operator as a running Fleet holds it: what is waiting to be told to its session and the standing
//! orders. In memory.

use std::collections::HashMap;
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
    /// When he last said something to each project's Operator, as epoch milliseconds: his press on one of its cards or his
    /// words delivered to it. **In memory, by project**, so a hand-on to a fresh session keeps it and a restart clears it.
    says: Mutex<HashMap<String, i64>>,
    /// Until when each project's Operator may reach another project, as `(from, to)` and epoch milliseconds: what he
    /// granted on a card Fleet raised. In memory, by project, for the same reason.
    grants: Mutex<HashMap<(String, String), i64>>,
}

impl Seat {
    pub(crate) fn orders(&self) -> super::Orders {
        held(&self.orders).clone()
    }

    pub(crate) fn orders_are(&self, orders: super::Orders) {
        *held(&self.orders) = orders;
    }

    /// `project`'s Operator was said something to at `at`.
    pub(crate) fn said_at(&self, project: &str, at: i64) {
        held(&self.says).insert(project.to_string(), at);
    }

    /// When `project`'s Operator was last said something to.
    pub(crate) fn last_said(&self, project: &str) -> Option<i64> {
        held(&self.says).get(project).copied()
    }

    /// `from`'s Operator may reach `to` until `until`.
    pub(crate) fn granted(&self, from: &str, to: &str, until: i64) {
        held(&self.grants).insert((from.to_string(), to.to_string()), until);
    }

    /// Whether `from`'s Operator may reach `to` at `now`.
    pub(crate) fn grant_open(&self, from: &str, to: &str, now: i64) -> bool {
        held(&self.grants).get(&(from.to_string(), to.to_string())).is_some_and(|until| *until > now)
    }
}
