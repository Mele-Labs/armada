//! `dispute_fleet`: an agent says a notice Fleet sent its job was wrong, and Fleet starts or joins
//! a review job in Armada that settles who was right.

use serde::{Deserialize, Serialize};

use crate::SessionId;

/// `dispute_fleet`: the job's latest Fleet notice was wrong, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputeFleet {
    pub why: String,
    /// Which session. **Read only where the connection places no hosted session**, as `AllDone`'s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// What `dispute_fleet` kept, and the review job it started or joined.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetDisputed {
    pub dispute_id: String,
    /// Absent where none was started: this Fleet does not serve Armada, or the start was refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_session_id: Option<String>,
    /// The review job was already open for the same notice, and was sent this dispute.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub joined: bool,
}

/// How a dispute was spotted: the agent called `dispute_fleet`, or Fleet read the agent's replies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisputeSpotted {
    Called,
    Read,
}

impl DisputeSpotted {
    /// The word the store keeps.
    pub fn as_str(self) -> &'static str {
        match self {
            DisputeSpotted::Called => "called",
            DisputeSpotted::Read => "read",
        }
    }
}
