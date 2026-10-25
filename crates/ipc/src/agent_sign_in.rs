//! The agents' account signed out, for the whole Fleet. `docs/concepts/session/a-job-whose-sign-in-expired.md`.

use serde::{Deserialize, Serialize};

use crate::ids::Instant;

/// `get_agent_sign_in`, what `start_agent_sign_in` answers, and `agent_sign_in.changed` whole. **Every Job
/// shares the account**, so this is one state a Fleet and names no Job. Default is signed in, with no
/// sign-in running.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSignIn {
    /// When the first Job was heard signed out. Absent is signed in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signed_out_since: Option<Instant>,
    /// Fleet is running the sign-in now, waiting on the browser.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub signing_in: bool,
}
