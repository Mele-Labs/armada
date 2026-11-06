//! An error Bridge showed the person, sent to the Operator. `crates/ipc/operations/report_bridge_error.toml`.

use serde::{Deserialize, Serialize};

/// One structured field of the error card, already flattened to a string.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeErrorField {
    pub key: String,
    pub value: String,
}

/// `report_bridge_error`: what the failure card carries, as Bridge drew it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportBridgeError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub fields: Vec<BridgeErrorField>,
    /// The cause chain, outermost cause first.
    #[serde(default)]
    pub chain: Vec<String>,
    /// Paths of the logs the card points at.
    #[serde(default)]
    pub logs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    /// When the card was taken, RFC 3339, on Bridge's clock.
    pub at: String,
}

/// What `report_bridge_error` answers: the arrival's id, and whether the same error was already with
/// the Operator that day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeErrorTaken {
    pub id: String,
    pub repeated: bool,
}
