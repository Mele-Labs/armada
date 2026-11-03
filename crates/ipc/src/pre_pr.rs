//! `check_before_pr`: the Checks a repository keeps for the moment a pull request opens, run on
//! the Job's changes.

use serde::{Deserialize, Serialize};

use crate::sessions::SessionId;

/// `POST /sessions/pre_pr`: the plugin's hook asks before an agent's `gh pr create` runs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckBeforePr {
    /// Which Job. **Read only where the connection places no hosted Job**, as `ShowWindow`'s is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
}

/// One pre-PR Check, run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrePrCheckRan {
    /// The Check's key: its name at the root, `<dir>:<name>` in a workspace.
    pub name: String,
    pub passed: bool,
    /// The last lines it printed, with the command that ran.
    pub tail: String,
}

/// What the pre-PR Checks came to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrePrChecked {
    /// Every Check that ran passed, or none was reached.
    pub passed: bool,
    /// Where it did not pass, what the agent is told: each failing Check and its log tail. Empty
    /// where it passed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub says: String,
    /// The Checks that ran, in the order they ran.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<PrePrCheckRan>,
}
