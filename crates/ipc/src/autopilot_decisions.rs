//! What a running Autopilot answered for the person, on the wire: the answers to review, and the
//! two things a person does to one. `docs/practices/protocol/autopilot-decisions.md`.

use serde::{Deserialize, Serialize};

use crate::autopilot::AutopilotDecision;
use crate::{Instant, ManifestId, SessionId};

/// One answer Autopilot gave while the person was away. `chose` is the way it took, in the item's
/// own words, and `because` the helper's reason, in its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutopilotDecided {
    /// `<session>:<item>` for a Job's item, so one item is answered once.
    pub id: String,
    pub manifest_id: ManifestId,
    /// The Job it answered for, or started. A correction is sent to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// The Job's name.
    pub who: String,
    pub kind: AutopilotDecision,
    pub asked: String,
    pub chose: String,
    pub because: String,
    pub at: Instant,
}

/// `get_autopilot_decisions`, what `review_autopilot_decision` and `override_autopilot_decision`
/// answer, and `autopilot.decisions_changed` published whole: every repository's answers not yet
/// reviewed, in the order they were given.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutopilotDecisions {
    pub to_review: Vec<AutopilotDecided>,
}

/// `review_autopilot_decision`: the person kept one answer as it was.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewAutopilotDecision {
    pub id: String,
}

/// `override_autopilot_decision`: the person's correction of one answer, sent to the Job it
/// answered for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverrideAutopilotDecision {
    pub id: String,
    pub text: String,
}
