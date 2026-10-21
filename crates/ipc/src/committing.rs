//! `commit_to_repository`: a copy of one piece a person keeps for a repository, committed by a
//! Job of its own. `docs/concepts/kit.md`, *Commit to repository*.
//!
//! **Committing copies.** The person's own piece stays where it is kept; a new Job's branch gains
//! a copy under `.armada/` (an `armada.yml` at the root), and its pull request carries that alone.

use serde::{Deserialize, Serialize};

use crate::ids::WorkflowId;
use crate::sessions::SessionId;

/// `commit_to_repository`'s body. `POST /repositories/commit`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitToRepository {
    /// A root `list_repositories` names.
    pub repository: String,
    pub piece: PieceToCommit,
    /// Replace what the repository's base already holds under the same id. Refused without it.
    #[serde(default)]
    pub overwrite: bool,
}

/// Which of the person's own pieces is copied, by the id its concept already uses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PieceToCommit {
    /// Tiles of the person's own for this repository, by `id` in each list, copied into
    /// `.armada/flight-controls.toml` beside the tiles the branch already holds.
    FlightControls {
        #[serde(default)]
        job: Vec<String>,
        #[serde(default)]
        deck: Vec<String>,
    },
    /// A workflow from Kit's `~/.armada/workflows/`, copied into `.armada/workflows/`.
    Workflow { workflow_id: WorkflowId },
    /// A review tool from Kit's `~/.armada/review-tools/`, by its dashed id, copied into
    /// `.armada/review-tools/`.
    ReviewTool { id: String },
    /// The person's own `armada.yml` for the repository, from the place their Kit keeps it,
    /// copied to the workspace root.
    Manifest,
}

/// `commit_to_repository`'s answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommittedToRepository {
    pub repository: String,
    /// The Job made for this commit alone.
    pub session_id: SessionId,
    /// The Job's own branch, which holds the one commit.
    pub branch: String,
    /// Each file written, relative to the repository's root.
    pub paths: Vec<String>,
    pub commit: String,
    /// The pull request's address, where one was opened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request: Option<String>,
    /// Why no pull request was opened, where none was. The commit stays on the Job's branch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request_not_opened: Option<String>,
}
