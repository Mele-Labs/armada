//! `commit_to_repository`: a copy of one piece a person keeps for a repository, committed on the
//! branch of the Job they are in. `docs/concepts/kit.md`, *Commit to repository*.
//!
//! **Committing copies.** The person's own piece stays where it is kept; the Job's branch gains a
//! copy under `.armada/`, so it lands with that Job's pull request.

use serde::{Deserialize, Serialize};

use crate::ids::WorkflowId;
use crate::sessions::SessionId;

/// `commit_to_repository`'s body. `POST /repositories/commit`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitToRepository {
    /// A root `list_repositories` names.
    pub repository: String,
    /// The Job open in the repository, whose branch takes the commit. With none it is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub piece: PieceToCommit,
    /// Replace what the Job's branch already holds under the same id. Refused without it.
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
}

/// `commit_to_repository`'s answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommittedToRepository {
    pub repository: String,
    /// The Job's branch the commit is on.
    pub branch: String,
    /// Each file written, relative to the repository's root.
    pub paths: Vec<String>,
    /// The commit made. Absent where the branch already held the files as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}
