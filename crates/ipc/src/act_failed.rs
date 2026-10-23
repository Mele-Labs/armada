//! `act.failed` — a background act that failed after its request had been answered.
//!
//! **A request that replies first and finishes in a spawned task has nothing to
//! refuse with.** The answer is gone, so the refusal rides the stream instead,
//! carrying the same [`WireError`] a request's refusal does, and Bridge words it
//! the way it words one.

use serde::{Deserialize, Serialize};

use crate::error::WireError;
use crate::ids::ManifestId;
use crate::sessions::SessionId;

/// The acts that finish after their reply. **Strict**: a spelling Bridge does not know is refused, not guessed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundAct {
    /// Send checkout as a pull request.
    CheckoutToPullRequest,
    /// Commit a piece to the repository.
    CommitToRepository,
    /// A press on a pull request: Ready, Merge or Auto-merge.
    PullRequestPress,
}

/// `act.failed`: what failed, for which repository or Job, and the refusal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActFailed {
    pub act: BackgroundAct,
    /// The repository the act was for, where it had one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<ManifestId>,
    /// The Job the act was for, where it had one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub error: WireError,
}
