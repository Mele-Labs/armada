//! What a Job needs first: the order of its open asks, and why each stands where it does.
//! `crates/ipc/operations/order_waiting.toml` is the contract; `docs/concepts/session/waiting-on-the-person.md` the rule.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// `order_waiting`: the Job's asks it knows the sequence of, first to last, each with why. Replaces the order set
/// before; an empty list clears it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderWaiting {
    pub items: Vec<WaitingPlace>,
    /// Which Job. **Read only where the connection places no hosted session**, as `SetWaitingFor`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// One ask placed: its id on the Job's waiting list, and one plain sentence on why it stands there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingPlace {
    pub id: String,
    pub why: String,
}

/// `fleet.waiting_order_unknown`: an id that names no open ask of the Job.
pub const WAITING_ORDER_UNKNOWN: &str = "fleet.waiting_order_unknown";
/// `fleet.waiting_order_blank`: an item with no id or no why, or two with one id.
pub const WAITING_ORDER_BLANK: &str = "fleet.waiting_order_blank";
