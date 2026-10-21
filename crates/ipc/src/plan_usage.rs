//! How much of the account's plan windows this machine has used. `docs/concepts/machine.md`, *Quota*.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// One window of the account's plan, as a session's mod last read it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanWindow {
    /// `five_hour`, `seven_day`, or a gateway's `spend_limit`. Text and not a closed set: Bridge
    /// draws the two it knows and leaves the rest.
    pub kind: String,
    /// How much of the window is used, whole percent. Past 100 on an exceeded spend limit.
    pub percent_used: u32,
    /// When the window resets. Absent where the reading carried none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<Instant>,
}

/// `get_plan_usage`, and `plan_usage.changed` whole: the newest reading of the account's windows,
/// from whichever session reported last. **The windows belong to the account**, so no session is
/// named. Empty `windows` with a `read_at` is an account off a subscription: a session reported
/// none. Without one, none has reported yet. Fleet keeps the last one, so a restarted Fleet serves
/// it without the windows already reset.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanUsage {
    pub windows: Vec<PlanWindow>,
    /// When Fleet took the reading. Absent while no session has reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_at: Option<Instant>,
}
