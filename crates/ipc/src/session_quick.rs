//! The replies a Job's quick bar offers that no setting holds. `get_session_quick`.

use serde::{Deserialize, Serialize};

/// `get_session_quick`: lines he might send next. Each list is empty where there is nothing to offer, and
/// where the bar is hidden for the Job or learning is off.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionQuick {
    /// At most three short answers to the agent's last message. Empty while the agent has not spoken last.
    pub asked: Vec<String>,
    /// At most six lines he sends often across every Job, the most useful first.
    pub learnt: Vec<String>,
}
